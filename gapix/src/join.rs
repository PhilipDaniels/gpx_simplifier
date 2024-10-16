use std::path::Path;

use anyhow::{bail, Result};
use gapix_core::{read::read_gpx_from_file, model::Gpx};
use log::info;
use logging_timer::time;

use crate::PROGRAM_NAME;

/// Joins multiple input files into a single file with 1 track and 1 track
/// segment that contains all the track points.
#[time]
pub fn join_input_files<P: AsRef<Path>>(files: &[P]) -> Result<Gpx> {
    if files.is_empty() {
        bail!("input file list is empty");
    }

    // Read in the first file. We will append the others to this one.
    let first_file = files[0].as_ref();
    let gpx = read_gpx_from_file(&first_file)?;
    let mut gpx = gpx.into_single_track();
    let pts = &mut gpx.tracks[0].segments[0].points;

    info!(
        "join: read {} trackpoints from {:?}",
        pts.len(),
        &first_file
    );

    for f in files.iter().skip(1) {
        let next_gpx = read_gpx_from_file(&f)?;
        let mut next_gpx = next_gpx.into_single_track();
        let next_pts = &mut next_gpx.tracks[0].segments[0].points;
        info!(
            "join: read {} trackpoints from {:?}",
            next_pts.len(),
            f.as_ref()
        );
        pts.append(next_pts);
    }

    // Sort all the points by ascending time in case we got the files in a wacky
    // order.
    pts.sort_by_key(|p| p.time);

    info!(
        "join: Successfully joined {} files with a total of {} trackpoints",
        files.len(),
        pts.len()
    );

    // Since we made a new structure.
    gpx.creator = PROGRAM_NAME.to_owned();

    Ok(gpx)
}
