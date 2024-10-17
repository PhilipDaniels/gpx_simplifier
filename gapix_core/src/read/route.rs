use anyhow::{bail, Result};
use quick_xml::{events::Event, Reader};

use crate::model::Route;

use super::{
    extensions::parse_extensions, link::parse_link, waypoint::parse_waypoint, XmlReaderConversions,
    XmlReaderExtensions,
};

pub(crate) fn parse_route(xml_reader: &mut Reader<&[u8]>) -> Result<Route> {
    let mut route = Route::default();

    loop {
        match xml_reader.read_event() {
            Ok(Event::Start(e)) => match e.name().as_ref() {
                b"name" => {
                    route.name = Some(xml_reader.read_inner_as()?);
                }
                b"cmt" => {
                    route.comment = Some(xml_reader.read_inner_as()?);
                }
                b"desc" => {
                    route.description = Some(xml_reader.read_inner_as()?);
                }
                b"src" => {
                    route.source = Some(xml_reader.read_inner_as()?);
                }
                b"link" => {
                    let link = parse_link(&e, xml_reader)?;
                    route.links.push(link);
                }
                b"number" => {
                    route.number = Some(xml_reader.read_inner_as()?);
                }
                b"type" => {
                    route.r#type = Some(xml_reader.read_inner_as()?);
                }
                b"extensions" => {
                    route.extensions = Some(parse_extensions(xml_reader)?);
                }
                b"rtept" => {
                    let point = parse_waypoint(&e, xml_reader, b"rtept")?;
                    route.points.push(point);
                }
                e => bail!("Unexpected Start element {:?}", xml_reader.bytes_to_cow(e)),
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
