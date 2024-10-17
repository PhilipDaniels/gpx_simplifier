use anyhow::{bail, Result};
use quick_xml::{events::Event, Reader};

use crate::model::Copyright;

use super::{bytes_to_string, read_inner_as, read_inner_as_string};

pub(crate) fn parse_copyright(
    buf: &mut Vec<u8>,
    xml_reader: &mut Reader<&[u8]>,
) -> Result<Copyright> {
    let mut copyright = Copyright::default();

    loop {
        match xml_reader.read_event_into(buf) {
            Ok(Event::Start(e)) => match e.name().as_ref() {
                b"year" => {
                    copyright.year = Some(read_inner_as(buf, xml_reader)?);
                }
                b"license" => {
                    copyright.license = Some(read_inner_as_string(buf, xml_reader)?);
                }
                b"author" => {
                    copyright.author = read_inner_as_string(buf, xml_reader)?;
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
