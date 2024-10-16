use anyhow::{bail, Result};
use geo::{point, Point};
use log::debug;
use logging_timer::time;
use time::{Duration, OffsetDateTime};

use crate::{
    model::{
        Email, EnrichedGpx, EnrichedTrackPoint, Gpx, Lat, Link, Lon, Metadata, Waypoint,
        XmlDeclaration,
    },
    stage::{distance_between_points_metres, speed_kmh_from_duration},
};

impl Default for Gpx {
    /// Creates a new Gpx with 'gapix' as the creator.
    fn default() -> Self {
        let mut gpx = Self::new(XmlDeclaration::default(), Metadata::default());
        gpx.creator = "gapix".into();
        gpx
    }
}

impl Gpx {
    /// Creates a new Gpx from the mandatory fields.
    pub fn new(declaration: XmlDeclaration, metadata: Metadata) -> Self {
        Self {
            declaration,
            filename: Default::default(),
            version: Default::default(),
            creator: Default::default(),
            attributes: Default::default(),
            metadata,
            waypoints: Default::default(),
            routes: Default::default(),
            tracks: Default::default(),
        }
    }

    /// Returns the total number of points across all tracks and segments.
    pub fn num_points(&self) -> usize {
        self.tracks
            .iter()
            .map(|track| {
                track
                    .segments
                    .iter()
                    .map(|segment| segment.points.len())
                    .sum::<usize>()
            })
            .sum()
    }

    /// Returns true if the GPX consists of a single track with one segment.
    pub fn is_single_track(&self) -> bool {
        self.tracks.len() == 1 && self.tracks[0].segments.len() == 1
    }

    /// Merges all the tracks and segments within the GPX into a single track
    /// with one segment containing all the points. The name and type of the
    /// first track in `self` is used to name the new track. If the GPX is
    /// already in single track form then self is simply returned as-is (this is
    /// a cheap operation in that case).
    pub fn into_single_track(mut self) -> Self {
        if self.is_single_track() {
            return self;
        }

        let mut points = Vec::with_capacity(self.num_points());

        // This copies the first track as well, which may seem a bit inefficient,
        // but the obvious optimisation of moving all but the first track doesn't
        // work because that track may have multiple segments. This function is
        // only called once and the simpler code wins out over the fix for that
        // problem.
        let mut track_count = 0;
        let mut segment_count = 0;
        let mut point_count = 0;

        for src_track in self.tracks.iter_mut() {
            track_count += 1;

            for src_segment in src_track.segments.iter_mut() {
                segment_count += 1;
                point_count += src_segment.points.len();
                points.append(&mut src_segment.points);
            }
        }

        self.tracks.truncate(1);
        self.tracks.shrink_to_fit();

        debug!(
            "Merged {} tracks with {} segments and {} points into a single track",
            track_count, segment_count, point_count,
        );

        self
    }

    /// Makes an EnrichedGpx from the Gpx. Each of the new trackpoints will have
    /// derived data calculated where possible. An error is returned if the Gpx
    /// is not in single-track form.
    pub fn to_enriched_gpx(&self) -> Result<EnrichedGpx> {
        if !self.is_single_track() {
            bail!("GPX must be in single track form before converting to Enriched format. See method into_single_track().");
        }

        let mut egpx = EnrichedGpx {
            filename: self.filename.clone(),
            declaration: self.declaration.clone(),
            metadata: self.metadata.clone(),
            track_name: self.tracks[0].name.clone(),
            track_type: self.tracks[0].r#type.clone(),
            points: self.tracks[0].segments[0]
                .points
                .iter()
                .enumerate()
                .map(|(idx, tp)| EnrichedTrackPoint::new(idx, tp))
                .collect(),
            version: self.version.clone(),
            creator: self.creator.clone(),
            attributes: self.attributes.clone(),
        };

        egpx.enrich_trackpoints();

        Ok(egpx)
    }
}

impl Default for XmlDeclaration {
    fn default() -> Self {
        Self {
            version: "1.0".to_owned(),
            encoding: Some("UTF-8".to_owned()),
            standalone: Default::default(),
        }
    }
}

impl Waypoint {
    pub fn with_lat_lon(lat: Lat, lon: Lon) -> Self {
        let mut v = Self::default();
        v.lat = lat;
        v.lon = lon;
        v
    }
}

impl Email {
    /// Constructs a new email element from the two mandatory fields, id and
    /// domain.
    pub fn new<S1, S2>(id: S1, domain: S2) -> Self
    where
        S1: Into<String>,
        S2: Into<String>,
    {
        Self {
            id: id.into(),
            domain: domain.into(),
        }
    }
}

impl Link {
    pub fn new<S>(href: S) -> Self
    where
        S: Into<String>,
    {
        Self {
            text: None,
            r#type: None,
            href: href.into(),
        }
    }
}

impl EnrichedGpx {
    /// Returns the last valid index in the points array.
    /// Just a convenience fn to avoid off-by-one errors (hopefully).
    pub fn last_valid_idx(&self) -> usize {
        self.points.len() - 1
    }

    /// Returns the average temperature across the entire track.
    pub fn avg_temperature(&self) -> Option<f64> {
        let sum: f64 = self
            .points
            .iter()
            .flat_map(|p| p.extensions.as_ref())
            .flat_map(|ext| ext.air_temp)
            .sum();

        if sum == 0.0 {
            None
        } else {
            Some(sum / self.points.len() as f64)
        }
    }

    /// Returns the average heart rate across the entire track.
    pub fn avg_heart_rate(&self) -> Option<f64> {
        let sum: f64 = self
            .points
            .iter()
            .flat_map(|p| p.extensions.as_ref())
            .flat_map(|ext| ext.heart_rate.map(|hr| hr as f64))
            .sum();

        if sum == 0.0 {
            None
        } else {
            Some(sum / self.points.len() as f64)
        }
    }

    /// Calculate a set of enriched TrackPoint information (distances, speed, climb).
    #[time]
    fn enrich_trackpoints(&mut self) {
        let start_time = self.points[0].time;
        let mut cum_ascent_metres = None;
        let mut cum_descent_metres = None;

        let mut p1 = self.points[0].as_geo_point();

        // If we have time and elevation, fill in the first point with some starting
        // values. There are quite a few calculations that rely on these values
        // being set (mainly 'running' data). The calculations will return None when
        // we don't know the data.
        if self.points[0].time.is_some() {
            self.points[0].delta_time = Some(Duration::ZERO);
            self.points[0].running_delta_time = Some(Duration::ZERO);
            self.points[0].speed_kmh = Some(0.0);
        }
        if self.points[0].ele.is_some() {
            self.points[0].ele_delta_metres = Some(0.0);
            self.points[0].running_ascent_metres = Some(0.0);
            self.points[0].running_descent_metres = Some(0.0);
            cum_ascent_metres = Some(0.0);
            cum_descent_metres = Some(0.0);
        }

        // Note we are iterating all points EXCEPT the first one.
        for idx in 1..self.points.len() {
            let p2 = self.points[idx].as_geo_point();
            self.points[idx].delta_metres = distance_between_points_metres(p1, p2);
            assert!(self.points[idx].delta_metres >= 0.0);

            self.points[idx].running_metres =
                self.points[idx - 1].running_metres + self.points[idx].delta_metres;
            assert!(self.points[idx].running_metres >= 0.0);

            // Time delta. Don't really need this stored, but is handy to spot
            // points that took more than usual when scanning the CSV.
            self.points[idx].delta_time = match (self.points[idx].time, self.points[idx - 1].time) {
                (Some(t1), Some(t2)) => {
                    let dt = t1 - t2;
                    assert!(dt.is_positive());
                    Some(dt)
                }
                _ => None,
            };

            // Speed. Based on the distance we just calculated.
            self.points[idx].speed_kmh = match self.points[idx].delta_time {
                Some(t) => {
                    let speed = speed_kmh_from_duration(self.points[idx].delta_metres, t);
                    assert!(speed >= 0.0);
                    Some(speed)
                }
                None => todo!(),
            };

            // How long it took to get here.
            self.points[idx].running_delta_time = match (self.points[idx].time, start_time) {
                (Some(t1), Some(t2)) => {
                    let dt = t1 - t2;
                    assert!(dt.is_positive());
                    Some(dt)
                }
                _ => None,
            };

            // Ascent and descent.
            let ele_delta_metres = match (self.points[idx].ele, self.points[idx - 1].ele) {
                (Some(ele1), Some(ele2)) => Some(ele1 - ele2),
                _ => None,
            };

            self.points[idx].ele_delta_metres = ele_delta_metres;

            if let Some(edm) = ele_delta_metres {
                if edm > 0.0 {
                    let cam = cum_ascent_metres.unwrap_or_default() + edm;
                    assert!(cam >= 0.0);
                    cum_ascent_metres = Some(cam);
                } else {
                    let cdm = cum_descent_metres.unwrap_or_default() + edm.abs();
                    assert!(cdm >= 0.0);
                    cum_descent_metres = Some(cdm);
                }
            }

            self.points[idx].running_ascent_metres = cum_ascent_metres;
            self.points[idx].running_descent_metres = cum_descent_metres;

            p1 = p2;
        }
    }
}

impl EnrichedTrackPoint {
    fn new(index: usize, value: &Waypoint) -> Self {
        Self {
            index,
            lat: value.lat,
            lon: value.lon,
            ele: value.ele,
            time: value.time,
            extensions: value.tp_extensions.clone(),
            delta_time: None,
            delta_metres: 0.0,
            running_metres: 0.0,
            speed_kmh: None,
            running_delta_time: None,
            ele_delta_metres: None,
            running_ascent_metres: None,
            running_descent_metres: None,
            location: Default::default(),
        }
    }

    /// The start time of the TrackPoint. TrackPoints are written after
    /// a period of time has expired. Most trackpoint are written at 1
    /// second intervals, but when you are stopped it can be a long time,
    /// say 20 minutes, before the trackpoint is written. So a TrackPoint
    /// may have a time of 14:40, and the previous TrackPoint has a time
    /// of 14:20, giving a delta_time of 20 minutes.
    ///
    /// Note that we can't work out the start time for the first point
    /// since it has no delta_time.
    ///
    /// It is important to use start_time() when calculating things like
    /// durations of stages.
    pub fn start_time(&self) -> Option<OffsetDateTime> {
        if self.index == 0 {
            return self.time;
        }

        match (self.time, self.delta_time) {
            (Some(t), Some(dt)) => Some(t - dt),
            _ => None,
        }
    }

    /// Makes a geo-Point based on the lat-lon coordinates of this point.
    /// n.b. x=lon, y=lat. If you do it the other way round the
    /// distances are wrong - a lot wrong.
    pub fn as_geo_point(&self) -> Point {
        point! { x: self.lon, y: self.lat }
    }

    /// Convenience function to extract the air_temp from
    /// the Garmin extensions.
    pub fn air_temp(&self) -> Option<f64> {
        self.extensions.as_ref().and_then(|ext| ext.air_temp)
    }

    /// Convenience function to extract the heart_rate from
    /// the Garmin extensions.
    pub fn heart_rate(&self) -> Option<u8> {
        self.extensions.as_ref().and_then(|ext| ext.heart_rate)
    }

    /// Convenience function to extract the air_temp from
    /// the Garmin extensions.
    pub fn cadence(&self) -> Option<u8> {
        self.extensions.as_ref().and_then(|ext| ext.cadence)
    }
}
