//! Contains the functionality relating to sections.
//! Detecting these is quite a bit of work. Once we get
//! the Sections determined we can calculate a lot of
//! other metrics fairly easily.

use std::io::Write;

use comfy_table::{modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL, ContentArrangement, Table};
use time::{Duration, OffsetDateTime};

use crate::model::TrackPoint;

/// Calculates speed in kmh from metres and seconds.
pub fn speed_kmh(metres: f32, seconds: f32) -> f32 {
    (metres / seconds) * 3.6
}

/// Calculates speed in kmh from metres and a Duration.
pub fn speed_kmh_from_duration(metres: f32, time: Duration) -> f32 {
    speed_kmh(metres, time.as_seconds_f32())
}

/// The type of a Section.
#[derive(Debug)]
pub enum SectionType {
    Moving,
    Stopped,
}

/// Represents an end of a Section - either the start
/// or the end.
#[derive(Debug)]
pub struct SectionBound {
    /// The index into the original trackpoint array
    /// for which this SectionBound was calculated.
    pub index: usize,

    // A clone of the corresponding trackpoint. This
    /// includes the lat-lon, elevation and time.
    pub point: TrackPoint,

    /// The cumulative distance that was travelled along
    /// the original track to reach this point.
    pub distance_metres: f32,
}

/// Represents a elevation point of interest (typically
/// we are interested in min and max elevations and where
/// they occurred.)
#[derive(Debug)]
pub struct ElevationPoint {
    /// A clone of the corresponding trackpoint. This
    /// includes the lat-lon, elevation and time.
    pub point: TrackPoint,

    /// The cumulative distance that was travelled along
    /// the original track to reach this point.
    pub distance_metres: f32,

    /// Geo-coded location of the point.
    pub location: String,
}

/// Represents a section from a GPX track. The section can represent
/// you moving, or stopped.
#[derive(Debug)]
pub struct Section {
    pub section_type: SectionType,
    pub start: SectionBound,
    pub end: SectionBound,

    /// Where the minimum elevation in this Section occurred.
    /// We fill it in for both Stopped and Moving section types,
    /// but it is only really useful for the Moving type. It does
    /// no harm for Stopped types.
    pub min_elevation: ElevationPoint,

    /// Where the maximum elevation in this Section occurred.
    /// We fill it in for both Stopped and Moving section types,
    /// but it is only really useful for the Moving type. It does
    /// no harm for Stopped types.
    pub max_elevation: ElevationPoint,

    /// The total ascent in metres during this Section.
    pub ascent_metres: f32,

    /// The total descent in metres during this Section.
    pub descent_metres: f32,
}

impl Section {
    /// Returns the duration of the section.
    pub fn duration(&self) -> Duration {
        self.end.point.time - self.start.point.time
    }

    /// Returns the distance of the section, in metres.
    pub fn distance_metres(&self) -> f32 {
        self.end.distance_metres - self.start.distance_metres
    }

    /// Returns the distance of the section, in km.
    pub fn distance_km(&self) -> f32 {
        self.distance_metres() / 1000.0
    }

    /// Returns the average speed of the section, in kmh.
    pub fn average_speed_kmh(&self) -> f32 {
        speed_kmh_from_duration(self.distance_metres(), self.duration())
    }
}

pub struct SectionList(Vec<Section>);

impl SectionList {
    fn first_point(&self) -> &TrackPoint {
        &self.0[0].start.point
    }

    fn last_point(&self) -> &TrackPoint {
        &self.0[self.0.len() - 1].end.point
    }

    /// Returns the start time of the first Section.
    pub fn start_time(&self) -> OffsetDateTime {
        self.first_point().time
    }

    /// Returns the end time of the last Section.
    pub fn end_time(&self) -> OffsetDateTime {
        self.last_point().time
    }

    /// Returns the total duration between the start of the first
    /// Section and the end of the last Section.
    pub fn duration(&self) -> Duration {
        self.end_time() - self.start_time()
    }

    /// Returns the total time Moving across all the sections.
    pub fn total_moving_time(&self) -> Duration {
        self.duration() - self.total_stopped_time()
    }

    /// Returns the total time Stopped across all the sections.
    pub fn total_stopped_time(&self) -> Duration {
        self.0
            .iter()
            .filter_map(|section| match section.section_type {
                SectionType::Moving => None,
                SectionType::Stopped => Some(section.duration()),
            })
            .sum()
    }

    /// Returns the total distance of all the Sections in metres.
    pub fn distance_metres(&self) -> f32 {
        self.0.iter().map(|s| s.distance_metres()).sum()
    }

    /// Returns the total distance of all the Sections in km.
    pub fn distance_km(&self) -> f32 {
        self.distance_metres() / 1000.0
    }

    /// Returns the point of minimum elevation across all the Sections.
    pub fn min_elevation(&self) -> &ElevationPoint {
        let min_ep = self
            .0
            .iter()
            .map(|section| &section.min_elevation)
            .min_by(|a, b| a.point.ele.total_cmp(&b.point.ele))
            .unwrap();

        &min_ep
    }

    /// Returns the point of maximum elevation across all the Sections.
    pub fn max_elevation(&self) -> &ElevationPoint {
        let min_ep = self
            .0
            .iter()
            .map(|section| &section.min_elevation)
            .max_by(|a, b| a.point.ele.total_cmp(&b.point.ele))
            .unwrap();

        &min_ep
    }

    /// Returns the total ascent in metres across all the Sections.
    pub fn total_ascent_metres(&self) -> f32 {
        self.0.iter().map(|section| section.ascent_metres).sum()
    }

    /// Returns the total descent in metres across all the Sections.
    pub fn total_descent_metres(&self) -> f32 {
        self.0.iter().map(|section| section.descent_metres).sum()
    }
}

/// Writes a tabular text report to the writer 'w', which can be stdout
/// and/or a file writer.
pub fn write_section_report<W: Write>(w: &mut W, sections: &SectionList) {
    let mut table = Table::new();

    // Location needs to be the start location for everything but
    // the last section, in which case it is is the end location.

    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_header(vec![
            "Section",
            "Start",
            "End",
            "Section\nDistance",
            "Duration",
            "Speed",
            "Cumulative\nDistance",
            "Location",
            "Ascent\nCum. Ascent",
            "Descent\nCum.Descent",
            "Min Elevation",
            "Max Elevation"
        ])
        .set_content_arrangement(ContentArrangement::Dynamic);

    writeln!(w, "{}", table).unwrap();
}
