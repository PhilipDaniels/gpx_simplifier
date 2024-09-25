use std::path::PathBuf;

use serde::Deserialize;
use time::{Duration, OffsetDateTime};

#[derive(Debug, Deserialize)]
pub struct Gpx {
    #[serde(skip)]
    pub filename: PathBuf,

    pub metadata: Metadata,

    #[serde(rename = "trk")]
    pub tracks: Vec<Track>,
}

/// Represents the <metadata> node from the header.
#[derive(Debug, Clone, Deserialize)]
pub struct Metadata {
    #[serde(with = "time::serde::rfc3339")]
    pub time: OffsetDateTime,
}

/// Represents a single <trk>
#[derive(Debug, Deserialize)]
pub struct Track {
    pub name: String,
    pub r#type: String,
    #[serde(rename = "trkseg")]
    pub segments: Vec<TrackSegment>,
}

/// Represents a single <trkseg>
#[derive(Debug, Deserialize)]
pub struct TrackSegment {
    #[serde(rename = "trkpt")]
    pub points: Vec<TrackPoint>,
}

/// Represents a single <trkpt>.
#[derive(Debug, Clone, Deserialize)]
pub struct TrackPoint {
    /// The latitude, read from the "lat" attribute.
    #[serde(rename = "@lat")]
    pub lat: f64,
    /// The longitude, read from the "lon" attribute.
    #[serde(rename = "@lon")]
    pub lon: f64,
    /// The elevation, as read from the <ele> tag.
    pub ele: f64,
    /// Represents the time as read from the <time> tag.
    /// Serde handles the parsing.
    #[serde(with = "time::serde::rfc3339")]
    pub time: OffsetDateTime,
}

impl Gpx {
    /// Merges all the tracks and segments within the GPX into
    /// a new structure that just has one set of points.
    /// The name and type of the first track in `self` is used
    /// to name the new track.
    pub fn merge_all_tracks(&self) -> MergedGpx {
        let mut result = MergedGpx {
            filename: self.filename.clone(),
            metadata_time: self.metadata.time.clone(),
            track_name: self.tracks[0].name.clone(),
            track_type: self.tracks[0].r#type.clone(),
            points: Vec::new(),
        };

        for src_track in &self.tracks {
            for src_segment in &src_track.segments {
                for src_point in &src_segment.points {
                    result.points.push(src_point.clone().into());
                }
            }
        }

        result
    }
}

/// Represents the result of merging several GPX files
/// into a single file.
#[derive(Clone)]
pub struct MergedGpx {
    pub filename: PathBuf,
    pub metadata_time: OffsetDateTime,
    pub track_name: String,
    pub track_type: String,
    pub points: Vec<TrackPoint>,
}

#[derive(Debug)]
pub struct EnrichedGpx {
    pub filename: PathBuf,
    pub metadata_time: OffsetDateTime,
    pub track_name: String,
    pub track_type: String,
    pub points: Vec<EnrichedTrackPoint>,
}

/// A TrackPoint with lots of extra stuff calculated. We need the extras
/// to find the stages.
#[derive(Debug)]
pub struct EnrichedTrackPoint {
    /// The index of the original trackpoint we used to create this value.
    pub index: usize,
    /// The latitude, read from the "lat" attribute.
    pub lat: f64,
    /// The longitude, read from the "lon" attribute.
    pub lon: f64,
    /// The elevation, as read from the <ele> tag.
    pub ele: f64,
    /// The time as read from the <time> tag.
    pub time: OffsetDateTime,
    /// The amount of time between this trackpoint and the previous one.
    pub delta_time: Duration,
    /// The distance between this trackpoint and the previous one.
    pub delta_metres: f64,
    /// The distance to this trackpoint from the beginning of the track.
    pub running_metres: f64,
    /// The instantaneous speed at this point.
    pub speed_kmh: f64,
    /// The elapsed time between the beginning of the track and this point.
    pub running_delta_time: Duration,
    /// The change in elevation between this trackpoint and the previous one.
    pub ele_delta_metres: f64,
    /// The running ascent between the beginning of the track and this point.
    pub running_ascent_metres: f64,
    /// The running descent between the beginning of the track and this point.
    pub running_descent_metres: f64,
    /// The location (reverse geo-coded based on lat-lon)
    pub location: Option<String>,
}

impl EnrichedTrackPoint {
    fn new(index: usize, value: TrackPoint) -> Self {
        Self {
            index,
            lat: value.lat,
            lon: value.lon,
            ele: value.ele,
            time: value.time,
            delta_time: Duration::ZERO,
            delta_metres: 0.0,
            running_metres: 0.0,
            speed_kmh: 0.0,
            running_delta_time: Duration::ZERO,
            ele_delta_metres: 0.0,
            running_ascent_metres: 0.0,
            running_descent_metres: 0.0,
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
    /// It is important to use start_time() when calculating things like
    /// durations of stages.
    pub fn start_time(&self) -> OffsetDateTime {
        self.time - self.delta_time
    }
}

impl From<MergedGpx> for EnrichedGpx {
    fn from(value: MergedGpx) -> Self {
        Self {
            filename: value.filename,
            metadata_time: value.metadata_time,
            track_name: value.track_name,
            track_type: value.track_type,
            points: value
                .points
                .into_iter()
                .enumerate()
                .map(|(idx, tp)| EnrichedTrackPoint::new(idx, tp))
                .collect(),
        }
    }
}
