use std::io::BufRead;

use anyhow::{bail, Result};
use quick_xml::{events::Event, Reader};

use crate::model::Route;

use super::{
    attributes::Attributes, bytes_to_string, extensions::parse_extensions, link::parse_link,
    read_inner_as, read_inner_as_string, waypoint::parse_waypoint,
};

pub(crate) fn parse_route<R: BufRead>(
    buf: &mut Vec<u8>,
    xml_reader: &mut Reader<R>,
) -> Result<Route> {
    let mut route = Route::default();

    loop {
        match xml_reader.read_event_into(buf) {
            Ok(Event::Start(e)) => match e.name().as_ref() {
                b"name" => {
                    route.name = Some(read_inner_as_string(buf, xml_reader)?);
                }
                b"cmt" => {
                    route.comment = Some(read_inner_as_string(buf, xml_reader)?);
                }
                b"desc" => {
                    route.description = Some(read_inner_as_string(buf, xml_reader)?);
                }
                b"src" => {
                    route.source = Some(read_inner_as_string(buf, xml_reader)?);
                }
                b"link" => {
                    let link = parse_link(Attributes::new(&e)?, buf, xml_reader)?;
                    route.links.push(link);
                }
                b"number" => {
                    route.number = Some(read_inner_as(buf, xml_reader)?);
                }
                b"type" => {
                    route.r#type = Some(read_inner_as_string(buf, xml_reader)?);
                }
                b"extensions" => {
                    route.extensions = Some(parse_extensions(buf, xml_reader)?);
                }
                b"rtept" => {
                    let point = parse_waypoint(Attributes::new(&e)?, buf, xml_reader, b"rtept")?;
                    route.points.push(point);
                }
                e => bail!("Unexpected element {:?}", bytes_to_string(e)?),
            },
            Ok(Event::End(e)) => match e.name().as_ref() {
                b"rte" => {
                    return Ok(route);
                }
                _ => {}
            },
            // Ignore spurious Event::Text, I think they are newlines.
            Ok(Event::Text(_)) => {}
            e => bail!("Unexpected element {:?}", e),
        }
    }
}
