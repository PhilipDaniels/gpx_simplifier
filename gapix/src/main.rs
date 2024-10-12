use anyhow::{Context, Ok, Result};
use args::{get_required_outputs, parse_args, Args, RequiredOutputFiles};
use clap::builder::styling::AnsiColor;
use env_logger::Builder;
use gapix_core::{
    excel::{create_summary_xlsx, write_summary_to_file, Hyperlink},
    gpx_reader::read_gpx_from_file,
    gpx_writer::write_gpx_to_file,
    model::Gpx,
    simplification::{metres_to_epsilon, reduce_trackpoints_by_rdp},
    stage::{detect_stages, StageDetectionParameters},
};
use join::join_input_files;
use log::{debug, info, warn};
use logging_timer::time;
use std::io::Write;

mod args;
mod join;

pub const PROGRAM_NAME: &str = env!("CARGO_PKG_NAME");
pub const AUTHOR: &str = env!("CARGO_PKG_AUTHORS");

#[time]
fn main() -> Result<()> {
    configure_logging();
    info!("Starting {PROGRAM_NAME}");

    let args = parse_args();
    debug!("{:?}", &args);
    if args.force {
        info!("'--force' specified, all existing output files will be overwritten");
    }

    // If we are running in "join mode" then we need to load all the
    // input files into RAM and merge them into a single file.
    let input_files = args.files();
    if input_files.is_empty() {
        warn!("No .gpx files specified, exiting");
        return Ok(());
    }

    // In join mode we join all the input files into a single file
    // and then process it. There is nothing to be done after that.
    if args.join {
        let rof = get_required_outputs(&args, &input_files[0]);
        debug!("In join mode: {:?}", &rof);

        if let Some(joined_filename) = &rof.joined_file {
            let mut gpx = join_input_files(&input_files)?;
            gpx.filename = joined_filename.clone();
            write_gpx_to_file(&joined_filename, &gpx)?;
            process_gpx(gpx, &args, rof)?;
        }

        return Ok(());
    }

    // The other modes break down to 'process each file separately'.
    debug!("In per-file mode");
    for f in &input_files {
        let rof = get_required_outputs(&args, &f);
        let gpx = read_gpx_from_file(f)?;
        let gpx = gpx.into_single_track();
        process_gpx(gpx, &args, rof)?;
    }

    Ok(())
}

fn process_gpx(mut gpx: Gpx, args: &Args, rof: RequiredOutputFiles) -> Result<()> {
    assert!(gpx.is_single_track());

    if let Some(analysis_file) = &rof.analysis_file {
        assert!(args.analyse);

        // Analysis requires us to enrich the GPX data with some
        // derived data such as speed and running distance.
        let enriched_gpx = gpx.to_enriched_gpx()?;
        let params = StageDetectionParameters {
            stopped_speed_kmh: args.control_speed,
            min_metres_to_resume: args.control_resumption_distance,
            min_duration_seconds: args.min_control_time * 60.0,
        };

        let stages = detect_stages(&enriched_gpx, params);

        let tp_hyper = if args.trackpoint_hyperlinks {
            Hyperlink::Yes
        } else {
            Hyperlink::No
        };

        let workbook = create_summary_xlsx(tp_hyper, &enriched_gpx, &stages).unwrap();
        write_summary_to_file(analysis_file, workbook).unwrap();
    }

    if let Some(simplified_file) = &rof.simplified_file {
        let metres = args
            .metres
            .context("The 'metres' argument should be specified if we are simplifying")?;
        let epsilon = metres_to_epsilon(metres);
        let start_count = gpx.num_points();
        reduce_trackpoints_by_rdp(&mut gpx.tracks[0].segments[0].points, epsilon);
        let end_count = gpx.num_points();

        info!(
            "Using Ramer-Douglas-Peucker with a precision of {metres}m (epsilon={epsilon}) reduced the trackpoint count from {start_count} to {end_count} for {:?}",
            gpx.filename
            );

        write_gpx_to_file(&simplified_file, &gpx)?;
    }

    Ok(())
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
