use anyhow::{bail, Result};
use quick_xml::{events::Event, Reader};

use crate::model::Person;

use super::{
    attributes::Attributes, email::parse_email, link::parse_link, XmlReaderConversions,
    XmlReaderExtensions,
};

pub(crate) fn parse_person(xml_reader: &mut Reader<&[u8]>) -> Result<Person> {
    let mut person = Person::default();

    loop {
        match xml_reader.read_event() {
            Ok(Event::Start(e)) => match e.name().as_ref() {
                b"name" => {
                    person.name = Some(xml_reader.read_inner_as()?);
                }
                b"email" => {
                    person.email = Some(parse_email(&e, xml_reader)?);
                }
                b"link" => {
                    person.link = Some(parse_link(Attributes::new(&e, xml_reader)?, xml_reader)?);
                }
                e => bail!("Unexpected Start element {:?}", xml_reader.bytes_to_cow(e)),
            },
            Ok(Event::End(e)) => match e.name().as_ref() {
                b"person" => {
                    return Ok(person);
                }
                _ => {}
            },
            // Ignore spurious Event::Text, I think they are newlines.
            Ok(Event::Text(_)) => {}
            e => bail!("Unexpected element {:?}", e),
        }
    }
}
