use std::{collections::HashMap, path::PathBuf};

use anyhow::{bail, Result};
use geo::{point, Point};
use log::debug;
use logging_timer::time;
use time::{Duration, OffsetDateTime};

use crate::stage::{distance_between_points_metres, speed_kmh_from_duration};

/// Data parsed from a GPX file, based on the XSD description at
/// https://www.topografix.com/GPX/1/1/gpx.xsd
#[derive(Debug, Clone)]
pub struct GpxFile {
    /// The filename field is not part of the XSD, but it is convenient to have
    /// it so it can be used as an identifier for the GPX data.
    pub filename: Option<PathBuf>,
    /// Represents the 'xml' declaration tag - the first line of an XML file.
    pub declaration: XmlDeclaration,
    /// Represents the 'gpx element, which is the main container element for the entire
    /// file.
    pub gpx: Gpx,
    /// Metadata about the file.
    pub metadata: Metadata,
    /// A list of waypoints.
    pub waypoints: Vec<Waypoint>,
    /// A list of routes.
    pub routes: Vec<Route>,
    /// A list of tracks.
    pub tracks: Vec<Track>,
}

/// Represents the 'xml' declaration - the first line of an XML file (not just
/// GPX files).
#[derive(Debug, Clone)]
pub struct XmlDeclaration {
    pub version: String,
    pub encoding: Option<String>,
    pub standalone: Option<String>,
}

/// Represents the 'gpx' element, which is the main container element for the entire
/// file.
#[derive(Debug, Clone)]
pub struct Gpx {
    /// The 'version' attribute. This should always be "1.1".
    pub version: String,
    /// The 'creator' attribute.
    pub creator: String,
    /// The other attributes (excluding creator and version, which
    /// are mandatory.)
    pub attributes: HashMap<String, String>,
}

/// The metadata element contains information about the GPX file, such as
/// author, and copyright restrictions.
#[derive(Debug, Clone, Default)]
pub struct Metadata {
    /// The name of the GPX file.
    pub name: Option<String>,
    /// A description of the GPX file.
    pub desc: Option<String>,
    /// The person or organization who created the GPX file.
    pub author: Option<Person>,
    /// Copyright and license information governing use of the file.
    pub copyright: Option<Copyright>,
    /// Zero or more URLs associated with the file.
    pub links: Vec<Link>,
    /// The creation date of the file.
    pub time: Option<OffsetDateTime>,
    /// Keywords associated with the file.
    pub keywords: Option<String>,
    /// Minimum and maximum coordinates which describe the extent of the
    /// coordinates in the file.
    pub bounds: Option<Bounds>,
    /// Arbitrary extended information. Represented as an unparsed string.
    pub extensions: Option<String>,
}

// TODO:
pub type Lat = f64; // -90..90
pub type Lon = f64; // -180..180

/// A pair of (lat, lon) coordinates which constitute a bounding box.
#[derive(Debug, Clone)]
pub struct Bounds {
    /// The minimum latitude.
    pub min_lat: Lat,
    /// The minimum longitude.
    pub min_lon: Lon,
    /// The maximum latitude.
    pub max_lat: Lat,
    /// The maximum longitude.
    pub max_lon: Lon,
}

/// Information about the copyright holder and any license governing use of this
/// file. By linking to an appropriate license, you may place your data into the
/// public domain or grant additional usage rights.
#[derive(Debug, Clone)]
pub struct Copyright {
    /// The year of copyright.
    pub year: i16,
    /// A link to an external resource containing the licence text.
    pub license: Option<String>,
    /// The author/holder of the copyright.
    pub author: String,
}

/// Represents the 'personType' from the XSD. This can be a person or an
/// organisation.
#[derive(Debug, Clone, Default)]
pub struct Person {
    /// Name of person or organization.
    pub name: Option<String>,
    /// Email address.
    pub email: Option<Email>,
    /// Link to Web site or other external information about person.
    pub link: Option<Link>,
}

/// Represents the 'emailType' from the XSD.
#[derive(Debug, Clone)]
pub struct Email {
    /// The first part of the email address (before the '@').
    pub id: String,
    /// The domain half of the email address (e.g. gmail.com).
    pub domain: String,
}

/// Represents the 'linkType' from the XSD. A link to an external resource (Web
/// page, digital photo, video clip, etc.) with additional information.
#[derive(Debug, Clone)]
pub struct Link {
    /// Text of hyperlink
    pub text: Option<String>,
    /// Mime type of content (image/jpeg)
    pub r#type: Option<String>,
    /// URL of hyperlink
    pub href: String,
}

/// A Route is an ordered list of waypoints representing a series of turn points
/// leading to a destination.
#[derive(Debug, Clone, Default)]
pub struct Route {
    /// GPS name of the route.
    pub name: Option<String>,
    /// GPS comment for the route.
    pub cmt: Option<String>,
    /// User description of the route.
    pub desc: Option<String>,
    /// Source of data. Included to give user some idea of reliability and accuracy of data.
    pub src: Option<String>,
    /// Zero or more URLs associated with the route.
    pub links: Vec<Link>,
    /// GPS route number.
    pub number: Option<u16>,
    /// Type (classification) of the track.
    pub r#type: Option<String>,
    /// Arbitrary extended information. Represented as an unparsed string.
    pub extensions: Option<String>,
    /// The list of points in the route.
    pub points: Vec<Waypoint>,
}

/// A Track is an ordered list of points describing a path.
#[derive(Debug, Clone, Default)]
pub struct Track {
    /// GPS name of the track.
    pub name: Option<String>,
    /// GPS comment for the track.
    pub comment: Option<String>,
    /// User description of the track.
    pub desc: Option<String>,
    /// Source of data. Included to give user some idea of reliability and accuracy of data.
    pub src: Option<String>,
    /// Zero or more URLs associated with the track.
    pub links: Vec<Link>,
    /// GPS track number.
    pub number: Option<u32>,
    /// Type (classification) of the track.
    pub r#type: Option<String>,
    /// Arbitrary extended information. Represented as an unparsed string.
    pub extensions: Option<String>,
    /// List of segments in the track. A Track Segment holds a list of Track
    /// Points which are logically connected in order. To represent a single GPS
    /// track where GPS reception was lost, or the GPS receiver was turned off,
    /// start a new Track Segment for each continuous span of track data.
    pub segments: Vec<TrackSegment>,
}

/// A Track Segment holds a list of Track Points which are logically connected
/// in order. To represent a single GPS track where GPS reception was lost, or
/// the GPS receiver was turned off, start a new Track Segment for each
/// continuous span of track data.
#[derive(Debug, Clone, Default)]
pub struct TrackSegment {
    /// The set of points in the segment.
    pub points: Vec<Waypoint>,
    /// Arbitrary extended information.
    pub extensions: Option<String>,
}

pub type Degrees = f64;
pub type DGPSStationType = u16;

/// Represents a waypoint, a point of interest, a named feature on a map or a
/// point within a track. In the case of a trackpoint, very few fields are
/// likely to be filled in by typical GPS units.
#[derive(Debug, Clone, Default)]
pub struct Waypoint {
    /// Elevation (in meters) of the point.
    pub ele: Option<f64>,
    /// Creation/modification timestamp for the waypoint. Date and time in are in
    /// Univeral Coordinated Time (UTC), not local time! Conforms to ISO 8601
    /// specification for date/time representation. Fractional seconds are
    /// allowed for millisecond timing in tracklogs.
    pub time: Option<OffsetDateTime>,
    /// Magnetic variation (in degrees) at the point
    pub magvar: Option<Degrees>,
    /// Height (in meters) of geoid (mean sea level) above WGS84 earth
    /// ellipsoid. As defined in NMEA GGA message.
    pub geoidheight: Option<f64>,
    /// The GPS name of the waypoint. This field will be transferred to and from
    /// the GPS. GPX does not place restrictions on the length of this field or
    /// the characters contained in it. It is up to the receiving application to
    /// validate the field before sending it to the GPS.
    pub name: Option<String>,
    /// GPS waypoint comment. Sent to GPS as comment.
    pub cmt: Option<String>,
    /// A text description of the element. Holds additional information about
    /// the element intended for the user, not the GPS.
    pub desc: Option<String>,
    /// Source of data. Included to give user some idea of reliability and
    /// accuracy of data.
    pub src: Option<String>,
    /// Links to additional information about the waypoint.
    pub links: Vec<Link>,
    /// Text of GPS symbol name.
    pub sym: Option<String>,
    /// Type (classification) of the waypoint.
    pub r#type: Option<String>,
    /// Type of GPX fix.
    pub fix: Option<FixType>,
    /// Number of satellites used to calculate the GPX fix.
    pub sat: Option<u16>,
    /// Horizontal dilution of precision.
    pub hdop: Option<f64>,
    /// Vertical dilution of precision.
    pub vdop: Option<f64>,
    /// Position dilution of precision.
    pub pdop: Option<f64>,
    /// Number of seconds since last DGPS update.
    pub age_of_dgps_data: Option<f64>,
    /// ID of DGPS station used in differential correction.
    pub dgps_id: Option<DGPSStationType>,
    /// The latitude of the point. This is always in decimal degrees, and always
    /// in WGS84 datum.
    pub lat: Lat,
    /// The longitude of the point. This is always in decimal degrees, and
    /// always in WGS84 datum.
    pub lon: Lon,
    /// Extended Garmin trackpoint information.
    pub tp_extensions: Option<GarminTrackpointExtensions>,
    /// Arbitrary extended information. Represented as an unparsed string.
    /// Garmin-specific trackpoint extensions as described at
    /// https://www8.garmin.com/xmlschemas/TrackPointExtensionv1.xsd are parsed
    /// into a separate field.
    pub extensions: Option<String>,
}

/// Type of GPS fix. none means GPS had no fix. To signify "the fix info is
/// unknown", leave out fixType entirely.
#[derive(Debug, Clone)]
pub enum FixType {
    None,
    TwoD,
    ThreeD,
    DGPS,
    /// Indicates a military signal was used
    PPS,
}

/// All the Garmin TrackPoint extensions according to
/// https://www8.garmin.com/xmlschemas/TrackPointExtensionv1.xsd
#[derive(Debug, Clone, Default)]
pub struct GarminTrackpointExtensions {
    /// Air temperature.
    pub air_temp: Option<f64>,
    /// Water temperature.
    pub water_temp: Option<f64>,
    /// Water depth.
    pub depth: Option<f64>,
    /// Heart rate in beats per minute. 1..=255.
    pub heart_rate: Option<u8>,
    /// Cadence in rpm. 0..=254.
    pub cadence: Option<u8>,
    /// Arbitrary extended information. Represented as an unparsed string.
    pub extensions: Option<String>,
}

impl GpxFile {
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

    /// Merges all the tracks and segments within the GPX into a new structure
    /// that has one track with one segment containing all the points. The name
    /// and type of the first track in `self` is used to name the new track.
    /// If the GPX is already in single track form then self is simply returned
    /// as-is (this is a cheap operation in that case).
    pub fn into_single_track(mut self) -> GpxFile {
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
            info: self.gpx.clone(),
            metadata: self.metadata.clone(),
            track_name: self.tracks[0].name.clone(),
            track_type: self.tracks[0].r#type.clone(),
            points: self.tracks[0].segments[0]
                .points
                .iter()
                .enumerate()
                .map(|(idx, tp)| EnrichedTrackPoint::new(idx, tp))
                .collect(),
        };

        egpx.enrich_trackpoints();

        Ok(egpx)
    }
}

/// An EnrichedGpx is one where we flatten the Tracks and Segments into a
/// simple vector of EnrichedTrackPoints. These are TrackPoints with a lot
/// of derived data fields that make later work easier.
#[derive(Debug)]
pub struct EnrichedGpx {
    pub filename: Option<PathBuf>,
    pub declaration: XmlDeclaration,
    pub info: Gpx,
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
    pub extensions: Option<GarminTrackpointExtensions>,

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
