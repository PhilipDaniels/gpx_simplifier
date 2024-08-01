use clap::{arg, command, error::ErrorKind, value_parser};

#[derive(Debug)]
pub enum Args {
    Keep(u8),
    RdpMetres(f32),
}

pub fn parse_args() -> Args {
    let mut cmd = command!()
        .arg(
            arg!(--keep <VALUE>)
                .help("Simplify by keeping every N trackpoints")
                .short('k')
                .conflicts_with("metres")
                .required(false)
                .value_parser(value_parser!(u8).range(1..50)),
        )
        .arg(
            arg!(--metres <VALUE>)
                .help("Simplify by using Ramer–Douglas–Peucker with METRES accuracy")
                .short('m')
                .conflicts_with("keep")
                .required(false)
                .value_parser(value_parser!(f32)),
        );

    let matches = cmd.get_matches_mut();

    if let Some(&n) = matches.get_one::<u8>("keep") {
        Args::Keep(n)
    } else if let Some(&metres) = matches.get_one::<f32>("metres") {
        if metres < 0.1 || metres > 1000.0 {
            cmd.error(
                ErrorKind::ValueValidation,
                "metres must be in range 0.1..1000.0",
            )
            .exit();
        }

        Args::RdpMetres(metres)
    } else {
        cmd.error(
            ErrorKind::TooFewValues,
            "Specify one of '--keep' or '--metres'",
        )
        .exit();
    }
}
