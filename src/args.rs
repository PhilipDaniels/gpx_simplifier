use clap::{arg, command, error::ErrorKind, value_parser};

#[derive(Debug)]
pub enum Args {
    Keep(u8),
    RdpEpsilon(f32),
}

pub fn parse_args() -> Args {
    let mut cmd = command!()
        .arg(
            arg!(--keep <VALUE>)
                .help("Simplify by keeping every N trackpoints")
                .short('k')
                .conflicts_with("epsilon")
                .required(false)
                .value_parser(value_parser!(u8).range(1..50)),
        )
        .arg(
            arg!(--epsilon <VALUE>)
                .help("Simplify by using Ramer–Douglas–Peucker with epsilon accuracy")
                .short('e')
                .conflicts_with("keep")
                .required(false)
                .value_parser(value_parser!(f32)),
        );

    let matches = cmd.get_matches_mut();

    if let Some(&n) = matches.get_one::<u8>("keep") {
        Args::Keep(n)
    } else if let Some(&eps) = matches.get_one::<f32>("epsilon") {
        // if eps < 0.1 || eps > 50.0 {
        //     cmd.error(
        //         ErrorKind::ValueValidation,
        //         "epsilon must be in range 0.1..50.0",
        //     )
        //     .exit();
        // }

        Args::RdpEpsilon(eps)
    } else {
        cmd.error(
            ErrorKind::TooFewValues,
            "Specify one of '--keep' or '--epsilon'",
        )
        .exit();
    }
}
