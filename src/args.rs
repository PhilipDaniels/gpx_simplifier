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
        help = "Whether to detect stops (intervals of 0 speed) in the GPX track"
    )]
    pub detect_stops: bool,

    #[arg(
        long,
        default_value = "15",
        help = "A stop is considered to end when you start moving with this speed, in km/h"
    )]
    pub resume_speed: u8,

    #[arg(
        long,
        default_value = "10",
        help = "Minimum length of a stop, in minutes, for it to be detected"
    )]
    pub min_stop_time: u8,
}


pub fn parse_args() -> Args {
    Args::parse()
}
