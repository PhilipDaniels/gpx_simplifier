use std::{
    collections::{hash_map::Entry, HashMap},
    io::BufRead,
};

use anyhow::{bail, Result};
use quick_xml::{
    events::{BytesStart, Event},
    Reader,
};

use crate::model::Gpx;

use super::{metadata::parse_metadata, parse_attributes, track::parse_track};

pub(crate) struct GpxAttributes {
    pub(crate) creator: String,
    pub(crate) version: String,
    pub(crate) other_attributes: HashMap<String, String>,
}

/// Parses the attributes on 'gpx' element itself. Gets around a multiple mut borrows
/// problem in the main read_gpx_from_reader() function.
pub(crate) fn parse_gpx_attributes(tag: &BytesStart<'_>) -> Result<GpxAttributes> {
    let mut attributes = parse_attributes(&tag)?;

    let creator = match attributes.entry("creator".to_string()) {
        Entry::Occupied(occupied_entry) => occupied_entry.remove(),
        _ => bail!("Mandatory attribute 'creator' was missing on the GPX element"),
    };

    let version = match attributes.entry("version".to_string()) {
        Entry::Occupied(occupied_entry) => occupied_entry.remove(),
        _ => bail!("Mandatory attribute 'version' was missing on the GPX element"),
    };

    Ok(GpxAttributes {
        creator,
        version,
        other_attributes: attributes,
    })
}

/// Parses the 'gpx' element itself.
pub(crate) fn parse_gpx<R: BufRead>(
    mut buf: &mut Vec<u8>,
    xml_reader: &mut Reader<R>,
) -> Result<Gpx> {
    let mut gpx = Gpx::default();

    loop {
        match xml_reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => match e.name().as_ref() {
                b"metadata" => {
                    gpx.metadata = parse_metadata(buf, xml_reader)?;
                }
                b"trk" => {
                    let track = parse_track(buf, xml_reader)?;
                    gpx.tracks.push(track);
                }
                _ => (),
            },
            Ok(Event::End(e)) => match e.name().as_ref() {
                b"gpx" => {
                    return Ok(gpx);
                }
                _ => (),
            },
            Ok(Event::Eof) => {
                bail!("Reached EOF unexpectedly. File is probably corrupt.");
            }
            Err(e) => bail!("Error at position {}: {:?}", xml_reader.error_position(), e),
            _ => (),
        }
    }
}
