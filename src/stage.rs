//! Contains the functionality relating to Stages.
//! Detecting these is quite a bit of work. Once we get
//! the Stages determined we can calculate a lot of
//! other metrics fairly easily.

use core::{fmt, slice};
use std::ops::Index;

use geo::{point, GeodesicDistance};
use time::{Duration, OffsetDateTime};

use crate::model::{EnrichedGpx, EnrichedTrackPoint};

/// Calculates speed in kmh from metres and seconds.
pub fn speed_kmh(metres: f64, seconds: f64) -> f64 {
    (metres / seconds) * 3.6
}

/// Calculates speed in kmh from metres and a Duration.
pub fn speed_kmh_from_duration(metres: f64, time: Duration) -> f64 {
    speed_kmh(metres, time.as_seconds_f64())
}

/// These are the parameters that control the 'Stage-finding'
/// algorithm.
pub struct StageDetectionParameters {
    /// You are considered "Stopped" if your speed drops below this.
    /// So that means a dead-stop.
    pub stopped_speed_kmh: f64,

    // You are considered to be "Moving Again" the first time your
    // speed goes above this. This is above a walking speed, so you
    // are probably riding again.
    pub resume_speed_kmh: f64,

    /// We want to eliminate tiny Stages caused by noisy data, for
    /// example these can occur when just starting off again.
    /// So set the minimum length of a stage, in seconds.
    pub min_duration_seconds: f64,
}

/// Represents a stage from a GPX track. The stage can represent
/// you moving, or stopped.
#[derive(Debug)]
pub struct Stage<'gpx> {
    pub stage_type: StageType,
    pub start: &'gpx EnrichedTrackPoint,
    pub end: &'gpx EnrichedTrackPoint,
    pub min_elevation: &'gpx EnrichedTrackPoint,
    pub max_elevation: &'gpx EnrichedTrackPoint,
    pub max_speed: &'gpx EnrichedTrackPoint,
    // The first point in the track. We could pass it into
    // the relevant methods, but storing it works ok too.
    // We will need this to calculate some metrics later.
    pub track_start_point: &'gpx EnrichedTrackPoint,
}

/// The type of a Stage.
#[derive(Debug, PartialEq, Eq)]
pub enum StageType {
    Moving,
    Stopped,
}

impl fmt::Display for StageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StageType::Moving => write!(f, "Moving"),
            StageType::Stopped => write!(f, "Stopped"),
        }
    }
}

impl<'gpx> Stage<'gpx> {
    /// Returns the duration of the stage.
    pub fn duration(&self) -> Duration {
        self.end.time - self.start.time
    }

    /// Returns the running duration to the end of the stage from
    /// the 'starting_track_point' (normally will be the first point in the track).
    pub fn running_duration(&self) -> Duration {
        self.end.time - self.track_start_point.time
    }

    /// Returns the distance (length) of the stage, in metres.
    pub fn distance_metres(&self) -> f64 {
        self.end.running_metres - self.start.running_metres
    }

    /// Returns the distance of the stage, in km.
    pub fn distance_km(&self) -> f64 {
        self.distance_metres() / 1000.0
    }

    /// Returns the cumulative distance to the end of the stage
    /// from the start of the entire track.
    pub fn running_distance_km(&self) -> f64 {
        self.end.running_metres / 1000.0
    }

    /// Returns the average speed of the stage, in kmh.
    pub fn average_speed_kmh(&self) -> f64 {
        speed_kmh_from_duration(self.distance_metres(), self.duration())
    }

    /// Returns the average speed, calculated over the distance from
    /// the start of the track to the end of the stage.
    pub fn running_average_speed_kmh(&self) -> f64 {
        speed_kmh_from_duration(self.end.running_metres, self.running_duration())
    }

    /// Returns the total ascent in metres over the stage.
    pub fn ascent_metres(&self) -> f64 {
        self.end.running_ascent_metres - self.start.running_ascent_metres
    }

    /// Returns the total ascent to the end of the stage from
    /// the beginning of the track.
    pub fn running_ascent_metres(&self) -> f64 {
        self.end.running_ascent_metres
    }

    /// Returns the total descent in metres over the stage.
    pub fn descent_metres(&self) -> f64 {
        self.end.running_descent_metres - self.start.running_descent_metres
    }

    /// Returns the total descent to the end of the stage from
    /// the beginning of the track.
    pub fn running_descent_metres(&self) -> f64 {
        self.end.running_descent_metres
    }
}

#[derive(Default)]
pub struct StageList<'gpx>(Vec<Stage<'gpx>>);

impl<'gpx> Index<usize> for StageList<'gpx> {
    type Output = Stage<'gpx>;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl<'gpx> StageList<'gpx> {
    // TODO: Implement Iterator properly.
    pub fn iter(&self) -> slice::Iter<Stage> {
        self.0.iter()
    }

    pub fn first_point(&self) -> &EnrichedTrackPoint {
        self.0[0].start
    }

    pub fn last_point(&self) -> &EnrichedTrackPoint {
        self.0[self.len() - 1].end
    }

    pub fn push(&mut self, stage: Stage<'gpx>) {
        self.0.push(stage);
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns the start time of the first Stage.
    pub fn start_time(&self) -> OffsetDateTime {
        self.first_point().time
    }

    /// Returns the end time of the last Stage.
    pub fn end_time(&self) -> OffsetDateTime {
        self.last_point().time
    }

    /// Returns the total duration between the start of the first
    /// stage and the end of the last stage.
    pub fn duration(&self) -> Duration {
        self.end_time() - self.start_time()
    }

    /// Returns the total time Moving across all the stages.
    pub fn total_moving_time(&self) -> Duration {
        self.duration() - self.total_stopped_time()
    }

    /// Returns the total time Stopped across all the stages.
    pub fn total_stopped_time(&self) -> Duration {
        self.0
            .iter()
            .filter_map(|stage| match stage.stage_type {
                StageType::Moving => None,
                StageType::Stopped => Some(stage.duration()),
            })
            .sum()
    }

    /// Returns the total distance of all the stages in metres.
    pub fn distance_metres(&self) -> f64 {
        self.0.iter().map(|s| s.distance_metres()).sum()
    }

    /// Returns the total distance of all the stages in km.
    pub fn distance_km(&self) -> f64 {
        self.distance_metres() / 1000.0
    }

    /// Returns the point of minimum elevation across all the stages.
    pub fn min_elevation(&self) -> &EnrichedTrackPoint {
        self.0
            .iter()
            .map(|stage| &stage.min_elevation)
            .min_by(|a, b| a.ele.total_cmp(&b.ele))
            .unwrap()
    }

    /// Returns the point of maximum elevation across all the stages.
    pub fn max_elevation(&self) -> &EnrichedTrackPoint {
        self.0
            .iter()
            .map(|stage| &stage.max_elevation)
            .max_by(|a, b| a.ele.total_cmp(&b.ele))
            .unwrap()
    }

    /// Returns the total ascent in metres across all the stages.
    pub fn total_ascent_metres(&self) -> f64 {
        self.0.iter().map(|stage| stage.ascent_metres()).sum()
    }

    /// Returns the total descent in metres across all the stages.
    pub fn total_descent_metres(&self) -> f64 {
        self.0.iter().map(|stage| stage.descent_metres()).sum()
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

        gpx.points[idx].running_metres =
            gpx.points[idx - 1].running_metres + gpx.points[idx].delta_metres;
        assert!(gpx.points[idx].running_metres >= 0.0);

        // Time delta. Don't really need this stored, but is handy to spot
        // points that took more than usual when scanning the CSV.
        gpx.points[idx].delta_time = gpx.points[idx].time - gpx.points[idx - 1].time;
        assert!(gpx.points[idx].delta_time.is_positive());

        // Speed. Based on the distance we just calculated.
        gpx.points[idx].speed_kmh =
            speed_kmh_from_duration(gpx.points[idx].delta_metres, gpx.points[idx].delta_time);
        assert!(gpx.points[idx].speed_kmh >= 0.0);

        // How long it took to get here.
        gpx.points[idx].running_delta_time = gpx.points[idx].time - start_time;
        assert!(gpx.points[idx].running_delta_time.is_positive());

        // Ascent and descent.
        let ele_delta_metres = gpx.points[idx].ele - gpx.points[idx - 1].ele;
        gpx.points[idx].ele_delta_metres = ele_delta_metres;

        if ele_delta_metres > 0.0 {
            cum_ascent_metres += ele_delta_metres;
        } else {
            cum_descent_metres += ele_delta_metres.abs();
        }

        gpx.points[idx].running_ascent_metres = cum_ascent_metres;
        assert!(gpx.points[idx].running_ascent_metres >= 0.0);
        gpx.points[idx].running_descent_metres = cum_descent_metres;
        assert!(gpx.points[idx].running_descent_metres >= 0.0);

        p1 = p2;
    }
}

/// Detects the stages in the GPX and returns them as a list.
///
/// Invariants: the first stage starts at TrackPoint 0
/// and goes to TrackPoint N. The next stage starts at
/// Trackpoint N and goes to TrackPoint M. The last stage
/// ends at the last TrackPoint.
///
/// In other words, there are no gaps, all TrackPoints are in a
/// stage, and TrackPoints in the middle will be in two adjacent
/// stages. TrackPoints are cloned as part of this construction.
///
/// A Stage is a Stopped stage if you speed drops below
/// a (very low) limit and does not go above a 'resume_speed'
/// for a 'min_stop_time' length of time.
///
/// All non-Stopped stages are considered Moving stages.
pub fn detect_stages(gpx: &EnrichedGpx, params: StageDetectionParameters) -> StageList {
    if gpx.points.len() < 2 {
        eprintln!("Warning: gpx {:?} does not have any points", gpx.filename);
        return Default::default();
    }

    let mut stages = StageList::default();

    // Note 1: The first TrackPoint always has a speed of 0, but it is unlikely
    // that you are actually in a Stopped stage. However, it's not impossible,
    // see Note 2 for why.

    // Note 2: We need to deal with the slightly bizarre situation where you turn
    // the GPS on and then don't go anywhere for a while - so your first stage
    // may be a Stopped stage!

    // We can get everything we need to create a stage if we have the
    // index of the first and last TrackPoints for that stage.
    let mut start_idx = 0;
    while let Some(stage) = get_next_stage(start_idx, gpx, &params) {
        // The next stage shares an index/TrackPoint with this one.
        start_idx = stage.end.index;
        stages.push(stage);
    }

    // Should include all TrackPoints and start/end indexes overlap.
    assert_eq!(
        stages[0].start.index, 0,
        "Should always start with the first point"
    );
    assert_eq!(
        stages[stages.len() - 1].end.index,
        gpx.points.len() - 1,
        "Should always end with the last point"
    );
    for idx in 0..stages.len() - 1 {
        assert_eq!(
            stages[idx].end.index,
            stages[idx + 1].start.index,
            "Stage boundaries should be shared"
        );
    }

    stages
}

fn get_next_stage<'gpx>(
    start_idx: usize,
    gpx: &'gpx EnrichedGpx,
    params: &StageDetectionParameters,
) -> Option<Stage<'gpx>> {
    // Get this out into a variable to avoid off-by-one errors (hopefully).
    let last_valid_idx = gpx.points.len() - 1;

    // Termination condition, we reached the end of the TrackPoints.
    if start_idx == last_valid_idx {
        return None;
    }

    // This assert exists so the check above can be '==' instead of '>='.
    // More likely to catch off-by-one bugs this way.
    assert!(start_idx < last_valid_idx);

    // We have said that a Stage must be at least this long, so we need to
    // advance this far as a minimum.
    let end_idx = advance_for_duration(gpx, start_idx, last_valid_idx, params.min_duration_seconds);
    assert!(end_idx <= last_valid_idx);
    assert!(end_idx > start_idx, "Empty stages are not allowed");

    if end_idx < last_valid_idx {
        // This is not necessarily true in the case where we exhaust all the TrackPoints.
        assert!(
            (gpx.points[end_idx].time - gpx.points[start_idx].time).as_seconds_f64()
                >= params.min_duration_seconds
        );
    } else {
        // But we can assert this weaker condition as a fallback.
        assert!((gpx.points[end_idx].time - gpx.points[start_idx].time).is_positive());
    }

    // Scan the TrackPoints we just got to determine the StageType.
    let stage_type = if gpx.points[start_idx..=end_idx]
        .iter()
        .any(|p| p.speed_kmh > params.resume_speed_kmh)
    {
        StageType::Moving
    } else {
        StageType::Stopped
    };

    // If we have not consumed all the trackpoints in advance_for_duration() above,
    // then the stage might actually continue past the current end_idx. Keep going
    // until we really find the end. It's possible that this act may consume some or
    // all of the remaining trackpoints.
    let mut end_idx = end_idx;

    if end_idx < last_valid_idx {
        end_idx = match stage_type {
            StageType::Moving => {
                find_stop_index(
                    gpx,
                    end_idx, // Start the scan from the current end that we just found.
                    last_valid_idx,
                    params,
                )
            }
            StageType::Stopped => {
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

    let stage = Stage {
        stage_type,
        start: &gpx.points[start_idx],
        end: &gpx.points[end_idx],
        min_elevation: min_ele,
        max_elevation: max_ele,
        max_speed: find_max_speed(gpx, start_idx, end_idx),
        track_start_point: &gpx.points[0],
    };

    // Just check we created everything correctly.
    assert!(end_idx <= last_valid_idx);
    assert_eq!(stage.start.index, start_idx);
    assert_eq!(stage.end.index, end_idx);
    assert!(stage.end.index > stage.start.index);
    assert!(stage.end.time > stage.start.time);
    assert!(stage.start.index >= stage.track_start_point.index);

    return Some(stage);
}

/// Scans forward through the points until we find a point
/// that is at least 'min_duration_seconds' ahead
/// of the start point.
fn advance_for_duration(
    gpx: &EnrichedGpx,
    start_idx: usize,
    last_valid_idx: usize,
    min_duration_seconds: f64,
) -> usize {
    let start_time = gpx.points[start_idx].time;
    let mut end_index = start_idx + 1;

    while end_index <= last_valid_idx {
        let delta_time = gpx.points[end_index].time - start_time;
        if delta_time.as_seconds_f64() >= min_duration_seconds {
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

/// Within a given range of trackpoints, finds the one with the maximum
/// speed.
fn find_max_speed<'gpx>(
    gpx: &'gpx EnrichedGpx,
    start_idx: usize,
    end_idx: usize,
) -> &'gpx EnrichedTrackPoint {
    let mut max = &gpx.points[start_idx];

    for tp in &gpx.points[start_idx..=end_idx] {
        if tp.speed_kmh > max.speed_kmh {
            max = tp;
        }
    }

    max
}

/// A Moving stage is ended when we stop. This occurs when we drop below the
/// 'stopped_speed_kmh' and do not attain 'resume_speed_kmh' for at least
/// 'min_duration_seconds'. Find the index of that point.
fn find_stop_index(
    gpx: &EnrichedGpx,
    start_idx: usize,
    last_valid_idx: usize,
    params: &StageDetectionParameters,
) -> usize {
    let mut end_idx = start_idx + 1;

    while end_idx <= last_valid_idx {
        // Find the first time we drop below 'stopped_speed_kmh'
        while end_idx <= last_valid_idx && gpx.points[end_idx].speed_kmh > params.stopped_speed_kmh
        {
            end_idx += 1;
        }

        // It's possible we exhausted all the TrackPoints - we were in a moving
        // Stage that went right to the end of the track. Note that the line
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
        // end for this current stage (which is a Moving Stage, remember).
        let stop_duration = gpx.points[end_idx].time - possible_stop_time;
        if stop_duration.as_seconds_f64() >= params.min_duration_seconds {
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

/// A Stopped stage is ended when we find the first TrackPoint
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
