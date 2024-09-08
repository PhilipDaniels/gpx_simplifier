use std::{error::Error, path::Path};

use rust_xlsxwriter::Workbook;

use crate::{model::EnrichedGpx, section::SectionList};

pub fn write_summary_file<'gpx>(
    summary_filename: &Path,
    gpx: &EnrichedGpx,
    sections: &SectionList<'gpx>,
) -> Result<(), Box<dyn Error>> {
    print!("Writing file {:?}", &summary_filename);

    let mut workbook = Workbook::new();

    let mut summary_ws = workbook.add_worksheet();
    summary_ws.set_name("Summary")?;

    let mut tp_ws = workbook.add_worksheet();
    tp_ws.set_name("Track Points")?;

    workbook.save(summary_filename).unwrap();
    let metadata = std::fs::metadata(summary_filename).unwrap();
    println!(", {} Kb", metadata.len() / 1024);
    Ok(())
}
