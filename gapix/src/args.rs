use std::path::{Path, PathBuf};

use clap::{arg, builder::ArgPredicate, command, value_parser, Parser};
use log::{info, warn};

/// Returns the parsed command line options. Uses the 'wild' crate to do glob
/// expansion on Windows, so that Windows and Linux behave identically.
///
/// Note that if you use a pattern such as '*.gpx' and there are no actually
/// matching gpx files, then you get 1 FILE with the name '*.gpx', i.e. the
/// unexpanded pattern. This is the same whether you use wild::args() or
/// std::env::args(), so it appears to be something we have to detect and work
/// around.
pub fn parse_args() -> Args {
    Args::parse_from(wild::args())
}

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[arg(
        short,
        long,
        default_value = "false",
        help = "Overwrite output files even if they already exist"
    )]
    pub force: bool,

    #[arg(
        short,
        long,
        default_value = "false",
        help = "Join the input GPX files into a single file with 1 track before \
                applying further processing and produce a '.joined.gpx' file"
    )]
    pub join: bool,

    #[arg(
        short,
        long,
        help = "Reduce the number of track points by using Ramer-Douglas-Peucker \
                with METRES accuracy and produce a '.simplified.gpx' file",
        value_parser = value_parser!(u16).range(1..=1000)
    )]
    pub metres: Option<u16>,

    #[arg(
        short,
        long,
        default_value_ifs([
            ("control_speed", ArgPredicate::IsPresent, "true"),
            ("min_control_time", ArgPredicate::IsPresent, "true"),
            ("control_resumption_distance", ArgPredicate::IsPresent, "true"),
            ("trackpoint_hyperlinks", ArgPredicate::IsPresent, "true"),
            ]),
        help = "Analyse the GPX and produce a summary spreadsheet in .xlsx format",
    )]
    pub analyse: bool,

    #[arg(
        long,
        default_value = "0.15",
        help = "The speed, in km/h, which you must drop below to be considered 'Controlling'. Implies 'analyse'."
    )]
    pub control_speed: f64,

    #[arg(
        long,
        default_value = "5.0",
        help = "Minimum length of a Control stop, in minutes, for it to be detected. Implies 'analyse'."
    )]
    pub min_control_time: f64,

    #[arg(
        long,
        default_value = "100.0",
        help = "When at a control, the distance (in metres) that you must move 'as the crow flies' from \
                your stop point before you are considered to be moving again. Implies 'analyse'."
    )]
    pub control_resumption_distance: f64,

    #[arg(
        short = 'g',
        long,
        help = "When analysing, whether to include a Google Maps hyperlink when writing TrackPoints to the Summary sheet. \
               WARNING: This can slow down the opening of the .xlsx in LibreOffice a lot. Implies 'analyse'."
    )]
    pub trackpoint_hyperlinks: bool,

    #[arg(
        help = "List of files to process. Any file that does not have a 'gpx' extension will be ignored."
    )]
    files: Vec<PathBuf>,
}

const JOINED_EXT: &'static str = "joined.gpx";
const SIMPLIFIED_EXT: &'static str = "simplified.gpx";
const JOINED_SIMPLIFIED_EXT: &'static str = "joined.simplified.gpx";
const ANALYSIS_EXT: &'static str = "xlsx";

impl Args {
    /// Returns the list of files to process, in sorted order. This is based on
    /// a simple list of things that was globbed to us on the command line; we
    /// need to apply some filtering to that to ensure we are only dealing with
    /// files ending in '.gpx'. An existence check and other errors (it might be
    /// a directory for example) is left to load time.
    pub fn files(&self) -> Vec<PathBuf> {
        let mut files = Vec::new();

        for f in &self.files {
            if Self::is_gpx_file(&f) {
                if Self::is_output_file(&f) {
                    warn!("Excluding {:?} because it is an output file", f);
                } else {
                    
                }
            } else {
                warn!("Excluding {:?} because it does not end in '.gpx'", f);
            }
        }

        files.sort();
        files
    }

    fn is_gpx_file(p: &Path) -> bool {
        p.extension()
            .is_some_and(|ext| ext.to_ascii_lowercase() == "gpx")
    }

    fn is_output_file(p: &Path) -> bool {
        let s = p.to_string_lossy().to_ascii_lowercase();
        s.ends_with(JOINED_EXT)
            || s.ends_with(SIMPLIFIED_EXT)
            || s.ends_with(JOINED_SIMPLIFIED_EXT)  // Redundant, but for reliability under future changes.
            || s.ends_with(ANALYSIS_EXT)
    }
}

/// The set of required outputs for any particular input file. If a field is
/// 'Some' then that file needs to be produced.
#[derive(Debug)]
pub struct RequiredOutputFiles {
    pub joined_file: Option<PathBuf>,
    pub simplified_file: Option<PathBuf>,
    pub analysis_file: Option<PathBuf>,
}

impl RequiredOutputFiles {
    /// Figure out what output files are required based on
    /// the command line arguments.
    fn new<P: AsRef<Path>>(args: &Args, file: P) -> Self {
        let file = file.as_ref();
        
        let set_ext = |ext: &str| {
            let mut f = file.to_owned();
            f.set_extension(ext);
            f
        };

        let joined_file = args.join.then(|| set_ext(JOINED_EXT));
        let analysis_file = args.join.then(|| set_ext(ANALYSIS_EXT));

        let simplified_file = if args.join && args.metres.is_some() {
            Some(set_ext(JOINED_SIMPLIFIED_EXT))
        } else if args.metres.is_some() {
            Some(set_ext(SIMPLIFIED_EXT))
        } else {
            None
        };


        Self {
            joined_file,
            simplified_file,
            analysis_file,
        }
    }
}

/// Determines the output files that need to be generated for a particular input
/// file. This depends on what command line arguments we were invoked with and
/// whether the specified outputs already exist. Yes, there is potentially a
/// TOCTOU bug here but it really doesn't matter for this program, no feasible
/// race conditions exist.
pub fn get_required_outputs<P: AsRef<Path>>(args: &Args, file: P) -> RequiredOutputFiles {
    let file = file.as_ref();
    let mut rof = RequiredOutputFiles::new(args, file);

    // If we aren't forcing overwrite of the output, then set some of the
    // options to None if the file already exists.
    if !args.force {
        if let Some(file) = rof.joined_file.as_ref() {
            if file.exists() {
                info!("File {:?} already exists, skipping", file);
                rof.joined_file = None;
            }
        }

        if let Some(file) = rof.simplified_file.as_ref() {
            if file.exists() {
                info!("File {:?} already exists, skipping", file);
                rof.simplified_file = None;
            }
        }

        if let Some(file) = rof.analysis_file.as_ref() {
            if file.exists() {
                info!("File {:?} already exists, skipping", file);
                rof.analysis_file = None;
            }
        }
    }

    rof
}
