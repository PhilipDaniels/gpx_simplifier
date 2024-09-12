use clap::{arg, command, value_parser, Parser};

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
        default_value = "10",
        help = "A stop is considered to end when you start moving with this speed, in km/h",
        requires = "detect_stages"
    )]
    pub resume_speed: u8,

    #[arg(
        long,
        default_value = "10",
        help = "Minimum length of a stop, in minutes, for it to be detected",
        requires = "detect_stages"
    )]
    pub min_stop_time: u8,

    #[arg(
        long,
        help = "Whether to write a tab to the summary spreadsheet containing all the TrackPoints in the GPX",
        requires = "detect_stages"
    )]
    pub write_trackpoints: bool,

    #[arg(
        long,
        help = "Whether to include a Google Maps hyperlink when writing TrackPoints to the summary sheet. WARNING: This can slow down the opening of the .xlsx in LibreOffice a lot",
        requires = "write_trackpoints, detect_stages"
    )]
    pub write_trackpoint_hyperlinks: bool,
}


pub fn parse_args() -> Args {
    Args::parse()
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Hyperlink {
    Yes,
    No,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum TrackpointSummaryOptions {
    NoTrackpoints,
    Trackpoints(Hyperlink)
}

impl Args {
    /// Gets the effective options for writing trackpoints
    /// to the summary sheet.
    pub fn trackpoint_options(&self) -> TrackpointSummaryOptions {
        if !self.write_trackpoints {
            TrackpointSummaryOptions::NoTrackpoints
        } else {
            if self.write_trackpoint_hyperlinks {
                TrackpointSummaryOptions::Trackpoints(Hyperlink::Yes)
            } else {
                TrackpointSummaryOptions::Trackpoints(Hyperlink::No)
            }
        }
    }
}
