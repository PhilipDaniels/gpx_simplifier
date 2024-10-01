use std::{error::Error, path::Path};

use logging_timer::time;
use rust_xlsxwriter::{
    Color, ExcelDateTime, Format, FormatAlign, FormatBorder, FormatPattern, Url, Workbook,
    Worksheet,
};
use time::{Duration, OffsetDateTime};

use crate::{
    args::Hyperlink,
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
const HEART_RATE_WIDTH_WITH_UNITS: f64 = 15.0;
const TEMPERATURE_COLUMN_WIDTH_WITH_UNITS: f64 = 10.0;
const CADENCE_COLUMN_WIDTH_WITH_UNITS: f64 = 13.0;

/// Builds the Workbook that is used for the summary.
#[time]
pub fn create_summary_xlsx<'gpx>(
    trackpoint_hyperlinks: Hyperlink,
    gpx: &EnrichedGpx,
    stages: &StageList<'gpx>,
) -> Result<Workbook, Box<dyn Error>> {
    let mut workbook = Workbook::new();

    // This will appear as the first sheet in the workbook.
    let stages_ws = workbook.add_worksheet();
    stages_ws.set_name("Stages")?;
    write_stages(stages_ws, stages)?;

    // This will appear as the second sheet in the workbook.
    let tp_ws = workbook.add_worksheet();
    tp_ws.set_name("Track Points")?;
    write_trackpoints(tp_ws, trackpoint_hyperlinks, &gpx.points)?;

    Ok(workbook)
}

/// Writes the summary workbook to file.
#[time]
pub fn write_summary_file<'gpx>(
    summary_filename: &Path,
    mut workbook: Workbook,
) -> Result<(), Box<dyn Error>> {
    print!("Writing file {:?}", &summary_filename);
    workbook.save(summary_filename).unwrap();
    let metadata = std::fs::metadata(summary_filename).unwrap();
    println!(", {} Kb", metadata.len() / 1024);
    Ok(())
}

fn write_stages<'gpx>(ws: &mut Worksheet, stages: &StageList<'gpx>) -> Result<(), Box<dyn Error>> {
    if stages.len() == 0 {
        // TODO: Write something.
        return Ok(());
    }

    let mut fc = FormatControl::new();

    const COL_STAGE: u16 = 0;
    write_header_blank(ws, &fc, (0, COL_STAGE))?;
    write_header(ws, &fc, (1, COL_STAGE), "Stage")?;
    fc.increment_column();

    const COL_TYPE: u16 = COL_STAGE + 1;
    write_header_blank(ws, &fc, (0, COL_TYPE))?;
    write_header(ws, &fc, (1, COL_TYPE), "Type")?;
    fc.increment_column();

    const COL_STAGE_LOCATION: u16 = COL_TYPE + 1;
    write_header_merged(
        ws,
        &fc,
        (0, COL_STAGE_LOCATION),
        (0, COL_STAGE_LOCATION + 3),
        "Stage Location",
    )?;
    write_header(ws, &fc, (1, COL_STAGE_LOCATION), "Lat")?;
    write_header(ws, &fc, (1, COL_STAGE_LOCATION + 1), "Lon")?;
    write_header(ws, &fc, (1, COL_STAGE_LOCATION + 2), "Map")?;
    write_header(ws, &fc, (1, COL_STAGE_LOCATION + 3), "Description")?;
    ws.set_column_width(COL_STAGE_LOCATION, LAT_LON_COLUMN_WIDTH)?;
    ws.set_column_width(COL_STAGE_LOCATION + 1, LAT_LON_COLUMN_WIDTH)?;
    ws.set_column_width(COL_STAGE_LOCATION + 2, LINKED_LAT_LON_COLUMN_WIDTH)?;
    ws.set_column_width(COL_STAGE_LOCATION + 3, LOCATION_DESCRIPTION_COLUMN_WIDTH)?;
    fc.increment_column();

    const COL_START_TIME: u16 = COL_STAGE_LOCATION + 4;
    write_header_merged(
        ws,
        &fc,
        (0, COL_START_TIME),
        (0, COL_START_TIME + 1),
        "Start Time",
    )?;
    write_header(ws, &fc, (1, COL_START_TIME), "UTC")?;
    write_header(ws, &fc, (1, COL_START_TIME + 1), "Local")?;
    ws.set_column_width(COL_START_TIME, DATE_COLUMN_WIDTH)?;
    ws.set_column_width(COL_START_TIME + 1, DATE_COLUMN_WIDTH)?;
    fc.increment_column();

    const COL_END_TIME: u16 = COL_START_TIME + 2;
    write_header_merged(
        ws,
        &fc,
        (0, COL_END_TIME),
        (0, COL_END_TIME + 1),
        "End Time",
    )?;
    write_header(ws, &fc, (1, COL_END_TIME), "UTC")?;
    write_header(ws, &fc, (1, COL_END_TIME + 1), "Local")?;
    ws.set_column_width(COL_END_TIME, DATE_COLUMN_WIDTH)?;
    ws.set_column_width(COL_END_TIME + 1, DATE_COLUMN_WIDTH)?;
    fc.increment_column();

    const COL_DURATION: u16 = COL_END_TIME + 2;
    write_header_merged(
        ws,
        &fc,
        (0, COL_DURATION),
        (0, COL_DURATION + 1),
        "Duration",
    )?;
    write_header(ws, &fc, (1, COL_DURATION), "hms")?;
    write_header(ws, &fc, (1, COL_DURATION + 1), "Running")?;
    ws.set_column_width(COL_DURATION, DURATION_COLUMN_WIDTH)?;
    ws.set_column_width(COL_DURATION + 1, DURATION_COLUMN_WIDTH)?;
    fc.increment_column();

    const COL_DISTANCE: u16 = COL_DURATION + 2;
    write_header_merged(
        ws,
        &fc,
        (0, COL_DISTANCE),
        (0, COL_DISTANCE + 1),
        "Distance (km)",
    )?;
    write_header(ws, &fc, (1, COL_DISTANCE), "Stage")?;
    write_header(ws, &fc, (1, COL_DISTANCE + 1), "Running")?;
    ws.set_column_width(COL_DISTANCE, KILOMETRES_COLUMN_WIDTH)?;
    ws.set_column_width(COL_DISTANCE + 1, METRES_COLUMN_WIDTH)?;
    fc.increment_column();

    const COL_AVG_SPEED: u16 = COL_DISTANCE + 2;
    write_header_merged(
        ws,
        &fc,
        (0, COL_AVG_SPEED),
        (0, COL_AVG_SPEED + 1),
        "Avg Speed (kmh)",
    )?;
    write_header(ws, &fc, (1, COL_AVG_SPEED), "Stage")?;
    write_header(ws, &fc, (1, COL_AVG_SPEED + 1), "Running")?;
    ws.set_column_width(COL_AVG_SPEED, SPEED_COLUMN_WIDTH)?;
    ws.set_column_width(COL_AVG_SPEED + 1, SPEED_COLUMN_WIDTH)?;
    fc.increment_column();

    const COL_ASCENT: u16 = COL_AVG_SPEED + 2;
    write_header_merged(ws, &fc, (0, COL_ASCENT), (0, COL_ASCENT + 1), "Ascent (m)")?;
    write_header(ws, &fc, (1, COL_ASCENT), "Stage")?;
    write_header(ws, &fc, (1, COL_ASCENT + 1), "Running")?;
    ws.set_column_width(COL_ASCENT, METRES_COLUMN_WIDTH)?;
    ws.set_column_width(COL_ASCENT + 1, METRES_COLUMN_WIDTH)?;
    fc.increment_column();

    const COL_DESCENT: u16 = COL_ASCENT + 2;
    write_header_merged(
        ws,
        &fc,
        (0, COL_DESCENT),
        (0, COL_DESCENT + 1),
        "Descent (m)",
    )?;
    write_header(ws, &fc, (1, COL_DESCENT), "Stage")?;
    write_header(ws, &fc, (1, COL_DESCENT + 1), "Running")?;
    ws.set_column_width(COL_DESCENT, METRES_COLUMN_WIDTH)?;
    ws.set_column_width(COL_DESCENT + 1, METRES_COLUMN_WIDTH)?;
    fc.increment_column();

    const COL_MIN_ELE: u16 = COL_DESCENT + 2;
    write_header_merged(
        ws,
        &fc,
        (0, COL_MIN_ELE),
        (0, COL_MIN_ELE + 2),
        "Minimum Elevation",
    )?;
    write_header(ws, &fc, (1, COL_MIN_ELE), "Elevation (m)")?;
    write_header(ws, &fc, (1, COL_MIN_ELE + 1), "Distance (km)")?;
    write_header(ws, &fc, (1, COL_MIN_ELE + 2), "Point")?;
    ws.set_column_width(COL_MIN_ELE, ELEVATION_COLUMN_WIDTH_WITH_UNITS)?;
    ws.set_column_width(COL_MIN_ELE + 1, KILOMETRES_COLUMN_WIDTH_WITH_UNITS)?;
    fc.increment_column();

    const COL_MAX_ELE: u16 = COL_MIN_ELE + 3;
    write_header_merged(
        ws,
        &fc,
        (0, COL_MAX_ELE),
        (0, COL_MAX_ELE + 2),
        "Maximum Elevation (m)",
    )?;
    write_header(ws, &fc, (1, COL_MAX_ELE), "Elevation")?;
    write_header(ws, &fc, (1, COL_MAX_ELE + 1), "Distance (km)")?;
    write_header(ws, &fc, (1, COL_MAX_ELE + 2), "Point")?;
    ws.set_column_width(COL_MAX_ELE, ELEVATION_COLUMN_WIDTH_WITH_UNITS)?;
    ws.set_column_width(COL_MAX_ELE + 1, KILOMETRES_COLUMN_WIDTH_WITH_UNITS)?;
    fc.increment_column();

    const COL_MAX_SPEED: u16 = COL_MAX_ELE + 3;
    write_header_merged(
        ws,
        &fc,
        (0, COL_MAX_SPEED),
        (0, COL_MAX_SPEED + 4),
        "Max Speed",
    )?;
    write_header(ws, &fc, (1, COL_MAX_SPEED), "Speed (kmh)")?;
    write_header(ws, &fc, (1, COL_MAX_SPEED + 1), "Distance (km)")?;
    write_header(ws, &fc, (1, COL_MAX_SPEED + 2), "Lat")?;
    write_header(ws, &fc, (1, COL_MAX_SPEED + 3), "Lon")?;
    write_header(ws, &fc, (1, COL_MAX_SPEED + 4), "Map")?;
    ws.set_column_width(COL_MAX_SPEED, SPEED_COLUMN_WIDTH_WITH_UNITS)?;
    ws.set_column_width(COL_MAX_SPEED + 1, KILOMETRES_COLUMN_WIDTH_WITH_UNITS)?;
    ws.set_column_width(COL_MAX_SPEED + 2, LAT_LON_COLUMN_WIDTH)?;
    ws.set_column_width(COL_MAX_SPEED + 3, LAT_LON_COLUMN_WIDTH)?;
    ws.set_column_width(COL_MAX_SPEED + 4, LINKED_LAT_LON_COLUMN_WIDTH)?;
    fc.increment_column();

    const COL_HEART_RATE: u16 = COL_MAX_SPEED + 5;
    write_header_merged(
        ws,
        &fc,
        (0, COL_HEART_RATE),
        (0, COL_HEART_RATE + 6),
        "Heart Rate",
    )?;
    write_header(ws, &fc, (1, COL_HEART_RATE), "Avg")?;
    write_header(ws, &fc, (1, COL_HEART_RATE + 1), "Max")?;
    write_header(ws, &fc, (1, COL_HEART_RATE + 2), "Speed (kmh)")?;
    write_header(ws, &fc, (1, COL_HEART_RATE + 3), "Distance (km)")?;
    write_header(ws, &fc, (1, COL_HEART_RATE + 4), "Lat")?;
    write_header(ws, &fc, (1, COL_HEART_RATE + 5), "Lon")?;
    write_header(ws, &fc, (1, COL_HEART_RATE + 6), "Map")?;
    ws.set_column_width(COL_HEART_RATE + 2, SPEED_COLUMN_WIDTH_WITH_UNITS)?;
    ws.set_column_width(COL_HEART_RATE + 3, KILOMETRES_COLUMN_WIDTH_WITH_UNITS)?;
    ws.set_column_width(COL_HEART_RATE + 4, LAT_LON_COLUMN_WIDTH)?;
    ws.set_column_width(COL_HEART_RATE + 5, LAT_LON_COLUMN_WIDTH)?;
    ws.set_column_width(COL_HEART_RATE + 6, LINKED_LAT_LON_COLUMN_WIDTH)?;
    fc.increment_column();

    const COL_MAX_TEMP: u16 = COL_HEART_RATE + 7;
    write_header_merged(ws, &fc, (0, COL_MAX_TEMP), (0, COL_MAX_TEMP + 6), "Temp °C")?;
    write_header(ws, &fc, (1, COL_MAX_TEMP), "Avg")?;
    write_header(ws, &fc, (1, COL_MAX_TEMP + 1), "Max")?;
    write_header(ws, &fc, (1, COL_MAX_TEMP + 2), "Speed (kmh)")?;
    write_header(ws, &fc, (1, COL_MAX_TEMP + 3), "Distance (km)")?;
    write_header(ws, &fc, (1, COL_MAX_TEMP + 4), "Lat")?;
    write_header(ws, &fc, (1, COL_MAX_TEMP + 5), "Lon")?;
    write_header(ws, &fc, (1, COL_MAX_TEMP + 6), "Map")?;
    ws.set_column_width(COL_MAX_TEMP + 2, SPEED_COLUMN_WIDTH_WITH_UNITS)?;
    ws.set_column_width(COL_MAX_TEMP + 3, KILOMETRES_COLUMN_WIDTH_WITH_UNITS)?;
    ws.set_column_width(COL_MAX_TEMP + 4, LAT_LON_COLUMN_WIDTH)?;
    ws.set_column_width(COL_MAX_TEMP + 5, LAT_LON_COLUMN_WIDTH)?;
    ws.set_column_width(COL_MAX_TEMP + 6, LINKED_LAT_LON_COLUMN_WIDTH)?;
    fc.increment_column();

    const COL_TRACKPOINTS: u16 = COL_MAX_TEMP + 7;
    write_header_merged(
        ws,
        &fc,
        (0, COL_TRACKPOINTS),
        (0, COL_TRACKPOINTS + 2),
        "Track Points",
    )?;
    write_header(ws, &fc, (1, COL_TRACKPOINTS), "First")?;
    write_header(ws, &fc, (1, COL_TRACKPOINTS + 1), "Last")?;
    write_header(ws, &fc, (1, COL_TRACKPOINTS + 2), "Count")?;

    // Regarding lat-lon hyperlinks: on the summary tab we generally always
    // write them, because they are few in number and so don't slow down Calc.
    // But they are optional on the Track Points tab because there are thousands
    // of them and they really slow down Calc.

    // Regenerate this so the formatting starts at the right point.
    let mut fc = FormatControl::new();
    let mut row = 2;
    for (idx, stage) in stages.iter().enumerate() {
        fc.reset_column();

        write_integer(ws, &fc, (row, COL_STAGE), (idx + 1) as u32)?;
        fc.increment_column();

        write_string(ws, &fc, (row, COL_TYPE), &stage.stage_type.to_string())?;
        fc.increment_column();

        write_lat_lon(
            ws,
            &fc,
            (row, COL_STAGE_LOCATION),
            (stage.start.lat, stage.start.lon),
            Hyperlink::Yes,
        )?;
        write_location_description(
            ws,
            &fc,
            (row, COL_STAGE_LOCATION + 3),
            &stage.start.location,
        )?;
        fc.increment_column();

        match stage.start.time {
            Some(start_time) => {
                write_utc_date(ws, &fc, (row, COL_START_TIME), start_time)?;
                write_utc_date_as_local(ws, &fc, (row, COL_START_TIME + 1), start_time)?;
            }
            None => {
                write_blank(ws, &fc, (row, COL_START_TIME))?;
                write_blank(ws, &fc, (row, COL_START_TIME + 1))?;
            }
        };
        fc.increment_column();

        match stage.end.time {
            Some(end_time) => {
                write_utc_date(ws, &fc, (row, COL_END_TIME), end_time)?;
                write_utc_date_as_local(ws, &fc, (row, COL_END_TIME + 1), end_time)?;
            }
            None => {
                write_blank(ws, &fc, (row, COL_END_TIME))?;
                write_blank(ws, &fc, (row, COL_END_TIME + 1))?;
            }
        }
        fc.increment_column();

        write_duration_option(ws, &fc, (row, COL_DURATION), stage.duration())?;
        write_duration_option(ws, &fc, (row, COL_DURATION + 1), stage.running_duration())?;
        fc.increment_column();

        if stage.stage_type == StageType::Moving {
            write_kilometres(ws, &fc, (row, COL_DISTANCE), stage.distance_km())?;
            write_kilometres(
                ws,
                &fc,
                (row, COL_DISTANCE + 1),
                stage.running_distance_km(),
            )?;
            fc.increment_column();
            write_speed_option(ws, &fc, (row, COL_AVG_SPEED), stage.average_speed_kmh())?;
            write_speed_option(
                ws,
                &fc,
                (row, COL_AVG_SPEED + 1),
                stage.running_average_speed_kmh(),
            )?;
            fc.increment_column();
            write_metres_option(ws, &fc, (row, COL_ASCENT), stage.ascent_metres())?;
            write_metres_option(
                ws,
                &fc,
                (row, COL_ASCENT + 1),
                stage.running_ascent_metres(),
            )?;
            fc.increment_column();
            write_metres_option(ws, &fc, (row, COL_DESCENT), stage.descent_metres())?;
            write_metres_option(
                ws,
                &fc,
                (row, COL_DESCENT + 1),
                stage.running_descent_metres(),
            )?;
            fc.increment_column();
            write_elevation_data(ws, &fc, (row, COL_MIN_ELE), stage.min_elevation)?;
            fc.increment_column();
            write_elevation_data(ws, &fc, (row, COL_MAX_ELE), stage.max_elevation)?;
            fc.increment_column();
            write_max_speed_data(ws, &fc, (row, COL_MAX_SPEED), stage.max_speed)?;
        } else {
            // Write blanks so that the banding formatting is applied.
            for col in COL_DISTANCE..=(COL_MAX_SPEED + 4) {
                write_blank(ws, &fc, (row, col))?;
            }
        }

        fc.increment_column();
        write_heart_rate_data(ws, &fc, (row, COL_HEART_RATE), stage.max_heart_rate)?;

        fc.increment_column();
        write_temperature_data(ws, &fc, (row, COL_MAX_TEMP), stage.max_air_temp)?;

        fc.increment_column();
        write_trackpoint_number(ws, &fc, (row, COL_TRACKPOINTS), stage.start.index)?;
        write_trackpoint_number(ws, &fc, (row, COL_TRACKPOINTS + 1), stage.end.index)?;
        write_integer(
            ws,
            &fc,
            (row, COL_TRACKPOINTS + 2),
            (stage.end.index - stage.start.index + 1).try_into()?,
        )?;

        row += 1;
        fc.increment_row();
    }

    ws.set_freeze_panes(2, 0)?;

    // Now write an overall summary row.
    let mut fc = FormatControl::new();
    row += 2;
    write_string_bold(ws, &fc, (row, COL_START_TIME - 1), "SUMMARY")?;
    fc.increment_column();
    write_utc_date_option(ws, &fc, (row, COL_START_TIME), stages.start_time())?;
    write_utc_date_as_local_option(ws, &fc, (row, COL_START_TIME + 1), stages.start_time())?;
    fc.increment_column();
    write_utc_date_option(ws, &fc, (row, COL_END_TIME), stages.end_time())?;
    write_utc_date_as_local_option(ws, &fc, (row, COL_END_TIME + 1), stages.end_time())?;
    fc.increment_column();
    write_string(ws, &fc, (row, COL_DURATION), "Total")?;
    write_duration_option(ws, &fc, (row, COL_DURATION + 1), stages.duration())?;
    write_string(ws, &fc, (row + 1, COL_DURATION), "Stopped")?;
    write_duration_option(
        ws,
        &fc,
        (row + 1, COL_DURATION + 1),
        stages.total_stopped_time(),
    )?;
    write_string(ws, &fc, (row + 2, COL_DURATION), "Moving")?;
    write_duration_option(
        ws,
        &fc,
        (row + 2, COL_DURATION + 1),
        stages.total_moving_time(),
    )?;
    fc.increment_column();
    write_blank(ws, &fc, (row, COL_DISTANCE))?;
    write_kilometres(ws, &fc, (row, COL_DISTANCE + 1), stages.distance_km())?;
    fc.increment_column();
    write_string(ws, &fc, (row, COL_AVG_SPEED), "Overall")?;
    write_speed_option(
        ws,
        &fc,
        (row, COL_AVG_SPEED + 1),
        stages.average_overall_speed(),
    )?;
    write_string(ws, &fc, (row + 1, COL_AVG_SPEED), "Moving")?;
    write_speed_option(
        ws,
        &fc,
        (row + 1, COL_AVG_SPEED + 1),
        stages.average_moving_speed(),
    )?;
    fc.increment_column();
    write_blank(ws, &fc, (row, COL_ASCENT))?;
    write_metres_option(ws, &fc, (row, COL_ASCENT + 1), stages.total_ascent_metres())?;
    fc.increment_column();
    write_blank(ws, &fc, (row, COL_DESCENT))?;
    write_metres_option(
        ws,
        &fc,
        (row, COL_DESCENT + 1),
        stages.total_descent_metres(),
    )?;
    fc.increment_column();
    write_elevation_data(ws, &fc, (row, COL_MIN_ELE), stages.min_elevation())?;
    fc.increment_column();
    write_elevation_data(ws, &fc, (row, COL_MAX_ELE), stages.max_elevation())?;
    fc.increment_column();
    write_max_speed_data(ws, &fc, (row, COL_MAX_SPEED), stages.max_speed())?;
    fc.increment_column();
    write_heart_rate_data(ws, &fc, (row, COL_HEART_RATE), stages.max_heart_rate())?;
    fc.increment_column();
    write_temperature_data(ws, &fc, (row, COL_MAX_TEMP), stages.max_temperature())?;
    fc.increment_column();
    write_trackpoint_number(ws, &fc, (row, COL_TRACKPOINTS), stages.first_point().index)?;
    write_trackpoint_number(
        ws,
        &fc,
        (row, COL_TRACKPOINTS + 1),
        stages.last_point().index,
    )?;
    write_integer(
        ws,
        &fc,
        (row, COL_TRACKPOINTS + 2),
        (stages.last_point().index - stages.first_point().index + 1).try_into()?,
    )?;

    Ok(())
}

fn write_trackpoints(
    ws: &mut Worksheet,
    hyperlink: Hyperlink,
    points: &[EnrichedTrackPoint],
) -> Result<(), Box<dyn Error>> {
    let mut fc = FormatControl::new();

    const COL_INDEX: u16 = 0;
    write_header_blank(ws, &fc, (0, COL_INDEX))?;
    write_header(ws, &fc, (1, COL_INDEX), "Index")?;
    fc.increment_column();

    const COL_TIME: u16 = COL_INDEX + 1;
    write_header_merged(ws, &fc, (0, COL_TIME), (0, COL_TIME + 3), "Time")?;
    write_header(ws, &fc, (1, COL_TIME), "UTC")?;
    write_header(ws, &fc, (1, COL_TIME + 1), "Local")?;
    write_header(ws, &fc, (1, COL_TIME + 2), "Delta")?;
    write_header(ws, &fc, (1, COL_TIME + 3), "Running")?;
    ws.set_column_width(COL_TIME, DATE_COLUMN_WIDTH)?;
    ws.set_column_width(COL_TIME + 1, DATE_COLUMN_WIDTH)?;
    ws.set_column_width(COL_TIME + 2, DURATION_COLUMN_WIDTH)?;
    ws.set_column_width(COL_TIME + 3, DURATION_COLUMN_WIDTH)?;
    fc.increment_column();

    const COL_LOCATION: u16 = COL_TIME + 4;
    write_header_merged(
        ws,
        &fc,
        (0, COL_LOCATION),
        (0, COL_LOCATION + 3),
        "Location",
    )?;
    write_header(ws, &fc, (1, COL_LOCATION), "Lat")?;
    write_header(ws, &fc, (1, COL_LOCATION + 1), "Lon")?;
    write_header(ws, &fc, (1, COL_LOCATION + 2), "Map")?;
    write_header(ws, &fc, (1, COL_LOCATION + 3), "Description")?;
    ws.set_column_width(COL_LOCATION, LAT_LON_COLUMN_WIDTH)?;
    ws.set_column_width(COL_LOCATION + 1, LAT_LON_COLUMN_WIDTH)?;
    ws.set_column_width(COL_LOCATION + 2, LINKED_LAT_LON_COLUMN_WIDTH)?;
    ws.set_column_width(COL_LOCATION + 3, LOCATION_DESCRIPTION_COLUMN_WIDTH)?;
    fc.increment_column();

    const COL_ELE: u16 = COL_LOCATION + 4;
    write_header_merged(ws, &fc, (0, COL_ELE), (0, COL_ELE + 3), "Elevation (m)")?;
    write_header(ws, &fc, (1, COL_ELE), "Height")?;
    write_header(ws, &fc, (1, COL_ELE + 1), "Delta")?;
    write_header(ws, &fc, (1, COL_ELE + 2), "Running Ascent")?;
    write_header(ws, &fc, (1, COL_ELE + 3), "Running Descent")?;
    ws.set_column_width(COL_ELE, METRES_COLUMN_WIDTH_WITH_UNITS)?;
    ws.set_column_width(COL_ELE + 1, METRES_COLUMN_WIDTH_WITH_UNITS)?;
    ws.set_column_width(COL_ELE + 2, KILOMETRES_COLUMN_WIDTH_WITH_UNITS)?;
    ws.set_column_width(COL_ELE + 3, KILOMETRES_COLUMN_WIDTH_WITH_UNITS)?;
    fc.increment_column();

    const COL_DISTANCE: u16 = COL_ELE + 4;
    write_header_merged(
        ws,
        &fc,
        (0, COL_DISTANCE),
        (0, COL_DISTANCE + 1),
        "Distance",
    )?;
    write_header(ws, &fc, (1, COL_DISTANCE), "Delta (m)")?;
    write_header(ws, &fc, (1, COL_DISTANCE + 1), "Running (km)")?;
    ws.set_column_width(COL_DISTANCE, METRES_COLUMN_WIDTH_WITH_UNITS)?;
    ws.set_column_width(COL_DISTANCE + 1, KILOMETRES_COLUMN_WIDTH_WITH_UNITS)?;
    fc.increment_column();

    const COL_SPEED: u16 = COL_DISTANCE + 2;
    write_header_blank(ws, &fc, (0, COL_SPEED))?;
    write_header(ws, &fc, (1, COL_SPEED), "Speed (kmh)")?;
    ws.set_column_width(COL_SPEED, SPEED_COLUMN_WIDTH_WITH_UNITS)?;

    const COL_HEART_RATE: u16 = COL_SPEED + 1;
    fc.increment_column();
    write_header_blank(ws, &fc, (0, COL_HEART_RATE))?;
    write_header(ws, &fc, (1, COL_HEART_RATE), "Heart Rate (bpm)")?;
    ws.set_column_width(COL_HEART_RATE, HEART_RATE_WIDTH_WITH_UNITS)?;

    const COL_AIR_TEMP: u16 = COL_HEART_RATE + 1;
    fc.increment_column();
    write_header_blank(ws, &fc, (0, COL_AIR_TEMP))?;
    write_header(ws, &fc, (1, COL_AIR_TEMP), "Temp (°C)")?;
    ws.set_column_width(COL_AIR_TEMP, TEMPERATURE_COLUMN_WIDTH_WITH_UNITS)?;

    const COL_CADENCE: u16 = COL_AIR_TEMP + 1;
    fc.increment_column();
    write_header_blank(ws, &fc, (0, COL_CADENCE))?;
    write_header(ws, &fc, (1, COL_CADENCE), "Cadence (rpm)")?;
    ws.set_column_width(COL_CADENCE, CADENCE_COLUMN_WIDTH_WITH_UNITS)?;

    // Regenerate this so the formatting starts at the right point.
    let mut fc = FormatControl::new();
    let mut row = 2;
    for p in points {
        fc.reset_column();

        write_integer(ws, &fc, (row, COL_INDEX), p.index as u32)?;
        fc.increment_column();

        match p.time {
            Some(time) => {
                write_utc_date(ws, &fc, (row, COL_TIME), time)?;
                write_utc_date_as_local(ws, &fc, (row, COL_TIME + 1), time)?;
            }
            None => {
                write_blank(ws, &fc, (row, COL_TIME))?;
                write_blank(ws, &fc, (row, COL_TIME + 1))?;
            }
        }
        write_duration_option(ws, &fc, (row, COL_TIME + 2), p.delta_time)?;
        write_duration_option(ws, &fc, (row, COL_TIME + 3), p.running_delta_time)?;
        fc.increment_column();

        write_lat_lon(ws, &fc, (row, COL_LOCATION), (p.lat, p.lon), hyperlink)?;
        write_location_description(ws, &fc, (row, COL_LOCATION + 3), &p.location)?;
        fc.increment_column();

        match p.ele {
            Some(ele) => {
                write_metres(ws, &fc, (row, COL_ELE), ele)?;
            }
            None => {
                write_blank(ws, &fc, (row, COL_ELE))?;
            }
        }

        write_metres_option(ws, &fc, (row, COL_ELE + 1), p.ele_delta_metres)?;
        write_metres_option(ws, &fc, (row, COL_ELE + 2), p.running_ascent_metres)?;
        write_metres_option(ws, &fc, (row, COL_ELE + 3), p.running_descent_metres)?;
        fc.increment_column();

        write_metres(ws, &fc, (row, COL_DISTANCE), p.delta_metres)?;
        write_kilometres(ws, &fc, (row, COL_DISTANCE + 1), p.running_metres / 1000.0)?;
        fc.increment_column();

        write_speed_option(ws, &fc, (row, COL_SPEED), p.speed_kmh)?;
        fc.increment_column();

        if let Some(Some(hr)) = p.extensions.as_ref().map(|ex| ex.heart_rate) {
            write_integer(ws, &fc, (row, COL_HEART_RATE), hr.into())?;
        } else {
            write_blank(ws, &fc, (row, COL_HEART_RATE))?;
        }
        fc.increment_column();

        if let Some(Some(at)) = p.extensions.as_ref().map(|ex| ex.air_temp) {
            write_temperature(ws, &fc, (row, COL_AIR_TEMP), at)?;
        } else {
            write_blank(ws, &fc, (row, COL_AIR_TEMP))?;
        }
        fc.increment_column();

        if let Some(Some(cadence)) = p.extensions.as_ref().map(|ex| ex.cadence) {
            write_integer(ws, &fc, (row, COL_CADENCE), cadence.into())?;
        } else {
            write_blank(ws, &fc, (row, COL_CADENCE))?;
        }
        fc.increment_column();

        row += 1;
        fc.increment_row();
    }

    ws.autofilter(1, 0, row - 1, COL_CADENCE)?;
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

/// Writes a string right aligned.
fn write_string(
    ws: &mut Worksheet,
    fc: &FormatControl,
    rc: (u32, u16),
    value: &str,
) -> Result<(), Box<dyn Error>> {
    ws.write_string_with_format(rc.0, rc.1, value, &fc.string_format())?;
    Ok(())
}

/// Writes a string right aligned and bold.
fn write_string_bold(
    ws: &mut Worksheet,
    fc: &FormatControl,
    rc: (u32, u16),
    value: &str,
) -> Result<(), Box<dyn Error>> {
    let format = fc.string_format().set_bold();
    ws.write_string_with_format(rc.0, rc.1, value, &format)?;
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

fn write_utc_date_option(
    ws: &mut Worksheet,
    fc: &FormatControl,
    rc: (u32, u16),
    utc_date: Option<OffsetDateTime>,
) -> Result<(), Box<dyn Error>> {
    if let Some(d) = utc_date {
        write_utc_date(ws, fc, rc, d)?;
    } else {
        write_blank(ws, fc, rc)?;
    }
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

fn write_utc_date_as_local_option(
    ws: &mut Worksheet,
    fc: &FormatControl,
    rc: (u32, u16),
    utc_date: Option<OffsetDateTime>,
) -> Result<(), Box<dyn Error>> {
    if let Some(d) = utc_date {
        write_utc_date_as_local(ws, fc, rc, d)?;
    } else {
        write_blank(ws, fc, rc)?;
    }
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

fn write_duration_option(
    ws: &mut Worksheet,
    fc: &FormatControl,
    rc: (u32, u16),
    duration: Option<Duration>,
) -> Result<(), Box<dyn Error>> {
    if let Some(dur) = duration {
        write_duration(ws, fc, rc, dur)?;
    } else {
        write_blank(ws, fc, rc)?;
    }

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
    point: Option<&EnrichedTrackPoint>,
) -> Result<(), Box<dyn Error>> {
    if point.is_none() {
        write_blank(ws, fc, (rc.0, rc.1))?;
        write_blank(ws, fc, (rc.0, rc.1 + 1))?;
        write_blank(ws, fc, (rc.0, rc.1 + 2))?;
        return Ok(());
    }

    let point = point.unwrap();

    match point.ele {
        Some(ele) => {
            write_metres(ws, &fc, (rc.0, rc.1), ele)?;
        }
        None => {
            write_blank(ws, fc, (rc.0, rc.1))?;
        }
    }

    write_kilometres_running_with_map_hyperlink(ws, fc, (rc.0, rc.1 + 1), point)?;
    write_trackpoint_number(ws, fc, (rc.0, rc.1 + 2), point.index)?;
    Ok(())
}

/// Writes an max speed data block (min or max) as found on the Stages tab.
fn write_max_speed_data(
    ws: &mut Worksheet,
    fc: &FormatControl,
    rc: (u32, u16),
    point: Option<&EnrichedTrackPoint>,
) -> Result<(), Box<dyn Error>> {
    if point.is_none() {
        write_blank(ws, fc, (rc.0, rc.1))?;
        write_blank(ws, fc, (rc.0, rc.1 + 1))?;
        write_blank(ws, fc, (rc.0, rc.1 + 2))?;
        write_blank(ws, fc, (rc.0, rc.1 + 3))?;
        write_blank(ws, fc, (rc.0, rc.1 + 4))?;
        return Ok(());
    }

    let point = point.unwrap();

    write_speed_option(ws, &fc, (rc.0, rc.1), point.speed_kmh)?;
    write_kilometres(ws, &fc, (rc.0, rc.1 + 1), point.running_metres / 1000.0)?;
    write_lat_lon(
        ws,
        &fc,
        (rc.0, rc.1 + 2),
        (point.lat, point.lon),
        Hyperlink::Yes,
    )?;
    Ok(())
}

fn write_heart_rate_data(
    ws: &mut Worksheet,
    fc: &FormatControl,
    rc: (u32, u16),
    point: Option<&EnrichedTrackPoint>,
) -> Result<(), Box<dyn Error>> {
    if let Some(point) = point {
        let extensions = point
            .extensions
            .as_ref()
            .expect("extensions should exist for hr");
        if let Some(mhr) = extensions.heart_rate {
            write_integer(ws, &fc, (rc.0, rc.1 + 1), mhr as u32)?;
        }
        write_speed_option(ws, fc, (rc.0, rc.1 + 2), point.speed_kmh)?;
        write_kilometres(ws, fc, (rc.0, rc.1 + 3), point.running_metres / 1000.0)?;
        write_lat_lon(
            ws,
            fc,
            (rc.0, rc.1 + 4),
            (point.lat, point.lon),
            Hyperlink::Yes,
        )?;
        return Ok(());
    }

    write_blank(ws, &fc, (rc.0, rc.1))?;
    write_blank(ws, &fc, (rc.0, rc.1 + 1))?;
    write_blank(ws, &fc, (rc.0, rc.1 + 2))?;
    write_blank(ws, &fc, (rc.0, rc.1 + 3))?;
    write_blank(ws, &fc, (rc.0, rc.1 + 4))?;
    write_blank(ws, &fc, (rc.0, rc.1 + 5))?;
    write_blank(ws, &fc, (rc.0, rc.1 + 6))?;

    Ok(())
}

fn write_temperature_data(
    ws: &mut Worksheet,
    fc: &FormatControl,
    rc: (u32, u16),
    point: Option<&EnrichedTrackPoint>,
) -> Result<(), Box<dyn Error>> {
    if let Some(point) = point {
        let extensions = point
            .extensions
            .as_ref()
            .expect("extensions should exist for air_temp");
        if let Some(at) = extensions.air_temp {
            write_temperature(ws, &fc, (rc.0, rc.1 + 1), at)?;
        }
        write_speed_option(ws, fc, (rc.0, rc.1 + 2), point.speed_kmh)?;
        write_kilometres(ws, fc, (rc.0, rc.1 + 3), point.running_metres / 1000.0)?;
        write_lat_lon(
            ws,
            fc,
            (rc.0, rc.1 + 4),
            (point.lat, point.lon),
            Hyperlink::Yes,
        )?;
        return Ok(());
    }

    write_blank(ws, &fc, (rc.0, rc.1))?;
    write_blank(ws, &fc, (rc.0, rc.1 + 1))?;
    write_blank(ws, &fc, (rc.0, rc.1 + 2))?;
    write_blank(ws, &fc, (rc.0, rc.1 + 3))?;
    write_blank(ws, &fc, (rc.0, rc.1 + 4))?;
    write_blank(ws, &fc, (rc.0, rc.1 + 5))?;
    write_blank(ws, &fc, (rc.0, rc.1 + 6))?;

    Ok(())
}

fn write_temperature(
    ws: &mut Worksheet,
    fc: &FormatControl,
    rc: (u32, u16),
    temperature: f64,
) -> Result<(), Box<dyn Error>> {
    let format = fc.temperature_format();
    ws.write_number_with_format(rc.0, rc.1, temperature, &format)?;
    Ok(())
}

fn make_hyperlink((lat, lon): (f64, f64)) -> Url {
    let text = format!("{:.6}, {:.6}", lat, lon);
    make_hyperlink_with_text((lat, lon), &text)
}

fn make_hyperlink_with_text((lat, lon): (f64, f64), text: &str) -> Url {
    Url::new(format!(
        "https://www.google.com/maps/search/?api=1&query={:.6},{:.6}",
        lat, lon
    ))
    .set_text(text)
}

/// Writes a lat-lon pair with the lat in the first cell as specified
/// by 'rc' and the lon in the next column. If 'hyperlink' is yes then
/// a hyperlink to Google Maps is written into the third column.
fn write_lat_lon(
    ws: &mut Worksheet,
    fc: &FormatControl,
    rc: (u32, u16),
    (lat, lon): (f64, f64),
    hyperlink: Hyperlink,
) -> Result<(), Box<dyn Error>> {
    let format = fc.lat_lon_format().set_font_color(Color::Black);

    ws.write_number_with_format(rc.0, rc.1, lat, &format)?;
    ws.write_number_with_format(rc.0, rc.1 + 1, lon, &format)?;

    match hyperlink {
        Hyperlink::Yes => {
            let url = make_hyperlink((lat, lon));
            // TODO: Font still blue.
            let format = format.set_align(FormatAlign::Right);
            ws.write_url_with_format(rc.0, rc.1 + 2, url, &format)?;
        }
        Hyperlink::No => {
            // So that banding occurs.
            write_blank(ws, fc, (rc.0, rc.1 + 2))?;
        }
    };

    Ok(())
}

/// Writes a TrackPoint index, including a hyperlink to
/// the 'Track Points' sheet.
fn write_trackpoint_number(
    ws: &mut Worksheet,
    fc: &FormatControl,
    rc: (u32, u16),
    trackpoint_index: usize,
) -> Result<(), Box<dyn Error>> {
    let format = fc
        .integer_format()
        .set_font_color(Color::Black)
        .set_align(FormatAlign::Right);
    let url = Url::new(format!(
        "internal:'Track Points'!A{}",
        trackpoint_index + 3 // allow for the heading on the 'Track Points' sheet.
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

fn write_metres_option(
    ws: &mut Worksheet,
    fc: &FormatControl,
    rc: (u32, u16),
    metres: Option<f64>,
) -> Result<(), Box<dyn Error>> {
    if let Some(m) = metres {
        write_metres(ws, fc, rc, m)?;
    } else {
        write_blank(ws, fc, rc)?;
    }
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

fn write_kilometres_running_with_map_hyperlink(
    ws: &mut Worksheet,
    fc: &FormatControl,
    rc: (u32, u16),
    point: &EnrichedTrackPoint,
) -> Result<(), Box<dyn Error>> {
    let km = point.running_metres / 1000.0;
    let url = make_hyperlink_with_text((point.lat, point.lon), &format!("{:.3}", km));
    let format = fc.kilometres_format();
    let format = format.set_align(FormatAlign::Right);
    ws.write_url_with_format(rc.0, rc.1, url, &format)?;
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

fn write_speed_option(
    ws: &mut Worksheet,
    fc: &FormatControl,
    rc: (u32, u16),
    speed: Option<f64>,
) -> Result<(), Box<dyn Error>> {
    if let Some(s) = speed {
        write_speed(ws, fc, rc, s)?;
    } else {
        write_blank(ws, fc, rc)?;
    }

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
        let format = Format::new().set_align(FormatAlign::Right);
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

    fn temperature_format(&self) -> Format {
        let format = Format::new().set_num_format("0.#");
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
