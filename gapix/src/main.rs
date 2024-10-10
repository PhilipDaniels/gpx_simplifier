use args::parse_args;
use clap::builder::styling::AnsiColor;
use env_logger::Builder;
use gapix_core::{
    excel::{create_summary_xlsx, write_summary_file},
    gpx_reader::read_gpx_file,
    model::{EnrichedGpx, Gpx},
    simplification::{metres_to_epsilon, reduce_trackpoints_by_rdp, write_simplified_gpx_file},
    stage::{detect_stages, enrich_trackpoints, StageDetectionParameters},
};
use log::info;
use logging_timer::time;
use std::{
    fs::read_dir,
    io::Write,
    path::{Path, PathBuf},
};

mod args;

pub const PROGRAM_NAME: &str = env!("CARGO_PKG_NAME");
pub const AUTHOR: &str = env!("CARGO_PKG_AUTHORS");

#[time]
fn main() {
    configure_logging();
    info!("Starting {PROGRAM_NAME}");

    let args = parse_args();

    // If we are running in "join mode" then we need to load all the
    // input files into RAM.
    if args.join {
        let exe_dir = get_exe_dir();
        let input_files = get_list_of_input_files(&exe_dir);
        if input_files.is_empty() {
            println!("No .gpx files found");
            return;
        }
    
        // Read all files into RAM.
        let mut gpxs: Vec<_> = input_files
            .iter()
            .map(|f| read_gpx_file(f).unwrap())
            .collect();

        // Join if necessary. Keep as a vec (of one element) so that
        // following loop can be used whether we join or not.
        if args.join {
            gpxs = vec![join_input_files(gpxs)];
        }

        return;
    }




    // // Within each file, merge multiple tracks and segments into a single
    // // track-segment. (join_input_files also does that)
    // gpxs = gpxs
    //     .into_iter()
    //     .map(|gpx| gpx.into_single_track())
    //     .collect();


    // for gpx in gpxs.into_iter() {
    //     let summary_filename = make_summary_filename(&gpx.filename);
    //     let simplified_filename = make_simplified_filename(&gpx.filename);

    //     if summary_filename.exists() && simplified_filename.exists() {
    //         continue;
    //     }

    //     // Always enrich the TrackPoints. Keeps the flow simple and though
    //     // it is one of the most expensive operations, it's still quick enough -
    //     // yay Rust!
    //     let mut gpx = EnrichedGpx::from(gpx);
    //     enrich_trackpoints(&mut gpx);

    //     // If we are detecting stops (really Stages now), then do that on
    //     // the original file, for more precision. Though whether it matters
    //     // much in practice is debatable - it only really makes a difference
    //     // if your 'metres' input to RDP is largish.
    //     if args.detect_stages {
    //         let params = StageDetectionParameters {
    //             stopped_speed_kmh: args.stopped_speed,
    //             min_metres_to_resume: args.stop_resumption_distance,
    //             min_duration_seconds: args.min_stop_time * 60.0,
    //         };

    //         let stages = detect_stages(&gpx, params);
    //         let workbook =
    //             create_summary_xlsx(args.trackpoint_hyperlinks(), &gpx, &stages).unwrap();
    //         write_summary_file(&summary_filename, workbook).unwrap();
    //     }

    //     // Always do simplification last because it mutates the track,
    //     // reducing its accuracy.
    //     if !simplified_filename.exists() {
    //         if let Some(metres) = args.metres {
    //             let epsilon = metres_to_epsilon(metres);

    //             let start_count = gpx.points.len();
    //             reduce_trackpoints_by_rdp(&mut gpx.points, epsilon);
    //             println!(
    //                 "Using Ramer-Douglas-Peucker with a precision of {metres}m (epsilon={epsilon}) reduced the trackpoint count from {start_count} to {} for {:?}",
    //                 gpx.points.len(),
    //                 gpx.filename
    //             );

    //             write_simplified_gpx_file(&simplified_filename, &gpx).unwrap();
    //         }
    //     }
    // }
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

fn join_input_files(mut input_files: Vec<Gpx>) -> Gpx {
    for gpx in &input_files {
        assert!(gpx.is_single_track());
    }

    // We can't simply re-use the first track/segment due to
    // multiple mut borrows. So create a new vec of points.
    let required_capacity: usize = input_files.iter().map(|f| f.num_points()).sum();
    let mut points = Vec::with_capacity(required_capacity);

    for f in &mut input_files {
        println!("Joining {:?}", f.filename);
        points.append(&mut f.tracks[0].segments[0].points);
    }

    // Sort all the points by ascending time in case
    // we got the files in a wacky order.
    points.sort_by_key(|p| p.time);

    println!("Joined {} files", input_files.len());

    input_files.swap_remove(0)
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
                        files.push(entry.path());
                    }
                }
            }
        }
    }

    files.sort_unstable();

    for f in &files {
        println!("Found GPX input file {:?}", f);
    }

    files
}

fn get_exe_dir() -> PathBuf {
    let mut exe_path = std::env::current_exe().unwrap();
    exe_path.pop();
    exe_path
}

fn configure_logging() {
    let mut builder = Builder::from_default_env();

    builder.format(|buf, record| {
        let level_style = buf.default_level_style(record.level());
        let level_style = match record.level() {
            log::Level::Error => level_style.fg_color(Some(AnsiColor::Red.into())),
            log::Level::Warn => level_style.fg_color(Some(AnsiColor::Yellow.into())),
            log::Level::Info => level_style.fg_color(Some(AnsiColor::Green.into())),
            log::Level::Debug => level_style.fg_color(Some(AnsiColor::Blue.into())),
            log::Level::Trace => level_style.fg_color(Some(AnsiColor::Magenta.into())),
        };

        let line_number_style = buf.default_level_style(record.level())
            .fg_color(Some(AnsiColor::Cyan.into()));

        match (record.file(), record.line()) {
            (Some(file), Some(line)) => writeln!(
                buf,
                "[{} {level_style}{}{level_style:#} {}/{line_number_style}{}{line_number_style:#}] {}",
                buf.timestamp(),
                record.level(),
                file,
                line,
                record.args()
            ),
            (Some(file), None) => writeln!(
                buf,
                "[{} {level_style}{}{level_style:#} {}] {}",
                buf.timestamp(),
                record.level(),
                file,
                record.args()
            ),
            (None, Some(_line)) => writeln!(
                buf,
                "[{} {level_style}{}{level_style:#}] {}",
                buf.timestamp(),
                record.level(),
                record.args()
            ),
            (None, None) => writeln!(
                buf,
                "[{} {level_style}{}{level_style:#}] {}",
                buf.timestamp(),
                record.level(),
                record.args()
            ),
        }
    });

    builder.init();
}
