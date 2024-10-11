use args::{get_required_outputs, parse_args};
use clap::builder::styling::AnsiColor;
use env_logger::Builder;
use gapix_core::{gpx_reader::read_gpx_from_file, gpx_writer::write_gpx_file};
use join::join_input_files;
use log::{debug, error, info, warn};
use logging_timer::time;
use std::io::Write;

mod args;
mod join;

pub const PROGRAM_NAME: &str = env!("CARGO_PKG_NAME");
pub const AUTHOR: &str = env!("CARGO_PKG_AUTHORS");

#[time]
fn main() {
    configure_logging();
    info!("Starting {PROGRAM_NAME}");

    let args = parse_args();
    debug!("{:?}", &args);

    // Exclude any file that is not ending .gpx.
    // Deal with files that don't exist at load time.
    // Don't load files that have no work required to be done.
    // Can you join and simplify in one step?

    // If we are running in "join mode" then we need to load all the
    // input files into RAM and merge them into a single file.
    let input_files = args.files();
    if input_files.is_empty() {
        println!("No .gpx files specified, exiting");
        return;
    }

    if args.join {
        debug!("In join mode");
        let rof = get_required_outputs(&args, &input_files[0]);
        debug!("{:?}", &rof);

        if let Some(joined_filename) = rof.joined_file {
            match join_input_files(&input_files) {
                Ok(gpx) => {
                    write_gpx_file(joined_filename, &gpx).unwrap();
                    // process_gpx()
                }
                Err(e) => error!("Error: {}", e),
            }
        }
    } else {
        debug!("In per-file mode");
        for f in &input_files {
            let gpx = read_gpx_from_file(f).unwrap();
            // process gpx
        }
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
