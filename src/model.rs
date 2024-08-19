use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Gpx {
    #[serde(rename = "@creator")]
    pub creator: String,

    #[serde(rename = "@version")]
    pub version: String,

    #[serde(rename = "@xmlns:ns3")]
    pub xmlns_ns3: String,

    #[serde(rename = "@xmlns")]
    pub xmlns: String,

    #[serde(rename = "@xmlns:xsi")]
    pub xmlns_xsi: String,

    #[serde(rename = "@xmlns:ns2")]
    pub xmlns_ns2: String,

    // TODO: Weird, can't get this to name the same as others.
    // This works for deserialization, but it you try to serialize
    // using serde it comes out as "schemaLocation" instead of
    // "xsi:schemaLocation". Don't know how to reconcile the two.
    // This is one of the reasons I am writing the output file manually.
    #[serde(rename = "@schemaLocation")]
    pub xsi_schema_location: String,

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
            creator: self.creator.clone(),
            version: self.version.clone(),
            xmlns_ns3: self.xmlns_ns3.clone(),
            xmlns: self.xmlns.clone(),
            xmlns_xsi: self.xmlns_xsi.clone(),
            xmlns_ns2: self.xmlns_ns2.clone(),
            xsi_schema_location: self.xsi_schema_location.clone(),
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

pub struct MergedGpx {
    pub creator: String,
    pub version: String,
    pub xmlns_ns3: String,
    pub xmlns: String,
    pub xmlns_xsi: String,
    pub xmlns_ns2: String,
    pub xsi_schema_location: String,
    pub metadata_time: String,
    pub track_name: String,
    pub track_type: String,
    pub points: Vec<TrackPoint>,
}
