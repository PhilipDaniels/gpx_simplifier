#![allow(clippy::single_match)]

use core::str;
use std::{
    borrow::{Borrow, Cow},
    collections::{hash_map::Entry, HashMap},
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
    str::FromStr,
};

use anyhow::{bail, Context, Result};
use log::info;
use logging_timer::time;
use quick_xml::{
    events::{BytesDecl, BytesStart, Event},
    Reader,
};
use time::{format_description::well_known, OffsetDateTime};

use crate::model::{
    Declaration, Extensions, Gpx, GpxInfo, Link, Metadata, Track, TrackPoint, TrackSegment,
};

/*
<xml>                                                  parse_decl
<gpx>                          type="gpxType"          parse_gpx_info
   <metadata>                  type="metadataType"     parse_metadata
   <wpt>                       type="wptType"          n.a.
   <rte>                       type="rteType"          n.a.
   <extensions>                type="extensionsType"   n.a.
   <trk>                       type="trkType"          parse_track
       <trkseg>                type="trksegType"       parse_track_segment
           <trkpt>             type="wptType"          parse_trackpoint
               <extensions>    type="extensions"       parse_trackpoint_extensions

*/

#[time]
pub fn read_gpx_from_reader<R: BufRead, P: AsRef<Path>>(input: R, input_file: &P) -> Result<Gpx> {
    let input_file = input_file.as_ref();
    let mut xml_reader = Reader::from_reader(input);
    let mut buf: Vec<u8> = Vec::with_capacity(512);

    let mut declaration = None;
    let mut gpx_info = None;
    let mut metadata = None;
    let mut tracks: Vec<Track> = Vec::new();

    loop {
        match xml_reader.read_event_into(&mut buf) {
            Ok(Event::Decl(decl)) => {
                declaration = Some(parse_decl(&decl)?);
            }
            Ok(Event::Start(e)) => match e.name().as_ref() {
                b"gpx" => {
                    gpx_info = Some(parse_gpx_info(&e)?);
                }
                b"metadata" => {
                    metadata = Some(parse_metadata(&mut buf, &mut xml_reader)?);
                }
                b"trk" => {
                    let track = parse_track(&mut buf, &mut xml_reader)?;
                    tracks.push(track);
                }
                _ => (),
            },
            Ok(Event::End(e)) => match e.name().as_ref() {
                b"gpx" => {
                    let gpx = Gpx {
                        filename: input_file.to_owned(),
                        declaration: declaration.context("Did not find the 'xml' declaration element")?,
                        info: gpx_info.context("Did not find the 'gpx' element")?,
                        metadata: metadata.context("Did not find the 'metadata' element")?,
                        tracks,
                    };

                    return Ok(gpx);
                }
                _ => (),
            },
            Ok(Event::Eof) => {
                bail!("Reached EOF unexpectedly (before the closing GPX tag) while parsing {:?}. File is probably corrupt.",
                    input_file);
            }
            Err(e) => bail!("Error at position {}: {:?}", xml_reader.error_position(), e),
            _ => (),
        }

        buf.clear();
    }
}

/// The XSD, which defines the format of a GPX file, is at https://www.topografix.com/GPX/1/1/gpx.xsd
/// This function doesn't parse everything, just the things that appear in my Garmin files.
pub fn read_gpx_from_file<P: AsRef<Path>>(input_file: &P) -> Result<Gpx> {
    let input_file = input_file.as_ref();
    info!("Reading GPX file {:?}", input_file);
    let buf_reader = BufReader::new(File::open(input_file)?);
    return read_gpx_from_reader(buf_reader, &input_file);
}

/// Parses an XML declaration, i.e. the very first line of the file which is:
///     <?xml version="1.0" encoding="UTF-8"?>
fn parse_decl(decl: &BytesDecl<'_>) -> Result<Declaration> {
    Ok(Declaration {
        version: rcow_to_string(decl.version())?,
        encoding: orcow_to_string(decl.encoding())?,
        standalone: orcow_to_string(decl.standalone())?,
    })
}

fn parse_gpx_info(tag: &BytesStart<'_>) -> Result<GpxInfo> {
    let mut attributes = parse_attributes(tag)?;

    let creator = match attributes.entry("creator".to_string()) {
        Entry::Occupied(occupied_entry) => occupied_entry.remove(),
        _ => bail!("Mandatory attribute 'creator' was missing on the GPX element"),
    };

    let version = match attributes.entry("version".to_string()) {
        Entry::Occupied(occupied_entry) => occupied_entry.remove(),
        _ => bail!("Mandatory attribute 'version' was missing on the GPX element"),
    };

    Ok(GpxInfo {
        creator,
        version,
        attributes,
    })
}

fn parse_metadata<R: BufRead>(buf: &mut Vec<u8>, reader: &mut Reader<R>) -> Result<Metadata> {
    let mut href = None;
    let mut text = None;
    let mut mime_type = None;
    let mut time = None;
    let mut desc = None;

    loop {
        match reader.read_event_into(buf) {
            // TODO: We could break out a 'parse_link' function, as it is a defined
            // element type in the XSD.
            Ok(Event::Start(e)) => match e.name().as_ref() {
                b"link" => {
                    href = Some(read_attribute_as_string(&e, "href")?);
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
                b"desc" => {
                    desc = Some(read_inner_as_string(buf, reader)?);
                }
                e => bail!("Unexpected element {:?}", bytes_to_string(e)),
            },
            Ok(Event::End(e)) => match e.name().as_ref() {
                b"metadata" => {
                    if let Some(href) = href {
                        return Ok(Metadata {
                            link: Link {
                                href,
                                text,
                                r#type: mime_type,
                            },
                            time,
                            desc,
                        });
                    } else {
                        bail!("href attribute not found, but it is mandatory according to the XSD");
                    }
                }
                _ => {}
            },
            // Ignore spurious Event::Text, I think they are newlines.
            Ok(Event::Text(_)) => {}
            e => bail!("Unexpected element {:?}", e),
        }
    }
}

fn parse_track<R: BufRead>(buf: &mut Vec<u8>, reader: &mut Reader<R>) -> Result<Track> {
    let mut name = None;
    let mut track_type = None;
    let mut segments = Vec::new();
    let mut desc = None;

    loop {
        match reader.read_event_into(buf) {
            Ok(Event::Start(e)) => match e.name().as_ref() {
                b"name" => {
                    name = Some(read_inner_as_string(buf, reader)?);
                }
                b"type" => {
                    track_type = Some(read_inner_as_string(buf, reader)?);
                }
                b"desc" => {
                    desc = Some(read_inner_as_string(buf, reader)?);
                }
                b"trkseg" => {
                    segments.push(parse_track_segment(buf, reader)?);
                }
                e => bail!("Unexpected element {:?}", bytes_to_string(e)?),
            },
            Ok(Event::End(e)) => match e.name().as_ref() {
                b"trk" => {
                    return Ok(Track {
                        name,
                        r#type: track_type,
                        desc,
                        segments,
                    })
                }
                _ => {}
            },
            // Ignore spurious Event::Text, I think they are newlines.
            Ok(Event::Text(_)) => {}
            e => bail!("Unexpected element {:?}", e),
        }
    }
}

fn parse_track_segment<R: BufRead>(
    buf: &mut Vec<u8>,
    reader: &mut Reader<R>,
) -> Result<TrackSegment> {
    let mut points = Vec::new();

    while let Some(point) = parse_trackpoint(buf, reader)? {
        points.push(point);
    }

    Ok(TrackSegment { points })
}

fn parse_trackpoint<R: BufRead>(
    buf: &mut Vec<u8>,
    reader: &mut Reader<R>,
) -> Result<Option<TrackPoint>> {
    let mut lat = None;
    let mut lon = None;
    let mut ele = None;
    let mut time = None;
    let mut extensions = None;

    loop {
        match reader.read_event_into(buf) {
            Ok(Event::Start(e)) => match e.name().as_ref() {
                b"trkpt" => {
                    lat = Some(read_attribute_as_f64(&e, "lat")?);
                    lon = Some(read_attribute_as_f64(&e, "lon")?);
                }
                b"ele" => {
                    ele = Some(read_inner_as_f64(buf, reader)?);
                }
                b"time" => {
                    time = Some(read_inner_as_time(buf, reader)?);
                }
                b"extensions" => {
                    extensions = Some(parse_trackpoint_extensions(buf, reader)?);
                }
                e => bail!("Unexpected element {:?}", bytes_to_string(e)),
            },
            Ok(Event::End(e)) => match e.name().as_ref() {
                b"trkpt" => {
                    return Ok(Some(TrackPoint {
                        lat: lat.context("lat attribute not found")?,
                        lon: lon.context("lon attribute not found")?,
                        ele,
                        time,
                        extensions,
                    }))
                }
                b"trkseg" => {
                    // Reached the end of the trackpoints for this segment.
                    return Ok(None);
                }
                _ => {}
            },
            // Ignore spurious Event::Text, I think they are newlines.
            Ok(Event::Text(_)) => {}
            e => bail!("Unexpected element {:?}", e),
        }
    }
}

fn parse_trackpoint_extensions<R: BufRead>(
    buf: &mut Vec<u8>,
    reader: &mut Reader<R>,
) -> Result<Extensions> {
    let mut air_temp = None;
    let mut water_temp = None;
    let mut depth = None;
    let mut heart_rate = None;
    let mut cadence = None;

    loop {
        match reader.read_event_into(buf) {
            Ok(Event::Start(e)) => match e.local_name().as_ref() {
                b"TrackPointExtension" => { /* ignore, just a container element */ }
                b"atemp" => {
                    air_temp = Some(read_inner_as_f64(buf, reader)?);
                }
                b"wtemp" => {
                    water_temp = Some(read_inner_as_f64(buf, reader)?);
                }
                b"depth" => {
                    depth = Some(read_inner_as_f64(buf, reader)?);
                }
                b"hr" => {
                    heart_rate = Some(read_inner_as_u16(buf, reader)?);
                }
                b"cad" => {
                    cadence = Some(read_inner_as_u16(buf, reader)?);
                }
                e => bail!("Unexpected element {:?}", bytes_to_string(e)),
            },
            Ok(Event::End(e)) => match e.local_name().as_ref() {
                b"TrackPointExtension" => { /* ignore, just a container element */ }
                b"extensions" => {
                    return Ok(Extensions {
                        air_temp,
                        water_temp,
                        depth,
                        heart_rate,
                        cadence,
                    });
                }
                b"atemp" | b"wtemp" | b"depth" | b"hr" | b"cad" => { /* ignore, just the closing tags */
                }
                e => bail!("Unexpected element {:?}", bytes_to_string(e)),
            },
            // Ignore spurious Event::Text, I think they are newlines.
            Ok(Event::Text(_)) => {}
            e => bail!("Unexpected element {:?}", e),
        }
    }
}

fn parse_attributes(tag: &BytesStart<'_>) -> Result<HashMap<String, String>> {
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

fn read_attribute_as_string(tag: &BytesStart<'_>, attribute_name: &str) -> Result<String> {
    let attrs = parse_attributes(tag)?;
    match attrs.get(attribute_name) {
        Some(value) => Ok(value.clone()),
        None => bail!("No attribute named {attribute_name}"),
    }
}

fn read_attribute_as_f64(tag: &BytesStart<'_>, attribute_name: &str) -> Result<f64> {
    let s = read_attribute_as_string(tag, attribute_name)?;
    Ok(s.parse::<f64>()?)
}

/// Reads the 'INNER TEXT' from a tag such as <tag>INNER TEXT</tag>.
fn read_inner_as_string<R: BufRead>(buf: &mut Vec<u8>, reader: &mut Reader<R>) -> Result<String> {
    match reader.read_event_into(buf) {
        Ok(Event::Text(ele)) => Ok(bytes_to_string(ele.as_ref())?),
        e => bail!(
            "Got unexpected XML element {:?} (was expecting Event::Text), this is either a bug or the document is corrupt",
            e
        ),
    }
}

/// Reads a <time>2024-09-21T06:59:46.000Z</time> tag.
fn read_inner_as_time<R: BufRead>(
    buf: &mut Vec<u8>,
    reader: &mut Reader<R>,
) -> Result<OffsetDateTime> {
    let t = read_inner_as_string(buf, reader)?;
    Ok(OffsetDateTime::parse(&t, &well_known::Rfc3339)?)
}

/// Reads inner text and converts it to an f64.
fn read_inner_as_f64<R: BufRead>(buf: &mut Vec<u8>, reader: &mut Reader<R>) -> Result<f64> {
    let t = read_inner_as_string(buf, reader)?;
    Ok(t.parse::<f64>()?)
}

/// Reads inner text and converts it to a u16.
/// TODO: Make these methods generic.
fn read_inner_as_u16<R: BufRead>(buf: &mut Vec<u8>, reader: &mut Reader<R>) -> Result<u16> {
    let t = read_inner_as_string(buf, reader)?;
    Ok(t.parse::<u16>()?)
}

fn read_inner_as<R: BufRead, T: FromStr>(buf: &mut Vec<u8>, reader: &mut Reader<R>) -> Result<T> {
    let t = read_inner_as_string(buf, reader)?;
    match t.parse::<T>() {
        Ok(v) => Ok(v),
        Err(_) => bail!(
            "Could not parse {:?} into {}",
            &buf,
            std::any::type_name::<T>()
        ),
    }
}

fn cow_to_string(v: Cow<'_, [u8]>) -> Result<String> {
    bytes_to_string(v.borrow())
}

fn rcow_to_string(v: Result<Cow<'_, [u8]>, quick_xml::Error>) -> Result<String> {
    match v {
        Ok(Cow::Borrowed(s)) => Ok(bytes_to_string(s)?),
        Ok(Cow::Owned(s)) => Ok(bytes_to_string(&s)?),
        Err(err) => Err(err.into()),
    }
}

fn orcow_to_string(v: Option<Result<Cow<'_, [u8]>, quick_xml::Error>>) -> Result<Option<String>> {
    match v {
        Some(Ok(Cow::Borrowed(s))) => Ok(Some(bytes_to_string(s)?)),
        Some(Ok(Cow::Owned(s))) => Ok(Some(bytes_to_string(&s)?)),
        Some(Err(err)) => Err(err.into()),
        None => Ok(None),
    }
}

fn bytes_to_string(value: &[u8]) -> Result<String> {
    str::from_utf8(value)
        .and_then(|s| Ok(s.to_string()))
        .map_err(|e| e.into())
}
