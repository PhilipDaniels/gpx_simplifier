use anyhow::{bail, Result};
use quick_xml::{events::Event, Reader};

use crate::model::Copyright;

use super::{bytes_to_string, XmlReaderExtensions};

pub(crate) fn parse_copyright(xml_reader: &mut Reader<&[u8]>) -> Result<Copyright> {
    let mut copyright = Copyright::default();

    loop {
        match xml_reader.read_event() {
            Ok(Event::Start(e)) => match e.name().as_ref() {
                b"year" => {
                    copyright.year = Some(xml_reader.read_inner_as()?);
                }
                b"license" => {
                    copyright.license = Some(xml_reader.read_inner_as()?);
                }
                b"author" => {
                    copyright.author = xml_reader.read_inner_as()?;
                }
                e => bail!("Unexpected element {:?}", bytes_to_string(e)),
            },
            Ok(Event::End(e)) => match e.name().as_ref() {
                b"copyright" => {
                    if copyright.author.is_empty() {
                        bail!("Did not find the 'author' element");
                    }

                    return Ok(copyright);
                }
                _ => {}
            },
            // Ignore spurious Event::Text, I think they are newlines.
            Ok(Event::Text(_)) => {}
            e => bail!("Unexpected element {:?}", e),
        }
    }
}
