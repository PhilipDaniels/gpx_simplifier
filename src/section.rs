//! Contains the functionality relating to sections.
//! Detecting these is quite a bit of work. Once we get
//! the Sections determined we can calculate a lot of
//! other metrics fairly easily.

use core::{fmt, slice};
use std::{io::Write, ops::Index, path::Path};

use geo::{point, GeodesicDistance};
use time::{Duration, OffsetDateTime};

use crate::{
    formatting::{format_utc_date, format_utc_date_as_local},
    model::{EnrichedGpx, EnrichedTrackPoint},
};

/// Calculates speed in kmh from metres and seconds.
pub fn speed_kmh(metres: f64, seconds: f64) -> f64 {
    (metres / seconds) * 3.6
}

/// Calculates speed in kmh from metres and a Duration.
pub fn speed_kmh_from_duration(metres: f64, time: Duration) -> f64 {
    speed_kmh(metres, time.as_seconds_f64())
}

/// These are the parameters that control the 'Section-finding'
/// algorithm.
pub struct SectionParameters {
    /// You are considered "Stopped" if your speed drops below this.
    /// So that means a dead-stop.
    pub stopped_speed_kmh: f64,

    // You are considered to be "Moving Again" the first time your
    // speed goes above this. This is above a walking speed, so you
    // are probably riding again.
    pub resume_speed_kmh: f64,

    /// We want to eliminate tiny Sections caused by noisy data, for
    /// example these can occur when just starting off again.
    /// So set the minimum length of a section, in seconds.
    pub min_section_duration_seconds: f64,
}

/// Represents a section from a GPX track. The section can represent
/// you moving, or stopped.
#[derive(Debug)]
pub struct Section<'gpx> {
    pub section_type: SectionType,
    pub start: &'gpx EnrichedTrackPoint,
    pub end: &'gpx EnrichedTrackPoint,
    pub min_elevation: &'gpx EnrichedTrackPoint,
    pub max_elevation: &'gpx EnrichedTrackPoint,
}

/// The type of a Section.
#[derive(Debug, PartialEq, Eq)]
pub enum SectionType {
    Moving,
    Stopped,
}

impl fmt::Display for SectionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SectionType::Moving => write!(f, "Moving"),
            SectionType::Stopped => write!(f, "Stopped"),
        }
    }
}

impl<'gpx> Section<'gpx> {
    /// Returns the duration of the section.
    pub fn duration(&self) -> Duration {
        self.end.time - self.start.time
    }

    /// Returns the distance (length) of the section, in metres.
    pub fn distance_metres(&self) -> f64 {
        self.end.cum_metres - self.start.cum_metres
    }

    /// Returns the distance of the section, in km.
    pub fn distance_km(&self) -> f64 {
        self.distance_metres() / 1000.0
    }

    /// Returns the cumulative distance to the end of the section.
    pub fn cum_distance_km(&self) -> f64 {
        self.end.cum_metres / 1000.0
    }

    /// Returns the average speed of the section, in kmh.
    pub fn average_speed_kmh(&self) -> f64 {
        speed_kmh_from_duration(self.distance_metres(), self.duration())
    }

    /// Returns the total ascent in metres over the section.
    pub fn ascent_metres(&self) -> f64 {
        self.end.cum_ascent_metres - self.start.cum_ascent_metres
    }

    /// Returns the total descent in metres over the section.
    pub fn descent_metres(&self) -> f64 {
        self.end.cum_descent_metres - self.start.cum_descent_metres
    }
}

#[derive(Default)]
pub struct SectionList<'gpx>(Vec<Section<'gpx>>);

impl<'gpx> Index<usize> for SectionList<'gpx> {
    type Output = Section<'gpx>;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl<'gpx> SectionList<'gpx> {
    // TODO: Implement Iterator properly.
    fn iter(&self) -> slice::Iter<Section> {
        self.0.iter()
    }

    fn first_point(&self) -> &EnrichedTrackPoint {
        self.0[0].start
    }

    fn last_point(&self) -> &EnrichedTrackPoint {
        self.0[self.len() - 1].end
    }

    pub fn push(&mut self, section: Section<'gpx>) {
        self.0.push(section);
    }

    pub fn len(&self) -> usize {
        self.0.len()
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
    pub fn distance_metres(&self) -> f64 {
        self.0.iter().map(|s| s.distance_metres()).sum()
    }

    /// Returns the total distance of all the Sections in km.
    pub fn distance_km(&self) -> f64 {
        self.distance_metres() / 1000.0
    }

    /// Returns the point of minimum elevation across all the Sections.
    pub fn min_elevation(&self) -> &EnrichedTrackPoint {
        self.0
            .iter()
            .map(|section| &section.min_elevation)
            .min_by(|a, b| a.ele.total_cmp(&b.ele))
            .unwrap()
    }

    /// Returns the point of maximum elevation across all the Sections.
    pub fn max_elevation(&self) -> &EnrichedTrackPoint {
        self.0
            .iter()
            .map(|section| &section.max_elevation)
            .max_by(|a, b| a.ele.total_cmp(&b.ele))
            .unwrap()
    }

    /// Returns the total ascent in metres across all the Sections.
    pub fn total_ascent_metres(&self) -> f64 {
        self.0.iter().map(|section| section.ascent_metres()).sum()
    }

    /// Returns the total descent in metres across all the Sections.
    pub fn total_descent_metres(&self) -> f64 {
        self.0.iter().map(|section| section.descent_metres()).sum()
    }
}

/// Calculate a set of enriched TrackPoint information (distances, speed, climb).
pub fn enrich_trackpoints(gpx: &mut EnrichedGpx) {
    let start_time = gpx.points[0].time;
    let mut cum_ascent_metres = 0.0;
    let mut cum_descent_metres = 0.0;

    let mut p1 = point!(x: gpx.points[0].lon, y: gpx.points[0].lat);

    for idx in 1..gpx.points.len() {
        let p2 = point!(x: gpx.points[idx].lon, y: gpx.points[idx].lat);

        // Distance.
        // n.b. x=lon, y=lat. If you do it the other way round the
        // distances are wrong - a lot wrong.
        gpx.points[idx].delta_metres = p1.geodesic_distance(&p2);
        assert!(gpx.points[idx].delta_metres >= 0.0);

        gpx.points[idx].cum_metres = gpx.points[idx - 1].cum_metres + gpx.points[idx].delta_metres;
        assert!(gpx.points[idx].cum_metres >= 0.0);

        // Time delta. Don't really need this stored, but is handy to spot
        // points that took more than usual when scanning the CSV.
        gpx.points[idx].delta_time = gpx.points[idx].time - gpx.points[idx - 1].time;
        assert!(gpx.points[idx].delta_time.is_positive());

        // Speed. Based on the distance we just calculated.
        gpx.points[idx].speed_kmh =
            speed_kmh_from_duration(gpx.points[idx].delta_metres, gpx.points[idx].delta_time);
        assert!(gpx.points[idx].speed_kmh >= 0.0);

        // How long it took to get here.
        gpx.points[idx].duration = gpx.points[idx].time - start_time;
        assert!(gpx.points[idx].duration.is_positive());

        // Ascent and descent.
        let ele_delta_metres = gpx.points[idx].ele - gpx.points[idx - 1].ele;
        gpx.points[idx].ele_delta_metres = ele_delta_metres;

        if ele_delta_metres > 0.0 {
            cum_ascent_metres += ele_delta_metres;
        } else {
            cum_descent_metres += ele_delta_metres.abs();
        }

        gpx.points[idx].cum_ascent_metres = cum_ascent_metres;
        assert!(gpx.points[idx].cum_ascent_metres >= 0.0);
        gpx.points[idx].cum_descent_metres = cum_descent_metres;
        assert!(gpx.points[idx].cum_descent_metres >= 0.0);

        p1 = p2;
    }
}

/// Detects the sections in the GPX and returns them as a list.
///
/// Invariants: the first section starts at TrackPoint 0
/// and goes to TrackPoint N. The next section starts at
/// Trackpoint N and goes to TrackPoint M. The last section
/// ends at the last TrackPoint.
///
/// In other words, there are no gaps, all TrackPoints are in a
/// section, and TrackPoints in the middle will be in two adjacent
/// Sections. TrackPoints are cloned as part of this construction.
///
/// A Section is a Stopped section if you speed drops below
/// a (very low) limit and does not go above a 'resume_speed'
/// for a 'min_stop_time' length of time.
///
/// All non-Stopped sections are considered Moving sections.
pub fn detect_sections(gpx: &EnrichedGpx, params: SectionParameters) -> SectionList {
    if gpx.points.len() < 2 {
        eprintln!("Warning: gpx {:?} does not have any points", gpx.filename);
        return Default::default();
    }

    let mut sections = SectionList::default();

    // Note 1: The first TrackPoint always has a speed of 0, but it is unlikely
    // that you are actually in a Stopped section. However, it's not impossible,
    // see Note 2 for why.

    // Note 2: We need to deal with the slightly bizarre situation where you turn
    // the GPS on and then don't go anywhere for a while - so your first Section
    // may be a Stopped Section!

    // We can get everything we need to create a Section if we have the
    // index of the first and last TrackPoints for that Section.
    let mut start_idx = 0;
    while let Some(section) = get_next_section(start_idx, gpx, &params) {
        // The next section shares an index/TrackPoint with this one.
        start_idx = section.end.index;
        sections.push(section);
    }

    // Should include all TrackPoints and start/end indexes overlap.
    assert_eq!(
        sections[0].start.index, 0,
        "Should always start with the first point"
    );
    assert_eq!(
        sections[sections.len() - 1].end.index,
        gpx.points.len() - 1,
        "Should always end with the last point"
    );
    for idx in 0..sections.len() - 1 {
        assert_eq!(
            sections[idx].end.index,
            sections[idx + 1].start.index,
            "Section boundaries should be shared"
        );
    }

    sections
}

fn get_next_section<'gpx>(
    start_idx: usize,
    gpx: &'gpx EnrichedGpx,
    params: &SectionParameters,
) -> Option<Section<'gpx>> {
    // Get this out into a variable to avoid off-by-one errors (hopefully).
    let last_valid_idx = gpx.points.len() - 1;

    // Termination condition, we reached the end of the TrackPoints.
    if start_idx == last_valid_idx {
        return None;
    }

    // This assert exists so the check above can be '==' instead of '>='.
    // More likely to catch off-by-one bugs this way.
    assert!(start_idx < last_valid_idx);

    // We have said that a Section must be at least this long, so we need to
    // advance this far as a minimum.
    let end_idx = advance_for_duration(
        gpx,
        start_idx,
        last_valid_idx,
        params.min_section_duration_seconds,
    );
    assert!(end_idx <= last_valid_idx);
    assert!(end_idx > start_idx, "Empty sections are not allowed");

    if end_idx < last_valid_idx {
        // This is not necessarily true in the case where we exhaust all the TrackPoints.
        assert!(
            (gpx.points[end_idx].time - gpx.points[start_idx].time).as_seconds_f64()
                >= params.min_section_duration_seconds
        );
    } else {
        // But we can assert this weaker condition as a fallback.
        assert!((gpx.points[end_idx].time - gpx.points[start_idx].time).is_positive());
    }

    // Scan the TrackPoints we just got to determine the SectionType.
    let section_type = if gpx.points[start_idx..=end_idx]
        .iter()
        .any(|p| p.speed_kmh > params.resume_speed_kmh)
    {
        SectionType::Moving
    } else {
        SectionType::Stopped
    };

    // If we have not consumed all the trackpoints in advance_for_duration() above,
    // then the section might actually continue past the current end_idx. Keep going
    // until we really find the end. It's possible that this act may consume some or
    // all of the remaining trackpoints.
    let mut end_idx = end_idx;

    if end_idx < last_valid_idx {
        end_idx = match section_type {
            SectionType::Moving => {
                find_stop_index(
                    gpx,
                    end_idx, // Start the scan from the current end that we just found.
                    last_valid_idx,
                    params,
                )
            }
            SectionType::Stopped => {
                find_resume_index(
                    gpx,
                    end_idx, // Start the scan from the current end that we just found.
                    last_valid_idx,
                    params.resume_speed_kmh,
                )
            }
        }
    };

    let (min_ele, max_ele) = find_min_and_max_elevation_points(gpx, start_idx, end_idx);

    let section = Section {
        section_type,
        start: &gpx.points[start_idx],
        end: &gpx.points[end_idx],
        min_elevation: min_ele,
        max_elevation: max_ele,
    };

    // Just check we created everything correctly.
    assert!(end_idx <= last_valid_idx);
    assert_eq!(section.start.index, start_idx);
    assert_eq!(section.end.index, end_idx);
    assert!(section.end.index > section.start.index);
    assert!(section.end.time > section.start.time);

    return Some(section);
}

/// Scans forward through the points until we find a point
/// that is at least 'min_section_duration_seconds' ahead
/// of the start point.
fn advance_for_duration(
    gpx: &EnrichedGpx,
    start_idx: usize,
    last_valid_idx: usize,
    min_section_duration_seconds: f64,
) -> usize {
    let start_time = gpx.points[start_idx].time;
    let mut end_index = start_idx + 1;

    while end_index <= last_valid_idx {
        let delta_time = gpx.points[end_index].time - start_time;
        if delta_time.as_seconds_f64() >= min_section_duration_seconds {
            return end_index;
        }
        end_index += 1;
    }

    // If we get here then we exhausted all the TrackPoints.
    last_valid_idx
}

/// Within a given range of trackpoints, finds the ones with the minimum
/// and maximum elevation.
fn find_min_and_max_elevation_points<'gpx>(
    gpx: &'gpx EnrichedGpx,
    start_idx: usize,
    end_idx: usize,
) -> (&'gpx EnrichedTrackPoint, &'gpx EnrichedTrackPoint) {
    let mut min = &gpx.points[start_idx];
    let mut max = &gpx.points[start_idx];

    for tp in &gpx.points[start_idx..=end_idx] {
        if tp.ele < min.ele {
            min = tp;
        } else if tp.ele > max.ele {
            max = tp;
        }
    }

    assert!(max.ele >= min.ele);

    (min, max)
}

/// A Moving section is ended when we stop. This occurs when we drop below the
/// 'stopped_speed_kmh' and do not attain 'resume_speed_kmh' for at least
/// 'min_section_duration_seconds'. Find the index of that point.
fn find_stop_index(
    gpx: &EnrichedGpx,
    start_idx: usize,
    last_valid_idx: usize,
    params: &SectionParameters,
) -> usize {
    let mut end_idx = start_idx + 1;

    while end_idx <= last_valid_idx {
        // Find the first time we drop below 'stopped_speed_kmh'
        while end_idx <= last_valid_idx && gpx.points[end_idx].speed_kmh > params.stopped_speed_kmh
        {
            end_idx += 1;
        }

        // It's possible we exhausted all the TrackPoints - we were in a moving
        // Section that went right to the end of the track. Note that the line
        // above which increments end_index means that it is possible that
        // end_index is GREATER than last_valid_index at this point.
        if end_idx >= last_valid_idx {
            return last_valid_idx;
        }

        // Now take note of this point and scan forward for attaining 'resume_speed_kmh'.
        let possible_stop_idx = end_idx;
        let possible_stop_time = gpx.points[possible_stop_idx].time;
        while end_idx <= last_valid_idx && gpx.points[end_idx].speed_kmh < params.resume_speed_kmh {
            end_idx += 1;
        }

        // Same logic as above.
        if end_idx >= last_valid_idx {
            return last_valid_idx;
        }

        // Is that a valid length of stop? If so, the point found above is a valid
        // end for this current section (which is a Moving Section, remember).
        let stop_duration = gpx.points[end_idx].time - possible_stop_time;
        if stop_duration.as_seconds_f64() >= params.min_section_duration_seconds {
            return possible_stop_idx;
        }

        // If that's not a valid stop (because it's too short),
        // we need to continue searching. Start again from the
        // point we have already reached.
        end_idx += 1;
    }

    // If we get here then we exhausted all the TrackPoints.
    last_valid_idx
}

/// A Stopped section is ended when we find the first TrackPoint
/// with a speed above the resumption threshold. Find the index
/// of that point.
fn find_resume_index(
    gpx: &EnrichedGpx,
    start_idx: usize,
    last_valid_idx: usize,
    resume_speed_kmh: f64,
) -> usize {
    let mut end_index = start_idx + 1;

    while end_index <= last_valid_idx {
        if gpx.points[end_index].speed_kmh > resume_speed_kmh {
            return end_index;
        }
        end_index += 1;
    }

    // If we get here then we exhausted all the TrackPoints.
    last_valid_idx
}

/// Writes the trackpoints and the extended information to a CSV file,
/// very handy for debugging.
#[rustfmt::skip]
pub fn write_enriched_trackpoints_to_csv(p: &Path, gpx: &EnrichedGpx) {
    let mut writer = csv::Writer::from_path(p).unwrap();

    // Header. 4 fields from the original point, then the extended info.
    writer
        .write_record(vec![
            "TP Index",
            "Time (UTC)",
            "Time (local)",
            "Lat",
            "Lon",
            "Elevation (m)",
            "Distance Delta (m)",
            "Cum. Distance (m)",
            "Time Delta",
            "Cum. Duration",
            "Speed (kmh)",
            "Elevation Delta (m)",
            "Cum Ascent (m)",
            "Cum Descent (m)",
            "Location"
        ])
        .unwrap();

    // TrackPoints.
    for idx in 0..gpx.points.len() {
        writer.write_field(gpx.points[idx].index.to_string()).unwrap();
        writer.write_field(format_utc_date(gpx.points[idx].time)).unwrap();
        writer.write_field(format_utc_date_as_local(gpx.points[idx].time)).unwrap();
        writer.write_field(gpx.points[idx].lat.to_string()).unwrap();
        writer.write_field(gpx.points[idx].lon.to_string()).unwrap();
        writer.write_field(gpx.points[idx].ele.to_string()).unwrap();
        writer.write_field(gpx.points[idx].delta_metres.to_string()).unwrap();
        writer.write_field(gpx.points[idx].cum_metres.to_string()).unwrap();
        writer.write_field(gpx.points[idx].delta_time.to_string()).unwrap();
        writer.write_field(gpx.points[idx].duration.to_string()).unwrap();
        writer.write_field(gpx.points[idx].speed_kmh.to_string()).unwrap();
        writer.write_field(gpx.points[idx].ele_delta_metres.to_string()).unwrap();
        writer.write_field(gpx.points[idx].cum_ascent_metres.to_string()).unwrap();
        writer.write_field(gpx.points[idx].cum_descent_metres.to_string()).unwrap();
        writer.write_field(&gpx.points[idx].location).unwrap();
        // Terminator.
        writer.write_record(None::<&[u8]>).unwrap();
    }

    writer.flush().unwrap();
}

#[rustfmt::skip]
pub fn write_sections_csv(p: &Path, sections: &SectionList) {
    let mut writer = csv::Writer::from_path(p).unwrap();

    // Header. 4 fields from the original point, then the extended info.
    writer
        .write_record(vec![
            "Num",
            "Type",
            "TP Start",
            "TP End",
            "Start Time (UTC)",
            "Start Time (local)",
            "End Time (UTC)",
            "End Time (local)",
            "Duration",
            "Running Duration",
            "Distance (km)",
            "Running Distance (km)",
            "Avg Speed (kmh)",
            "Running Avg Speed (kmh)",
            "Ascent (m)",
            "Running Ascent (m)",
            "Descent (m)",
            "Running Descent (m)",
            "Min Ele (m)",
            "Min Ele Distance (m)",
            "Min Ele Time (local)",
            "Max Ele (m)",
            "Max Ele Distance (m)",
            "Max Ele Time (local)",
            "Lat",
            "Lon",
            "Location"
        ])
        .unwrap();

    for (idx, section) in sections.iter().enumerate() {
        writer.write_field((idx + 1).to_string()).unwrap();
        writer.write_field(section.section_type.to_string()).unwrap();
        writer.write_field(section.start.index.to_string()).unwrap();
        writer.write_field(section.end.index.to_string()).unwrap();
        writer.write_field(format_utc_date(section.start.time)).unwrap();
        writer.write_field(format_utc_date_as_local(section.start.time)).unwrap();
        writer.write_field(format_utc_date(section.end.time)).unwrap();
        writer.write_field(format_utc_date_as_local(section.end.time)).unwrap();
        writer.write_field(section.duration().to_string()).unwrap();
        writer.write_field("TODO").unwrap();
        if section.section_type == SectionType::Moving {
            writer.write_field(format!("{:.2}", section.distance_km())).unwrap();
            writer.write_field(format!("{:.2}", section.cum_distance_km())).unwrap();
            writer.write_field(format!("{:.2}", section.average_speed_kmh())).unwrap();
        } else {
            writer.write_field("").unwrap();
            writer.write_field("").unwrap();
            writer.write_field("").unwrap();
        }
        writer.write_field("TODO").unwrap();
        if section.section_type == SectionType::Moving {
            writer.write_field(format!("{:.2}", section.ascent_metres())).unwrap();
            writer.write_field(format!("{:.2}", section.end.cum_ascent_metres)).unwrap();
            writer.write_field(format!("{:.2}", section.descent_metres())).unwrap();
            writer.write_field(format!("{:.2}", section.end.cum_descent_metres)).unwrap();
        } else {
            writer.write_field("").unwrap();
            writer.write_field("").unwrap();
            writer.write_field("").unwrap();
            writer.write_field("").unwrap();
        }
        // Always write min elevation, so we have an elevation for a Stopped section as well.
        writer.write_field(format!("{:.2}", section.min_elevation.ele)).unwrap();
        writer.write_field(format!("{:.2}", section.min_elevation.cum_metres / 1000.0)).unwrap();
        writer.write_field(format_utc_date_as_local(section.min_elevation.time)).unwrap();
        if section.section_type == SectionType::Moving {
            writer.write_field(format!("{:.2}", section.max_elevation.ele)).unwrap();
            writer.write_field(format!("{:.2}", section.max_elevation.cum_metres / 1000.0)).unwrap();
            writer.write_field(format_utc_date_as_local(section.max_elevation.time)).unwrap();
   
        } else {
            writer.write_field("").unwrap();
            writer.write_field("").unwrap();
            writer.write_field("").unwrap();
        }
        writer.write_field(format!("{:.6}", section.start.lat)).unwrap();
        writer.write_field(format!("{:.6}", section.start.lon)).unwrap();

        if section.section_type == SectionType::Moving {
            if idx == 0 {
                // The start control.
                writer.write_field(&section.start.location).unwrap();
            } else if idx == sections.len() - 1 {
                // The finish control.
                writer.write_field(&section.end.location).unwrap();
            } else {
                // Irrelevant, see the Stopped location instead.
                writer.write_field("").unwrap();
            }
        } else {
            writer.write_field(&section.start.location).unwrap();
        }
        // Terminator.
        writer.write_record(None::<&[u8]>).unwrap();
    }

    // Now write a summary.
    writer.write_record(None::<&[u8]>).unwrap();
    writer.write_record(None::<&[u8]>).unwrap();
    writer.write_record(vec!["Summary"]).unwrap();

    writer.flush().unwrap();
}

/*
THIS IS THE ORIGINAL FN
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
 */
