use anyhow::{bail, Result};
use quick_xml::{events::BytesStart, Reader};

use crate::model::Email;

use super::attributes::Attributes;

pub(crate) fn parse_email<R>(tag: &BytesStart<'_>, xml_reader: &Reader<R>) -> Result<Email> {
    let mut attributes = Attributes::new(tag, xml_reader)?;

    let id: String = attributes.get("id")?;
    let domain: String = attributes.get("domain")?;

    if !attributes.is_empty() {
        bail!("Found extra attributes on 'email' element");
    }

    Ok(Email::new(id, domain))
}
