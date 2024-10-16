use std::io::BufRead;

use anyhow::{bail, Result};
use quick_xml::{events::Event, Reader};

use crate::model::{Link, Metadata};

use super::{bytes_to_string, read_attribute_as_string, read_inner_as_string, read_inner_as_time};

pub(crate) fn parse_metadata<R: BufRead>(
    buf: &mut Vec<u8>,
    reader: &mut Reader<R>,
) -> Result<Metadata> {
    let mut href = None;
    let mut text = None;
    let mut mime_type = None;
    let mut time = None;
    let mut desc = None;

    loop {
        match reader.read_event_into(buf) {
            // TODO: We could break out a 'parse_link' function, as it is a defined
            // element type in the XSD.
            Ok(Event::Start(e)) => match e.name().as_ref() {
                b"link" => {
                    href = Some(read_attribute_as_string(&e, "href")?);
                }
                b"text" => {
                    text = Some(read_inner_as_string(buf, reader)?);
                }
                b"type" => {
                    mime_type = Some(read_inner_as_string(buf, reader)?);
                }
                b"time" => {
                    time = Some(read_inner_as_time(buf, reader)?);
                }
                b"desc" => {
                    desc = Some(read_inner_as_string(buf, reader)?);
                }
                e => bail!("Unexpected element {:?}", bytes_to_string(e)),
            },
            Ok(Event::End(e)) => match e.name().as_ref() {
                b"metadata" => {
                    if let Some(href) = href {
                        let mut link = Link::new(href);
                        link.text = text;
                        link.r#type = mime_type;
                        let mut md = Metadata::default();
                        md.links.push(link);
                        md.time = time;
                        md.description = desc;
                        return Ok(md);
                    } else {
                        bail!("href attribute not found, but it is mandatory according to the XSD");
                    }
                }
                _ => {}
            },
            // Ignore spurious Event::Text, I think they are newlines.
            Ok(Event::Text(_)) => {}
            e => bail!("Unexpected element {:?}", e),
        }
    }
}
