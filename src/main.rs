use args::parse_args;
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

    // We can operate in 3 modes depending on the command line
    // arguments.
    // --metres=NN          - simplify each input file individually
    // --join               - join all the input files into a single file
    // --join --metres=NN   - join into a single file then simplify

    // Read all files into RAM.
    let gpxs: Vec<Gpx> = input_files.iter().map(|f| read_gpx_file(f)).collect();

    // Within each file, merge multiple tracks and segments into a single
    // track-segment.
    let mut gpxs: Vec<MergedGpx> =
        gpxs.iter().map(|f| f.merge_all_tracks()).collect();

    // Join if necessary. Keep as a vec (of one element) so that
    // following loop can be used whether we join or not.
    if args.join {
        gpxs = vec![join_input_files(gpxs)];
    }

    // Skip any files if the output already exists. It's wasteful to do this
    // after the load and parse and join, but it keeps the logic simpler.
    gpxs.retain(|gpx| {
        let output_filename = make_simplified_filename(&gpx.filename);
        if output_filename.exists() {
            println!("Skipping {:?} because the output file already exists", &gpx.filename);
            false
        } else {
            true
        }
    });

    // Simplify if necessary.
    if let Some(metres) = args.metres {
        let epsilon = metres_to_epsilon(metres);

        for merged_gpx in &mut gpxs {
            let start_count = merged_gpx.points.len();
            reduce_trackpoints_by_rdp(&mut merged_gpx.points, epsilon);
            println!(
                "Using Ramer-Douglas-Peucker with a precision of {metres}m (epsilon={epsilon}) reduced the trackpoint count from {start_count} to {} for {:?}",
                merged_gpx.points.len(),
                merged_gpx.filename
            );
        }
    }

    for merged_gpx in gpxs {
        let output_filename = make_simplified_filename(&merged_gpx.filename);
        write_output_file(&output_filename, &merged_gpx);
    }
}

fn make_simplified_filename(p: &Path) -> PathBuf {
    let mut p = p.to_owned();
    p.set_extension("simplified.gpx");
    p
}

/// TODO: This is awful, does a clone of the first element.
fn join_input_files(mut input_files: Vec<MergedGpx>) -> MergedGpx {
    let required_capacity: usize = input_files.iter().map(|f| f.points.len()).sum();
    let mut m = input_files[0].clone();
    m.points = Vec::with_capacity(required_capacity);
    
    for f in &mut input_files {
        println!("Joining {:?}", f.filename);
        m.points.append(&mut f.points);
    }

    println!("Joined {} files", input_files.len());

    m
}

/// We take input from the user in "metres of accuracy".
/// The 'geo' implementation of RDP requires an epsilon
/// which is relative to the coordinate scale in use.
/// Since we are using lat-lon, we need to convert metres
/// using the following relation: 1 degree of latitude = 111,111 metres
fn metres_to_epsilon(metres: u16) -> f32 {
    metres as f32 / 111111.0
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
    let line_string: LineString<f32> = points
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

/// The serde/quick-xml deserialization integration does a "good enough" job of parsing
/// the XML file. We also tag on the original filename as it's handy to track this
/// through the program for when we come to the point of writing output.
fn read_gpx_file(input_file: &Path) -> Gpx {
    let reader = Reader::from_file(input_file).expect("Could not create XML reader");
    let mut doc: Gpx = quick_xml::de::from_reader(reader.into_inner()).unwrap();
    doc.filename = input_file.to_owned();
    doc
}

fn write_output_file(output_file: &Path, gpx: &MergedGpx) {
    const HDR: &str = include_str!("header.txt");

    print!("Writing file {:?}", &output_file);

    let mut w = BufWriter::new(File::create(output_file).expect("Could not open output_file"));
    writeln!(w, "{}", HDR).unwrap();
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
    let metadata = std::fs::metadata(output_file).unwrap();
    println!(", {}Kb", metadata.len() / 1024);
}

/// Get a list of all files in the exe_dir that have the ".gpx" extension.
/// Be careful to exclude files that actually end in ".simplified.gpx" -
/// they are output files we already created! If we don't exclude them here,
/// we end up generating ".simplified.simplified.gpx", etc.
/// Remarks: the list of files is guaranteed to be sorted, this is
/// important for the joining algorithm (the first file is expected to
/// be the first part of the track, and so on).
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

    files.sort_unstable();

    files
}

fn get_exe_dir() -> PathBuf {
    let mut exe_path = std::env::current_exe().unwrap();
    exe_path.pop();
    exe_path
}
