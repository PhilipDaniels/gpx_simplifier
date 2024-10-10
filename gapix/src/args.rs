use std::path::PathBuf;

use clap::{arg, builder::ArgPredicate, command, value_parser, Parser};

/*
 --join FILES                      join & output full track
 --join --metres=5 FILES           join & output full track and simplified track
 --metres=5 FILES                  output simplified track
 --analyse [other params] FILES    analyse into an xlsx


 --force                           always write output even if it exists. Global option.
 FILES                             all commands require a list of files to operate on


 FULL SYNTAX
 ===========
 [--force] [--join] [--metres=5] \
   [--analyse [--stopped_speed] [--stop_resumption_distance] [--write_trackpoint_hyperlinks] ] \
   FILES
*/

/// Returns the parsed command line options. Uses the 'wild' crate to do glob
/// expansion on Windows. so that Windows and Linux behave identically.
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
    force: bool,

    #[arg(
        short,
        long,
        default_value = "false",
        help = "Join the input GPX files into a single file with 1 track before \
                applying further processing"
    )]
    join: bool,

    #[arg(
        short,
        long,
        help = "Reduce the number of track points by using Ramer-Douglas-Peucker \
                with METRES accuracy and produce a '.simplified.gpx' file",
        value_parser = value_parser!(u16).range(1..=1000)
    )]
    metres: Option<u16>,

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
    analyse: bool,

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
               WARNING: This can slow down the opening of the .xlsx in LibreOffice a lot. Implies 'analyse'.",
    )]
    pub trackpoint_hyperlinks: bool,

    #[arg(
        help = "List of files to process. Any file that does not have a 'gpx' extension will be ignored.",
    )]
    pub files: Vec<PathBuf>,
}

// impl Args {
//     /// We always write the list of trackpoints, but adding
//     /// hyperlinks is optional.
//     /// TODO: Move this somewhere else. Only needed in the Excel writer.
//     pub fn trackpoint_hyperlinks(&self) -> Hyperlink {
//         if self.write_trackpoint_hyperlinks {
//             Hyperlink::Yes
//         } else {
//             Hyperlink::No
//         }
//     }
// }
