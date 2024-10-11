use std::{collections::HashMap, path::PathBuf};

use geo::{point, Point};
use log::debug;
use time::{Duration, OffsetDateTime};

/// Data parsed from a GPX file, based on the XSD description at
/// https://www.topografix.com/GPX/1/1/gpx.xsd
#[derive(Debug, Clone)]
pub struct Gpx {
    pub filename: PathBuf,
    pub declaration: Declaration,
    pub info: GpxInfo,
    pub metadata: Metadata,
    pub tracks: Vec<Track>,
    // TODO: There can also be a list of waypoints and/or routes.
}

/// Represents the 'xml' declaration tag - the first line of an XML file.
#[derive(Debug, Clone)]
pub struct Declaration {
    pub version: String,
    pub encoding: Option<String>,
    pub standalone: Option<String>,
}

/// Represents the 'gpx' tag, which is the main container element for the entire
/// file.
#[derive(Debug, Clone)]
pub struct GpxInfo {
    /// The 'creator' attribute.
    pub creator: String,
    /// The 'version' attribute. This should always be "1.1".
    pub version: String,
    /// The other attributes (excluding creator and version, which
    /// are mandatory.)
    pub attributes: HashMap<String, String>,
}

/// TODO: Parse all fields.
#[derive(Debug, Clone)]
pub struct Metadata {
    pub link: Link,
    pub time: Option<OffsetDateTime>,
    pub desc: Option<String>,
}

/// Data parsed from a <link> tag.
/// This is all the fields per the XSD.
#[derive(Debug, Clone)]
pub struct Link {
    /// URL of hyperlink
    pub href: String,
    /// Text of hyperlink
    pub text: Option<String>,
    /// Mime type of content (image/jpeg)
    pub r#type: Option<String>,
}

/// TODO: Parse all fields.
#[derive(Debug, Clone)]
pub struct Track {
    pub name: Option<String>,
    pub r#type: Option<String>,
    pub desc: Option<String>,
    pub segments: Vec<TrackSegment>,
}

#[derive(Debug, Clone)]
pub struct TrackSegment {
    pub points: Vec<TrackPoint>,
}

#[derive(Debug, Clone)]
pub struct TrackPoint {
    pub lat: f64,
    pub lon: f64,
    pub ele: Option<f64>,
    pub time: Option<OffsetDateTime>,
    pub extensions: Option<Extensions>,
}

/// All the Garmin TrackPoint extensions according to
/// https://www8.garmin.com/xmlschemas/TrackPointExtensionv1.xsd
#[derive(Debug, Clone)]
pub struct Extensions {
    pub air_temp: Option<f64>,
    pub water_temp: Option<f64>,
    pub depth: Option<f64>,
    pub heart_rate: Option<u16>,
    pub cadence: Option<u16>,
}

impl Gpx {
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

    /// Merges all the tracks and segments within the GPX into
    /// a new structure that has one track with one segment containing
    /// all the points.
    /// The name and type of the first track in `self` is used
    /// to name the new track.
    pub fn into_single_track(mut self) -> Gpx {
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

        for idx in (1..self.tracks.len() - 1).rev() {
            self.tracks.remove(idx);
        }

        debug!(
            "Merged {} tracks with {} segments and {} points into a single track",
            track_count, segment_count, point_count,
        );

        self
    }
}

/// An EnrichedGpx is one where we flatten the Tracks and Segments into a
/// simple vector of EnrichedTrackPoints. These are TrackPoints with a lot
/// of derived data fields that make later work easier.
#[derive(Debug)]
pub struct EnrichedGpx {
    pub filename: PathBuf,
    pub declaration: Declaration,
    pub info: GpxInfo,
    pub metadata: Metadata,
    pub track_name: Option<String>,
    pub track_type: Option<String>,
    pub points: Vec<EnrichedTrackPoint>,
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
}

/// A TrackPoint with lots of extra stuff calculated. We need the extras
/// to find the stages.
#[derive(Debug, Clone)]
pub struct EnrichedTrackPoint {
    /// The index of the original trackpoint we used to create this value.
    pub index: usize,
    /// The latitude, read from the "lat" attribute.
    pub lat: f64,
    /// The longitude, read from the "lon" attribute.
    pub lon: f64,
    /// The elevation, as read from the <ele> tag.
    pub ele: Option<f64>,
    /// The time as read from the <time> tag.
    pub time: Option<OffsetDateTime>,
    /// The Garmin TrackPoint extensions.
    pub extensions: Option<Extensions>,

    // All the below fields are the 'enriched' ones.
    /// The amount of time between this trackpoint and the previous one.
    pub delta_time: Option<Duration>,
    /// The distance between this trackpoint and the previous one.
    pub delta_metres: f64,
    /// The distance to this trackpoint from the beginning of the track.
    pub running_metres: f64,
    /// The instantaneous speed at this point.
    pub speed_kmh: Option<f64>,
    /// The elapsed time between the beginning of the track and this point.
    pub running_delta_time: Option<Duration>,
    /// The change in elevation between this trackpoint and the previous one.
    pub ele_delta_metres: Option<f64>,
    /// The running ascent between the beginning of the track and this point.
    pub running_ascent_metres: Option<f64>,
    /// The running descent between the beginning of the track and this point.
    pub running_descent_metres: Option<f64>,
    /// The location (reverse geo-coded based on lat-lon)
    pub location: Option<String>,
}

impl EnrichedTrackPoint {
    fn new(index: usize, value: &TrackPoint) -> Self {
        Self {
            index,
            lat: value.lat,
            lon: value.lon,
            ele: value.ele,
            time: value.time,
            extensions: value.extensions.clone(),
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
    pub fn heart_rate(&self) -> Option<u16> {
        self.extensions.as_ref().and_then(|ext| ext.heart_rate)
    }

    /// Convenience function to extract the air_temp from
    /// the Garmin extensions.
    pub fn cadence(&self) -> Option<u16> {
        self.extensions.as_ref().and_then(|ext| ext.cadence)
    }
}

impl From<Gpx> for EnrichedGpx {
    fn from(value: Gpx) -> Self {
        let value = value.into_single_track();

        Self {
            filename: value.filename,
            declaration: value.declaration,
            info: value.info,
            metadata: value.metadata,
            track_name: value.tracks[0].name.clone(),
            track_type: value.tracks[0].r#type.clone(),
            points: value.tracks[0].segments[0]
                .points
                .iter()
                .enumerate()
                .map(|(idx, tp)| EnrichedTrackPoint::new(idx, tp))
                .collect(),
        }
    }
}
