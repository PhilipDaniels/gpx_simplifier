use clap::{arg, command, value_parser, Parser};
use gapix_core::excel::Hyperlink;

#[derive(Debug, Default, Parser)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[arg(
        short = 'm',
        long,
        help = "Simplify by using Ramer-Douglas-Peucker with METRES accuracy",
        value_parser = value_parser!(u16).range(1..=1000)
    )]
    pub metres: Option<u16>,

    #[arg(
        short,
        long,
        help = "Join multiple input GPX files into a single file with 1 track"
    )]
    pub join: bool,


    #[arg(
        short,
        long,
        default_value = "false",
        help = "Whether to detect stages (periods of moving alternating with stops) in the GPX track and write a 'summary.xlsx' file"
    )]
    pub detect_stages: bool,

    #[arg(
        long,
        default_value = "0.15",
        help = "The speed, in km/h, which you must drop below for us to think you are stopped",
        requires = "detect_stages"
    )]
    pub stopped_speed: f64,

    #[arg(
        long,
        default_value = "5.0",
        help = "Minimum length of a stage stop, in minutes, for it to be detected",
        requires = "detect_stages"
    )]
    pub min_stop_time: f64,

    #[arg(
        long,
        default_value = "100.0",
        help = "The distance you must move (as the crow flies from your stop point) before you are considered to be moving again",
        requires = "detect_stages"
    )]
    pub stop_resumption_distance: f64,

    #[arg(
        long,
        help = "Whether to include a Google Maps hyperlink when writing TrackPoints to the summary sheet. WARNING: This can slow down the opening of the .xlsx in LibreOffice a lot",
        requires = "write_trackpoints, detect_stages"
    )]
    pub write_trackpoint_hyperlinks: bool,
}


pub fn parse_args() -> Args {
    // Use the wild crate to do glob expansion on Windows.
    Args::parse_from(wild::args())
}

impl Args {
    /// We always write the list of trackpoints, but adding
    /// hyperlinks is optional.
    /// TODO: Move this somewhere else. Only needed in the Excel writer.
    pub fn trackpoint_hyperlinks(&self) -> Hyperlink {
        if self.write_trackpoint_hyperlinks {
            Hyperlink::Yes
        } else {
            Hyperlink::No
        }
    }
}
