
/// Represents a single <trkpt>.
#[derive(Debug)]
pub struct Trackpoint {
    /// The latitude, read from the "lat" attribute.
    pub lat: f32,
    /// The longitude, read from the "lon" attribute.
    pub lon: f32,
    /// The elevation, as read from the <ele> tag.
    pub ele: f32,
    /// Represents the time as read from the <time> tag. We don't
    /// do any "time processing" on the time, so just a string is
    /// good enough.
    pub time: String,
}
