#![allow(clippy::single_match)]

use core::str;
use std::{borrow::Cow, path::Path, str::FromStr};

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
    let mut xml_declaration: Option<XmlDeclaration> = None;
    let mut gpx: Option<Gpx> = None;

    loop {
        match xml_reader.read_event() {
            Ok(Event::Decl(decl)) => {
                xml_declaration = Some(parse_declaration(&decl, &xml_reader)?);
            }
            Ok(Event::Start(e)) => match e.name().as_ref() {
                b"gpx" => {
                    let attrs = parse_gpx_attributes(&e, &xml_reader)?;
                    let mut partial_gpx = parse_gpx(&mut xml_reader)?;
                    partial_gpx.creator = attrs.creator;
                    partial_gpx.version = attrs.version;
                    partial_gpx.attributes = attrs.other_attributes;
                    gpx = Some(partial_gpx);
                }
                e => bail!("Unexpected Start element {:?}", xml_reader.bytes_to_cow(e)),
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
    }
}

pub(crate) trait XmlReaderConversions {
    fn bytes_to_cow<'a, 'b>(&'a self, bytes: &'b [u8]) -> Result<Cow<'b, str>>;
    fn bytes_to_string(&self, bytes: &[u8]) -> Result<String>;
    fn cow_to_string(&self, bytes: Cow<'_, [u8]>) -> Result<String>;
}

impl<R> XmlReaderConversions for Reader<R> {
    #[inline]
    fn bytes_to_cow<'a, 'b>(&'a self, bytes: &'b [u8]) -> Result<Cow<'b, str>> {
        Ok(self.decoder().decode(bytes)?)
    }

    #[inline]
    fn bytes_to_string(&self, bytes: &[u8]) -> Result<String> {
        // Ensure everything goes through decode().
        Ok(self.bytes_to_cow(bytes)?.into())
    }

    #[inline]
    fn cow_to_string(&self, bytes: Cow<'_, [u8]>) -> Result<String> {
        match bytes {
            // Ensure everything goes through decode().
            Cow::Borrowed(slice) => Ok(self.bytes_to_string(slice)?),
            Cow::Owned(vec) => Ok(self.bytes_to_string(&vec)?)
        }
    }
}

pub(crate) trait XmlReaderExtensions {
    fn read_inner_as_string(&mut self) -> Result<String>;
    fn read_inner_as_time(&mut self) -> Result<OffsetDateTime>;
    fn read_inner_as<T: FromStr>(&mut self) -> Result<T>;
}

impl XmlReaderExtensions for Reader<&[u8]> {
    #[inline]
    fn read_inner_as_string(&mut self) -> Result<String> {
        match self.read_event() {
            Ok(Event::Text(text)) => {
                Ok(self.bytes_to_string(&text)?)
            }
            e => bail!(
                "Got unexpected XML element {:?} (was expecting Event::Text), this is either a bug or the document is corrupt",
                e
            ),
        }
    }

    #[inline]
    fn read_inner_as_time(&mut self) -> Result<OffsetDateTime> {
        let t = self.read_inner_as_string()?;
        Ok(OffsetDateTime::parse(&t, &well_known::Rfc3339)?)
    }

    #[inline]
    fn read_inner_as<T: FromStr>(&mut self) -> Result<T> {
        let t = self.read_inner_as_string()?;

        match t.parse::<T>() {
            Ok(v) => Ok(v),
            Err(_) => bail!("Could not parse {} into {}", t, std::any::type_name::<T>()),
        }
    }
}
