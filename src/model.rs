use std::path::PathBuf;

use serde::Deserialize;
use time::OffsetDateTime;

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
    pub lat: f32,
    /// The longitude, read from the "lon" attribute.
    #[serde(rename = "@lon")]
    pub lon: f32,
    /// The elevation, as read from the <ele> tag.
    pub ele: f32,
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

impl MergedGpx {
    // pub fn start_time(&self) -> OffsetDateTime {
    //     self.points[0].time
    // }

    // pub fn end_time(&self) -> OffsetDateTime {
    //     self.last_point().time
    // }

    // pub fn total_time(&self) -> Duration {
    //     self.end_time() - self.start_time()
    // }

    // pub fn distance_metres(&self) -> f32 {
    //     self.last_point().cumulative_distance_metres
    // }

    // pub fn distance_km(&self) -> f32 {
    //     self.distance_metres() / 1000.0
    // }

    // fn last_point(&self) -> &TrackPoint {
    //     &self.points[self.points.len() - 1]
    // }

    // pub fn min_elevation(&self) -> &TrackPoint {
    //     let mut min = &self.points[0];

    //     for p in &self.points {
    //         if p.ele < min.ele {
    //             min = &p
    //         }
    //     }
        
    //     min
    // }

    // pub fn max_elevation(&self) -> &TrackPoint {
    //     let mut max = &self.points[0];

    //     for p in &self.points {
    //         if p.ele > max.ele {
    //             max = &p
    //         }
    //     }
        
    //     max
    // }

    // pub fn total_ascent_metres(&self) -> f32 {
    //     self.last_point().cumulative_ascent_metres
    // }

    // pub fn total_descent_metres(&self) -> f32 {
    //     self.last_point().cumulative_descent_metres
    // }
}