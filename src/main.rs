use args::{parse_args, Args};
use geo::{coord, LineString, SimplifyIdx};
use model::{Gpx, MergedGpx, TrackPoint};
use quick_xml::reader::Reader;
use std::collections::HashSet;
use std::io::Write;
use std::{
    fs::{read_dir, File},
    io::BufWriter,
    path::{Path, PathBuf},
};

mod args;
mod model;

fn main() {
    let args = parse_args();

    let exe_dir = get_exe_dir();
    let input_files = get_list_of_input_files(&exe_dir);
    if input_files.is_empty() {
        println!("No .gpx files found");
        return;
    }

    for f in input_files {
        let mut output_file = f.to_owned();
        output_file.set_extension("simplified.gpx");
        if output_file.exists() {
            println!(
                "Simplified file {:?} already exists, skipping...",
                &output_file
            );
            continue;
        }

        let gpx = read_gpx_file(&f);
        let mut gpx = gpx.to_merged_gpx();
        let start_count = gpx.points.len();

        match args {
            Args::Keep(keep) => {
                reduce_trackpoints_by_keep(&mut gpx.points, keep.into());
                println!(
                    "Keeping every {keep} trackpoints reduced the count from {start_count} to {}",
                    gpx.points.len()
                );
            }

            Args::RdpMetres(metres) => {
                let epsilon = metres_to_epsilon(metres);
                reduce_trackpoints_by_rdp(&mut gpx.points, epsilon);
                println!(
                    "Using Ramer-Douglas-Peucker with a precision of {metres}m (epsilon={epsilon}) reduced the trackpoint count from {start_count} to {}",
                    gpx.points.len()
                );
            }
        }

        write_output_file(&output_file, &gpx);
    }
}

/// Reduces the number of points in the track. The Garmin Edge 1040 writes
/// 1 point per second, which is ridiculous. Example: if keep_each is 3,
/// every 3rd point is kept, starting with the first.
/// Up to 10 seems fine.
/// The max size of an upload file is 1.25Mb - and that can be after zipping.
fn reduce_trackpoints_by_keep(points: &mut Vec<TrackPoint>, keep_each: i32) {
    let mut n = 0;
    points.retain(|_| {
        let keep = n % keep_each == 0;
        n += 1;
        keep
    })
}

/// We take input from the user in "metres of accuracy".
/// The 'geo' implementation of RDP requires an epsilon
/// which is relative to the coordinate scale in use.
/// Since we are using lat-lon, we need to convert metres
/// using the following relation: 1 degree of latitude = 111,111 metres
fn metres_to_epsilon(metres: f32) -> f32 {
    metres / 111111.0
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
fn reduce_trackpoints_by_rdp(points: &mut Vec<TrackPoint>, epsilon: f32) {
    let coords_iter = points.iter().map(|p| coord! { x: p.lon, y: p.lat });
    let line_string: LineString<f32> = coords_iter.collect();
    let indices_to_keep: HashSet<usize> = HashSet::from_iter(line_string.simplify_idx(&epsilon));

    let mut n = 0;
    points.retain(|_| {
        let keep = indices_to_keep.contains(&n);
        n += 1;
        keep
    });
}

/// The serde/quick-xml deserialization integration does a "good enough" job of parsing
/// the XML file.
fn read_gpx_file(input_file: &Path) -> Gpx {
    let reader = Reader::from_file(input_file).expect("Could not create XML reader");
    let doc: Gpx = quick_xml::de::from_reader(reader.into_inner()).unwrap();
    doc
}

fn write_output_file(output_file: &Path, gpx: &MergedGpx) {
    print!("Writing file {:?}", &output_file);

    // TODO: If Garmin ever changes this then what we need to do is read the GPX node in the way
    // we used to do, using the streaming interface, then write it to the output file.
    // But for now, let's wing it...
    let mut w = BufWriter::new(File::create(&output_file).expect("Could not open output_file"));
    writeln!(w, "<?xml version=\"1.0\" encoding=\"UTF-8\"?>").unwrap();
    writeln!(
        w,
        "<gpx creator=\"{}\" version=\"{}\"",
        gpx.creator, gpx.version
    )
    .unwrap();
    writeln!(w, "  xsi:schemaLocation=\"{}\"", gpx.xsi_schema_location).unwrap();
    writeln!(w, "  xmlns:ns3=\"{}\"", gpx.xmlns_ns3).unwrap();
    writeln!(w, "  xmlns=\"{}\"", gpx.xmlns).unwrap();
    writeln!(w, "  xmlns:xsi=\"{}\"", gpx.xmlns_xsi).unwrap();
    writeln!(w, "  xmlns:ns2=\"{}\">", gpx.xmlns_ns2).unwrap();
    writeln!(w, "  <metadata>").unwrap();
    writeln!(w, "    <time>{}</time>", gpx.metadata_time).unwrap();
    writeln!(w, "  </metadata>").unwrap();

    writeln!(w, "  <trk>").unwrap();
    writeln!(w, "    <name>{}</name>", gpx.track_name).unwrap();
    writeln!(w, "    <type>{}</type>", gpx.track_type).unwrap();
    writeln!(w, "    <trkseg>").unwrap();
    for tp in &gpx.points {
        writeln!(w, "      <trkpt lat=\"{}\" lon=\"{}\">", tp.lat, tp.lon).unwrap();
        writeln!(w, "        <ele>{}</ele>", tp.ele).unwrap();
        writeln!(w, "        <time>{}</time>", tp.time).unwrap();
        writeln!(w, "      </trkpt>").unwrap();
    }
    writeln!(w, "    </trkseg>").unwrap();
    writeln!(w, "  </trk>").unwrap();
    writeln!(w, "</gpx>").unwrap();

    w.flush().unwrap();
    let metadata = std::fs::metadata(&output_file).unwrap();
    println!(", {}Kb", metadata.len() / 1024);
}

// Get a list of all files in the exe_dir that have the ".gpx" extension.
// Be careful to exclude files that actually end in ".simplified.gpx" -
// they are output files we already created! If we don't exclude them here,
// we end up generating ".simplified.simplified.gpx", etc.
fn get_list_of_input_files(exe_dir: &PathBuf) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let Ok(entries) = read_dir(exe_dir) else {
        return files;
    };

    for entry in entries {
        let entry = entry.unwrap();
        let meta = entry.metadata().unwrap();
        if meta.is_file() {
            let s = &entry.file_name();
            let p = Path::new(s);
            if let Some(ext) = p.extension() {
                if ext.to_ascii_lowercase() == "gpx" {
                    let s = s.to_string_lossy().to_ascii_lowercase();
                    if !s.ends_with(".simplified.gpx") {
                        println!("Found GPX input file {:?}", entry.path());
                        files.push(entry.path());
                    }
                }
            }
        }
    }

    files
}

fn get_exe_dir() -> PathBuf {
    let mut exe_path = std::env::current_exe().unwrap();
    exe_path.pop();
    exe_path
}
