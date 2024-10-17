use anyhow::{bail, Result};
use quick_xml::events::BytesStart;

use crate::model::Email;

use super::{attributes::Attributes, XmlReaderConversions};

pub(crate) fn parse_email<C: XmlReaderConversions>(tag: &BytesStart<'_>, converter: &C) -> Result<Email> {
    let mut attributes = Attributes::new(tag, converter)?;

    let id: String = attributes.get("id")?;
    let domain: String = attributes.get("domain")?;

    if !attributes.is_empty() {
        bail!("Found extra attributes on 'email' element");
    }

    Ok(Email::new(id, domain))
}
