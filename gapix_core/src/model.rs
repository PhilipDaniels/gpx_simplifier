use std::{collections::HashMap, path::PathBuf};
use time::{Duration, OffsetDateTime};

// Comparison of the GPX crate. In addition to this, it is 8 times
// slower when parsing a GPX.
//
// Gpx: lacks attributes, the XML declaration
// Metadata: full
// Waypoint: lacks magvar, extensions, lat/lon is available via the Point method
// Route: full (apart from extensions)
// Track: full (apart from extensions)
// Person: full
// Email: they represent as a single string
// Copyright: full (their author is optional)
// Link: full (they use type_)
// Bounds: full
// TrackSegment: full (apart from extensions)

/// Data parsed from a GPX file, based on the XSD description at
/// https://www.topografix.com/GPX/1/1/gpx.xsd
#[derive(Debug, Clone)]
pub struct Gpx {
    /// Represents the 'xml' declaration tag - the first line of an XML file.
    pub declaration: XmlDeclaration,
    /// The filename field is not part of the XSD, but it is convenient to have
    /// it so it can be used as an identifier for the GPX data.
    pub filename: Option<PathBuf>,

    /// The 'version' attribute. This should always be "1.1".
    pub version: String,
    /// The 'creator' attribute.
    pub creator: String,
    /// The other attributes (excluding creator and version, which
    /// are mandatory.)
    pub attributes: HashMap<String, String>,
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

/// The metadata element contains information about the GPX file, such as
/// author, and copyright restrictions.
#[derive(Debug, Clone, Default)]
pub struct Metadata {
    /// The name of the GPX file.
    pub name: Option<String>,
    /// A description of the GPX file.
    pub description: Option<String>,
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
#[derive(Debug, Clone, Default)]
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
#[derive(Debug, Clone, Default)]
pub struct Copyright {
    /// The year of copyright.
    pub year: Option<i16>,
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
#[derive(Debug, Clone, Default)]
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
    pub comment: Option<String>,
    /// User description of the route.
    pub description: Option<String>,
    /// Source of data. Included to give user some idea of reliability and accuracy of data.
    pub source: Option<String>,
    /// Zero or more URLs associated with the route.
    pub links: Vec<Link>,
    /// GPS route number.
    pub number: Option<u32>,
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
    pub description: Option<String>,
    /// Source of data. Included to give user some idea of reliability and accuracy of data.
    pub source: Option<String>,
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
    pub comment: Option<String>,
    /// A text description of the element. Holds additional information about
    /// the element intended for the user, not the GPS.
    pub description: Option<String>,
    /// Source of data. Included to give user some idea of reliability and
    /// accuracy of data.
    pub source: Option<String>,
    /// Links to additional information about the waypoint.
    pub links: Vec<Link>,
    /// Text of GPS symbol name.
    pub symbol: Option<String>,
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
    TwoDimensional,
    ThreeDimensional,
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

/// An EnrichedGpx is one where we flatten the Tracks and Segments into a
/// simple vector of EnrichedTrackPoints. These are TrackPoints with a lot
/// of derived data fields that make later work easier.
#[derive(Debug)]
pub struct EnrichedGpx {
    pub declaration: XmlDeclaration,
    pub filename: Option<PathBuf>,
    pub version: String,
    pub creator: String,
    pub attributes: HashMap<String, String>,
    pub metadata: Metadata,
    pub track_name: Option<String>,
    pub track_type: Option<String>,
    pub points: Vec<EnrichedTrackPoint>,
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
