#![allow(clippy::single_match)]

use core::str;
use std::{
    borrow::Cow,
    collections::HashMap,
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
    str::FromStr,
};

use anyhow::{bail, Context, Result};
use declaration::parse_declaration;
use gpx::{parse_gpx, parse_gpx_attributes};
use log::info;
use logging_timer::time;
use metadata::parse_metadata;
use quick_xml::{
    events::{BytesStart, Event},
    Reader,
};
use time::{format_description::well_known, OffsetDateTime};
use track::parse_track;

use crate::model::{Gpx, Track, XmlDeclaration};

mod declaration;
mod gpx;
mod metadata;
mod track;
mod track_segment;
mod trackpoint_extensions;
mod waypoint;

/*
<xml>                                                  parse_declaration
<gpx>                          type="gpxType"          parse_gpx
   <metadata>                  type="metadataType"     parse_metadata
   <wpt>                       type="wptType"          n.a.
   <rte>                       type="rteType"          n.a.
   <extensions>                type="extensionsType"   n.a.
   <trk>                       type="trkType"          parse_track
       <trkseg>                type="trksegType"       parse_track_segment
           <trkpt>             type="wptType"          parse_waypoint
               <extensions>    type="extensions"       parse_trackpoint_extensions

*/

/// The XSD, which defines the format of a GPX file, is at https://www.topografix.com/GPX/1/1/gpx.xsd
/// This function doesn't parse everything, just the things that appear in my Garmin files.
pub fn read_gpx_from_file<P: AsRef<Path>>(input_file: P) -> Result<Gpx> {
    let input_file = input_file.as_ref();
    info!("Reading GPX file {:?}", input_file);
    let f = File::open(input_file)?;

    // Q: Is it quicker to just read everything into a String first?
    // A: About 5-10% quicker. Not enough to justify the memory load, I think.
    // let mut s = String::new();
    // f.read_to_string(&mut s)?;
    // let c = Cursor::new(s);
    // let mut gpx = read_gpx_from_reader(c)?;

    let buf_reader = BufReader::new(f);
    let mut gpx = read_gpx_from_reader(buf_reader)?;
    gpx.filename = Some(input_file.to_owned());
    return Ok(gpx);
}

#[time]
pub fn read_gpx_from_reader<R: BufRead>(input: R) -> Result<Gpx> {
    let mut xml_reader = Reader::from_reader(input);
    let mut buf: Vec<u8> = Vec::with_capacity(512);
    let mut xml_declaration: Option<XmlDeclaration> = None;
    let mut gpx: Option<Gpx> = None;

    loop {
        match xml_reader.read_event_into(&mut buf) {
            Ok(Event::Decl(decl)) => {
                xml_declaration = Some(parse_declaration(&decl)?);
            }
            Ok(Event::Start(e)) => match e.name().as_ref() {
                b"gpx" => {
                    let attrs = parse_gpx_attributes(&e)?;
                    let mut partial_gpx = parse_gpx(&mut buf, &mut xml_reader)?;
                    partial_gpx.creator = attrs.creator;
                    partial_gpx.version = attrs.version;
                    partial_gpx.attributes = attrs.other_attributes;
                    gpx = Some(partial_gpx);
                }
                e => bail!("Unexpected opening element {:?}", bytes_to_string(e)),
            },
            Ok(Event::Eof) => {
                // We should already have consumed the closing '<gpx>' tag in parse_gpx().
                // So the next thing will be EOF.
                let mut gpx = gpx.context("Did not find the 'gpx' element")?;
                gpx.declaration =
                    xml_declaration.context("Did not find the 'xml' declaration element")?;
                return Ok(gpx);
            }
            Err(e) => bail!("Error at position {}: {:?}", xml_reader.error_position(), e),
            _ => (),
        }

        buf.clear();
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

/// Reads the inner text, e.g. in a '<time>2024-09-21T06:59:46.000Z</time>' tag
/// and converts it into a time.
fn read_inner_as_time<R: BufRead>(
    buf: &mut Vec<u8>,
    reader: &mut Reader<R>,
) -> Result<OffsetDateTime> {
    let t = read_inner_as_string(buf, reader)?;
    Ok(OffsetDateTime::parse(&t, &well_known::Rfc3339)?)
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

/// Converts a byte slice to a String.
fn bytes_to_string(value: &[u8]) -> Result<String> {
    str::from_utf8(value)
        .and_then(|s| Ok(s.to_string()))
        .map_err(|e| e.into())
}

/// Converts a Cow<u8> to a String in the most efficient manner possible.
fn cow_to_string(v: Cow<'_, [u8]>) -> Result<String> {
    match v {
        Cow::Borrowed(s) => Ok(bytes_to_string(s)?),
        Cow::Owned(s) => Ok(String::from_utf8(s)?),
    }
}
