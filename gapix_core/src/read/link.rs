use std::io::BufRead;

use anyhow::{bail, Result};
use quick_xml::{events::Event, Reader};

use crate::model::Link;

use super::{attributes::Attributes, bytes_to_string, read_inner_as_string};

pub(crate) fn parse_link<R: BufRead>(mut attributes: Attributes, buf: &mut Vec<u8>, xml_reader: &mut Reader<R>) -> Result<Link> {
    let mut link = Link::default();
    link.href = attributes.get("href")?;
    if !attributes.is_empty() {
        bail!("Found extra attributes on 'link' element");
    }

    loop {
        match xml_reader.read_event_into(buf) {
            Ok(Event::Start(e)) => match e.name().as_ref() {
                b"text" => {
                    link.text = Some(read_inner_as_string(buf, xml_reader)?);
                }
                b"type" => {
                    link.r#type = Some(read_inner_as_string(buf, xml_reader)?);
                }
                e => bail!("Unexpected element {:?}", bytes_to_string(e)),
            },
            Ok(Event::End(e)) => match e.name().as_ref() {
                b"link" => {
                    return Ok(link);
                }
                _ => {}
            },
            // Ignore spurious Event::Text, I think they are newlines.
            Ok(Event::Text(_)) => {}
            e => bail!("Unexpected element {:?}", e),
        }
    }
}
