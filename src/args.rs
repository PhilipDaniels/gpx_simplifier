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
}

pub fn parse_args() -> Args {
    Args::parse()
}
