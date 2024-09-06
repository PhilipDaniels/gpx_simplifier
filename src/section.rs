//! Contains the functionality relating to sections.
//! Detecting these is quite a bit of work. Once we get
//! the Sections determined we can calculate a lot of
//! other metrics fairly easily.

use core::{fmt, slice};
use std::{
    io::Write,
    ops::Index,
};

use comfy_table::{
    modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL, Cell, CellAlignment, ContentArrangement,
    Table,
};
use geo::{point, GeodesicDistance};
use time::{Duration, OffsetDateTime};

use crate::{
    formatting::{format_local_date, format_utc_and_local_date, format_utc_date},
    model::{MergedGpx, TrackPoint},
};

/// Calculates speed in kmh from metres and seconds.
pub fn speed_kmh(metres: f64, seconds: f64) -> f64 {
    (metres / seconds) * 3.6
}

/// Calculates speed in kmh from metres and a Duration.
pub fn speed_kmh_from_duration(metres: f64, time: Duration) -> f64 {
    speed_kmh(metres, time.as_seconds_f64())
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
    pub ascent_metres: f64,

    /// The total descent in metres during this Section.
    pub descent_metres: f64,

    /// The cumulative ascent in metres to the end of this Section.
    pub cum_ascent_metres: f64,

    /// The cumulative descent in metres to the end of this Section.
    pub cum_descent_metres: f64,
}

/// The type of a Section.
#[derive(Debug)]
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
    pub cum_distance_metres: f64,
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
    pub cum_distance_metres: f64,

    /// Geo-coded location of the point.
    pub location: String,
}

impl ElevationPoint {
    /// Returns the cumulative distance to the point.
    pub fn cum_distance_km(&self) -> f64 {
        self.cum_distance_metres / 1000.0
    }
}

impl Section {
    /// Returns the duration of the section.
    pub fn duration(&self) -> Duration {
        self.end.point.time - self.start.point.time
    }

    /// Returns the distance (length) of the section, in metres.
    pub fn distance_metres(&self) -> f64 {
        self.end.cum_distance_metres - self.start.cum_distance_metres
    }

    /// Returns the distance of the section, in km.
    pub fn distance_km(&self) -> f64 {
        self.distance_metres() / 1000.0
    }

    /// Returns the cumulative distance to the end of the section.
    pub fn cum_distance_km(&self) -> f64 {
        self.end.cum_distance_metres / 1000.0
    }

    /// Returns the average speed of the section, in kmh.
    pub fn average_speed_kmh(&self) -> f64 {
        speed_kmh_from_duration(self.distance_metres(), self.duration())
    }
}

#[derive(Default)]
pub struct SectionList(Vec<Section>);

impl Index<usize> for SectionList {
    type Output = Section;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl SectionList {
    // TODO: Implement Iterator properly.
    fn iter(&self) -> slice::Iter<Section> {
        self.0.iter()
    }

    fn first_point(&self) -> &TrackPoint {
        &self.0[0].start.point
    }

    fn last_point(&self) -> &TrackPoint {
        &self.0[self.0.len() - 1].end.point
    }

    pub fn push(&mut self, section: Section) {
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
    pub fn total_ascent_metres(&self) -> f64 {
        self.0.iter().map(|section| section.ascent_metres).sum()
    }

    /// Returns the total descent in metres across all the Sections.
    pub fn total_descent_metres(&self) -> f64 {
        self.0.iter().map(|section| section.descent_metres).sum()
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
pub fn detect_sections(
    gpx: &MergedGpx,
    resume_speed: f64,
    min_stop_time_seconds: f64,
) -> SectionList {
    let mut sections = Default::default();
    if gpx.points.len() < 2 {
        eprintln!("Warning: gpx {:?} does not have any points", gpx.filename);
        return sections;
    }

    let ext_trackpoints = calculate_enriched_trackpoints(gpx);
    write_to_csv(gpx, &ext_trackpoints);

    calculate_sections(gpx, &ext_trackpoints)
}

/// Writes a tabular text report to the writer 'w', which can be stdout
/// and/or a file writer.
#[rustfmt::skip]
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
            "Duration",
            "Avg Speed\n(km/h)",
            "Distance (km)\nCum. Distance",
            "Ascent (m)\nCum. Ascent",
            "Descent\nCum. Descent",
            "Location",
            "Min Elevation",
            "Max Elevation",
        ])
        .set_content_arrangement(ContentArrangement::Dynamic);

    let mut section_number = 1;
    for section in sections.iter() {
        table.add_row(vec![
            Cell::new(format!("{section_number}\n{}", section.section_type)).set_alignment(CellAlignment::Right),
            Cell::new(format_utc_and_local_date(section.start.point.time, "\n")),
            Cell::new(format_utc_and_local_date(section.end.point.time, "\n")),
            Cell::new(section.duration()),
            match section.section_type {
                SectionType::Moving => Cell::new(format!("{:.2}", section.average_speed_kmh())),
                SectionType::Stopped => Cell::new(""),
            },
            match section.section_type {
                SectionType::Moving => Cell::new(format!("{:.2}\n{:.2}", section.distance_km(), section.cum_distance_km())),
                SectionType::Stopped => Cell::new(""),
            },
            match section.section_type {
                SectionType::Moving => Cell::new(format!("{:.2}\n{:.2}", section.ascent_metres, section.cum_ascent_metres)),
                SectionType::Stopped => Cell::new(""),
            },
            match section.section_type {
                SectionType::Moving => Cell::new(format!("{:.2}\n{:.2}", section.descent_metres, section.cum_descent_metres)),
                SectionType::Stopped => Cell::new(""),
            },
            Cell::new("unk"),
            match section.section_type {
                SectionType::Moving => Cell::new(format!("{:.2} m at {:.2} km\n{}",
                    section.min_elevation.point.ele,
                    section.min_elevation.cum_distance_km(),
                    format_local_date(section.min_elevation.point.time)
                    )),
                SectionType::Stopped => Cell::new(""),
            },
            match section.section_type {
                SectionType::Moving => Cell::new(format!("{:.2} m at {:.2} km\n{}",
                    section.max_elevation.point.ele,
                    section.max_elevation.cum_distance_km(),
                    format_local_date(section.max_elevation.point.time)
                    )),
                SectionType::Stopped => Cell::new(""),
            },
        ]);

        section_number += 1;
    }

    writeln!(w, "{}", table).unwrap();
}

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
 */

#[derive(Debug, Default)]
struct ExtendedTrackPointInfo {
    distance_delta_metres: f64,
    cum_distance_metres: f64,
    speed_kmh: f64,
    cum_duration: Duration,
    ele_delta_metres: f64,
    cum_ascent_metres: f64,
    cum_descent_metres: f64,
}

/// Calculate a set of enriched TrackPoint information (distances, speed, climb)
/// in a Vec whose indexes are parallel (1-1) with the indexes in gpx.points.
fn calculate_enriched_trackpoints(gpx: &MergedGpx) -> Vec<ExtendedTrackPointInfo> {
    // Push a dummy first element so that the indices in ext_info[]
    // match 1-1 with the indices into gpx.points[].
    let mut ext_infos = Vec::new();
    ext_infos.push(ExtendedTrackPointInfo::default());

    // Cumulative figures.
    let mut cum_distance_metres = 0.0;
    let mut cum_ascent_metres = 0.0;
    let mut cum_descent_metres = 0.0;
    let start_time = gpx.points[0].time;

    let mut p1 = point!(x: gpx.points[0].lon, y: gpx.points[0].lat);

    for idx in 1..gpx.points.len() {
        let p2 = point!(x: gpx.points[idx].lon, y: gpx.points[idx].lat);

        // Distance.
        // n.b. x=lon, y=lat. If you do it the other way round the
        // distances are wrong - a lot wrong.
        let distance_delta_metres = p1.geodesic_distance(&p2);
        cum_distance_metres += distance_delta_metres;
        assert!(distance_delta_metres >= 0.0);
        assert!(cum_distance_metres >= 0.0);

        // Speed. Based on the distance we just calculated.
        let time_delta = gpx.points[idx].time - gpx.points[idx - 1].time;
        let speed_kmh = speed_kmh_from_duration(distance_delta_metres, time_delta);
        assert!(time_delta.is_positive());
        assert!(speed_kmh >= 0.0);

        // Ascent and descent.
        let ele_delta_metres = gpx.points[idx].ele - gpx.points[idx - 1].ele;
        if ele_delta_metres > 0.0 {
            cum_ascent_metres += ele_delta_metres;
        } else {
            cum_descent_metres += ele_delta_metres.abs();
        }
        assert!(cum_ascent_metres >= 0.0);
        assert!(cum_descent_metres >= 0.0);

        // How long it took to get here.
        let cum_duration = gpx.points[idx].time - start_time;
        assert!(cum_duration.is_positive());

        ext_infos.push(ExtendedTrackPointInfo {
            distance_delta_metres,
            cum_distance_metres,
            speed_kmh,
            cum_duration,
            ele_delta_metres,
            cum_ascent_metres,
            cum_descent_metres,
        });

        p1 = p2;
    }

    assert_eq!(gpx.points.len(), ext_infos.len());

    ext_infos
}

/// Writes the trackpoints and the extended information to a CSV file,
/// very handy for debugging.
#[rustfmt::skip]
fn write_to_csv(gpx: &MergedGpx, ext_trackpoints: &[ExtendedTrackPointInfo]) {
    let mut p = gpx.filename.clone();
    p.set_extension("trackpoints.csv");
    let mut writer = csv::Writer::from_path(p).unwrap();

    // Header. 4 fields from the original point, then the extended info.
    writer
        .write_record(vec![
            "Time",
            "Lat",
            "Lon",
            "Ele",
            "exDistanceDeltaMetres",
            "exCumMetres",
            "exSpeed",
            "exCumDuration",
            "exEleDeltaMetres",
            "exCumAscentMetres",
            "exCumDescentMetres",
        ])
        .unwrap();

    // TrackPoints.
    for idx in 0..gpx.points.len() {
        // 4 fields from the original point
        writer.write_field(format_utc_date(gpx.points[idx].time)).unwrap();
        writer.write_field(gpx.points[idx].lat.to_string()).unwrap();
        writer.write_field(gpx.points[idx].lon.to_string()).unwrap();
        writer.write_field(gpx.points[idx].ele.to_string()).unwrap();
        // Then the extended info
        writer.write_field(ext_trackpoints[idx].distance_delta_metres.to_string()).unwrap();
        writer.write_field(ext_trackpoints[idx].cum_distance_metres.to_string()).unwrap();
        writer.write_field(ext_trackpoints[idx].speed_kmh.to_string()).unwrap();
        writer.write_field(ext_trackpoints[idx].cum_duration.to_string()).unwrap();
        writer.write_field(ext_trackpoints[idx].ele_delta_metres.to_string()).unwrap();
        writer.write_field(ext_trackpoints[idx].cum_ascent_metres.to_string()).unwrap();
        writer.write_field(ext_trackpoints[idx].cum_descent_metres.to_string()).unwrap();
        // Terminator.
        writer.write_record(None::<&[u8]>).unwrap();
    }

    writer.flush().unwrap();
}

/// Split the GPX into consecutive Sections, which are of type Moving
/// or Stopped.
fn calculate_sections(gpx: &MergedGpx, ext_trackpoints: &[ExtendedTrackPointInfo]) -> SectionList {
    assert!(gpx.points.len() == ext_trackpoints.len());

    let mut sections = SectionList::default();

    let params = SectionParameters {
        stopped_speed_kmh: 0.01,
        resume_speed_kmh: 10.0,
        min_section_duration_seconds: 120.0, // Info controls! Do we care? TODO: This has a large effect. Maybe a bug.
    };

    // Note 1: The first TrackPoint always has a speed of 0, but it is unlikely
    // that you are actually in a Stopped section. However, it's not impossible,
    // see Note 2 for why.

    // Note 2: We need to deal with the slightly bizarre situation where you turn
    // the GPS on and then don't go anywhere for a while - so your first Section
    // may be a Stopped Section!

    // We can get everything we need to create a Section if we have the
    // index of the first and last TrackPoints for that Section.
    let mut start_idx = 0;
    while let Some((end_idx, section_type)) =
        get_section_end(gpx, ext_trackpoints, start_idx, &params)
    {
        sections.push(make_section(
            gpx,
            ext_trackpoints,
            start_idx,
            end_idx,
            section_type,
        ));

        // The next section shares an index/TrackPoint with this one.
        start_idx = end_idx;
    }

    // Should include all TrackPoints and start/end indexes overlap.
    assert_eq!(sections[0].start.index, 0);
    assert_eq!(sections[sections.len() - 1].end.index, gpx.points.len() - 1);
    for idx in 0..sections.len() - 1 {
        assert_eq!(sections[idx].end.index, sections[idx + 1].start.index);
    }

    sections
}

struct SectionParameters {
    /// You are considered "Stopped" if your speed drops below this.
    /// So that means a dead-stop.
    stopped_speed_kmh: f64,

    // You are considered to be "Moving Again" the first time your
    // speed goes above this. This is above a walking speed, so you
    // are probably riding again.
    resume_speed_kmh: f64,

    /// We want to eliminate tiny Sections caused by noisy data, for
    /// example these can occur when just starting off again.
    /// So set the minimum length of a section, in seconds.
    min_section_duration_seconds: f64,
}

fn get_section_end(
    gpx: &MergedGpx,
    ext_trackpoints: &[ExtendedTrackPointInfo],
    start_idx: usize,
    params: &SectionParameters,
) -> Option<(usize, SectionType)> {
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
    assert!(end_idx > start_idx);
    // This is not necessarily true in the case where we exhaust all the TrackPoints.
    //assert!((gpx.points[end_idx].time - gpx.points[start_idx].time).as_seconds_f64() >= params.min_section_duration_seconds);
    assert!((gpx.points[end_idx].time - gpx.points[start_idx].time).is_positive());

    // Scan the TrackPoints we just got to determine the SectionType.
    let section_type = if ext_trackpoints[start_idx..=end_idx] // TODO: Off-by-one on start_idx + 1?
        .iter()
        .any(|p| p.speed_kmh > params.resume_speed_kmh)
    {
        SectionType::Moving
    } else {
        SectionType::Stopped
    };

    // It's possible we have consumed all the TrackPoints.
    if end_idx == last_valid_idx {
        return Some((end_idx, section_type));
    }

    // We now need to look ahead to find the end of this Section. How we scan
    // depends on the SectionType. It's possible that these functions will
    // consume all or only some of the remaining trackpoints.
    let end_index = match section_type {
        SectionType::Moving => {
            find_stop_index(
                gpx,
                ext_trackpoints,
                end_idx, // Start the scan from the current end that we just found.
                last_valid_idx,
                params,
            )
        }
        SectionType::Stopped => {
            find_resume_index(
                ext_trackpoints,
                end_idx, // Start the scan from the current end that we just found.
                last_valid_idx,
                params.resume_speed_kmh,
            )
        }
    };

    assert!(end_idx <= last_valid_idx);
    assert!(end_idx > start_idx);
    return Some((end_index, section_type));
}

/// Find the next Stopped point.
/// A Moving section is ended when we stop. This occurs when we drop below the
/// 'stopped_speed_kmh' and do not attain 'resume_speed_kmh' for at least
/// 'min_section_duration_seconds'
fn find_stop_index(
    gpx: &MergedGpx,
    ext_trackpoints: &[ExtendedTrackPointInfo],
    start_idx: usize,
    last_valid_idx: usize,
    params: &SectionParameters,
) -> usize {
    let mut end_idx = start_idx + 1;

    while end_idx <= last_valid_idx {
        // Find the first time we drop below 'stopped_speed_kmh'
        while end_idx <= last_valid_idx
            && ext_trackpoints[end_idx].speed_kmh > params.stopped_speed_kmh
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
        while end_idx <= last_valid_idx
            && ext_trackpoints[end_idx].speed_kmh < params.resume_speed_kmh
        {
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
/// with a speed above the resumption threshold.
fn find_resume_index(
    ext_trackpoints: &[ExtendedTrackPointInfo],
    start_idx: usize,
    last_valid_idx: usize,
    resume_speed_kmh: f64,
) -> usize {
    let mut end_index = start_idx + 1;

    while end_index <= last_valid_idx {
        if ext_trackpoints[end_index].speed_kmh > resume_speed_kmh {
            return end_index;
        }
        end_index += 1;
    }

    // If we get here then we exhausted all the TrackPoints.
    last_valid_idx
}

fn advance_for_duration(
    gpx: &MergedGpx,
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

fn make_section(
    gpx: &MergedGpx,
    ext_trackpoints: &[ExtendedTrackPointInfo],
    start_idx: usize,
    end_idx: usize,
    section_type: SectionType,
) -> Section {
    assert!(end_idx > start_idx);

    let start = SectionBound {
        index: start_idx,
        point: gpx.points[start_idx].clone(),
        cum_distance_metres: ext_trackpoints[start_idx].cum_distance_metres,
    };

    let end = SectionBound {
        index: end_idx,
        point: gpx.points[end_idx].clone(),
        cum_distance_metres: ext_trackpoints[end_idx].cum_distance_metres,
    };

    assert!(end.cum_distance_metres >= start.cum_distance_metres);
    assert_eq!(end.index, end_idx);
    assert!(end.point.time > start.point.time);
    assert_eq!(start.index, start_idx);

    // Can't do this easily with min_by_key because you need to enumerate()
    // to get the index, plus floats are PartialOrd only. In any case, a
    // simple loop lets us calculate both min and max at the same time.
    let mut min_idx = start_idx;
    let mut max_idx = start_idx;
    for i in start_idx..=end_idx {
        if gpx.points[i].ele < gpx.points[min_idx].ele {
            min_idx = i;
        } else if gpx.points[i].ele > gpx.points[max_idx].ele {
            max_idx = i;
        }
    }

    let min_elevation = ElevationPoint {
        point: gpx.points[min_idx].clone(),
        cum_distance_metres: ext_trackpoints[min_idx].cum_distance_metres,
        location: Default::default(),
    };

    let max_elevation = ElevationPoint {
        point: gpx.points[max_idx].clone(),
        cum_distance_metres: ext_trackpoints[max_idx].cum_distance_metres,
        location: Default::default(),
    };

    assert!(max_elevation.point.ele >= min_elevation.point.ele);

    let ascent_metres =
        ext_trackpoints[end_idx].cum_ascent_metres - ext_trackpoints[start_idx].cum_ascent_metres;
    assert!(ascent_metres >= 0.0);

    let descent_metres =
        ext_trackpoints[end_idx].cum_descent_metres - ext_trackpoints[start_idx].cum_descent_metres;
    assert!(descent_metres >= 0.0);

    Section {
        section_type,
        start,
        end,
        min_elevation,
        max_elevation,
        ascent_metres,
        descent_metres,
        cum_ascent_metres: ext_trackpoints[end_idx].cum_ascent_metres,
        cum_descent_metres: ext_trackpoints[end_idx].cum_descent_metres
    }
}
