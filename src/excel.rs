use std::{error::Error, path::Path, sync::LazyLock};

use rust_xlsxwriter::{
    Color, ExcelDateTime, Format, FormatAlign, FormatBorder, FormatPattern, Workbook, Worksheet,
};
use time::{Duration, OffsetDateTime};

use crate::{
    formatting::to_local_date,
    model::{EnrichedGpx, EnrichedTrackPoint},
    section::SectionList,
};

pub fn write_summary_file<'gpx>(
    summary_filename: &Path,
    gpx: &EnrichedGpx,
    sections: &SectionList<'gpx>,
) -> Result<(), Box<dyn Error>> {
    print!("Writing file {:?}", &summary_filename);

    let mut workbook = Workbook::new();

    // This will appear as the first sheet in the workbook.
    let summary_ws = workbook.add_worksheet();
    summary_ws.set_name("Summary")?;

    // This will appear as the second sheet in the workbook.
    let tp_ws = workbook.add_worksheet();
    tp_ws.set_name("Track Points")?;
    write_trackpoints(&gpx.points, tp_ws)?;

    workbook.save(summary_filename).unwrap();
    let metadata = std::fs::metadata(summary_filename).unwrap();
    println!(", {} Kb", metadata.len() / 1024);
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

    write_minor_header_merged(ws, (0, 5), (0, 7), "Location")?;
    write_minor_header(ws, (1, 5), "Lat")?;
    write_minor_header(ws, (1, 6), "Lon")?;
    write_minor_header(ws, (1, 7), "Description")?;

    write_minor_header_merged(ws, (0, 8), (0, 11), "Elevation (m)")?;
    write_minor_header(ws, (1, 8), "Height")?;
    write_minor_header(ws, (1, 9), "Delta")?;
    write_minor_header(ws, (1, 10), "Running Ascent")?;
    write_minor_header(ws, (1, 11), "Running Descent")?;

    write_minor_header_merged(ws, (0, 12), (0, 13), "Distance")?;
    write_minor_header(ws, (1, 12), "Delta (m)")?;
    write_minor_header(ws, (1, 13), "Running (km)")?;

    write_minor_header_blank(ws, (0, 14))?;
    write_minor_header(ws, (1, 14), "Speed (kmh)")?;

    // TODO: Use row banding?
    let mut row = 2;
    for p in points {
        ws.write_number(row, 0, p.index as u32)?;
        write_utc_date(ws, (row, 1), p.time)?;
        write_utc_date_as_local(ws, (row, 2), p.time)?;
        write_duration(ws, (row, 3), p.delta_time)?;
        write_duration(ws, (row, 4), p.running_delta_time)?;
        write_lat_lon(ws, (row, 5), (p.lat, p.lon), Hyperlink::No)?;
        write_location(ws, (row, 7), &p.location)?;
        write_metres(ws, (row, 8), p.ele)?;
        write_metres(ws, (row, 9), p.ele_delta_metres)?;
        write_metres(ws, (row, 10), p.running_ascent_metres)?;
        write_metres(ws, (row, 11), p.running_descent_metres)?;
        write_metres(ws, (row, 12), p.delta_metres)?;
        write_kilometres(ws, (row, 13), p.running_metres / 1000.0)?;
        write_speed(ws, (row, 14), p.speed_kmh)?;
        row += 1;
    }

    const DATE_COLUMN_WIDTH: f64 = 18.0;
    const DURATION_COLUMN_WIDTH: f64 = 12.0;
    const LAT_LON_COLUMN_WIDTH: f64 = 10.0;
    const LOCATION_DESCRIPTION_COLUMN_WIDTH: f64 = 18.0;
    const STANDARD_METRES_COLUMN_WIDTH: f64 = 11.0;
    const RUNNING_KILOMETRES_COLUMN_WIDTH: f64 = 15.0;
    const SPEED_COLUMN_WIDTH: f64 = 14.0;
    ws.set_column_width(1, DATE_COLUMN_WIDTH)?;
    ws.set_column_width(2, DATE_COLUMN_WIDTH)?;
    ws.set_column_width(3, DURATION_COLUMN_WIDTH)?;
    ws.set_column_width(4, DURATION_COLUMN_WIDTH)?;
    ws.set_column_width(5, LAT_LON_COLUMN_WIDTH)?;
    ws.set_column_width(6, LAT_LON_COLUMN_WIDTH)?;
    ws.set_column_width(7, LOCATION_DESCRIPTION_COLUMN_WIDTH)?;
    ws.set_column_width(8, STANDARD_METRES_COLUMN_WIDTH)?;
    ws.set_column_width(9, STANDARD_METRES_COLUMN_WIDTH)?;
    ws.set_column_width(10, RUNNING_KILOMETRES_COLUMN_WIDTH)?;
    ws.set_column_width(11, RUNNING_KILOMETRES_COLUMN_WIDTH)?;
    ws.set_column_width(12, STANDARD_METRES_COLUMN_WIDTH)?;
    ws.set_column_width(13, RUNNING_KILOMETRES_COLUMN_WIDTH)?;
    ws.set_column_width(14, SPEED_COLUMN_WIDTH)?;

    ws.autofilter(1, 0, row - 1, 14)?;
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
    No
}

/// Writes a lat-lon pair with the lat in the first cell as specified
/// by 'rc' and the lon in the next column. If 'hyperlink' is yes then
/// the 'lat' is written as a hyperlink to Google Maps.
fn write_lat_lon(
    ws: &mut Worksheet,
    rc: (u32, u16),
    lat_lon: (f64, f64),
    hyperlink: Hyperlink
) -> Result<(), Box<dyn Error>> {
    static LAT_LON_FORMAT: LazyLock<Format> =
        LazyLock::new(|| Format::new().set_num_format("#.000000"));

    //let link = format!("{}{}", lat_lon.0, lat_lon.1);

    match hyperlink {
        Hyperlink::Yes => todo!(),
        Hyperlink::No => ws.write_number_with_format(rc.0, rc.1, lat_lon.0, &LAT_LON_FORMAT)?
    };

    ws.write_number_with_format(rc.0, rc.1 + 1, lat_lon.1, &LAT_LON_FORMAT)?;
    Ok(())
}

fn write_location(
    ws: &mut Worksheet,
    rc: (u32, u16),
    location: &str
) -> Result<(), Box<dyn Error>> {
    if !location.is_empty() {
        ws.write_string(rc.0, rc.1, location)?;
    }
    Ok(())
}

fn write_metres(
    ws: &mut Worksheet,
    rc: (u32, u16),
    metres: f64,
) -> Result<(), Box<dyn Error>> {
    static METRES_FORMAT: LazyLock<Format> =
        LazyLock::new(|| Format::new().set_num_format("0.##"));

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

fn write_speed(
    ws: &mut Worksheet,
    rc: (u32, u16),
    speed: f64,
) -> Result<(), Box<dyn Error>> {
    static SPEED_FORMAT: LazyLock<Format> =
        LazyLock::new(|| Format::new().set_num_format("0.##"));

    ws.write_number_with_format(rc.0, rc.1, speed, &SPEED_FORMAT)?;
    Ok(())
}
