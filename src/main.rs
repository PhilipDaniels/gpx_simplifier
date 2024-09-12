use args::parse_args;
use excel::write_summary_file;
use model::{EnrichedGpx, Gpx, MergedGpx};
use quick_xml::reader::Reader;
use simplification::{metres_to_epsilon, reduce_trackpoints_by_rdp, write_simplified_gpx_file};
use stage::{detect_stages, enrich_trackpoints, StageDetectionParameters};
use std::{
    fs::read_dir,
    path::{Path, PathBuf},
};

mod args;
mod excel;
mod formatting;
mod model;
mod simplification;
mod stage;

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
    let mut gpxs: Vec<MergedGpx> = gpxs.iter().map(|f| f.merge_all_tracks()).collect();

    // Join if necessary. Keep as a vec (of one element) so that
    // following loop can be used whether we join or not.
    if args.join {
        gpxs = vec![join_input_files(gpxs)];
    }

    for gpx in gpxs.into_iter() {
        let summary_filename = make_summary_filename(&gpx.filename);
        let simplified_filename = make_simplified_filename(&gpx.filename);

        if summary_filename.exists() && simplified_filename.exists() {
            continue;
        }

        // Always enrich the TrackPoints. Keeps the flow simple and though
        // it is one of the most expensive operations, it's still quick enough -
        // yay Rust!
        let mut gpx = EnrichedGpx::from(gpx);
        enrich_trackpoints(&mut gpx);

        // // If we are detecting stops (really Stages now), then do that on
        // the original file, for more precision. Though whether it matters
        // much in practice is debatable - it only really makes a difference
        // if your 'metres' input to RDP is largish.
        if args.detect_stages {
            let params = StageDetectionParameters {
                stopped_speed_kmh: 0.01,
                resume_speed_kmh: 10.0,
                min_duration_seconds: 120.0, // Info controls! Do we care? TODO: This has a large effect. Maybe a bug.
            };

            let stages = detect_stages(&gpx, params);
            // TODO: We can't write files in parallel.
            write_summary_file(&summary_filename, args.trackpoint_options(), &gpx, &stages)
                .unwrap();
        }

        // Always do simplification last because it mutates the track,
        // reducing its accuracy.
        if !simplified_filename.exists() {
            if let Some(metres) = args.metres {
                let epsilon = metres_to_epsilon(metres);

                let start_count = gpx.points.len();
                reduce_trackpoints_by_rdp(&mut gpx.points, epsilon);
                println!(
                    "Using Ramer-Douglas-Peucker with a precision of {metres}m (epsilon={epsilon}) reduced the trackpoint count from {start_count} to {} for {:?}",
                    gpx.points.len(),
                    gpx.filename
                );

                // TODO: We can't write files in parallel.
                write_simplified_gpx_file(&simplified_filename, &gpx);
            }
        }
    }
}

fn make_simplified_filename(p: &Path) -> PathBuf {
    let mut p = p.to_owned();
    p.set_extension("simplified.gpx");
    p
}

fn make_summary_filename(p: &Path) -> PathBuf {
    let mut p = p.to_owned();
    p.set_extension("summary.xlsx");
    p
}

fn join_input_files(mut input_files: Vec<MergedGpx>) -> MergedGpx {
    let required_capacity: usize = input_files.iter().map(|f| f.points.len()).sum();
    let mut m = input_files[0].clone();
    m.points = Vec::with_capacity(required_capacity);

    for f in &mut input_files {
        println!("Joining {:?}", f.filename);
        m.points.append(&mut f.points);
    }

    // If we got the files in a wacky order, ensure we
    // sort all the points by ascending time.
    m.points.sort_by_key(|p| p.time);

    println!("Joined {} files", input_files.len());

    m
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
