use std::{error::Error, path::Path, sync::LazyLock};

use rust_xlsxwriter::{
    Color, ExcelDateTime, Format, FormatAlign, FormatBorder, FormatPattern, Workbook, Worksheet,
};
use time::{Duration, OffsetDateTime};

use crate::{
    formatting::to_local_date,
    model::{EnrichedGpx, EnrichedTrackPoint},
    stage::{StageList, StageType},
};

const DATE_COLUMN_WIDTH: f64 = 18.0;
const DURATION_COLUMN_WIDTH: f64 = 12.0;
const LAT_LON_COLUMN_WIDTH: f64 = 10.0;
const LINKED_LAT_LON_COLUMN_WIDTH: f64 = 18.0;
const LOCATION_DESCRIPTION_COLUMN_WIDTH: f64 = 18.0;
const STANDARD_METRES_COLUMN_WIDTH: f64 = 11.0;
const RUNNING_KILOMETRES_COLUMN_WIDTH: f64 = 15.0;
const SPEED_COLUMN_WIDTH: f64 = 14.0;

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
    //write_trackpoints(&gpx.points, tp_ws)?;

    workbook.save(summary_filename).unwrap();
    let metadata = std::fs::metadata(summary_filename).unwrap();
    println!(", {} Kb", metadata.len() / 1024);
    Ok(())
}

fn write_stages<'gpx>(stages: &StageList<'gpx>, ws: &mut Worksheet) -> Result<(), Box<dyn Error>> {
    write_minor_header_blank(ws, (0, 0))?;
    write_minor_header(ws, (1, 0), "Stage")?;
    write_minor_header_blank(ws, (0, 1))?;
    write_minor_header(ws, (1, 1), "Type")?;

    write_minor_header_merged(ws, (0, 2), (0, 5), "Stage Location")?;
    write_minor_header(ws, (1, 2), "Lat")?;
    write_minor_header(ws, (1, 3), "Lon")?;
    write_minor_header(ws, (1, 4), "Map")?;
    write_minor_header(ws, (1, 5), "Description")?;

    write_minor_header_merged(ws, (0, 6), (0, 7), "Start Time")?;
    write_minor_header(ws, (1, 6), "UTC")?;
    write_minor_header(ws, (1, 7), "Local")?;

    write_minor_header_merged(ws, (0, 8), (0, 9), "End Time")?;
    write_minor_header(ws, (1, 8), "UTC")?;
    write_minor_header(ws, (1, 9), "Local")?;

    write_minor_header_merged(ws, (0, 10), (0, 11), "Duration")?;
    write_minor_header(ws, (1, 10), "hms")?;
    write_minor_header(ws, (1, 11), "Running")?;

    write_minor_header_merged(ws, (0, 12), (0, 13), "Distance (km)")?;
    write_minor_header(ws, (1, 12), "Stage")?;
    write_minor_header(ws, (1, 13), "Running")?;

    write_minor_header_merged(ws, (0, 14), (0, 15), "Avg Speed (kmh)")?;
    write_minor_header(ws, (1, 14), "Stage")?;
    write_minor_header(ws, (1, 15), "Running")?;

    write_minor_header_merged(ws, (0, 16), (0, 17), "Ascent (m)")?;
    write_minor_header(ws, (1, 16), "Stage")?;
    write_minor_header(ws, (1, 17), "Running")?;

    write_minor_header_merged(ws, (0, 18), (0, 19), "Descent (m)")?;
    write_minor_header(ws, (1, 18), "Stage")?;
    write_minor_header(ws, (1, 19), "Running")?;

    write_minor_header_merged(ws, (0, 20), (0, 25), "Minimum Elevation (m)")?;
    write_minor_header(ws, (1, 20), "Elevation")?;
    write_minor_header(ws, (1, 21), "Distance (km)")?;
    write_minor_header(ws, (1, 22), "Time (local)")?;
    write_minor_header(ws, (1, 23), "Lat")?;
    write_minor_header(ws, (1, 24), "Lon")?;
    write_minor_header(ws, (1, 25), "Map")?;

    write_minor_header_merged(ws, (0, 26), (0, 31), "Maximum Elevation (m)")?;
    write_minor_header(ws, (1, 26), "Elevation")?;
    write_minor_header(ws, (1, 27), "Distance (km)")?;
    write_minor_header(ws, (1, 28), "Time (local)")?;
    write_minor_header(ws, (1, 29), "Lat")?;
    write_minor_header(ws, (1, 30), "Lon")?;
    write_minor_header(ws, (1, 31), "Map")?;

    write_minor_header_merged(ws, (0, 32), (0, 35), "Max Speed (kmh)")?;
    write_minor_header(ws, (1, 32), "Speed")?;
    write_minor_header(ws, (1, 33), "Lat")?;
    write_minor_header(ws, (1, 34), "Lon")?;
    write_minor_header(ws, (1, 35), "Map")?;

    let mut row = 2;
    for (idx, stage) in stages.iter().enumerate() {
        ws.write_number(row, 0, (idx + 1) as u32)?;
        ws.write_string(row, 1, stage.stage_type.to_string())?;
        write_lat_lon(ws, (row, 2), (stage.start.lat, stage.start.lon), Hyperlink::Yes)?;
        write_location(ws, (row, 5), &stage.start.location)?;
        write_utc_date(ws, (row, 6), stage.start.time)?;
        write_utc_date_as_local(ws, (row, 7), stage.start.time)?;
        write_utc_date(ws, (row, 8), stage.end.time)?;
        write_utc_date_as_local(ws, (row, 9), stage.end.time)?;
        write_duration(ws, (row, 10), stage.duration())?;
        write_duration(ws, (row, 11), stage.duration())?;

        if stage.stage_type == StageType::Moving {
            write_kilometres(ws, (row, 12), stage.distance_km())?;
            write_kilometres(ws, (row, 13), stage.running_distance_km())?;
            write_speed(ws, (row, 14), stage.average_speed_kmh())?;
            write_speed(ws, (row, 15), stage.running_average_speed_kmh())?;
            write_metres(ws, (row, 16), stage.ascent_metres())?;
            write_metres(ws, (row, 17), stage.running_ascent_metres())?;
            write_metres(ws, (row, 18), stage.descent_metres())?;
            write_metres(ws, (row, 19), stage.running_descent_metres())?;

            write_metres(ws, (row, 20), stage.min_elevation.ele)?;
            write_metres(ws, (row, 21), stage.min_elevation.running_metres / 1000.0)?;
            write_utc_date_as_local(ws, (row, 22), stage.min_elevation.time)?;
            write_lat_lon(ws, (row, 23), (stage.min_elevation.lat, stage.min_elevation.lon), Hyperlink::Yes)?;

            write_metres(ws, (row, 26), stage.max_elevation.ele)?;
            write_metres(ws, (row, 27), stage.max_elevation.running_metres / 1000.0)?;
            write_utc_date_as_local(ws, (row, 28), stage.max_elevation.time)?;
            write_lat_lon(ws, (row, 29), (stage.max_elevation.lat, stage.max_elevation.lon), Hyperlink::Yes)?;

            write_speed(ws, (row, 32), stage.max_speed.speed_kmh)?;
            write_lat_lon(ws, (row, 33), (stage.max_speed.lat, stage.max_speed.lon), Hyperlink::Yes)?;
        }

        row += 1;
    }

    ws.set_column_width(2, LAT_LON_COLUMN_WIDTH)?;
    ws.set_column_width(3, LAT_LON_COLUMN_WIDTH)?;
    ws.set_column_width(4, LINKED_LAT_LON_COLUMN_WIDTH)?;
    ws.set_column_width(5, LOCATION_DESCRIPTION_COLUMN_WIDTH)?;
    ws.set_column_width(6, DATE_COLUMN_WIDTH)?;
    ws.set_column_width(7, DATE_COLUMN_WIDTH)?;
    ws.set_column_width(8, DATE_COLUMN_WIDTH)?;
    ws.set_column_width(9, DATE_COLUMN_WIDTH)?;
    ws.set_column_width(10, DURATION_COLUMN_WIDTH - 2.0)?;
    ws.set_column_width(11, DURATION_COLUMN_WIDTH - 2.0)?;
    ws.set_column_width(12, RUNNING_KILOMETRES_COLUMN_WIDTH - 6.0)?;
    ws.set_column_width(13, STANDARD_METRES_COLUMN_WIDTH - 3.0)?;
    ws.set_column_width(14, SPEED_COLUMN_WIDTH - 6.0)?;
    ws.set_column_width(15, SPEED_COLUMN_WIDTH - 6.0)?;
    ws.set_column_width(16, STANDARD_METRES_COLUMN_WIDTH - 2.0)?;
    ws.set_column_width(17, STANDARD_METRES_COLUMN_WIDTH - 2.0)?;
    ws.set_column_width(18, STANDARD_METRES_COLUMN_WIDTH - 2.0)?;
    ws.set_column_width(19, STANDARD_METRES_COLUMN_WIDTH - 2.0)?;

    ws.set_column_width(20, STANDARD_METRES_COLUMN_WIDTH - 2.0)?;
    ws.set_column_width(21, RUNNING_KILOMETRES_COLUMN_WIDTH - 2.0)?;
    ws.set_column_width(22, DATE_COLUMN_WIDTH)?;
    ws.set_column_width(23, LAT_LON_COLUMN_WIDTH)?;
    ws.set_column_width(24, LAT_LON_COLUMN_WIDTH)?;
    ws.set_column_width(25, LINKED_LAT_LON_COLUMN_WIDTH)?;

    ws.set_column_width(26, STANDARD_METRES_COLUMN_WIDTH - 2.0)?;
    ws.set_column_width(27, RUNNING_KILOMETRES_COLUMN_WIDTH - 2.0)?;
    ws.set_column_width(28, DATE_COLUMN_WIDTH)?;
    ws.set_column_width(29, LAT_LON_COLUMN_WIDTH)?;
    ws.set_column_width(30, LAT_LON_COLUMN_WIDTH)?;
    ws.set_column_width(31, LINKED_LAT_LON_COLUMN_WIDTH)?;

    ws.set_column_width(32, SPEED_COLUMN_WIDTH - 6.0)?;
    ws.set_column_width(33, LAT_LON_COLUMN_WIDTH)?;
    ws.set_column_width(34, LAT_LON_COLUMN_WIDTH)?;
    ws.set_column_width(35, LINKED_LAT_LON_COLUMN_WIDTH)?;

    ws.set_freeze_panes(2, 0)?;
    
    Ok(())
}

fn write_trackpoints(
    points: &[EnrichedTrackPoint],
    ws: &mut Worksheet,
) -> Result<(), Box<dyn Error>> {
    //write_major_header(ws, (0, 0), "Track Points")?;

    write_minor_header_blank(ws, (0, 0))?;
    write_minor_header(ws, (1, 0), "Index")?;

    write_minor_header_merged(ws, (0, 1), (0, 4), "Time")?;
    write_minor_header(ws, (1, 1), "UTC")?;
    write_minor_header(ws, (1, 2), "Local")?;
    write_minor_header(ws, (1, 3), "Delta")?;
    write_minor_header(ws, (1, 4), "Running")?;

    write_minor_header_merged(ws, (0, 5), (0, 8), "Location")?;
    write_minor_header(ws, (1, 5), "Lat")?;
    write_minor_header(ws, (1, 6), "Lon")?;
    write_minor_header(ws, (1, 7), "Map")?;
    write_minor_header(ws, (1, 8), "Description")?;

    write_minor_header_merged(ws, (0, 9), (0, 12), "Elevation (m)")?;
    write_minor_header(ws, (1, 9), "Height")?;
    write_minor_header(ws, (1, 10), "Delta")?;
    write_minor_header(ws, (1, 11), "Running Ascent")?;
    write_minor_header(ws, (1, 12), "Running Descent")?;

    write_minor_header_merged(ws, (0, 13), (0, 14), "Distance")?;
    write_minor_header(ws, (1, 13), "Delta (m)")?;
    write_minor_header(ws, (1, 14), "Running (km)")?;

    write_minor_header_blank(ws, (0, 15))?;
    write_minor_header(ws, (1, 15), "Speed (kmh)")?;

    // TODO: Use row banding?
    let mut row = 2;
    for p in points {
        ws.write_number(row, 0, p.index as u32)?;
        write_utc_date(ws, (row, 1), p.time)?;
        write_utc_date_as_local(ws, (row, 2), p.time)?;
        write_duration(ws, (row, 3), p.delta_time)?;
        write_duration(ws, (row, 4), p.running_delta_time)?;
        write_lat_lon(ws, (row, 5), (p.lat, p.lon), Hyperlink::Yes)?;
        write_location(ws, (row, 8), &p.location)?;
        write_metres(ws, (row, 9), p.ele)?;
        write_metres(ws, (row, 10), p.ele_delta_metres)?;
        write_metres(ws, (row, 11), p.running_ascent_metres)?;
        write_metres(ws, (row, 12), p.running_descent_metres)?;
        write_metres(ws, (row, 13), p.delta_metres)?;
        write_kilometres(ws, (row, 14), p.running_metres / 1000.0)?;
        write_speed(ws, (row, 15), p.speed_kmh)?;
        row += 1;
    }

    ws.set_column_width(1, DATE_COLUMN_WIDTH)?;
    ws.set_column_width(2, DATE_COLUMN_WIDTH)?;
    ws.set_column_width(3, DURATION_COLUMN_WIDTH)?;
    ws.set_column_width(4, DURATION_COLUMN_WIDTH)?;
    ws.set_column_width(5, LAT_LON_COLUMN_WIDTH)?;
    ws.set_column_width(6, LAT_LON_COLUMN_WIDTH)?;
    ws.set_column_width(7, LINKED_LAT_LON_COLUMN_WIDTH)?;
    ws.set_column_width(8, LOCATION_DESCRIPTION_COLUMN_WIDTH)?;
    ws.set_column_width(9, STANDARD_METRES_COLUMN_WIDTH)?;
    ws.set_column_width(10, STANDARD_METRES_COLUMN_WIDTH)?;
    ws.set_column_width(11, RUNNING_KILOMETRES_COLUMN_WIDTH)?;
    ws.set_column_width(12, RUNNING_KILOMETRES_COLUMN_WIDTH)?;
    ws.set_column_width(13, STANDARD_METRES_COLUMN_WIDTH)?;
    ws.set_column_width(14, RUNNING_KILOMETRES_COLUMN_WIDTH)?;
    ws.set_column_width(15, SPEED_COLUMN_WIDTH)?;

    ws.autofilter(1, 0, row - 1, 15)?;
    ws.set_freeze_panes(2, 0)?;

    Ok(())
}

// Utility functions.
fn write_major_header(
    ws: &mut Worksheet,
    rc: (u32, u16),
    heading: &str,
) -> Result<(), Box<dyn Error>> {
    static MAJOR_HDR_FORMAT: LazyLock<Format> = LazyLock::new(|| {
        Format::new()
            .set_bold()
            .set_italic()
            .set_background_color(Color::Blue)
            .set_pattern(FormatPattern::Solid)
            .set_font_color(Color::White)
            .set_border(FormatBorder::Thin)
            .set_border_color(Color::Black)
            .set_font_size(22)
    });

    ws.merge_range(rc.0, rc.1, rc.0, rc.1 + 5, heading, &MAJOR_HDR_FORMAT)?;

    Ok(())
}

/// Writes formatted minor header text to a single cell.
fn write_minor_header(
    ws: &mut Worksheet,
    rc: (u32, u16),
    heading: &str,
) -> Result<(), Box<dyn Error>> {
    ws.write_string_with_format(rc.0, rc.1, heading, &MINOR_HDR_FORMAT)?;
    Ok(())
}

/// Writes a blank minor header cell.
fn write_minor_header_blank(ws: &mut Worksheet, rc: (u32, u16)) -> Result<(), Box<dyn Error>> {
    ws.write_blank(rc.0, rc.1, &MINOR_HDR_FORMAT)?;
    Ok(())
}

/// Writes formatted minor header text to a range of merged cells.
fn write_minor_header_merged(
    ws: &mut Worksheet,
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
        &MINOR_HDR_FORMAT,
    )?;
    Ok(())
}

static MINOR_HDR_FORMAT: LazyLock<Format> = LazyLock::new(|| {
    Format::new()
        .set_bold()
        .set_background_color(Color::Black)
        .set_pattern(FormatPattern::Solid)
        .set_font_color(Color::White)
        .set_border(FormatBorder::Thin)
        .set_border_color(Color::Gray)
        .set_align(FormatAlign::Center)
});

/// Formats 'utc_date' into a string like "2024-09-01T05:10:44Z".
/// This is the format that GPX files contain.
fn write_utc_date(
    ws: &mut Worksheet,
    rc: (u32, u16),
    utc_date: OffsetDateTime,
) -> Result<(), Box<dyn Error>> {
    assert!(utc_date.offset().is_utc());
    let excel_date = date_to_excel_date(utc_date)?;
    ws.write_with_format(rc.0, rc.1, &excel_date, &UTC_DATE_FORMAT)?;
    Ok(())
}

/// Converts 'utc_date' to a local date and then formats it into
/// a string like "2024-09-01 05:10:44".
fn write_utc_date_as_local(
    ws: &mut Worksheet,
    rc: (u32, u16),
    utc_date: OffsetDateTime,
) -> Result<(), Box<dyn Error>> {
    assert!(utc_date.offset().is_utc());
    let excel_date = date_to_excel_date(to_local_date(utc_date))?;
    ws.write_with_format(rc.0, rc.1, &excel_date, &LOCAL_DATE_FORMAT)?;
    Ok(())
}

static UTC_DATE_FORMAT: LazyLock<Format> =
    LazyLock::new(|| Format::new().set_num_format("yyyy-mm-ddThh:mm:ssZ"));

static LOCAL_DATE_FORMAT: LazyLock<Format> =
    LazyLock::new(|| Format::new().set_num_format("yyyy-mm-dd hh:mm:ss"));

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
    rc: (u32, u16),
    duration: Duration,
) -> Result<(), Box<dyn Error>> {
    static DURATION_FORMAT: LazyLock<Format> =
        LazyLock::new(|| Format::new().set_num_format("hh:mm:ss"));

    let excel_duration = duration_to_excel_date(duration)?;
    ws.write_with_format(rc.0, rc.1, excel_duration, &DURATION_FORMAT)?;

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

enum Hyperlink {
    Yes,
    No,
}

/// Writes a lat-lon pair with the lat in the first cell as specified
/// by 'rc' and the lon in the next column. If 'hyperlink' is yes then
/// the 'lat' is written as a hyperlink to Google Maps.
fn write_lat_lon(
    ws: &mut Worksheet,
    rc: (u32, u16),
    lat_lon: (f64, f64),
    hyperlink: Hyperlink,
) -> Result<(), Box<dyn Error>> {
    static LAT_LON_FORMAT: LazyLock<Format> =
        LazyLock::new(|| Format::new().set_num_format("#.000000"));

    let link = format!(
        "https://www.google.com/maps/search/?api=1&query={:.6},{:.6}",
        lat_lon.0, lat_lon.1
    );

    let text = format!("{:.6},{:.6}", lat_lon.0, lat_lon.1);

    ws.write_number_with_format(rc.0, rc.1, lat_lon.0, &LAT_LON_FORMAT)?;
    ws.write_number_with_format(rc.0, rc.1 + 1, lat_lon.1, &LAT_LON_FORMAT)?;

    match hyperlink {
        Hyperlink::Yes => {
            ws.write_url_with_text(rc.0, rc.1 + 2, &*link, &text)?;
        }
        Hyperlink::No => {}
    };

    Ok(())
}

/// Writes a lat-lon pair into a single cell.
/// If 'hyperlink' is yes then a hyperlink to Google Maps is written as
/// well as the text. This function saves 2 columns of space compared
/// to 'write_lat_lon'.
fn write_lat_lon_single_cell(
    ws: &mut Worksheet,
    rc: (u32, u16),
    lat_lon: (f64, f64),
    hyperlink: Hyperlink,
) -> Result<(), Box<dyn Error>> {
    let link = format!(
        "https://www.google.com/maps/search/?api=1&query={},{}",
        lat_lon.0, lat_lon.1
    );

    let text = format!("{:.6},{:.6}", lat_lon.0, lat_lon.1);

    match hyperlink {
        Hyperlink::Yes => {
            ws.write_url_with_text(rc.0, rc.1, &*link, text)?;
        }
        Hyperlink::No => { ws.write_string(rc.0, rc.1, &text)?;
        }
    };

    Ok(())
}

fn write_location(
    ws: &mut Worksheet,
    rc: (u32, u16),
    location: &Option<String>,
) -> Result<(), Box<dyn Error>> {
    if let Some(location) = location {
        if !location.is_empty() {
            ws.write_string(rc.0, rc.1, location)?;
        }
    }

    Ok(())
}

fn write_metres(ws: &mut Worksheet, rc: (u32, u16), metres: f64) -> Result<(), Box<dyn Error>> {
    static METRES_FORMAT: LazyLock<Format> = LazyLock::new(|| Format::new().set_num_format("0.##"));

    ws.write_number_with_format(rc.0, rc.1, metres, &METRES_FORMAT)?;
    // TODO: Use conditional formatting to indicate negatives?
    Ok(())
}

fn write_kilometres(
    ws: &mut Worksheet,
    rc: (u32, u16),
    kilometres: f64,
) -> Result<(), Box<dyn Error>> {
    static KILOMETRES_FORMAT: LazyLock<Format> =
        LazyLock::new(|| Format::new().set_num_format("0.000"));

    ws.write_number_with_format(rc.0, rc.1, kilometres, &KILOMETRES_FORMAT)?;
    Ok(())
}

fn write_speed(ws: &mut Worksheet, rc: (u32, u16), speed: f64) -> Result<(), Box<dyn Error>> {
    static SPEED_FORMAT: LazyLock<Format> = LazyLock::new(|| Format::new().set_num_format("0.##"));

    ws.write_number_with_format(rc.0, rc.1, speed, &SPEED_FORMAT)?;
    Ok(())
}
