use std::{error::Error, path::Path};

use rust_xlsxwriter::{
    Color, ExcelDateTime, Format, FormatAlign, FormatBorder, FormatPattern, Url, Workbook,
    Worksheet,
};
use time::{Duration, OffsetDateTime};

use crate::{
    formatting::to_local_date,
    model::{EnrichedGpx, EnrichedTrackPoint},
    stage::{StageList, StageType},
};

const DATE_COLUMN_WIDTH: f64 = 18.0;
const DURATION_COLUMN_WIDTH: f64 = 9.0;
const LAT_LON_COLUMN_WIDTH: f64 = 9.0;
const LINKED_LAT_LON_COLUMN_WIDTH: f64 = 18.0;
const LOCATION_DESCRIPTION_COLUMN_WIDTH: f64 = 18.0;
const ELEVATION_COLUMN_WIDTH_WITH_UNITS: f64 = 12.0;
const METRES_COLUMN_WIDTH_WITH_UNITS: f64 = 11.0;
const METRES_COLUMN_WIDTH: f64 = 8.0;
const KILOMETRES_COLUMN_WIDTH_WITH_UNITS: f64 = 14.0;
const KILOMETRES_COLUMN_WIDTH: f64 = 8.0;
const SPEED_COLUMN_WIDTH_WITH_UNITS: f64 = 14.0;
const SPEED_COLUMN_WIDTH: f64 = 8.0;

pub fn write_summary_file<'gpx>(
    summary_filename: &Path,
    gpx: &EnrichedGpx,
    stages: &StageList<'gpx>,
) -> Result<(), Box<dyn Error>> {
    print!("Writing file {:?}", &summary_filename);

    let mut workbook = Workbook::new();

    // This will appear as the first sheet in the workbook.
    let stages_ws = workbook.add_worksheet();
    stages_ws.set_name("Stages")?;
    write_stages(stages, stages_ws)?;

    // This will appear as the second sheet in the workbook.
    let tp_ws = workbook.add_worksheet();
    tp_ws.set_name("Track Points")?;
    write_trackpoints(&gpx.points, tp_ws)?;

    workbook.save(summary_filename).unwrap();
    let metadata = std::fs::metadata(summary_filename).unwrap();
    println!(", {} Kb", metadata.len() / 1024);
    Ok(())
}

fn write_stages<'gpx>(stages: &StageList<'gpx>, ws: &mut Worksheet) -> Result<(), Box<dyn Error>> {
    let mut fc = FormatControl::new();

    write_header_blank(ws, &fc, (0, 0))?;
    write_header(ws, &fc, (1, 0), "Stage")?;
    fc.increment_column();

    write_header_blank(ws, &fc, (0, 1))?;
    write_header(ws, &fc, (1, 1), "Type")?;
    fc.increment_column();

    write_header_merged(ws, &fc, (0, 2), (0, 5), "Stage Location")?;
    write_header(ws, &fc, (1, 2), "Lat")?;
    write_header(ws, &fc, (1, 3), "Lon")?;
    write_header(ws, &fc, (1, 4), "Map")?;
    write_header(ws, &fc, (1, 5), "Description")?;
    ws.set_column_width(2, LAT_LON_COLUMN_WIDTH)?;
    ws.set_column_width(3, LAT_LON_COLUMN_WIDTH)?;
    ws.set_column_width(4, LINKED_LAT_LON_COLUMN_WIDTH)?;
    ws.set_column_width(5, LOCATION_DESCRIPTION_COLUMN_WIDTH)?;
    fc.increment_column();

    write_header_merged(ws, &fc, (0, 6), (0, 7), "Start Time")?;
    write_header(ws, &fc, (1, 6), "UTC")?;
    write_header(ws, &fc, (1, 7), "Local")?;
    ws.set_column_width(6, DATE_COLUMN_WIDTH)?;
    ws.set_column_width(7, DATE_COLUMN_WIDTH)?;
    fc.increment_column();

    write_header_merged(ws, &fc, (0, 8), (0, 9), "End Time")?;
    write_header(ws, &fc, (1, 8), "UTC")?;
    write_header(ws, &fc, (1, 9), "Local")?;
    ws.set_column_width(8, DATE_COLUMN_WIDTH)?;
    ws.set_column_width(9, DATE_COLUMN_WIDTH)?;
    fc.increment_column();

    write_header_merged(ws, &fc, (0, 10), (0, 11), "Duration")?;
    write_header(ws, &fc, (1, 10), "hms")?;
    write_header(ws, &fc, (1, 11), "Running")?;
    ws.set_column_width(10, DURATION_COLUMN_WIDTH)?;
    ws.set_column_width(11, DURATION_COLUMN_WIDTH)?;
    fc.increment_column();

    write_header_merged(ws, &fc, (0, 12), (0, 13), "Distance (km)")?;
    write_header(ws, &fc, (1, 12), "Stage")?;
    write_header(ws, &fc, (1, 13), "Running")?;
    ws.set_column_width(12, KILOMETRES_COLUMN_WIDTH)?;
    ws.set_column_width(13, METRES_COLUMN_WIDTH)?;
    fc.increment_column();

    write_header_merged(ws, &fc, (0, 14), (0, 15), "Avg Speed (kmh)")?;
    write_header(ws, &fc, (1, 14), "Stage")?;
    write_header(ws, &fc, (1, 15), "Running")?;
    ws.set_column_width(14, SPEED_COLUMN_WIDTH)?;
    ws.set_column_width(15, SPEED_COLUMN_WIDTH)?;
    fc.increment_column();

    write_header_merged(ws, &fc, (0, 16), (0, 17), "Ascent (m)")?;
    write_header(ws, &fc, (1, 16), "Stage")?;
    write_header(ws, &fc, (1, 17), "Running")?;
    ws.set_column_width(16, METRES_COLUMN_WIDTH)?;
    ws.set_column_width(17, METRES_COLUMN_WIDTH)?;
    fc.increment_column();

    write_header_merged(ws, &fc, (0, 18), (0, 19), "Descent (m)")?;
    write_header(ws, &fc, (1, 18), "Stage")?;
    write_header(ws, &fc, (1, 19), "Running")?;
    ws.set_column_width(18, METRES_COLUMN_WIDTH)?;
    ws.set_column_width(19, METRES_COLUMN_WIDTH)?;
    fc.increment_column();

    write_header_merged(ws, &fc, (0, 20), (0, 25), "Minimum Elevation")?;
    write_header(ws, &fc, (1, 20), "Elevation (m)")?;
    write_header(ws, &fc, (1, 21), "Distance (km)")?;
    write_header(ws, &fc, (1, 22), "Time (local)")?;
    write_header(ws, &fc, (1, 23), "Lat")?;
    write_header(ws, &fc, (1, 24), "Lon")?;
    write_header(ws, &fc, (1, 25), "Map")?;
    ws.set_column_width(20, ELEVATION_COLUMN_WIDTH_WITH_UNITS)?;
    ws.set_column_width(21, KILOMETRES_COLUMN_WIDTH_WITH_UNITS)?;
    ws.set_column_width(22, DATE_COLUMN_WIDTH)?;
    ws.set_column_width(23, LAT_LON_COLUMN_WIDTH)?;
    ws.set_column_width(24, LAT_LON_COLUMN_WIDTH)?;
    ws.set_column_width(25, LINKED_LAT_LON_COLUMN_WIDTH)?;
    fc.increment_column();

    write_header_merged(ws, &fc, (0, 26), (0, 31), "Maximum Elevation (m)")?;
    write_header(ws, &fc, (1, 26), "Elevation")?;
    write_header(ws, &fc, (1, 27), "Distance (km)")?;
    write_header(ws, &fc, (1, 28), "Time (local)")?;
    write_header(ws, &fc, (1, 29), "Lat")?;
    write_header(ws, &fc, (1, 30), "Lon")?;
    write_header(ws, &fc, (1, 31), "Map")?;
    ws.set_column_width(26, ELEVATION_COLUMN_WIDTH_WITH_UNITS)?;
    ws.set_column_width(27, KILOMETRES_COLUMN_WIDTH_WITH_UNITS)?;
    ws.set_column_width(28, DATE_COLUMN_WIDTH)?;
    ws.set_column_width(29, LAT_LON_COLUMN_WIDTH)?;
    ws.set_column_width(30, LAT_LON_COLUMN_WIDTH)?;
    ws.set_column_width(31, LINKED_LAT_LON_COLUMN_WIDTH)?;
    fc.increment_column();

    write_header_merged(ws, &fc, (0, 32), (0, 35), "Max Speed (kmh)")?;
    write_header(ws, &fc, (1, 32), "Speed")?;
    write_header(ws, &fc, (1, 33), "Lat")?;
    write_header(ws, &fc, (1, 34), "Lon")?;
    write_header(ws, &fc, (1, 35), "Map")?;
    ws.set_column_width(32, SPEED_COLUMN_WIDTH)?;
    ws.set_column_width(33, LAT_LON_COLUMN_WIDTH)?;
    ws.set_column_width(34, LAT_LON_COLUMN_WIDTH)?;
    ws.set_column_width(35, LINKED_LAT_LON_COLUMN_WIDTH)?;
    fc.increment_column();
    
    write_header_merged(ws, &fc, (0, 36), (0, 40), "Heart Rate")?;
    write_header(ws, &fc, (1, 36), "Avg")?;
    write_header(ws, &fc, (1, 37), "Max")?;
    write_header(ws, &fc, (1, 38), "Lat")?;
    write_header(ws, &fc, (1, 39), "Lon")?;
    write_header(ws, &fc, (1, 40), "Map")?;
    ws.set_column_width(39, LAT_LON_COLUMN_WIDTH)?;
    ws.set_column_width(39, LAT_LON_COLUMN_WIDTH)?;
    ws.set_column_width(40, LINKED_LAT_LON_COLUMN_WIDTH)?;
    fc.increment_column();
    
    write_header_merged(ws, &fc, (0, 41), (0, 45), "Temp °C")?;
    write_header(ws, &fc, (1, 41), "Avg")?;
    write_header(ws, &fc, (1, 42), "Max")?;
    write_header(ws, &fc, (1, 43), "Lat")?;
    write_header(ws, &fc, (1, 44), "Lon")?;
    write_header(ws, &fc, (1, 45), "Map")?;
    ws.set_column_width(43, LAT_LON_COLUMN_WIDTH)?;
    ws.set_column_width(44, LAT_LON_COLUMN_WIDTH)?;
    ws.set_column_width(45, LINKED_LAT_LON_COLUMN_WIDTH)?;
    fc.increment_column();

    write_header_merged(ws, &fc, (0, 46), (0, 48), "Track Points")?;
    write_header(ws, &fc, (1, 46), "First")?;
    write_header(ws, &fc, (1, 47), "Last")?;
    write_header(ws, &fc, (1, 48), "Count")?;

    // Regenerate this so the formatting starts at the right point.
    let mut fc = FormatControl::new();
    let mut row = 2;
    for (idx, stage) in stages.iter().enumerate() {
        fc.reset_column();

        write_integer(ws, &fc, (row, 0), (idx + 1) as u32)?;
        fc.increment_column();

        write_string(ws, &fc, (row, 1), &stage.stage_type.to_string())?;
        fc.increment_column();

        write_lat_lon(
            ws,
            &fc,
            (row, 2),
            (stage.start.lat, stage.start.lon),
            Hyperlink::Yes,
        )?;
        write_location_description(ws, &fc, (row, 5), &stage.start.location)?;
        fc.increment_column();

        write_utc_date(ws, &fc, (row, 6), stage.start.time)?;
        write_utc_date_as_local(ws, &fc, (row, 7), stage.start.time)?;
        fc.increment_column();

        write_utc_date(ws, &fc, (row, 8), stage.end.time)?;
        write_utc_date_as_local(ws, &fc, (row, 9), stage.end.time)?;
        fc.increment_column();

        write_duration(ws, &fc, (row, 10), stage.duration())?;
        write_duration(ws, &fc, (row, 11), stage.duration())?;
        fc.increment_column();

        if stage.stage_type == StageType::Moving {
            write_kilometres(ws, &fc, (row, 12), stage.distance_km())?;
            write_kilometres(ws, &fc, (row, 13), stage.running_distance_km())?;
            fc.increment_column();
            write_speed(ws, &fc, (row, 14), stage.average_speed_kmh())?;
            write_speed(ws, &fc, (row, 15), stage.running_average_speed_kmh())?;
            fc.increment_column();
            write_metres(ws, &fc, (row, 16), stage.ascent_metres())?;
            write_metres(ws, &fc, (row, 17), stage.running_ascent_metres())?;
            fc.increment_column();
            write_metres(ws, &fc, (row, 18), stage.descent_metres())?;
            write_metres(ws, &fc, (row, 19), stage.running_descent_metres())?;
            fc.increment_column();
            write_elevation_data(ws, &fc, (row, 20), stage.min_elevation)?;
            fc.increment_column();
            write_elevation_data(ws, &fc, (row, 26), stage.max_elevation)?;
            fc.increment_column();
            write_max_speed_data(ws, &fc, (row, 32), stage.max_speed)?;
        } else {
            // Write blanks so that the banding formatting is applied.
            for col in 12..=34 {
                write_blank(ws, &fc, (row, col))?;
            }
        }

        fc.increment_column();
        // heart rate here

        fc.increment_column();
        // temp here

        fc.increment_column();
        write_trackpoint_hyperlink(ws, &fc, (row, 46), stage.start.index)?;
        write_trackpoint_hyperlink(ws, &fc, (row, 47), stage.end.index)?;
        write_integer(ws, &fc, (row, 48), (stage.end.index - stage.start.index + 1).try_into()?)?;

        row += 1;
        fc.increment_row();
    }

    ws.set_freeze_panes(2, 0)?;

    // Now write an overall summary row.
    let mut fc = FormatControl::new();
    row += 2;
    write_string(ws, &fc, (row, 5), "SUMMARY")?;
    fc.increment_column();
    write_utc_date(ws, &fc, (row, 6), stages.start_time())?;
    write_utc_date_as_local(ws, &fc, (row, 7), stages.start_time())?;
    fc.increment_column();
    write_utc_date(ws, &fc, (row, 8), stages.end_time())?;
    write_utc_date_as_local(ws, &fc, (row, 9), stages.end_time())?;
    fc.increment_column();
    write_string(ws, &fc, (row, 10), "Total")?;
    write_duration(ws, &fc, (row, 11), stages.duration())?;
    write_string(ws, &fc, (row + 1, 10), "Stopped")?;
    write_duration(ws, &fc, (row + 1, 11), stages.total_stopped_time())?;
    write_string(ws, &fc, (row + 2, 10), "Moving")?;
    write_duration(ws, &fc, (row + 2, 11), stages.total_moving_time())?;
    fc.increment_column();
    write_blank(ws, &fc, (row, 12))?;
    write_kilometres(ws, &fc, (row, 13), stages.distance_km())?;
    fc.increment_column();
    write_string(ws, &fc, (row, 14), "Overall")?;
    write_speed(ws, &fc, (row, 15), stages.average_overall_speed())?;
    write_string(ws, &fc, (row + 1, 14), "Moving")?;
    write_speed(ws, &fc, (row + 1, 15), stages.average_moving_speed())?;
    fc.increment_column();
    write_blank(ws, &fc, (row, 16))?;
    write_metres(ws, &fc, (row, 17), stages.total_ascent_metres())?;
    fc.increment_column();
    write_blank(ws, &fc, (row, 18))?;
    write_metres(ws, &fc, (row, 19), stages.total_descent_metres())?;
    fc.increment_column();
    write_elevation_data(ws, &fc, (row, 20), stages.min_elevation())?;
    fc.increment_column();
    write_elevation_data(ws, &fc, (row, 26), stages.max_elevation())?;
    fc.increment_column();
    write_max_speed_data(ws, &fc, (row, 32), stages.max_speed())?;
    fc.increment_column();
    // Heart rate here
    fc.increment_column();
    // Temperature here
    fc.increment_column();
    write_trackpoint_hyperlink(ws, &fc, (row, 46), stages.first_point().index)?;
    write_trackpoint_hyperlink(ws, &fc, (row, 47), stages.last_point().index)?;
    write_integer(ws, &fc, (row, 48), (stages.last_point().index - stages.first_point().index + 1).try_into()?)?;

    Ok(())
}

fn write_trackpoints(
    points: &[EnrichedTrackPoint],
    ws: &mut Worksheet,
) -> Result<(), Box<dyn Error>> {
    let mut fc = FormatControl::new();

    write_header_blank(ws, &fc, (0, 0))?;
    write_header(ws, &fc, (1, 0), "Index")?;
    fc.increment_column();

    write_header_merged(ws, &fc, (0, 1), (0, 4), "Time")?;
    write_header(ws, &fc, (1, 1), "UTC")?;
    write_header(ws, &fc, (1, 2), "Local")?;
    write_header(ws, &fc, (1, 3), "Delta")?;
    write_header(ws, &fc, (1, 4), "Running")?;
    ws.set_column_width(1, DATE_COLUMN_WIDTH)?;
    ws.set_column_width(2, DATE_COLUMN_WIDTH)?;
    ws.set_column_width(3, DURATION_COLUMN_WIDTH)?;
    ws.set_column_width(4, DURATION_COLUMN_WIDTH)?;
    fc.increment_column();

    write_header_merged(ws, &fc, (0, 5), (0, 8), "Location")?;
    write_header(ws, &fc, (1, 5), "Lat")?;
    write_header(ws, &fc, (1, 6), "Lon")?;
    write_header(ws, &fc, (1, 7), "Map")?;
    write_header(ws, &fc, (1, 8), "Description")?;
    ws.set_column_width(5, LAT_LON_COLUMN_WIDTH)?;
    ws.set_column_width(6, LAT_LON_COLUMN_WIDTH)?;
    ws.set_column_width(7, LINKED_LAT_LON_COLUMN_WIDTH)?;
    ws.set_column_width(8, LOCATION_DESCRIPTION_COLUMN_WIDTH)?;
    fc.increment_column();

    write_header_merged(ws, &fc, (0, 9), (0, 12), "Elevation (m)")?;
    write_header(ws, &fc, (1, 9), "Height")?;
    write_header(ws, &fc, (1, 10), "Delta")?;
    write_header(ws, &fc, (1, 11), "Running Ascent")?;
    write_header(ws, &fc, (1, 12), "Running Descent")?;
    ws.set_column_width(9, METRES_COLUMN_WIDTH_WITH_UNITS)?;
    ws.set_column_width(10, METRES_COLUMN_WIDTH_WITH_UNITS)?;
    ws.set_column_width(11, KILOMETRES_COLUMN_WIDTH_WITH_UNITS)?;
    ws.set_column_width(12, KILOMETRES_COLUMN_WIDTH_WITH_UNITS)?;
    fc.increment_column();

    write_header_merged(ws, &fc, (0, 13), (0, 14), "Distance")?;
    write_header(ws, &fc, (1, 13), "Delta (m)")?;
    write_header(ws, &fc, (1, 14), "Running (km)")?;
    ws.set_column_width(13, METRES_COLUMN_WIDTH_WITH_UNITS)?;
    ws.set_column_width(14, KILOMETRES_COLUMN_WIDTH_WITH_UNITS)?;
    fc.increment_column();

    write_header_blank(ws, &fc, (0, 15))?;
    write_header(ws, &fc, (1, 15), "Speed (kmh)")?;
    ws.set_column_width(15, SPEED_COLUMN_WIDTH_WITH_UNITS)?;

    // Regenerate this so the formatting starts at the right point.
    let mut fc = FormatControl::new();
    let mut row = 2;
    for p in points {
        fc.reset_column();

        write_integer(ws, &fc, (row, 0), p.index as u32)?;
        fc.increment_column();

        write_utc_date(ws, &fc, (row, 1), p.time)?;
        write_utc_date_as_local(ws, &fc, (row, 2), p.time)?;
        write_duration(ws, &fc, (row, 3), p.delta_time)?;
        write_duration(ws, &fc, (row, 4), p.running_delta_time)?;
        fc.increment_column();

        write_lat_lon(ws, &fc, (row, 5), (p.lat, p.lon), Hyperlink::Yes)?;
        write_location_description(ws, &fc, (row, 8), &p.location)?;
        fc.increment_column();

        write_metres(ws, &fc, (row, 9), p.ele)?;
        write_metres(ws, &fc, (row, 10), p.ele_delta_metres)?;
        write_metres(ws, &fc, (row, 11), p.running_ascent_metres)?;
        write_metres(ws, &fc, (row, 12), p.running_descent_metres)?;
        fc.increment_column();

        write_metres(ws, &fc, (row, 13), p.delta_metres)?;
        write_kilometres(ws, &fc, (row, 14), p.running_metres / 1000.0)?;
        fc.increment_column();

        write_speed(ws, &fc, (row, 15), p.speed_kmh)?;

        row += 1;
        fc.increment_row();
    }

    ws.autofilter(1, 0, row - 1, 15)?;
    ws.set_freeze_panes(2, 0)?;

    Ok(())
}

// Utility functions.

/// Writes formatted minor header text to a single cell.
fn write_header(
    ws: &mut Worksheet,
    fc: &FormatControl,
    rc: (u32, u16),
    heading: &str,
) -> Result<(), Box<dyn Error>> {
    ws.write_string_with_format(rc.0, rc.1, heading, &fc.minor_header_format())?;
    Ok(())
}

/// Writes a blank minor header cell.
fn write_header_blank(
    ws: &mut Worksheet,
    fc: &FormatControl,
    rc: (u32, u16),
) -> Result<(), Box<dyn Error>> {
    ws.write_blank(rc.0, rc.1, &fc.minor_header_format())?;
    Ok(())
}

/// Writes formatted minor header text to a range of merged cells.
fn write_header_merged(
    ws: &mut Worksheet,
    fc: &FormatControl,
    start_rc: (u32, u16),
    end_rc: (u32, u16),
    heading: &str,
) -> Result<(), Box<dyn Error>> {
    ws.merge_range(
        start_rc.0,
        start_rc.1,
        end_rc.0,
        end_rc.1,
        heading,
        &fc.minor_header_format(),
    )?;
    Ok(())
}

/// Writes an integer.
fn write_integer(
    ws: &mut Worksheet,
    fc: &FormatControl,
    rc: (u32, u16),
    value: u32,
) -> Result<(), Box<dyn Error>> {
    ws.write_number_with_format(rc.0, rc.1, value, &fc.integer_format())?;
    Ok(())
}

/// Writes an integer.
fn write_string(
    ws: &mut Worksheet,
    fc: &FormatControl,
    rc: (u32, u16),
    value: &str,
) -> Result<(), Box<dyn Error>> {
    ws.write_string_with_format(rc.0, rc.1, value, &fc.string_format())?;
    Ok(())
}

/// Formats 'utc_date' into a string like "2024-09-01T05:10:44Z".
/// This is the format that GPX files contain.
fn write_utc_date(
    ws: &mut Worksheet,
    fc: &FormatControl,
    rc: (u32, u16),
    utc_date: OffsetDateTime,
) -> Result<(), Box<dyn Error>> {
    assert!(utc_date.offset().is_utc());
    let excel_date = date_to_excel_date(utc_date)?;
    ws.write_with_format(rc.0, rc.1, &excel_date, &fc.utc_date_format())?;
    Ok(())
}

/// Converts 'utc_date' to a local date and then formats it into
/// a string like "2024-09-01 05:10:44".
fn write_utc_date_as_local(
    ws: &mut Worksheet,
    fc: &FormatControl,
    rc: (u32, u16),
    utc_date: OffsetDateTime,
) -> Result<(), Box<dyn Error>> {
    assert!(utc_date.offset().is_utc());
    let excel_date = date_to_excel_date(to_local_date(utc_date))?;
    ws.write_with_format(rc.0, rc.1, &excel_date, &&fc.local_date_format())?;
    Ok(())
}

fn date_to_excel_date(date: OffsetDateTime) -> Result<ExcelDateTime, Box<dyn Error>> {
    let excel_date = ExcelDateTime::from_ymd(
        date.year().try_into()?,
        date.month().try_into()?,
        date.day(),
    )?;

    // Clamp these values to the values Excel will take.
    // Issue a warning if out of bounds.
    let mut hour = date.hour() as u16;
    if hour > 23 {
        eprintln!("WARNING: Clamped hour value of {hour} to 23 for Excel compatibility");
        hour = 23;
    }

    let mut minute = date.minute();
    if minute > 59 {
        eprintln!("WARNING: Clamped minute value of {minute} to 59 for Excel compatibility");
        minute = 59;
    }

    let mut second = date.second();
    if second > 59 {
        eprintln!("WARNING: Clamped second value of {second} to 59 for Excel compatibility");
        second = 59;
    }

    Ok(excel_date.and_hms(hour, minute, second)?)
}

fn write_duration(
    ws: &mut Worksheet,
    fc: &FormatControl,
    rc: (u32, u16),
    duration: Duration,
) -> Result<(), Box<dyn Error>> {
    let excel_duration = duration_to_excel_date(duration)?;
    ws.write_with_format(rc.0, rc.1, excel_duration, &fc.duration_format())?;
    Ok(())
}

fn duration_to_excel_date(duration: Duration) -> Result<ExcelDateTime, Box<dyn Error>> {
    const SECONDS_PER_MINUTE: u32 = 60;
    const SECONDS_PER_HOUR: u32 = SECONDS_PER_MINUTE * 60;

    let mut all_secs: u32 = duration.as_seconds_f64() as u32;
    let hours: u16 = (all_secs / SECONDS_PER_HOUR).try_into()?;
    all_secs = all_secs - (hours as u32 * SECONDS_PER_HOUR);

    let minutes: u8 = (all_secs / SECONDS_PER_MINUTE).try_into()?;
    all_secs = all_secs - (minutes as u32 * SECONDS_PER_MINUTE);

    let seconds: u16 = all_secs.try_into()?;

    Ok(ExcelDateTime::from_hms(hours, minutes, seconds)?)
}

/// Writes an elevation data block (min or max) as found on the Stages tab.
fn write_elevation_data(
    ws: &mut Worksheet,
    fc: &FormatControl,
    rc: (u32, u16),
    point: &EnrichedTrackPoint
) -> Result<(), Box<dyn Error>> {
    write_metres(ws, &fc, (rc.0, rc.1), point.ele)?;
    write_kilometres(ws, &fc, (rc.0, rc.1 + 1), point.running_metres / 1000.0)?;
    write_utc_date_as_local(ws, &fc, (rc.0, rc.1 + 2), point.time)?;
    write_lat_lon(ws, &fc, (rc.0, rc.1 + 3), (point.lat, point.lon), Hyperlink::Yes)?;
    Ok(())
}

/// Writes an max speed data block (min or max) as found on the Stages tab.
fn write_max_speed_data(
    ws: &mut Worksheet,
    fc: &FormatControl,
    rc: (u32, u16),
    point: &EnrichedTrackPoint
) -> Result<(), Box<dyn Error>> {
    write_speed(ws, &fc, (rc.0, rc.1), point.speed_kmh)?;
    write_lat_lon(ws, &fc, (rc.0, rc.1 + 1), (point.lat, point.lon), Hyperlink::Yes)?;
    Ok(())
}

enum Hyperlink {
    Yes,
    No,
}

/// Writes a lat-lon pair with the lat in the first cell as specified
/// by 'rc' and the lon in the next column. If 'hyperlink' is yes then
/// a hyperlink to Google Maps is written into the third column.
fn write_lat_lon(
    ws: &mut Worksheet,
    fc: &FormatControl,
    rc: (u32, u16),
    lat_lon: (f64, f64),
    hyperlink: Hyperlink,
) -> Result<(), Box<dyn Error>> {
    let format = fc.lat_lon_format().set_font_color(Color::Black);

    ws.write_number_with_format(rc.0, rc.1, lat_lon.0, &format)?;
    ws.write_number_with_format(rc.0, rc.1 + 1, lat_lon.1, &format)?;

    // See https://developers.google.com/maps/documentation/urls/get-started

    match hyperlink {
        Hyperlink::Yes => {
            let url = Url::new(format!(
                "https://www.google.com/maps/search/?api=1&query={:.6},{:.6}",
                lat_lon.0, lat_lon.1
            ))
            .set_text(format!("{:.6}, {:.6}", lat_lon.0, lat_lon.1));

            // TODO: Font still blue.
            ws.write_url_with_format(rc.0, rc.1 + 2, url, &format)?;
        }
        Hyperlink::No => {}
    };

    Ok(())
}

/// Writes a hyperlink to the trackpoints sheet.
fn write_trackpoint_hyperlink(
    ws: &mut Worksheet,
    fc: &FormatControl,
    rc: (u32, u16),
    trackpoint_index: usize
) -> Result<(), Box<dyn Error>> {
    let format = fc.integer_format().set_font_color(Color::Black);
    let url = Url::new(format!(
        "internal:'Track Points'!A{}",
        trackpoint_index + 3    // allow for the heading.
    ))
    .set_text(trackpoint_index.to_string());

    ws.write_url_with_format(rc.0, rc.1, url, &format)?;
    Ok(())
}

fn write_location_description(
    ws: &mut Worksheet,
    fc: &FormatControl,
    rc: (u32, u16),
    location: &Option<String>,
) -> Result<(), Box<dyn Error>> {
    if let Some(location) = location {
        if !location.is_empty() {
            ws.write_string_with_format(rc.0, rc.1, location, &fc.location_format())?;
        } else {
            write_blank(ws, fc, rc)?;
        }
    } else {
        write_blank(ws, fc, rc)?;
    }

    Ok(())
}

fn write_blank(
    ws: &mut Worksheet,
    fc: &FormatControl,
    rc: (u32, u16),
) -> Result<(), Box<dyn Error>> {
    ws.write_blank(rc.0, rc.1, &fc.string_format())?;
    Ok(())
}

fn write_metres(
    ws: &mut Worksheet,
    fc: &FormatControl,
    rc: (u32, u16),
    metres: f64,
) -> Result<(), Box<dyn Error>> {
    ws.write_number_with_format(rc.0, rc.1, metres, &fc.metres_format())?;
    // TODO: Use conditional formatting to indicate negatives?
    Ok(())
}

fn write_kilometres(
    ws: &mut Worksheet,
    fc: &FormatControl,
    rc: (u32, u16),
    kilometres: f64,
) -> Result<(), Box<dyn Error>> {
    ws.write_number_with_format(rc.0, rc.1, kilometres, &fc.kilometres_format())?;
    Ok(())
}

fn write_speed(
    ws: &mut Worksheet,
    fc: &FormatControl,
    rc: (u32, u16),
    speed: f64,
) -> Result<(), Box<dyn Error>> {
    ws.write_number_with_format(rc.0, rc.1, speed, &fc.speed_format())?;
    Ok(())
}

struct FormatControl {
    current_color: Color,
    row_alt: bool,
}

impl FormatControl {
    const COLOR1: Color = Color::Theme(3, 1);
    const COLOR2: Color = Color::Theme(2, 1);

    fn new() -> Self {
        Self {
            current_color: Self::COLOR1,
            row_alt: false,
        }
    }

    fn increment_row(&mut self) {
        self.row_alt = !self.row_alt;
    }

    fn reset_column(&mut self) {
        self.current_color = Self::COLOR1;
    }

    fn increment_column(&mut self) {
        if self.current_color == Self::COLOR1 {
            self.current_color = Self::COLOR2;
        } else {
            self.current_color = Self::COLOR1;
        }
    }

    fn minor_header_format(&self) -> Format {
        Format::new()
            .set_bold()
            .set_font_color(Color::Black)
            .set_border(FormatBorder::Thin)
            .set_border_color(Color::Gray)
            .set_align(FormatAlign::Center)
            .set_pattern(FormatPattern::Solid)
            .set_background_color(self.current_color)
    }

    fn speed_format(&self) -> Format {
        let format = Format::new().set_num_format("0.##");
        self.apply_banding(format)
    }

    fn lat_lon_format(&self) -> Format {
        let format = Format::new().set_num_format("0.000000");
        self.apply_banding(format)
    }

    fn integer_format(&self) -> Format {
        let format = Format::new().set_num_format("0");
        self.apply_banding(format)
    }

    fn string_format(&self) -> Format {
        let format = Format::new();
        self.apply_banding(format)
    }

    fn location_format(&self) -> Format {
        let format = Format::new().set_align(FormatAlign::Left);
        self.apply_banding(format)
    }

    fn utc_date_format(&self) -> Format {
        let format = Format::new().set_num_format("yyyy-mm-ddThh:mm:ssZ");
        self.apply_banding(format)
    }

    fn local_date_format(&self) -> Format {
        let format = Format::new().set_num_format("yyyy-mm-dd hh:mm:ss");
        self.apply_banding(format)
    }

    fn duration_format(&self) -> Format {
        let format = Format::new().set_num_format("hh:mm:ss");
        self.apply_banding(format)
    }

    fn metres_format(&self) -> Format {
        let format = Format::new().set_num_format("0.##");
        self.apply_banding(format)
    }

    fn kilometres_format(&self) -> Format {
        let format = Format::new().set_num_format("0.000");
        self.apply_banding(format)
    }

    /// Helper method.
    fn apply_banding(&self, format: Format) -> Format {
        let mut format = format;
        if !self.row_alt {
            format = format.set_background_color(self.current_color);
        }
        format
    }
}
