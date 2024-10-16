use anyhow::{bail, Result};
use quick_xml::{
    events::{BytesStart, Event},
    Reader,
};

use crate::model::Link;

use super::{attributes::Attributes, XmlReaderConversions, XmlReaderExtensions};

pub(crate) fn parse_link(
    start_element: &BytesStart<'_>,
    xml_reader: &mut Reader<&[u8]>,
) -> Result<Link> {
    let mut attributes = Attributes::new(start_element, xml_reader)?;
    let mut link = Link::default();
    link.href = attributes.get("href")?;
    if !attributes.is_empty() {
        bail!("Found extra attributes on 'link' element");
    }

    loop {
        match xml_reader.read_event() {
            Ok(Event::Start(start)) => match start.name().as_ref() {
                b"text" => {
                    link.text = Some(xml_reader.read_inner_as()?);
                }
                b"type" => {
                    link.r#type = Some(xml_reader.read_inner_as()?);
                }
                e => bail!("Unexpected Start element {:?}", xml_reader.bytes_to_cow(e)),
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
