use std::{
    collections::HashSet,
    error::Error,
    fs::File,
    io::{BufWriter, Write},
    path::Path,
};

use geo::{coord, LineString, SimplifyIdx};
use logging_timer::time;

use crate::{
    formatting::format_utc_date,
    model::{Declaration, EnrichedGpx, EnrichedTrackPoint, GpxInfo, Metadata},
};

/// We take input from the user in "metres of accuracy".
/// The 'geo' implementation of RDP requires an epsilon
/// which is relative to the coordinate scale in use.
/// Since we are using lat-lon, we need to convert metres
/// using the following relation: 1 degree of latitude = 111,111 metres
pub fn metres_to_epsilon(metres: u16) -> f64 {
    metres as f64 / 111111.0
}

/// Feed the points into the GEO crate so we can use its implementation
/// of https://en.wikipedia.org/wiki/Ramer%E2%80%93Douglas%E2%80%93Peucker_algorithm
///
/// These measurements are based on a 200km track from a Garmin Edge 1040,
/// which records 1 trackpoint every second. The original file is 11.5Mb, that
/// includes a lot of extension data such as heartrate which this program also
/// strips out. The percentages shown below are based solely on point counts.
///
/// The Audax UK DIY upload form allows a max file size of 1.25Mb.
///
/// Input Points    Metres  Output Points       Quality
/// 31358           1       4374 (13%, 563Kb)   Near-perfect map to the road
/// 31358           5       1484 (4.7%, 192Kb)  Very close map to the road, mainly stays within the road lines
/// 31358           10      978 (3.1%, 127Kb)   OK - good enough for submission
/// 31358           20      636 (2.0%, 83Kb)    Ok - within a few metres of the road
/// 31358           50      387 (1.2%, 51Kb)    Poor - cuts off a lot of corners
/// 31358           100     236 (0.8%, 31Kb)    Very poor - significant corner truncation
#[time]
pub fn reduce_trackpoints_by_rdp(points: &mut Vec<EnrichedTrackPoint>, epsilon: f64) {
    let line_string: LineString<_> = points
        .iter()
        .map(|p| coord! { x: p.lon, y: p.lat })
        .collect();
    let indices_to_keep: HashSet<usize> = HashSet::from_iter(line_string.simplify_idx(&epsilon));

    let mut n = 0;
    points.retain(|_| {
        let keep = indices_to_keep.contains(&n);
        n += 1;
        keep
    });
}

#[time]
pub fn write_simplified_gpx_file(
    output_file: &Path,
    gpx: &EnrichedGpx,
) -> Result<(), Box<dyn Error>> {
    print!("Writing file {:?}", &output_file);
    let mut w = BufWriter::new(File::create(output_file)?);

    write_declaration_tag(&mut w, &gpx.declaration)?;
    write_gpx_tag_open(&mut w, &gpx.info)?;
    write_metadata_tag(&mut w, &gpx.metadata)?;
    write_track(&mut w, &gpx.track_name, &gpx.track_type, &gpx.points)?;
    write_gpx_tag_close(&mut w)?;

    w.flush().unwrap();
    let metadata = std::fs::metadata(output_file)?;
    println!(", {} Kb", metadata.len() / 1024);

    Ok(())
}

fn write_declaration_tag<W: Write>(
    w: &mut W,
    declaration: &Declaration,
) -> Result<(), Box<dyn Error>> {
    write!(w, "<?xml version=\"{}\"", declaration.version)?;
    if let Some(encoding) = &declaration.encoding {
        write!(w, " encoding=\"{}\"", encoding)?;
    }
    if let Some(standalone) = &declaration.standalone {
        write!(w, " standalone=\"{}\"", standalone)?;
    }
    writeln!(w, "?>")?;
    Ok(())
}

fn write_gpx_tag_open<W: Write>(w: &mut W, info: &GpxInfo) -> Result<(), Box<dyn Error>> {
    writeln!(
        w,
        "<gpx creator=\"{}\" version=\"{}\"",
        info.creator, info.version
    )?;
    for (key, value) in &info.attributes {
        writeln!(w, "  {}=\"{}\"", key, value)?;
    }
    writeln!(w, ">")?;
    Ok(())
}

fn write_gpx_tag_close<W: Write>(w: &mut W) -> Result<(), Box<dyn Error>> {
    writeln!(w, "</gpx>")?;
    Ok(())
}

fn write_metadata_tag<W: Write>(w: &mut W, metadata: &Metadata) -> Result<(), Box<dyn Error>> {
    writeln!(w, "  <metadata>")?;
    writeln!(w, "    <link href=\"{}\"", metadata.link.href)?;
    if let Some(text) = &metadata.link.text {
        writeln!(w, "      <text>{}</text>", text)?;
    }
    if let Some(r#type) = &metadata.link.r#type {
        writeln!(w, "      <type>{}</type>", r#type)?;
    }
    writeln!(w, "    </link>")?;
    if let Some(time) = &metadata.time {
        writeln!(w, "    <time>{}</time>", format_utc_date(time))?;
    }
    writeln!(w, "  </metadata>")?;
    Ok(())
}

fn write_track<W: Write>(
    w: &mut W,
    track_name: &Option<String>,
    track_type: &Option<String>,
    points: &[EnrichedTrackPoint],
) -> Result<(), Box<dyn Error>> {
    writeln!(w, "    <trk>")?;
    if let Some(track_name) = track_name {
        writeln!(w, "      <name>{}</name>", track_name)?;
    }
    if let Some(track_type) = track_type {
        writeln!(w, "      <type>{}</type>", track_type)?;
    }

    writeln!(w, "      <trkseg>")?;
    for p in points {
        write_trackpoint(w, &p)?;
    }
    writeln!(w, "      </trkseg>")?;

    writeln!(w, "    </trk>")?;
    Ok(())
}

fn write_trackpoint<W: Write>(w: &mut W, point: &EnrichedTrackPoint) -> Result<(), Box<dyn Error>> {
    writeln!(
        w,
        "      <trkpt lat=\"{:.6}\" lon=\"{:.6}\">",
        point.lat, point.lon
    )?;

    writeln!(w, "      </trkpt>")?;

    Ok(())
}
