use std::{error::Error, path::Path, sync::LazyLock};

use rust_xlsxwriter::{
    Color, ExcelDateTime, Format, FormatAlign, FormatBorder, FormatPattern, Workbook, Worksheet,
};
use time::OffsetDateTime;

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
    write_major_header(ws, (0, 0), "Track Points")?;

    write_minor_header_blank(ws, (1, 0))?;
    write_minor_header(ws, (2, 0), "Index")?;

    write_minor_header_merged(ws, (1, 1), (1, 4), "Time")?;
    write_minor_header(ws, (2, 1), "UTC")?;
    write_minor_header(ws, (2, 2), "Local")?;
    write_minor_header(ws, (2, 3), "Delta")?;
    write_minor_header(ws, (2, 4), "Running")?;

    write_minor_header_merged(ws, (1, 5), (1, 7), "Location")?;
    write_minor_header(ws, (2, 5), "Lat")?;
    write_minor_header(ws, (2, 6), "Lon")?;
    write_minor_header(ws, (2, 7), "Description")?;

    write_minor_header_merged(ws, (1, 8), (1, 11), "Elevation (m)")?;
    write_minor_header(ws, (2, 8), "Height")?;
    write_minor_header(ws, (2, 9), "Delta")?;
    write_minor_header(ws, (2, 10), "Running Ascent")?;
    write_minor_header(ws, (2, 11), "Running Descent")?;

    write_minor_header_merged(ws, (1, 12), (1, 13), "Distance")?;
    write_minor_header(ws, (2, 12), "Delta (m)")?;
    write_minor_header(ws, (2, 13), "Running (km)")?;

    write_minor_header_blank(ws, (1, 14))?;
    write_minor_header(ws, (2, 14), "Speed (kmh)")?;

    let mut row = 3;
    for p in points {
        ws.write_number(row, 0, p.index as u32)?;
        write_utc_date(ws, (row, 1), p.time)?;
        write_utc_date_as_local(ws, (row, 2), p.time)?;
        row += 1;
    }

    const DATE_COLUMN_WIDTH: f64 = 18.0;
    ws.set_column_width(1, DATE_COLUMN_WIDTH)?;
    ws.set_column_width(2, DATE_COLUMN_WIDTH)?;

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
    let excel_date = to_excel_date(utc_date)?;
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
    let excel_date = to_excel_date(to_local_date(utc_date))?;
    ws.write_with_format(rc.0, rc.1, &excel_date, &LOCAL_DATE_FORMAT)?;
    Ok(())
}

static UTC_DATE_FORMAT: LazyLock<Format> =
    LazyLock::new(|| Format::new().set_num_format("yyyy-mm-ddThh:mm:ssZ"));

static LOCAL_DATE_FORMAT: LazyLock<Format> =
    LazyLock::new(|| Format::new().set_num_format("yyyy-mm-dd hh:mm:ss"));

fn to_excel_date(date: OffsetDateTime) -> Result<ExcelDateTime, Box<dyn Error>> {
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

    let excel_date = excel_date.and_hms(
        hour,
        minute,
        second
    )?;

    Ok(excel_date)
}