use std::io::BufRead;

use anyhow::{bail, Result};
use quick_xml::{events::Event, Reader};

use crate::model::Metadata;

use super::{
    attributes::Attributes, bounds::parse_bounds, bytes_to_string, copyright::parse_copyright,
    extensions::parse_extensions, link::parse_link, person::parse_person,
    read_inner_as_string, read_inner_as_time,
};

pub(crate) fn parse_metadata<R: BufRead>(
    buf: &mut Vec<u8>,
    xml_reader: &mut Reader<R>,
) -> Result<Metadata> {
    let mut md = Metadata::default();

    loop {
        match xml_reader.read_event_into(buf) {
            Ok(Event::Start(e)) => match e.name().as_ref() {
                b"name" => {
                    md.name = Some(read_inner_as_string(buf, xml_reader)?);
                }
                b"desc" => {
                    md.description = Some(read_inner_as_string(buf, xml_reader)?);
                }
                b"author" => {
                    md.author = Some(parse_person(buf, xml_reader)?);
                }
                b"copyright" => {
                    md.copyright = Some(parse_copyright(buf, xml_reader)?);
                }
                b"link" => {
                    let link = parse_link(Attributes::new(&e)?, buf, xml_reader)?;
                    md.links.push(link);
                }
                b"time" => {
                    md.time = Some(read_inner_as_time(buf, xml_reader)?);
                }
                b"keywords" => {
                    md.keywords = Some(read_inner_as_string(buf, xml_reader)?);
                }
                b"bounds" => {
                    md.bounds = Some(parse_bounds(&e)?);
                }
                b"extensions" => {
                    md.extensions = Some(parse_extensions(buf, xml_reader)?);
                }
                e => bail!("Unexpected element {:?}", bytes_to_string(e)),
            },
            Ok(Event::End(e)) => match e.name().as_ref() {
                b"metadata" => {
                    return Ok(md);
                }
                _ => {}
            },
            // Ignore spurious Event::Text, I think they are newlines.
            Ok(Event::Text(_)) => {}
            e => bail!("Unexpected element {:?}", e),
        }
    }
}
