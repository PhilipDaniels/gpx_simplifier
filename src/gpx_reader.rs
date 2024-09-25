use core::str;
use std::{error::Error, fs::File, path::Path};

use quick_xml::{events::{BytesStart, Event}, Reader};

use crate::model::Gpx;


/// The serde/quick-xml deserialization integration does a "good enough" job of parsing
/// the XML file. We also tag on the original filename as it's handy to track this
/// through the program for when we come to the point of writing output.
pub fn read_gpx_file2(input_file: &Path) -> Result<Gpx, Box<dyn Error>> {
    let mut reader = Reader::from_file(input_file)?;
    let mut buf: Vec<u8> = Vec::with_capacity(4096);

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Decl(decl)) => {
                //writer.write_event(Event::Decl(decl)).unwrap();
            }
            Ok(Event::Start(e)) => {
                match e.name().as_ref() {
                    b"gpx" => {
                        println!("Found the gpx tag");
                        //writer.write_event(Event::Start(e)).unwrap();
                    }
                    b"trk" => {
                        println!("Found a trk tag");
                        //writer.write_event(Event::Start(e)).unwrap();
                    }
                    b"trkseg" => {
                        println!("Found a trkseg tag");
                        //writer.write_event(Event::Start(e)).unwrap();
                    }
                    b"trkpt" => {
                        let lat = get_f32_attr(&e, "lat");
                        let lon = get_f32_attr(&e, "lon");
                        let ele: f32;
                        let time: String;

                        let eot1 = read_ele_or_time(&mut reader, &mut buf);
                        let eot2 = read_ele_or_time(&mut reader, &mut buf);

                        match (eot1, eot2) {
                            (EleOrTime::ele(e), EleOrTime::time(t)) => {
                                ele = e;
                                time = t;
                            }
                            (EleOrTime::time(t), EleOrTime::ele(e)) => {
                                ele = e;
                                time = t;
                            }
                            _ => panic!("Did not get both the <ele> and <time> tags"),
                        }

                        // let tp = Trackpoint {
                        //     lat,
                        //     lon,
                        //     ele,
                        //     time,
                        // };

                        //println!("{:?}", tp);
                        //trackpoints.push(tp);
                    }
                    b"ele" => {
                        // Read again to get the text inside the <ele>...</ele> tags.
                        match reader.read_event_into(&mut buf) {
                            Ok(Event::Text(t)) => {
                                //writer.create_element("ele").write_text_content(t).unwrap();
                            }
                            _ => panic!("Got unexpected XML node, document is probably corrupt"),
                        }
                    }
                    b"time" => {
                        // Read again to get the text inside the <time>...</time> tags.
                        match reader.read_event_into(&mut buf) {
                            Ok(Event::Text(t)) => {
                                //writer.create_element("time").write_text_content(t).unwrap();
                            }
                            _ => panic!("Got unexpected XML node, document is probably corrupt"),
                        }
                    }
                    _ => (),
                }
            }
            Ok(Event::End(e)) => match e.name().as_ref() {
                b"gpx" => {
                    //writer.write_event(Event::End(e)).unwrap();
                }
                b"trk" => {
                    //writer.write_event(Event::End(e)).unwrap();
                }
                b"trkseg" => {
                    //writer.write_event(Event::End(e)).unwrap();
                }
                b"trkpt" => {
                    //writer.write_event(Event::End(e)).unwrap();
                }
                _ => (),
            },
            Ok(Event::Eof) => break,
            Err(e) => panic!("Error at position {}: {:?}", reader.error_position(), e),
            _ => (),
        }

        buf.clear();
    }



    let mut doc: Gpx = quick_xml::de::from_reader(reader.into_inner()).unwrap();
    doc.filename = input_file.to_owned();


    Ok(doc)
}

enum EleOrTime {
    ele(f32),
    time(String),
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
                                let ele = str::from_utf8(ele).expect("The bytes should be ASCII, therefore valid UTF-8");
                                let ele: f32 = ele.parse().expect("The string should be a valid number");
                                // All the ele's will come out to 1 d.p., because that is all that my Garmin
                                // Edge 1040 can actually manage, even though it records them as
                                // "151.1999969482421875" or "149.8000030517578125".
                                return EleOrTime::ele(ele)
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
                                return EleOrTime::time(time)
                            }
                            _ => panic!("Got unexpected XML node, document is probably corrupt"),
                        }
                    }
                    _ => panic!("Unexpected element")
                }
            }
            _ => {}
        }
    }
}

fn get_f32_attr(e: &BytesStart, arg: &str) -> f32 {
    let attr = e
        .try_get_attribute(arg)
        .expect("Unless the file is corrupt the attributes we asl for always exist")
        .expect("And always have values")
        .value;
    let attr = attr.as_ref();
    let attr = str::from_utf8(attr).expect("The bytes should be ASCII, therefore valid UTF-8");
    let attr: f32 = attr.parse().expect("The string should be a valid number");
    attr
}