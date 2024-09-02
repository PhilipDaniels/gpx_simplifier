use std::path::PathBuf;

use serde::Deserialize;

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
    pub time: String,
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
    pub lat: f32,
    /// The longitude, read from the "lon" attribute.
    #[serde(rename = "@lon")]
    pub lon: f32,
    /// The elevation, as read from the <ele> tag.
    pub ele: f32,
    /// Represents the time as read from the <time> tag. We don't
    /// do any "time processing" on the time, so just a string is
    /// good enough.
    pub time: String,
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
                    result.points.push(src_point.clone());
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
    pub metadata_time: String,
    pub track_name: String,
    pub track_type: String,
    pub points: Vec<TrackPoint>,
}

/// Represents a stop as detected in a GPX track.
/// A stop is a point where you were at speed 0
/// for some time.
pub struct Stop {

}