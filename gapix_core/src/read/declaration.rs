use anyhow::Result;
use quick_xml::{events::BytesDecl, Reader};

use crate::model::XmlDeclaration;

use super::XmlReaderConversions;

/// Parses an XML declaration, i.e. the very first line of the file which is:
///     <?xml version="1.0" encoding="UTF-8"?>
pub(crate) fn parse_declaration(
    decl: &BytesDecl<'_>,
    xml_reader: &Reader<&[u8]>,
) -> Result<XmlDeclaration> {
    let version = xml_reader.cow_to_string(decl.version()?)?;

    let encoding = if let Some(enc) = decl.encoding() {
        let enc = enc?;
        Some(xml_reader.cow_to_string(enc)?)
    } else {
        None
    };

    let standalone = if let Some(sa) = decl.standalone() {
        let sa = sa?;
        Some(xml_reader.cow_to_string(sa)?)
    } else {
        None
    };

    Ok(XmlDeclaration {
        version,
        encoding,
        standalone,
    })
}
