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
#[derive(Debug, Deserialize)]
pub struct Metadata {
    pub time: String
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
    pub points: Vec<TrackPoint>
}

/// Represents a single <trkpt>.
#[derive(Debug, Deserialize)]
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
