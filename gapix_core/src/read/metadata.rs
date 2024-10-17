use anyhow::{bail, Result};
use quick_xml::{events::Event, Reader};

use crate::model::Metadata;

use super::{
    bounds::parse_bounds, copyright::parse_copyright, extensions::parse_extensions,
    link::parse_link, person::parse_person, XmlReaderConversions, XmlReaderExtensions,
};

pub(crate) fn parse_metadata(xml_reader: &mut Reader<&[u8]>) -> Result<Metadata> {
    let mut md = Metadata::default();

    loop {
        match xml_reader.read_event() {
            Ok(Event::Start(e)) => match e.name().as_ref() {
                b"name" => {
                    md.name = Some(xml_reader.read_inner_as()?);
                }
                b"desc" => {
                    md.description = Some(xml_reader.read_inner_as()?);
                }
                b"author" => {
                    md.author = Some(parse_person(xml_reader)?);
                }
                b"copyright" => {
                    md.copyright = Some(parse_copyright(xml_reader)?);
                }
                b"link" => {
                    let link = parse_link(&e, xml_reader)?;
                    md.links.push(link);
                }
                b"time" => {
                    md.time = Some(xml_reader.read_inner_as_time()?);
                }
                b"keywords" => {
                    md.keywords = Some(xml_reader.read_inner_as()?);
                }
                b"bounds" => {
                    md.bounds = Some(parse_bounds(&e, xml_reader)?);
                }
                b"extensions" => {
                    md.extensions = Some(parse_extensions(xml_reader)?);
                }
                e => bail!("Unexpected Start element {:?}", xml_reader.bytes_to_cow(e)),
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
