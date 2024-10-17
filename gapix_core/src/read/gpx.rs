use std::collections::HashMap;

use anyhow::{bail, Result};
use quick_xml::{
    events::{BytesStart, Event},
    Reader,
};

use crate::model::Gpx;

use super::{
    attributes::Attributes, extensions::parse_extensions, metadata::parse_metadata,
    route::parse_route, track::parse_track, waypoint::parse_waypoint,
};

pub(crate) struct GpxAttributes {
    pub(crate) creator: String,
    pub(crate) version: String,
    pub(crate) other_attributes: HashMap<String, String>,
}

/// Parses the attributes on 'gpx' element itself. Gets around a multiple mut borrows
/// problem in the main read_gpx_from_reader() function.
pub(crate) fn parse_gpx_attributes(tag: &BytesStart<'_>) -> Result<GpxAttributes> {
    let mut attributes = Attributes::new(tag)?;

    let creator: String = attributes.get("creator")?;
    let version: String = attributes.get("version")?;

    Ok(GpxAttributes {
        creator,
        version,
        other_attributes: attributes.into_inner(),
    })
}

/// Parses the 'gpx' element itself.
pub(crate) fn parse_gpx(xml_reader: &mut Reader<&[u8]>) -> Result<Gpx> {
    let mut gpx = Gpx::default();

    loop {
        match xml_reader.read_event() {
            Ok(Event::Start(e)) => match e.name().as_ref() {
                b"metadata" => {
                    gpx.metadata = parse_metadata(xml_reader)?;
                }
                b"wpt" => {
                    let waypoint = parse_waypoint(Attributes::new(&e)?, xml_reader, b"wpt")?;
                    gpx.waypoints.push(waypoint);
                }
                b"rte" => {
                    let route = parse_route(xml_reader)?;
                    gpx.routes.push(route);
                }
                b"trk" => {
                    let track = parse_track(xml_reader)?;
                    gpx.tracks.push(track);
                }
                b"extensions" => {
                    gpx.extensions = Some(parse_extensions(xml_reader)?);
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
