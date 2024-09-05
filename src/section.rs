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

/*
/// Calculates the distance from one trackpoint to the next using the geo
/// crate. This seems to be reasonably accurate, but over-estimates the
/// total distance by approx 0.5% compared to plotaroute.com.
fn calculate_distance_and_speed(points: &mut [TrackPoint]) {
    if points.len() < 2 {
        return;
    }

    // Distances first.
    // n.b. x=lon, y=lat. If you do it the other way round the
    // distances are wrong - a lot wrong.
    let mut cum_distance = 0.0;
    let mut p1 = point!(x: points[0].lon as f64, y: points[0].lat as f64);
    for i in 1..points.len() {
        let p2 = point!(x: points[i].lon as f64, y: points[i].lat as f64);
        let distance = p1.geodesic_distance(&p2);
        cum_distance += distance;
        points[i].distance_from_prev_metres = distance as f32;
        points[i].cumulative_distance_metres = cum_distance as f32;
        p1 = p2;
    }

    // Then speed is easy. We can calculate this for every point but the first.
    // Again, this is heavily dependent upon the accuracy of the distance
    // calculation, but seems "about right".
    // TODO: Probably would be better with smoothing.
    for i in 1..points.len() {
        let time_delta_seconds = (points[i].time - points[i - 1].time).as_seconds_f32();
        points[i].speed_kmh = calc_speed_kmh(points[i].distance_from_prev_metres, time_delta_seconds);

        // While we are at it, also fill in the climb figures.
        // TODO: Doesn't work, probably not enough change per point.
        points[i].ascent_from_prev_metres = points[i].ele - points[i - 1].ele;
        if points[i].ascent_from_prev_metres > 0.0 {
            points[i].cumulative_ascent_metres = points[i - 1].cumulative_ascent_metres + points[i].ascent_from_prev_metres;
        } else {
            points[i].cumulative_descent_metres = points[i - 1].cumulative_descent_metres + points[i].ascent_from_prev_metres.abs();
        }
    }
}
*/

/*
/// You are determined to be stopped if your speed drops below MIN_SPEED km/h and does not
/// go above 'resume_speed' until at least 'min_stop_time' minutes have passed.
fn detect_stops(points: &[TrackPoint], resume_speed: u8, min_stop_time: u8) -> Vec<Stop> {
    const MIN_SPEED: f32 = 0.1;
    let resume_speed = resume_speed as f32;
    let min_stop_time = min_stop_time as f32 * 60.0; // convert to seconds

    let mut iter = points.iter().enumerate();

    // Skip the first point, it always has speed 0.
    iter.next();

    let mut stops = Vec::new();

    while let Some((start_idx, start_point)) = iter.find(|(_, p)| p.speed_kmh < MIN_SPEED) {
        // Find the next point that has a speed of at least resume_speed, i.e. we started riding again.
        if let Some((end_idx, end_point)) = iter.find(|(_, p)| p.speed_kmh > resume_speed) {
            if (end_point.time - start_point.time).as_seconds_f32() > min_stop_time {
                let stop = Stop {
                    start: start_point.clone(),
                    start_idx,
                    end: end_point.clone(),
                    end_idx
                };
    
                stops.push(stop);
            }
        }
    }

    stops
}
*/

/*
fn write_stop_report<W: Write>(w: &mut W, gpx: &MergedGpx, stops: &[Stop]) {
    let stopped_time: Duration = stops.iter().map(|s| s.duration()).sum();
    let moving_time = gpx.total_time() - stopped_time;
    let min_ele = gpx.min_elevation();
    let max_ele = gpx.max_elevation();

    writeln!(w, "Distance     : {:.2} km", gpx.distance_km()).unwrap();
    writeln!(w, "Start time   : {}", format_utc_date(gpx.start_time())).unwrap();
    writeln!(w, "End time     : {}", format_utc_date(gpx.end_time())).unwrap();
    writeln!(w, "Total time   : {}", gpx.total_time()).unwrap();
    writeln!(w, "Moving time  : {}", moving_time).unwrap();
    writeln!(w, "Stopped time : {}", stopped_time).unwrap();
    writeln!(w, "Moving speed : {:.2} km/h", calc_speed_kmh(gpx.distance_metres(), moving_time.as_seconds_f32())).unwrap();
    writeln!(w, "Overall speed: {:.2} km/h", calc_speed_kmh(gpx.distance_metres(), gpx.total_time().as_seconds_f32())).unwrap();
    writeln!(w, "Total ascent : {:.2} m", gpx.total_ascent_metres()).unwrap();
    writeln!(w, "Total descent: {:.2} m", gpx.total_descent_metres()).unwrap();
    writeln!(w, "Min elevation: {} m, at {:.2} km, {}",
        min_ele.ele, min_ele.cumulative_distance_metres / 1000.0, format_utc_date(min_ele.time)
        ).unwrap();
    writeln!(w, "Max elevation: {} m, at {:.2} km, {}",
        max_ele.ele, max_ele.cumulative_distance_metres / 1000.0, format_utc_date(max_ele.time)
        ).unwrap();
    writeln!(w).unwrap();

    let mut table = Table::new();
    table.load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_header(vec!["Stop", "Start", "End", "Length", "Location"])
        .set_content_arrangement(ContentArrangement::Dynamic);
        
    for (idx, stop) in stops.iter().enumerate() {
        table.add_row(vec![
            Cell::new(idx + 1).set_alignment(CellAlignment::Right),
            Cell::new(format_utc_and_local_date(stop.start.time, "\n")),
            Cell::new(format_utc_and_local_date(stop.end.time, "\n")),
            Cell::new(stop.duration()),
            Cell::new(format!("{}\n({},{})", "unk", stop.start.lat, stop.start.lon)),
        ]);
    }

    writeln!(w, "{}", table).unwrap();
}
 */