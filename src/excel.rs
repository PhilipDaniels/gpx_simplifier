use std::{collections::HashSet, error::Error, path::Path};

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
const KILOMETRES_COLUMN_WIDTH_WITH_UNITS: f64 = 14.5;
const KILOMETRES_COLUMN_WIDTH: f64 = 8.0;
const SPEED_COLUMN_WIDTH_WITH_UNITS: f64 = 14.0;
const SPEED_COLUMN_WIDTH: f64 = 8.0;
const HEART_RATE_WIDTH_WITH_UNITS: f64 = 17.5;
const TEMPERATURE_COLUMN_WIDTH_WITH_UNITS: f64 = 12.0;
const CADENCE_COLUMN_WIDTH_WITH_UNITS: f64 = 15.5;

/// Builds the Workbook that is used for the summary.
#[time]
pub fn create_summary_xlsx(
    trackpoint_hyperlinks: Hyperlink,
    gpx: &EnrichedGpx,
    stages: &StageList,
) -> Result<Workbook, Box<dyn Error>> {
    let mut workbook = Workbook::new();

    // This will appear as the first sheet in the workbook.
    let stages_ws = workbook.add_worksheet();
    stages_ws.set_name("Stages")?;
    write_stages(stages_ws, gpx, stages)?;

    // This will appear as the second sheet in the workbook.
    let tp_ws = workbook.add_worksheet();
    tp_ws.set_name("Track Points")?;
    write_trackpoints(
        tp_ws,
        &gpx.points,
        trackpoint_hyperlinks,
        &stages.highlighted_trackpoints(),
    )?;

    Ok(workbook)
}

/// Writes the summary workbook to file.
#[time]
pub fn write_summary_file(
    summary_filename: &Path,
    mut workbook: Workbook,
) -> Result<(), Box<dyn Error>> {
    print!("Writing file {:?}", &summary_filename);
    workbook.save(summary_filename).unwrap();
    let metadata = std::fs::metadata(summary_filename).unwrap();
    println!(", {} Kb", metadata.len() / 1024);
    Ok(())
}

/// Write the "Stages" tab of the summary spreadsheet.
///
/// We write the data in vertical fashion to keep the code for the headers and
/// their corresponding data together. Doing it horizontally leads to a very
/// large function with the headers and the data separated by a large distance.
///
/// Regarding lat-lon hyperlinks: on the summary tab we generally always write
/// them, because they are few in number and so don't slow down Calc. But they
/// are optional on the Track Points tab because there are thousands of them and
/// they really slow down Calc.
#[time]
fn write_stages(
    ws: &mut Worksheet,
    gpx: &EnrichedGpx,
    stages: &StageList,
) -> Result<(), Box<dyn Error>> {
    let mut fc = FormatControl::new();

    if stages.len() == 0 {
        write_string(ws, &fc, "No stages detected")?;
        return Ok(());
    }

    ws.set_freeze_panes(0, 2)?;
    ws.set_freeze_panes(2, 0)?;

    output_stage_number(ws, &mut fc, stages)?;
    output_stage_type(ws, &mut fc, stages)?;
    output_stage_location(ws, &mut fc, stages)?;
    output_start_time(ws, &mut fc, stages)?;
    output_end_time(ws, &mut fc, stages)?;
    output_duration(ws, &mut fc, stages)?;
    output_distance(ws, &mut fc, stages)?;
    output_average_speed(ws, &mut fc, stages)?;
    output_ascent(ws, &mut fc, stages)?;
    output_descent(ws, &mut fc, stages)?;
    output_min_elevation(ws, &mut fc, stages)?;
    output_max_elevation(ws, &mut fc, stages)?;
    output_max_speed(ws, &mut fc, stages)?;
    output_heart_rate(ws, &mut fc, stages, gpx.avg_heart_rate())?;
    output_temperature(ws, &mut fc, stages, gpx.avg_temperature())?;
    output_track_points(ws, &mut fc, stages)?;

    Ok(())
}

fn output_stage_number(
    ws: &mut Worksheet,
    fc: &mut FormatControl,
    stages: &StageList,
) -> Result<(), Box<dyn Error>> {
    write_headers(ws, fc, "", &["Stage"])?;

    for _ in stages {
        write_integer(ws, fc, fc.row - 1)?;
        fc.increment_row();
    }

    fc.next_colour_block(1);
    Ok(())
}

fn output_stage_type(
    ws: &mut Worksheet,
    fc: &mut FormatControl,
    stages: &StageList,
) -> Result<(), Box<dyn Error>> {
    write_headers(ws, fc, "", &["Type"])?;

    for stage in stages {
        write_string(ws, fc, &stage.stage_type.to_string())?;
        fc.increment_row();
    }

    fc.next_colour_block(1);
    Ok(())
}

fn output_stage_location(
    ws: &mut Worksheet,
    fc: &mut FormatControl,
    stages: &StageList,
) -> Result<(), Box<dyn Error>> {
    write_headers(
        ws,
        fc,
        "Stage Location",
        &["Lat", "Lon", "Map", "Description"],
    )?;
    ws.set_column_width(fc.col, LAT_LON_COLUMN_WIDTH)?;
    ws.set_column_width(fc.col + 1, LAT_LON_COLUMN_WIDTH)?;
    ws.set_column_width(fc.col + 2, LINKED_LAT_LON_COLUMN_WIDTH)?;
    ws.set_column_width(fc.col + 3, LOCATION_DESCRIPTION_COLUMN_WIDTH)?;

    for stage in stages {
        write_lat_lon(
            ws,
            fc,
            (stage.start.lat, stage.start.lon),
            Hyperlink::Yes,
            stage.start.location.as_ref(),
        )?;

        fc.increment_row();
    }

    fc.start_summary_row();
    write_string_bold(ws, &fc.col_offset(3), "SUMMARY")?;

    fc.next_colour_block(4);
    Ok(())
}

fn output_start_time(
    ws: &mut Worksheet,
    fc: &mut FormatControl,
    stages: &StageList,
) -> Result<(), Box<dyn Error>> {
    write_headers(ws, fc, "Start Time", &["UTC", "Local"])?;
    ws.set_column_width(fc.col, DATE_COLUMN_WIDTH)?;
    ws.set_column_width(fc.col + 1, DATE_COLUMN_WIDTH)?;

    for stage in stages {
        match stage.start.time {
            Some(start_time) => {
                write_utc_date(ws, fc, start_time)?;
                write_utc_date_as_local(ws, &fc.col_offset(1), start_time)?;
            }
            None => {
                write_blank(ws, fc)?;
                write_blank(ws, &fc.col_offset(1))?;
            }
        };

        fc.increment_row();
    }

    fc.start_summary_row();
    write_utc_date_option(ws, fc, stages.start_time())?;
    write_utc_date_as_local_option(ws, &fc.col_offset(1), stages.start_time())?;

    fc.next_colour_block(2);
    Ok(())
}

fn output_end_time(
    ws: &mut Worksheet,
    fc: &mut FormatControl,
    stages: &StageList,
) -> Result<(), Box<dyn Error>> {
    write_headers(ws, fc, "End Time", &["UTC", "Local"])?;
    ws.set_column_width(fc.col, DATE_COLUMN_WIDTH)?;
    ws.set_column_width(fc.col + 1, DATE_COLUMN_WIDTH)?;

    for stage in stages {
        match stage.end.time {
            Some(end_time) => {
                write_utc_date(ws, fc, end_time)?;
                write_utc_date_as_local(ws, &fc.col_offset(1), end_time)?;
            }
            None => {
                write_blank(ws, fc)?;
                write_blank(ws, &fc.col_offset(1))?;
            }
        };

        fc.increment_row();
    }

    fc.start_summary_row();
    write_utc_date_option(ws, fc, stages.end_time())?;
    write_utc_date_as_local_option(ws, &fc.col_offset(1), stages.end_time())?;

    fc.next_colour_block(2);
    Ok(())
}

fn output_duration(
    ws: &mut Worksheet,
    fc: &mut FormatControl,
    stages: &StageList,
) -> Result<(), Box<dyn Error>> {
    write_headers(ws, fc, "Duration", &["hms", "Running"])?;
    ws.set_column_width(fc.col, DURATION_COLUMN_WIDTH)?;
    ws.set_column_width(fc.col + 1, DURATION_COLUMN_WIDTH)?;

    for stage in stages {
        write_duration_option(ws, fc, stage.duration())?;
        write_duration_option(ws, &fc.col_offset(1), stage.running_duration())?;
        fc.increment_row();
    }

    fc.start_summary_row();
    write_string(ws, fc, "Total")?;
    write_duration_option(ws, &fc.col_offset(1), stages.duration())?;

    write_string(ws, &fc.row_offset(1), "Control")?;
    write_duration_option(ws, &fc.offset(1, 1), stages.total_control_time())?;

    write_string(ws, &fc.row_offset(2), "Moving")?;
    write_duration_option(ws, &fc.offset(2, 1), stages.total_moving_time())?;

    fc.next_colour_block(2);
    Ok(())
}

fn output_distance(
    ws: &mut Worksheet,
    fc: &mut FormatControl,
    stages: &StageList,
) -> Result<(), Box<dyn Error>> {
    write_headers(ws, fc, "Distance (km)", &["Stage", "Running"])?;
    ws.set_column_width(fc.col, KILOMETRES_COLUMN_WIDTH)?;
    ws.set_column_width(fc.col + 1, METRES_COLUMN_WIDTH)?;

    for stage in stages {
        if stage.stage_type == StageType::Moving {
            write_kilometres(ws, fc, stage.distance_km())?;
            write_kilometres(ws, &fc.col_offset(1), stage.running_distance_km())?;
        } else {
            write_blank(ws, fc)?;
            write_blank(ws, &fc.col_offset(1))?;
        }

        fc.increment_row();
    }

    fc.start_summary_row();
    write_blank(ws, fc)?;
    write_kilometres(ws, &fc.col_offset(1), stages.distance_km())?;

    fc.next_colour_block(2);
    Ok(())
}

fn output_average_speed(
    ws: &mut Worksheet,
    fc: &mut FormatControl,
    stages: &StageList,
) -> Result<(), Box<dyn Error>> {
    write_headers(ws, fc, "Avg Speed (km/h)", &["Stage", "Running"])?;
    ws.set_column_width(fc.col, SPEED_COLUMN_WIDTH)?;
    ws.set_column_width(fc.col + 1, SPEED_COLUMN_WIDTH)?;

    for stage in stages {
        if stage.stage_type == StageType::Moving {
            write_speed_option(ws, fc, stage.average_speed_kmh())?;
            write_speed_option(ws, &fc.col_offset(1), stage.running_average_speed_kmh())?;
        } else {
            write_blank(ws, fc)?;
            write_blank(ws, &fc.col_offset(1))?;
        }

        fc.increment_row();
    }

    fc.start_summary_row();
    write_string(ws, fc, "Overall")?;
    write_speed_option(ws, &fc.col_offset(1), stages.average_overall_speed())?;
    write_string(ws, &fc.row_offset(1), "Moving")?;
    write_speed_option(ws, &fc.offset(1, 1), stages.average_moving_speed())?;

    fc.next_colour_block(2);
    Ok(())
}

fn output_ascent(
    ws: &mut Worksheet,
    fc: &mut FormatControl,
    stages: &StageList,
) -> Result<(), Box<dyn Error>> {
    write_headers(ws, fc, "Ascent (m)", &["Stage", "Running", "m/km"])?;
    ws.set_column_width(fc.col, METRES_COLUMN_WIDTH)?;
    ws.set_column_width(fc.col + 1, METRES_COLUMN_WIDTH)?;
    ws.set_column_width(fc.col + 2, METRES_COLUMN_WIDTH)?;

    for stage in stages {
        if stage.stage_type == StageType::Moving {
            write_metres_option(ws, fc, stage.ascent_metres())?;
            write_metres_option(ws, &fc.col_offset(1), stage.running_ascent_metres())?;
            write_metres_option(ws, &fc.col_offset(2), stage.ascent_rate_per_km())?;
        } else {
            write_blank(ws, fc)?;
            write_blank(ws, &fc.col_offset(1))?;
            write_blank(ws, &fc.col_offset(2))?;
        }

        fc.increment_row();
    }

    fc.start_summary_row();
    write_blank(ws, fc)?;
    write_metres_option(ws, &fc.col_offset(1), stages.total_ascent_metres())?;
    let rate = stages
        .total_ascent_metres()
        .map(|a| a / stages.distance_km());
    write_metres_option(ws, &fc.col_offset(2), rate)?;

    fc.next_colour_block(3);
    Ok(())
}

fn output_descent(
    ws: &mut Worksheet,
    fc: &mut FormatControl,
    stages: &StageList,
) -> Result<(), Box<dyn Error>> {
    write_headers(ws, fc, "Descent (m)", &["Stage", "Running", "m/km"])?;
    ws.set_column_width(fc.col, METRES_COLUMN_WIDTH)?;
    ws.set_column_width(fc.col + 1, METRES_COLUMN_WIDTH)?;
    ws.set_column_width(fc.col + 2, METRES_COLUMN_WIDTH)?;

    for stage in stages {
        if stage.stage_type == StageType::Moving {
            write_metres_option(ws, fc, stage.descent_metres())?;
            write_metres_option(ws, &fc.col_offset(1), stage.running_descent_metres())?;
            write_metres_option(ws, &fc.col_offset(2), stage.descent_rate_per_km())?;
        } else {
            write_blank(ws, fc)?;
            write_blank(ws, &fc.col_offset(1))?;
            write_blank(ws, &fc.col_offset(2))?;
        }

        fc.increment_row();
    }

    fc.start_summary_row();
    write_blank(ws, fc)?;
    write_metres_option(ws, &fc.col_offset(1), stages.total_descent_metres())?;
    let rate = stages
        .total_descent_metres()
        .map(|a| a / stages.distance_km());
    write_metres_option(ws, &fc.col_offset(2), rate)?;

    fc.next_colour_block(3);
    Ok(())
}

fn output_min_elevation(
    ws: &mut Worksheet,
    fc: &mut FormatControl,
    stages: &StageList,
) -> Result<(), Box<dyn Error>> {
    write_headers(
        ws,
        fc,
        "Min Elevation",
        &["Elevation (m)", "Distance (km)", "Point"],
    )?;
    ws.set_column_width(fc.col, ELEVATION_COLUMN_WIDTH_WITH_UNITS)?;
    ws.set_column_width(fc.col + 1, KILOMETRES_COLUMN_WIDTH_WITH_UNITS)?;

    for stage in stages {
        if stage.stage_type == StageType::Moving {
            write_elevation_data(ws, fc, stage.min_elevation.as_ref())?;
        } else {
            write_blank(ws, fc)?;
            write_blank(ws, &fc.col_offset(1))?;
            write_blank(ws, &fc.col_offset(2))?;
        }

        fc.increment_row();
    }

    fc.start_summary_row();
    write_elevation_data(ws, fc, stages.min_elevation())?;

    fc.next_colour_block(3);
    Ok(())
}

fn output_max_elevation(
    ws: &mut Worksheet,
    fc: &mut FormatControl,
    stages: &StageList,
) -> Result<(), Box<dyn Error>> {
    write_headers(
        ws,
        fc,
        "Max Elevation",
        &["Elevation (m)", "Distance (km)", "Point"],
    )?;
    ws.set_column_width(fc.col, ELEVATION_COLUMN_WIDTH_WITH_UNITS)?;
    ws.set_column_width(fc.col + 1, KILOMETRES_COLUMN_WIDTH_WITH_UNITS)?;

    for stage in stages {
        if stage.stage_type == StageType::Moving {
            write_elevation_data(ws, fc, stage.max_elevation.as_ref())?;
        } else {
            write_blank(ws, fc)?;
            write_blank(ws, &fc.col_offset(1))?;
            write_blank(ws, &fc.col_offset(2))?;
        }

        fc.increment_row();
    }

    fc.start_summary_row();
    write_elevation_data(ws, fc, stages.max_elevation())?;

    fc.next_colour_block(3);
    Ok(())
}

fn output_max_speed(
    ws: &mut Worksheet,
    fc: &mut FormatControl,
    stages: &StageList,
) -> Result<(), Box<dyn Error>> {
    write_headers(
        ws,
        fc,
        "Max Speed",
        &["Speed (km/h)", "Distance (km)", "Point"],
    )?;
    ws.set_column_width(fc.col, SPEED_COLUMN_WIDTH_WITH_UNITS)?;
    ws.set_column_width(fc.col + 1, KILOMETRES_COLUMN_WIDTH_WITH_UNITS)?;

    for stage in stages {
        if stage.stage_type == StageType::Moving {
            write_max_speed_data(ws, fc, stage.max_speed.as_ref())?;
        } else {
            write_blank(ws, fc)?;
            write_blank(ws, &fc.col_offset(1))?;
            write_blank(ws, &fc.col_offset(2))?;
        }

        fc.increment_row();
    }

    fc.start_summary_row();
    write_max_speed_data(ws, fc, stages.max_speed())?;

    fc.next_colour_block(3);
    Ok(())
}

fn output_heart_rate(
    ws: &mut Worksheet,
    fc: &mut FormatControl,
    stages: &StageList,
    avg_heart_rate: Option<f64>,
) -> Result<(), Box<dyn Error>> {
    write_headers(
        ws,
        fc,
        "Heart Rate",
        &["Avg", "Max", "Distance (km)", "Point"],
    )?;
    ws.set_column_width(fc.col + 2, KILOMETRES_COLUMN_WIDTH_WITH_UNITS)?;

    for stage in stages {
        write_heart_rate_data(ws, fc, stage.max_heart_rate.as_ref(), stage.avg_heart_rate)?;
        fc.increment_row();
    }

    fc.start_summary_row();
    write_heart_rate_data(ws, fc, stages.max_heart_rate(), avg_heart_rate)?;

    fc.next_colour_block(4);
    Ok(())
}

fn output_temperature(
    ws: &mut Worksheet,
    fc: &mut FormatControl,
    stages: &StageList,
    avg_temp: Option<f64>,
) -> Result<(), Box<dyn Error>> {
    write_headers(
        ws,
        fc,
        "Temp °C",
        &[
            "Avg",
            "Min",
            "Time (local)",
            "Point",
            "Max",
            "Time (local)",
            "Point",
        ],
    )?;
    ws.set_column_width(fc.col + 2, DATE_COLUMN_WIDTH)?;
    ws.set_column_width(fc.col + 5, DATE_COLUMN_WIDTH)?;

    for stage in stages {
        write_temperature_data(
            ws,
            fc,
            stage.min_air_temp.as_ref(),
            stage.max_air_temp.as_ref(),
            stage.avg_air_temp,
        )?;

        fc.increment_row();
    }

    fc.start_summary_row();
    write_temperature_data(
        ws,
        fc,
        stages.min_temperature(),
        stages.max_temperature(),
        avg_temp,
    )?;

    fc.next_colour_block(7);
    Ok(())
}

fn output_track_points(
    ws: &mut Worksheet,
    fc: &mut FormatControl,
    stages: &StageList,
) -> Result<(), Box<dyn Error>> {
    write_headers(ws, fc, "Track Points", &["First", "Last", "Count"])?;

    for stage in stages {
        write_trackpoint_number(ws, fc, stage.start.index)?;
        write_trackpoint_number(ws, &fc.col_offset(1), stage.end.index)?;
        write_integer(
            ws,
            &fc.col_offset(2),
            (stage.end.index - stage.start.index + 1).try_into()?,
        )?;

        fc.increment_row();
    }

    fc.start_summary_row();
    write_trackpoint_number(ws, fc, stages.first_point().index)?;
    write_trackpoint_number(ws, &fc.col_offset(1), stages.last_point().index)?;
    let count = (stages.last_point().index - stages.first_point().index + 1).try_into()?;
    write_integer(ws, &fc.col_offset(2), count)?;

    fc.next_colour_block(3);
    Ok(())
}

#[time]
fn write_trackpoints(
    ws: &mut Worksheet,
    points: &[EnrichedTrackPoint],
    hyperlink: Hyperlink,
    mandatory_hyperlinks: &HashSet<usize>,
) -> Result<(), Box<dyn Error>> {
    let mut fc = FormatControl::new();

    ws.set_freeze_panes(2, 0)?;

    output_tp_index(ws, &mut fc, points)?;
    output_tp_time(ws, &mut fc, points)?;
    output_tp_location(ws, &mut fc, points, hyperlink, mandatory_hyperlinks)?;
    output_tp_elevation(ws, &mut fc, points)?;
    output_tp_distance(ws, &mut fc, points)?;
    output_tp_speed(ws, &mut fc, points)?;
    output_tp_heart_rate(ws, &mut fc, points)?;
    output_tp_air_temp(ws, &mut fc, points)?;
    output_tp_cadence(ws, &mut fc, points)?;

    ws.autofilter(1, 0, points.len() as u32 + 1, fc.col)?;
    Ok(())
}

fn output_tp_index(
    ws: &mut Worksheet,
    fc: &mut FormatControl,
    points: &[EnrichedTrackPoint],
) -> Result<(), Box<dyn Error>> {
    write_headers(ws, fc, "", &["Index"])?;

    for p in points {
        write_integer(ws, fc, p.index as u32)?;
        fc.increment_row();
    }

    fc.next_colour_block(1);
    Ok(())
}

fn output_tp_time(
    ws: &mut Worksheet,
    fc: &mut FormatControl,
    points: &[EnrichedTrackPoint],
) -> Result<(), Box<dyn Error>> {
    write_headers(ws, fc, "Time", &["UTC", "Local", "Delta", "Running"])?;
    ws.set_column_width(fc.col, DATE_COLUMN_WIDTH)?;
    ws.set_column_width(fc.col + 1, DATE_COLUMN_WIDTH)?;
    ws.set_column_width(fc.col + 2, DURATION_COLUMN_WIDTH)?;
    ws.set_column_width(fc.col + 3, DURATION_COLUMN_WIDTH)?;

    for p in points {
        match p.time {
            Some(time) => {
                write_utc_date(ws, fc, time)?;
                write_utc_date_as_local(ws, &fc.col_offset(1), time)?;
            }
            None => {
                write_blank(ws, fc)?;
                write_blank(ws, &fc.col_offset(1))?;
            }
        }

        write_duration_option(ws, &fc.col_offset(2), p.delta_time)?;
        write_duration_option(ws, &fc.col_offset(3), p.running_delta_time)?;
        fc.increment_row();
    }

    fc.next_colour_block(4);
    Ok(())
}

fn output_tp_location(
    ws: &mut Worksheet,
    fc: &mut FormatControl,
    points: &[EnrichedTrackPoint],
    hyperlink: Hyperlink,
    mandatory_hyperlinks: &HashSet<usize>,
) -> Result<(), Box<dyn Error>> {
    write_headers(ws, fc, "Location", &["Lat", "Lon", "Map", "Description"])?;
    ws.set_column_width(fc.col, LAT_LON_COLUMN_WIDTH)?;
    ws.set_column_width(fc.col + 1, LAT_LON_COLUMN_WIDTH)?;
    ws.set_column_width(fc.col + 2, LINKED_LAT_LON_COLUMN_WIDTH)?;
    ws.set_column_width(fc.col + 3, LOCATION_DESCRIPTION_COLUMN_WIDTH)?;

    for p in points {
        let hyp = match hyperlink {
            Hyperlink::Yes => Hyperlink::Yes,
            Hyperlink::No => {
                if mandatory_hyperlinks.contains(&p.index) {
                    Hyperlink::Yes
                } else {
                    Hyperlink::No
                }
            }
        };

        write_lat_lon(ws, fc, (p.lat, p.lon), hyp, p.location.as_ref())?;

        fc.increment_row();
    }

    fc.next_colour_block(4);
    Ok(())
}

fn output_tp_elevation(
    ws: &mut Worksheet,
    fc: &mut FormatControl,
    points: &[EnrichedTrackPoint],
) -> Result<(), Box<dyn Error>> {
    write_headers(
        ws,
        fc,
        "Elevation (m)",
        &["Height", "Delta", "Running Ascent", "Running Descent"],
    )?;
    ws.set_column_width(fc.col, METRES_COLUMN_WIDTH_WITH_UNITS)?;
    ws.set_column_width(fc.col + 1, METRES_COLUMN_WIDTH_WITH_UNITS)?;
    ws.set_column_width(fc.col + 2, KILOMETRES_COLUMN_WIDTH_WITH_UNITS)?;
    ws.set_column_width(fc.col + 3, KILOMETRES_COLUMN_WIDTH_WITH_UNITS)?;

    for p in points {
        match p.ele {
            Some(ele) => {
                write_metres(ws, fc, ele)?;
            }
            None => {
                write_blank(ws, fc)?;
            }
        }

        write_metres_option(ws, &fc.col_offset(1), p.ele_delta_metres)?;
        write_metres_option(ws, &fc.col_offset(2), p.running_ascent_metres)?;
        write_metres_option(ws, &fc.col_offset(3), p.running_descent_metres)?;
        fc.increment_row();
    }

    fc.next_colour_block(4);
    Ok(())
}

fn output_tp_distance(
    ws: &mut Worksheet,
    fc: &mut FormatControl,
    points: &[EnrichedTrackPoint],
) -> Result<(), Box<dyn Error>> {
    write_headers(ws, fc, "Distance", &["Delta (m)", "Running (km)"])?;
    ws.set_column_width(fc.col, METRES_COLUMN_WIDTH_WITH_UNITS)?;
    ws.set_column_width(fc.col + 1, KILOMETRES_COLUMN_WIDTH_WITH_UNITS)?;

    for p in points {
        write_metres(ws, fc, p.delta_metres)?;
        write_kilometres(ws, &fc.col_offset(1), p.running_metres / 1000.0)?;
        fc.increment_row();
    }

    fc.next_colour_block(2);
    Ok(())
}

fn output_tp_speed(
    ws: &mut Worksheet,
    fc: &mut FormatControl,
    points: &[EnrichedTrackPoint],
) -> Result<(), Box<dyn Error>> {
    write_headers(ws, fc, "", &["Speed (km/h)"])?;
    ws.set_column_width(fc.col, SPEED_COLUMN_WIDTH_WITH_UNITS)?;

    for p in points {
        write_speed_option(ws, fc, p.speed_kmh)?;
        fc.increment_row();
    }

    fc.next_colour_block(1);
    Ok(())
}

fn output_tp_heart_rate(
    ws: &mut Worksheet,
    fc: &mut FormatControl,
    points: &[EnrichedTrackPoint],
) -> Result<(), Box<dyn Error>> {
    write_headers(ws, fc, "", &["Heart Rate (bpm)"])?;
    ws.set_column_width(fc.col, HEART_RATE_WIDTH_WITH_UNITS)?;

    for p in points {
        if let Some(hr) = p.heart_rate() {
            write_integer(ws, fc, hr.into())?;
        } else {
            write_blank(ws, fc)?;
        }

        fc.increment_row();
    }

    fc.next_colour_block(1);
    Ok(())
}

fn output_tp_air_temp(
    ws: &mut Worksheet,
    fc: &mut FormatControl,
    points: &[EnrichedTrackPoint],
) -> Result<(), Box<dyn Error>> {
    write_headers(ws, fc, "", &["Temp (°C)"])?;
    ws.set_column_width(fc.col, TEMPERATURE_COLUMN_WIDTH_WITH_UNITS)?;

    for p in points {
        write_f64_option(ws, fc, p.air_temp())?;
        fc.increment_row();
    }

    fc.next_colour_block(1);
    Ok(())
}

fn output_tp_cadence(
    ws: &mut Worksheet,
    fc: &mut FormatControl,
    points: &[EnrichedTrackPoint],
) -> Result<(), Box<dyn Error>> {
    write_headers(ws, fc, "", &["Cadence (rpm)"])?;
    ws.set_column_width(fc.col, CADENCE_COLUMN_WIDTH_WITH_UNITS)?;

    for p in points {
        if let Some(cad) = p.cadence() {
            write_integer(ws, fc, cad.into())?;
        } else {
            write_blank(ws, fc)?;
        }

        fc.increment_row();
    }

    Ok(())
}

// Utility functions.

/// Writes a main heading (which can be blank) and a set of
/// sub-headings, and automatically merges the columns of
/// the main heading if necessary.
fn write_headers(
    ws: &mut Worksheet,
    fc: &FormatControl,
    main_heading: &str,
    sub_headings: &[&str],
) -> Result<(), Box<dyn Error>> {
    if main_heading.is_empty() {
        ws.write_blank(0, fc.col, &fc.minor_header_format())?;
    } else {
        ws.merge_range(
            0,
            fc.col,
            0,
            fc.col + sub_headings.len() as u16 - 1,
            main_heading,
            &fc.minor_header_format(),
        )?;
    }

    for (idx, &heading) in sub_headings.iter().enumerate() {
        ws.write_string_with_format(1, fc.col + idx as u16, heading, &fc.minor_header_format())?;
    }

    Ok(())
}

/// Writes a lat-lon pair with the lat in the first cell as specified
/// by 'rc' and the lon in the next column. If 'hyperlink' is yes then
/// a hyperlink to Google Maps is written into the third column.
fn write_lat_lon(
    ws: &mut Worksheet,
    fc: &FormatControl,
    (lat, lon): (f64, f64),
    hyperlink: Hyperlink,
    location: Option<&String>,
) -> Result<(), Box<dyn Error>> {
    let format = fc.lat_lon_format().set_font_color(Color::Black);

    ws.write_number_with_format(fc.row, fc.col, lat, &format)?;
    ws.write_number_with_format(fc.row, fc.col + 1, lon, &format)?;

    match hyperlink {
        Hyperlink::Yes => {
            let url = make_hyperlink((lat, lon));
            // TODO: Font still blue.
            let format = format.set_align(FormatAlign::Right);
            ws.write_url_with_format(fc.row, fc.col + 2, url, &format)?;
        }
        Hyperlink::No => {
            write_blank(ws, &fc.col_offset(2))?;
        }
    };

    let fc = fc.col_offset(3);
    if let Some(location) = location {
        if !location.is_empty() {
            ws.write_string_with_format(fc.row, fc.col, location, &fc.location_format())?;
        } else {
            write_blank(ws, &fc)?;
        }
    } else {
        write_blank(ws, &fc)?;
    }

    Ok(())
}

/// Writes an integer.
fn write_integer(ws: &mut Worksheet, fc: &FormatControl, value: u32) -> Result<(), Box<dyn Error>> {
    ws.write_number_with_format(fc.row, fc.col, value, &fc.integer_format())?;
    Ok(())
}

/// Writes a blank into a cell. We often want to do this when there is no data
/// so that banding formatting is applied to the cell.
fn write_blank(ws: &mut Worksheet, fc: &FormatControl) -> Result<(), Box<dyn Error>> {
    ws.write_blank(fc.row, fc.col, &fc.string_format())?;
    Ok(())
}

/// Writes an elevation data block (min or max) as found on the Stages tab.
fn write_elevation_data(
    ws: &mut Worksheet,
    fc: &FormatControl,
    point: Option<&EnrichedTrackPoint>,
) -> Result<(), Box<dyn Error>> {
    if point.is_none() {
        write_blank(ws, fc)?;
        write_blank(ws, &fc.col_offset(1))?;
        write_blank(ws, &fc.col_offset(2))?;
        return Ok(());
    }

    let point = point.unwrap();

    match point.ele {
        Some(ele) => {
            write_metres(ws, fc, ele)?;
        }
        None => {
            write_blank(ws, fc)?;
        }
    }

    write_kilometres_running_with_map_hyperlink(ws, &fc.col_offset(1), point)?;
    write_trackpoint_number(ws, &fc.col_offset(2), point.index)?;
    Ok(())
}

/// Writes a string right aligned.
fn write_string(ws: &mut Worksheet, fc: &FormatControl, value: &str) -> Result<(), Box<dyn Error>> {
    ws.write_string_with_format(fc.row, fc.col, value, &fc.string_format())?;
    Ok(())
}

/// Writes a string right aligned and bold.
fn write_string_bold(
    ws: &mut Worksheet,
    fc: &FormatControl,
    value: &str,
) -> Result<(), Box<dyn Error>> {
    let format = fc.string_format().set_bold();
    ws.write_string_with_format(fc.row, fc.col, value, &format)?;
    Ok(())
}

/// Writes a float.
fn write_f64(ws: &mut Worksheet, fc: &FormatControl, value: f64) -> Result<(), Box<dyn Error>> {
    ws.write_number_with_format(fc.row, fc.col, value, &fc.float_format())?;
    Ok(())
}

/// Writes an optional float.
fn write_f64_option(
    ws: &mut Worksheet,
    fc: &FormatControl,
    value: Option<f64>,
) -> Result<(), Box<dyn Error>> {
    if let Some(value) = value {
        write_f64(ws, fc, value)?;
    } else {
        write_blank(ws, fc)?;
    }
    Ok(())
}

/// Formats 'utc_date' into a string like "2024-09-01T05:10:44Z".
/// This is the format that GPX files contain.
fn write_utc_date(
    ws: &mut Worksheet,
    fc: &FormatControl,
    utc_date: OffsetDateTime,
) -> Result<(), Box<dyn Error>> {
    assert!(utc_date.offset().is_utc());
    let excel_date = date_to_excel_date(utc_date)?;
    ws.write_with_format(fc.row, fc.col, &excel_date, &fc.utc_date_format())?;
    Ok(())
}

fn write_utc_date_option(
    ws: &mut Worksheet,
    fc: &FormatControl,
    utc_date: Option<OffsetDateTime>,
) -> Result<(), Box<dyn Error>> {
    if let Some(d) = utc_date {
        write_utc_date(ws, fc, d)?;
    } else {
        write_blank(ws, fc)?;
    }
    Ok(())
}

/// Converts 'utc_date' to a local date and then formats it into
/// a string like "2024-09-01 05:10:44".
fn write_utc_date_as_local(
    ws: &mut Worksheet,
    fc: &FormatControl,
    utc_date: OffsetDateTime,
) -> Result<(), Box<dyn Error>> {
    assert!(utc_date.offset().is_utc());
    let excel_date = date_to_excel_date(to_local_date(utc_date))?;
    ws.write_with_format(fc.row, fc.col, &excel_date, &fc.local_date_format())?;
    Ok(())
}

fn write_utc_date_as_local_option(
    ws: &mut Worksheet,
    fc: &FormatControl,
    utc_date: Option<OffsetDateTime>,
) -> Result<(), Box<dyn Error>> {
    if let Some(d) = utc_date {
        write_utc_date_as_local(ws, fc, d)?;
    } else {
        write_blank(ws, fc)?;
    }
    Ok(())
}

fn date_to_excel_date(date: OffsetDateTime) -> Result<ExcelDateTime, Box<dyn Error>> {
    let excel_date =
        ExcelDateTime::from_ymd(date.year().try_into()?, date.month().into(), date.day())?;

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
    duration: Duration,
) -> Result<(), Box<dyn Error>> {
    let excel_duration = duration_to_excel_date(duration)?;
    ws.write_with_format(fc.row, fc.col, excel_duration, &fc.duration_format())?;
    Ok(())
}

fn write_duration_option(
    ws: &mut Worksheet,
    fc: &FormatControl,
    duration: Option<Duration>,
) -> Result<(), Box<dyn Error>> {
    if let Some(dur) = duration {
        write_duration(ws, fc, dur)?;
    } else {
        write_blank(ws, fc)?;
    }

    Ok(())
}

fn duration_to_excel_date(duration: Duration) -> Result<ExcelDateTime, Box<dyn Error>> {
    const SECONDS_PER_MINUTE: u32 = 60;
    const SECONDS_PER_HOUR: u32 = SECONDS_PER_MINUTE * 60;

    let mut all_secs: u32 = duration.as_seconds_f64() as u32;
    let hours: u16 = (all_secs / SECONDS_PER_HOUR).try_into()?;
    all_secs -= hours as u32 * SECONDS_PER_HOUR;

    let minutes: u8 = (all_secs / SECONDS_PER_MINUTE).try_into()?;
    all_secs -= minutes as u32 * SECONDS_PER_MINUTE;

    let seconds: u16 = all_secs.try_into()?;

    Ok(ExcelDateTime::from_hms(hours, minutes, seconds)?)
}

/// Writes a max speed data block as found on the Stages tab.
fn write_max_speed_data(
    ws: &mut Worksheet,
    fc: &FormatControl,
    point: Option<&EnrichedTrackPoint>,
) -> Result<(), Box<dyn Error>> {
    if point.is_none() {
        write_blank(ws, fc)?;
        write_blank(ws, &fc.col_offset(1))?;
        write_blank(ws, &fc.col_offset(2))?;
        return Ok(());
    }

    let point = point.unwrap();

    write_speed_option(ws, fc, point.speed_kmh)?;
    write_kilometres_running_with_map_hyperlink(ws, &fc.col_offset(1), point)?;
    write_trackpoint_number(ws, &fc.col_offset(2), point.index)?;
    Ok(())
}

fn write_heart_rate_data(
    ws: &mut Worksheet,
    fc: &FormatControl,
    max_hr_point: Option<&EnrichedTrackPoint>,
    avg_hr: Option<f64>,
) -> Result<(), Box<dyn Error>> {
    write_f64_option(ws, fc, avg_hr)?;

    if let Some(point) = max_hr_point {
        if let Some(mhr) = point.heart_rate() {
            write_integer(ws, &fc.col_offset(1), mhr as u32)?;
            write_kilometres_running_with_map_hyperlink(ws, &fc.col_offset(2), point)?;
            write_trackpoint_number(ws, &fc.col_offset(3), point.index)?;
            return Ok(());
        }
    }

    write_blank(ws, &fc.col_offset(1))?;
    write_blank(ws, &fc.col_offset(2))?;
    write_blank(ws, &fc.col_offset(3))?;
    Ok(())
}

fn write_temperature_data(
    ws: &mut Worksheet,
    fc: &FormatControl,
    min: Option<&EnrichedTrackPoint>,
    max: Option<&EnrichedTrackPoint>,
    avg: Option<f64>,
) -> Result<(), Box<dyn Error>> {
    write_f64_option(ws, fc, avg)?;

    if let Some(min) = min {
        write_temperature_option(ws, &fc.col_offset(1), min.air_temp())?;
        write_utc_date_as_local_option(ws, &fc.col_offset(2), min.time)?;
        write_trackpoint_number(ws, &fc.col_offset(3), min.index)?;
    } else {
        write_blank(ws, &fc.col_offset(1))?;
        write_blank(ws, &fc.col_offset(2))?;
        write_blank(ws, &fc.col_offset(3))?;
    }

    if let Some(max) = max {
        write_temperature_option(ws, &fc.col_offset(4), max.air_temp())?;
        write_utc_date_as_local_option(ws, &fc.col_offset(5), max.time)?;
        write_trackpoint_number(ws, &fc.col_offset(6), max.index)?;
    } else {
        write_blank(ws, &fc.col_offset(4))?;
        write_blank(ws, &fc.col_offset(5))?;
        write_blank(ws, &fc.col_offset(6))?;
    }

    Ok(())
}

fn write_temperature(
    ws: &mut Worksheet,
    fc: &FormatControl,
    temperature: f64,
) -> Result<(), Box<dyn Error>> {
    let format = fc.temperature_format();
    ws.write_number_with_format(fc.row, fc.col, temperature, &format)?;
    Ok(())
}

fn write_temperature_option(
    ws: &mut Worksheet,
    fc: &FormatControl,
    temperature: Option<f64>,
) -> Result<(), Box<dyn Error>> {
    if let Some(t) = temperature {
        write_temperature(ws, fc, t)?;
    } else {
        write_blank(ws, fc)?;
    }
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

/// Writes a TrackPoint index, including a hyperlink to
/// the 'Track Points' sheet.
fn write_trackpoint_number(
    ws: &mut Worksheet,
    fc: &FormatControl,
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

    ws.write_url_with_format(fc.row, fc.col, url, &format)?;

    Ok(())
}

fn write_metres(ws: &mut Worksheet, fc: &FormatControl, metres: f64) -> Result<(), Box<dyn Error>> {
    ws.write_number_with_format(fc.row, fc.col, metres, &fc.metres_format())?;
    // TODO: Use conditional formatting to indicate negatives?
    Ok(())
}

fn write_metres_option(
    ws: &mut Worksheet,
    fc: &FormatControl,
    metres: Option<f64>,
) -> Result<(), Box<dyn Error>> {
    if let Some(m) = metres {
        write_metres(ws, fc, m)?;
    } else {
        write_blank(ws, fc)?;
    }
    Ok(())
}

fn write_kilometres(
    ws: &mut Worksheet,
    fc: &FormatControl,
    kilometres: f64,
) -> Result<(), Box<dyn Error>> {
    ws.write_number_with_format(fc.row, fc.col, kilometres, &fc.kilometres_format())?;
    Ok(())
}

fn write_kilometres_running_with_map_hyperlink(
    ws: &mut Worksheet,
    fc: &FormatControl,
    point: &EnrichedTrackPoint,
) -> Result<(), Box<dyn Error>> {
    let km = point.running_metres / 1000.0;
    let url = make_hyperlink_with_text((point.lat, point.lon), &format!("{:.3}", km));
    let format = fc.kilometres_format();
    let format = format.set_align(FormatAlign::Right);
    ws.write_url_with_format(fc.row, fc.col, url, &format)?;
    Ok(())
}

fn write_speed(ws: &mut Worksheet, fc: &FormatControl, speed: f64) -> Result<(), Box<dyn Error>> {
    ws.write_number_with_format(fc.row, fc.col, speed, &fc.speed_format())?;
    Ok(())
}

fn write_speed_option(
    ws: &mut Worksheet,
    fc: &FormatControl,
    speed: Option<f64>,
) -> Result<(), Box<dyn Error>> {
    if let Some(s) = speed {
        write_speed(ws, fc, s)?;
    } else {
        write_blank(ws, fc)?;
    }

    Ok(())
}

/// Little struct to control the colours and banding of the Excel output. 16
/// bytes in size = 128 bits. These will fit into 2 registers, but if you change
/// the write* methods to do pass-by-value you have to de-reference in a million
/// places in the output* methods. So it's best to leave it all as pass by
/// reference.
struct FormatControl {
    row: u32,
    col: u16,
    current_background_color: Color,
    always_set_background_color: bool,
}

impl FormatControl {
    const COLOR1: Color = Color::Theme(3, 1);
    const COLOR2: Color = Color::Theme(2, 1);
    const STARTING_ROW: u32 = 2;

    fn new() -> Self {
        Self {
            current_background_color: Self::COLOR1,
            col: 0,
            row: Self::STARTING_ROW,
            always_set_background_color: false,
        }
    }

    /// Returns a new FormatControl with an offset applied to the column.
    fn col_offset(&self, col_offset: u16) -> Self {
        Self {
            always_set_background_color: self.always_set_background_color,
            current_background_color: self.current_background_color,
            row: self.row,
            col: self.col + col_offset,
        }
    }

    /// Returns a new FormatControl with an offset applied to the row.
    fn row_offset(&self, row_offset: u32) -> Self {
        Self {
            always_set_background_color: self.always_set_background_color,
            current_background_color: self.current_background_color,
            row: self.row + row_offset,
            col: self.col,
        }
    }

    /// Returns a new FormatControl with offsets applied to the row and column.
    fn offset(&self, row_offset: u32, col_offset: u16) -> Self {
        Self {
            always_set_background_color: self.always_set_background_color,
            current_background_color: self.current_background_color,
            row: self.row + row_offset,
            col: self.col + col_offset,
        }
    }

    fn increment_row(&mut self) {
        self.row += 1;
    }

    /// This is used to get to a good row to show the summary.
    /// We want the banding colours to be "on" for this row.
    fn start_summary_row(&mut self) {
        self.increment_row();
        self.increment_row();
        self.always_set_background_color = true;
    }

    fn next_colour_block(&mut self, col_increment: u16) {
        if self.current_background_color == Self::COLOR1 {
            self.current_background_color = Self::COLOR2;
        } else {
            self.current_background_color = Self::COLOR1;
        }
        self.col += col_increment;
        self.row = Self::STARTING_ROW;
        self.always_set_background_color = false;
    }

    fn minor_header_format(&self) -> Format {
        Format::new()
            .set_bold()
            .set_font_color(Color::Black)
            .set_border(FormatBorder::Thin)
            .set_border_color(Color::Gray)
            .set_align(FormatAlign::Center)
            .set_pattern(FormatPattern::Solid)
            .set_background_color(self.current_background_color)
    }

    fn speed_format(&self) -> Format {
        let format = Format::new().set_num_format("0.##");
        self.apply_background_color_if_needed(format)
    }

    fn lat_lon_format(&self) -> Format {
        let format = Format::new().set_num_format("0.000000");
        self.apply_background_color_if_needed(format)
    }

    fn integer_format(&self) -> Format {
        let format = Format::new().set_num_format("0");
        self.apply_background_color_if_needed(format)
    }

    fn float_format(&self) -> Format {
        let format = Format::new().set_num_format("0.0");
        self.apply_background_color_if_needed(format)
    }

    fn string_format(&self) -> Format {
        let format = Format::new().set_align(FormatAlign::Right);
        self.apply_background_color_if_needed(format)
    }

    fn location_format(&self) -> Format {
        let format = Format::new().set_align(FormatAlign::Left);
        self.apply_background_color_if_needed(format)
    }

    fn utc_date_format(&self) -> Format {
        let format = Format::new().set_num_format("yyyy-mm-ddThh:mm:ssZ");
        self.apply_background_color_if_needed(format)
    }

    fn local_date_format(&self) -> Format {
        let format = Format::new().set_num_format("yyyy-mm-dd hh:mm:ss");
        self.apply_background_color_if_needed(format)
    }

    fn duration_format(&self) -> Format {
        let format = Format::new().set_num_format("hh:mm:ss");
        self.apply_background_color_if_needed(format)
    }

    fn metres_format(&self) -> Format {
        let format = Format::new().set_num_format("0.##");
        self.apply_background_color_if_needed(format)
    }

    fn temperature_format(&self) -> Format {
        let format = Format::new().set_num_format("0.#");
        self.apply_background_color_if_needed(format)
    }

    fn kilometres_format(&self) -> Format {
        let format = Format::new().set_num_format("0.000");
        self.apply_background_color_if_needed(format)
    }

    /// Helper method.
    fn apply_background_color_if_needed(&self, format: Format) -> Format {
        let mut format = format;
        if self.row % 2 == 0 || self.always_set_background_color {
            format = format.set_background_color(self.current_background_color);
        }
        format
    }
}
