use model::{Gpx, MergedGpx, Track, TrackSegment};
use quick_xml::reader::Reader;
use std::io::Write;
use std::{
    fs::{read_dir, File},
    io::BufWriter,
    path::{Path, PathBuf},
};

mod model;

fn main() {
    let exe_dir = get_exe_dir();
    let input_files = get_list_of_input_files(&exe_dir);
    if input_files.is_empty() {
        println!("No .gpx files found");
        return;
    }

    for f in input_files {
        let mut output_file = f.to_owned();
        output_file.set_extension("simplified.gpx");
        if output_file.exists() {
            println!(
                "Simplified file {:?} already exists, skipping...",
                &output_file
            );
            continue;
        }

        let gpx = read_gpx_file(&f);
        let gpx = gpx.to_merged_gpx();
        write_output_file(&output_file, &gpx);
    }
}

/// The serde/quick-xml deserialization integration does a "good enough" job of parsing
/// the XML file.
fn read_gpx_file(input_file: &Path) -> Gpx {
    let reader = Reader::from_file(input_file).expect("Could not create XML reader");
    let doc: Gpx = quick_xml::de::from_reader(reader.into_inner()).unwrap();
    doc
}

/*
/// Joins all the tracks and track segments in the file so there
/// is only one trackk and one segment containing all the points.
/// TODO: An alternative implementation would have a simpler model, with
/// just Gpx, Metadata and a Vec of TrackPoints.
fn join_all_tracks(gpx: &Gpx) -> Gpx {
    // Duplicate all the track points into one vec.
    let mut points = Vec::new();
    for src_track in &gpx.tracks {
        for src_segment in &src_track.segments {
            for src_point in &src_segment.points {
                points.push(src_point.clone());
            }
        }
    }

    // Happy to use the name of the first track in the file as
    // the name for the merged track.
    let track = Track {
        name: gpx.tracks[0].name.clone(),
        r#type: gpx.tracks[0].r#type.clone(),
        segments: vec![TrackSegment { points }]
    };

    let mut result = gpx.duplicate();
    result.tracks.push(track);

    // Join any tracks together so there is only 1.
    assert!(result.tracks.len() == 1);
    assert!(result.tracks[0].segments.len() == 1);

    result
}
*/

fn write_output_file(output_file: &Path, gpx: &MergedGpx) {
    println!("Writing file {:?}", &output_file);

    // TODO: If Garmin ever changes this then what we need to do is read the GPX node in the way
    // we used to do, using the streaming interface, then write it to the output file.
    // But for now, let's wing it...
    let mut w = BufWriter::new(File::create(&output_file).expect("Could not open output_file"));
    writeln!(w, "<?xml version=\"1.0\" encoding=\"UTF-8\"?>").unwrap();
    writeln!(
        w,
        "<gpx creator=\"{}\" version=\"{}\"",
        gpx.creator, gpx.version
    )
    .unwrap();
    writeln!(w, "  xsi::schemaLocation=\"{}\"", gpx.xsi_schema_location).unwrap();
    writeln!(w, "  xmlns:ns3=\"{}\"", gpx.xmlns_ns3).unwrap();
    writeln!(w, "  xmlns=\"{}\"", gpx.xmlns).unwrap();
    writeln!(w, "  xmlns:xsi=\"{}\"", gpx.xmlns_xsi).unwrap();
    writeln!(w, "  xmlns:ns2=\"{}\">", gpx.xmlns_ns2).unwrap();
    writeln!(w, "  <metadata>").unwrap();
    writeln!(w, "    <time>{}</time>", gpx.metadata_time).unwrap();
    writeln!(w, "  </metadata>").unwrap();

    writeln!(w, "  <trk>").unwrap();
    writeln!(w, "    <name>{}</name>", gpx.track_name).unwrap();
    writeln!(w, "    <type>{}</type>", gpx.track_type).unwrap();
    writeln!(w, "    <trkseg>").unwrap();
    for tp in &gpx.points {
        writeln!(w, "      <trkpt lat=\"{}\" lon=\"{}\">", tp.lat, tp.lon).unwrap();
        writeln!(w, "        <ele>{}</ele>", tp.ele).unwrap();
        writeln!(w, "        <time>{}</time>", tp.time).unwrap();
        writeln!(w, "      </trkpt>").unwrap();
    }
    writeln!(w, "    </trkseg>").unwrap();
    writeln!(w, "  </trk>").unwrap();
    writeln!(w, "</gpx>").unwrap();

    w.flush().unwrap();
}

// Get a list of all files in the exe_dir that have the ".gpx" extension.
// Be careful to exclude files that actually end in ".simplified.gpx" -
// they are output files we already created! If we don't exclude them here,
// we end up generating ".simplified.simplified.gpx", etc.
fn get_list_of_input_files(exe_dir: &PathBuf) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let Ok(entries) = read_dir(exe_dir) else {
        return Vec::new();
    };

    for entry in entries {
        let entry = entry.unwrap();
        let meta = entry.metadata().unwrap();
        if meta.is_file() {
            let s = &entry.file_name();
            let p = Path::new(s);
            if let Some(ext) = p.extension() {
                if ext.to_ascii_lowercase() == "gpx" {
                    let s = s.to_string_lossy().to_ascii_lowercase();
                    if !s.ends_with(".simplified.gpx") {
                        println!("Found GPX input file {:?}", entry.path());
                        files.push(entry.path());
                    }
                }
            }
        }
    }

    files
}

fn get_exe_dir() -> PathBuf {
    let mut exe_path = std::env::current_exe().unwrap();
    exe_path.pop();
    exe_path
}
