use anyhow::{bail, Result};
use quick_xml::events::BytesStart;

use crate::model::Bounds;

use super::attributes::Attributes;

pub(crate) fn parse_bounds(tag: &BytesStart<'_>) -> Result<Bounds> {
    let mut attributes = Attributes::new(tag)?;
    let mut bounds = Bounds::default();
    bounds.min_lat = attributes.get("minlat")?;
    bounds.min_lon = attributes.get("minlon")?;
    bounds.max_lat = attributes.get("maxlat")?;
    bounds.max_lon = attributes.get("maxlon")?;

    if !attributes.is_empty() {
        bail!("Found extra attributes on 'bounds' element");
    }

    Ok(bounds)
}
