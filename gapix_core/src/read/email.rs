use anyhow::{bail, Result};
use quick_xml::events::BytesStart;

use crate::model::Email;

use super::attributes::Attributes;

pub(crate) fn parse_email(tag: &BytesStart<'_>) -> Result<Email> {
    let mut attributes = Attributes::new(tag)?;

    let id: String = attributes.get("id")?;
    let domain: String = attributes.get("domain")?;

    if !attributes.is_empty() {
        bail!("Found extra attributes on 'email' element");
    }

    Ok(Email::new(id, domain))
}
