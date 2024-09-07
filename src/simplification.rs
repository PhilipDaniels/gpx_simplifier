use std::{collections::HashSet, fs::File, io::{BufWriter, Write}, path::Path};

use geo::{coord, LineString, SimplifyIdx};

use crate::{formatting::format_utc_date, model::{EnrichedGpx, EnrichedTrackPoint}};

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

pub fn write_simplified_gpx_file(output_file: &Path, gpx: &EnrichedGpx) {
    const HDR: &str = include_str!("header.txt");
    print!("Writing file {:?}", &output_file);

    let mut w = BufWriter::new(File::create(output_file).expect("Could not open output_file"));
    writeln!(w, "{}", HDR).unwrap();
    writeln!(w, "  <metadata>").unwrap();
    writeln!(w, "    <time>{}</time>", format_utc_date(gpx.metadata_time)).unwrap();
    writeln!(w, "  </metadata>").unwrap();
    writeln!(w, "  <trk>").unwrap();
    writeln!(w, "    <name>{}</name>", gpx.track_name).unwrap();
    writeln!(w, "    <type>{}</type>", gpx.track_type).unwrap();
    writeln!(w, "    <trkseg>").unwrap();
    for tp in &gpx.points {
        writeln!(
            w,
            "      <trkpt lat=\"{:.6}\" lon=\"{:.6}\">",
            tp.lat, tp.lon
        )
        .unwrap();
        writeln!(w, "        <ele>{:.1}</ele>", tp.ele).unwrap();
        writeln!(w, "        <time>{}</time>", format_utc_date(tp.time)).unwrap();
        writeln!(w, "      </trkpt>").unwrap();
    }
    writeln!(w, "    </trkseg>").unwrap();
    writeln!(w, "  </trk>").unwrap();
    writeln!(w, "</gpx>").unwrap();

    w.flush().unwrap();
    let metadata = std::fs::metadata(output_file).unwrap();
    println!(", {}Kb", metadata.len() / 1024);
}
