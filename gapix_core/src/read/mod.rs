#![allow(clippy::single_match)]

use core::str;
use std::{borrow::Cow, io::BufRead, path::Path, str::FromStr};

use anyhow::{bail, Context, Result};
use declaration::parse_declaration;
use gpx::{parse_gpx, parse_gpx_attributes};
use log::info;
use logging_timer::time;
use quick_xml::{events::Event, Reader};
use time::{format_description::well_known, OffsetDateTime};

use crate::model::{Gpx, XmlDeclaration};

mod attributes;
mod bounds;
mod copyright;
mod declaration;
mod email;
mod extensions;
mod gpx;
mod link;
mod metadata;
mod person;
mod route;
mod track;
mod track_segment;
mod trackpoint_extensions;
mod waypoint;

/// The XSD, which defines the format of a GPX file, is at https://www.topografix.com/GPX/1/1/gpx.xsd
#[time]
pub fn read_gpx_from_file<P: AsRef<Path>>(input_file: P) -> Result<Gpx> {
    let input_file = input_file.as_ref();
    info!("Reading GPX file {:?}", input_file);
    let contents = std::fs::read(input_file)?;
    let mut gpx = read_gpx_from_slice(&contents)?;
    gpx.filename = Some(input_file.to_owned());
    Ok(gpx)
}

pub fn read_gpx_from_slice(data: &[u8]) -> Result<Gpx> {
    let xml_reader = Reader::from_reader(data);
    read_gpx_from_reader(xml_reader)
}

#[time]
pub fn read_gpx_from_reader(mut xml_reader: Reader<&[u8]>) -> Result<Gpx> {
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

// TODO: Use reader.decoder().decode(...)

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
