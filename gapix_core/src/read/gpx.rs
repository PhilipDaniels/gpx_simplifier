use std::collections::{hash_map::Entry, HashMap};

use anyhow::{bail, Result};
use quick_xml::events::BytesStart;

use super::parse_attributes;

pub(crate) struct GpxTag {
    pub(crate) creator: String,
    pub(crate) version: String,
    pub(crate) attributes: HashMap<String, String>,
}

/// Parses the 'gpx' element itself.
pub(crate) fn parse_gpx(tag: &BytesStart<'_>) -> Result<GpxTag> {
    let mut attributes = parse_attributes(tag)?;

    let creator = match attributes.entry("creator".to_string()) {
        Entry::Occupied(occupied_entry) => occupied_entry.remove(),
        _ => bail!("Mandatory attribute 'creator' was missing on the GPX element"),
    };

    let version = match attributes.entry("version".to_string()) {
        Entry::Occupied(occupied_entry) => occupied_entry.remove(),
        _ => bail!("Mandatory attribute 'version' was missing on the GPX element"),
    };

    Ok(GpxTag {
        creator,
        version,
        attributes,
    })
}
