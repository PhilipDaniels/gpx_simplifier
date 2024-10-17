use anyhow::Result;
use quick_xml::{events::BytesStart, Reader};

use crate::model::Extensions;

pub(crate) fn parse_extensions(xml_reader: &mut Reader<&[u8]>) -> Result<Extensions> {
    let start = BytesStart::new("extensions");
    let end = start.to_end();
    let text = xml_reader.read_text(end.name())?;
    let ext = Extensions::new(text.trim());
    Ok(ext)
}
