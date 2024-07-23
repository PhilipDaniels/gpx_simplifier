use core::{panic, str};
use model::Trackpoint;
use quick_xml::events::BytesText;
use quick_xml::reader::Reader;
use quick_xml::Error;
use quick_xml::{
    events::{BytesStart, Event},
    Writer,
};
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
        simplify(&f);
    }
}

fn simplify(input_file: &Path) {
    let mut output_file = input_file.to_owned();
    output_file.set_extension("simplified.gpx");
    if output_file.exists() {
        println!(
            "Simplified file {:?} already exists, skipping...",
            &output_file
        );
        return;
    }

    println!("Writing file {:?}", &output_file);

    // Reading.
    let mut reader = Reader::from_file(input_file).expect("Could not create XML reader");
    let mut buf = Vec::with_capacity(8096);
    // Writing.
    let bw = BufWriter::new(File::create(&output_file).expect("Could not open output_file"));
    let mut writer = Writer::new_with_indent(bw, b' ', 2);

    let mut trackpoints = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            // Write this node direct to the output file so we
            // don't have to parse it.
            Ok(Event::Decl(decl)) => {
                writer.write_event(Event::Decl(decl)).unwrap();
            }
            Ok(Event::Start(e)) => {
                match e.name().as_ref() {
                    // Write this node direct to the output file so we
                    // don't have to parse it.
                    b"gpx" => {
                        println!("Found the gpx tag");
                        writer.write_event(Event::Start(e)).unwrap();
                    }
                    b"trkpt" => {
                        // Accumulate all the trackpoints into a Vec.
                        let lat = get_f32_attr(&e, "lat");
                        let lon = get_f32_attr(&e, "lon");

                        let ele: f32;
                        let time: String;
                        let eot1 = read_ele_or_time(&mut reader, &mut buf);
                        let eot2 = read_ele_or_time(&mut reader, &mut buf);

                        match (eot1, eot2) {
                            (EleOrTime::Ele(e), EleOrTime::Time(t)) => {
                                ele = e;
                                time = t;
                            }
                            (EleOrTime::Time(t), EleOrTime::Ele(e)) => {
                                ele = e;
                                time = t;
                            }
                            _ => panic!("Did not get both the <ele> and <time> tags"),
                        }

                        let tp = Trackpoint {
                            lat,
                            lon,
                            ele,
                            time,
                        };

                        trackpoints.push(tp);
                    }
                    _ => (),
                }
            }
            Ok(Event::End(e)) => match e.name().as_ref() {
                // Once we hit the end <gpx> tag we can write out all the trkpts.
                // We also need the other tags to make a valid GPX file. Writing <trk>
                // and <trkseg> at the end instead of as we go along means that we also
                // effectively merge split tracks that are recorded in a single file.
                // And having all the trkpts in a big vec means we can run various
                // simplification algorithms on them all in memory (it's impossible
                // to do this in a purely streaming approach.)
                b"gpx" => {
                    writer.create_element("trk").write_inner_content::<_, Error>(|w| {
                        w.create_element("trkseg").write_inner_content::<_, Error>(|w| {
                            for tp in &trackpoints {
                                w.create_element("trkpt")
                                    .with_attribute(("lat", format!("{}", tp.lat).as_str()))
                                    .with_attribute(("lon", format!("{}", tp.lon).as_str()))
                                    .write_inner_content::<_, Error>(|w| {
                                        let ele = format!("{}", tp.ele);
                                        w.create_element("ele").write_text_content(BytesText::new(&ele)).unwrap();
                                        let time = format!("{}", tp.time);
                                        w.create_element("time").write_text_content(BytesText::new(&time)).unwrap();
                                        Ok(())
                                    })
                                .unwrap();
                            }
                            Ok(())
                        }).unwrap();
                        Ok(())
                    }
                    ).unwrap();
                    
                    writer.write_event(Event::End(e)).unwrap();
                }
                _ => (),
            },
            Ok(Event::Eof) => break,
            Err(e) => panic!("Error at position {}: {:?}", reader.error_position(), e),
            _ => (),
        }

        buf.clear();
    }

    println!("Found {} trkpt nodes", trackpoints.len());
}

enum EleOrTime {
    Ele(f32),
    Time(String),
}

/// Read the <ele> or <time> sub-node. I am not assuming which comes first in the file,
/// (so we return an enum) but I am assuming they are the first sub-nodes, e.g. before
/// any <extensions>.
fn read_ele_or_time(reader: &mut Reader<std::io::BufReader<File>>, buf: &mut Vec<u8>) -> EleOrTime {
    loop {
        match reader.read_event_into(buf) {
            Ok(Event::Start(e)) => {
                match e.name().as_ref() {
                    b"ele" => {
                        // Read again to get the text inside the <ele>...</ele> tags.
                        match reader.read_event_into(buf) {
                            Ok(Event::Text(ele)) => {
                                let ele = ele.as_ref();
                                let ele = str::from_utf8(ele).unwrap();
                                let ele: f32 = ele.parse().unwrap();
                                // All the ele's will come out to 1 d.p., because that is all that my Garmin
                                // Edge 1040 can actually manage, even though it records them as
                                // "151.1999969482421875" or "149.8000030517578125".
                                return EleOrTime::Ele(ele);
                            }
                            _ => panic!("Got unexpected XML node, document is probably corrupt"),
                        }
                    }
                    b"time" => {
                        // Read again to get the text inside the <time>...</time> tags.
                        match reader.read_event_into(buf) {
                            Ok(Event::Text(time)) => {
                                let time = time.as_ref();
                                let time = String::from_utf8_lossy(time).into_owned();
                                return EleOrTime::Time(time);
                            }
                            _ => panic!("Got unexpected XML node, document is probably corrupt"),
                        }
                    }
                    _ => panic!("Unexpected element"),
                }
            }
            _ => {}
        }
    }
}

/// Reads an attribute value and converts it to an f32.
fn get_f32_attr(e: &BytesStart, arg: &str) -> f32 {
    let lat2 = e
        .try_get_attribute(arg)
        .expect("Unless the file is corrupt the attributes we ask for always exist")
        .expect("And always have values")
        .value;
    let lat2 = lat2.as_ref();
    let lat2 = str::from_utf8(lat2).expect("The bytes should be ASCII, therefore valid UTF-8");
    let lat2: f32 = lat2.parse().expect("The string should be a valid number");
    lat2
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
