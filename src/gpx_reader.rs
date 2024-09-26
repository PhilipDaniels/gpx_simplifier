use core::{panic, str};
use std::{
    borrow::{Borrow, Cow},
    collections::HashMap,
    error::Error,
    fs::File,
    io::BufReader,
    path::Path,
};

use quick_xml::{
    events::{BytesDecl, BytesStart, Event},
    Reader,
};
use time::{format_description::well_known, OffsetDateTime};

use crate::model::{Declaration, Gpx2, GpxInfo, GpxMetadata, Link, Track, Track2};

/// The XSD, which defines the format of a GPX file, is at https://www.topografix.com/GPX/1/1/gpx.xsd
/// This function doesn't parse everything, just the things that appear in my Garmin files.
pub fn read_gpx_file2(input_file: &Path) -> Result<Gpx2, Box<dyn Error>> {
    let mut reader = Reader::from_file(input_file)?;
    let mut buf: Vec<u8> = Vec::with_capacity(512);

    let mut declaration: Declaration;
    let mut gpx_info: GpxInfo;
    let mut metadata: GpxMetadata;
    let mut tracks: Vec<Track2> = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Decl(decl)) => {
                // This is the very first line: <?xml version="1.0" encoding="UTF-8"?>
                declaration = parse_decl(decl)?;
                dbg!(declaration);
            }
            Ok(Event::Start(e)) => {
                match e.name().as_ref() {
                    b"gpx" => {
                        gpx_info = parse_gpx_info(e)?;
                        dbg!(gpx_info);
                    }
                    b"metadata" => {
                        metadata = parse_metadata(&mut buf, &mut reader)?;
                        dbg!(metadata);
                    }
                    b"trk" => {
                        let track = parse_track(&mut buf, &mut reader)?;
                        tracks.push(track);
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

    todo!();
}

/// Parses a declaration, i.e. the very first line of the file which is:
///     <?xml version="1.0" encoding="UTF-8"?>
fn parse_decl(decl: BytesDecl<'_>) -> Result<Declaration, Box<dyn Error>> {
    Ok(Declaration {
        version: rcow_to_string(decl.version())?,
        encoding: orcow_to_string(decl.encoding())?,
        standalone: orcow_to_string(decl.standalone())?,
    })
}

fn parse_gpx_info(tag: BytesStart<'_>) -> Result<GpxInfo, Box<dyn Error>> {
    Ok(GpxInfo {
        attributes: parse_attributes(tag)?,
    })
}

fn parse_metadata(
    buf: &mut Vec<u8>,
    reader: &mut Reader<BufReader<File>>,
) -> Result<GpxMetadata, Box<dyn Error>> {
    let mut href = None;
    let mut text = None;
    let mut mime_type = None;
    let mut time = None;

    loop {
        match reader.read_event_into(buf) {
            // TODO: We could break out a 'parse_link' function, as it is a defined
            // element type in the XSD.
            Ok(Event::Start(e)) => match e.name().as_ref() {
                b"link" => {
                    href = Some(read_attribute_as_string(e, "href")?);
                }
                b"text" => {
                    text = Some(read_inner_as_string(buf, reader)?);
                }
                b"type" => {
                    mime_type = Some(read_inner_as_string(buf, reader)?);
                }
                b"time" => {
                    time = Some(read_inner_as_time(buf, reader)?);
                }
                e @ _ => panic!("Unexpected element {:?}", e),
            },
            Ok(Event::End(e)) => {
                match e.name().as_ref() {
                    b"metadata" => {
                        if href.is_none() {
                            return Err("href attribute not found, but it is mandatory according to the XSD")?;
                        } else {
                            return Ok(GpxMetadata {
                                link: Link {
                                    href: href.unwrap(),
                                    text,
                                    r#type: mime_type,
                                },
                                time,
                            });
                        }
                    }
                    _ => {}
                }
            }
            // Ignore spurious Event::Text, I think they are newlines.
            Ok(Event::Text(_)) => {}
            e @ _ => panic!("Unexpected element {:?}", e),
        }
    }
}

fn parse_track(
    buf: &mut Vec<u8>,
    reader: &mut Reader<BufReader<File>>,
) -> Result<Track2, Box<dyn Error>> {
    todo!()
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
                                let ele = str::from_utf8(ele)
                                    .expect("The bytes should be ASCII, therefore valid UTF-8");
                                let ele: f32 =
                                    ele.parse().expect("The string should be a valid number");
                                // All the ele's will come out to 1 d.p., because that is all that my Garmin
                                // Edge 1040 can actually manage, even though it records them as
                                // "151.1999969482421875" or "149.8000030517578125".
                                return EleOrTime::ele(ele);
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
                                return EleOrTime::time(time);
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

fn parse_attributes(tag: BytesStart<'_>) -> Result<HashMap<String, String>, Box<dyn Error>> {
    let mut result = HashMap::new();

    for attr in tag.attributes() {
        let attr = attr?;
        let key = attr.key.into_inner();
        let key = bytes_to_string(key)?;
        let value = cow_to_string(attr.value)?;

        result.insert(key, value);
    }

    Ok(result)
}

fn read_attribute_as_string(
    tag: BytesStart<'_>,
    attribute_name: &str,
) -> Result<String, Box<dyn Error>> {
    let attrs = parse_attributes(tag)?;
    match attrs.get(attribute_name) {
        Some(value) => Ok(value.clone()),
        None => Err(format!("No attribute named {attribute_name}"))?,
    }
}

/// Reads the 'INNER TEXT' from a tag such as <tag>INNER TEXT</tag>.
fn read_inner_as_string(
    buf: &mut Vec<u8>,
    reader: &mut Reader<BufReader<File>>,
) -> Result<String, Box<dyn Error>> {
    match reader.read_event_into(buf) {
        Ok(Event::Text(ele)) => Ok(bytes_to_string(ele.as_ref())?),
        e @ _ => Err(format!(
            "Got unexpected XML node, document is probably corrupt: {:?}",
            e
        )
        .into()),
    }
}

/// Reads a <time>2024-09-21T06:59:46.000Z</time> tag.
fn read_inner_as_time(
    buf: &mut Vec<u8>,
    reader: &mut Reader<BufReader<File>>,
) -> Result<OffsetDateTime, Box<dyn Error>> {
    let t = read_inner_as_string(buf, reader)?;
    Ok(OffsetDateTime::parse(&t, &well_known::Rfc3339)?)
}

/// Reads inner text and converts it to an f64.
fn read_inner_as_f64(
    buf: &mut Vec<u8>,
    reader: &mut Reader<BufReader<File>>,
) -> Result<f64, Box<dyn Error>> {
    let t = read_inner_as_string(buf, reader)?;
    Ok(t.parse::<f64>()?)
}

/// Reads inner text and converts it to a u16.
fn read_inner_as_u16(
    buf: &mut Vec<u8>,
    reader: &mut Reader<BufReader<File>>,
) -> Result<u16, Box<dyn Error>> {
    let t = read_inner_as_string(buf, reader)?;
    Ok(t.parse::<u16>()?)
}

fn cow_to_string(v: Cow<'_, [u8]>) -> Result<String, Box<dyn Error>> {
    bytes_to_string(v.borrow())
}

fn rcow_to_string(v: Result<Cow<'_, [u8]>, quick_xml::Error>) -> Result<String, Box<dyn Error>> {
    match v {
        Ok(Cow::Borrowed(s)) => Ok(bytes_to_string(s)?),
        Ok(Cow::Owned(s)) => Ok(bytes_to_string(&s)?),
        Err(err) => Err(Box::new(err)),
    }
}

fn orcow_to_string(
    v: Option<Result<Cow<'_, [u8]>, quick_xml::Error>>,
) -> Result<Option<String>, Box<dyn Error>> {
    match v {
        Some(Ok(Cow::Borrowed(s))) => Ok(Some(bytes_to_string(s)?)),
        Some(Ok(Cow::Owned(s))) => Ok(Some(bytes_to_string(&s)?)),
        Some(Err(err)) => Err(Box::new(err)),
        None => Ok(None),
    }
}

fn bytes_to_string(value: &[u8]) -> Result<String, Box<dyn Error>> {
    match str::from_utf8(value) {
        Ok(s) => Ok(s.to_string()),
        Err(err) => Err(Box::new(err)),
    }
}
