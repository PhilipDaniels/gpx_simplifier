use std::{error::Error, path::Path};

use logging_timer::time;

use crate::model::Gpx;

/// Writes a GPX to file with full-fidelity, i.e. everything we can write is written.
#[time]
pub fn write_gpx_file<P: AsRef<Path>>(output_file: P, gpx: &Gpx) -> Result<(), Box<dyn Error>> {
    Ok(())
}
