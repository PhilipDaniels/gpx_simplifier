use anyhow::Result;
use quick_xml::events::BytesDecl;

use crate::model::XmlDeclaration;

use super::XmlReaderConversions;

/// Parses an XML declaration, i.e. the very first line of the file which is:
///     <?xml version="1.0" encoding="UTF-8"?>
pub(crate) fn parse_declaration<C: XmlReaderConversions>(
    decl: &BytesDecl<'_>,
    converter: &C,
) -> Result<XmlDeclaration> {
    let version = converter.cow_to_string(decl.version()?)?;

    let encoding = if let Some(enc) = decl.encoding() {
        let enc = enc?;
        Some(converter.cow_to_string(enc)?)
    } else {
        None
    };

    let standalone = if let Some(sa) = decl.standalone() {
        let sa = sa?;
        Some(converter.cow_to_string(sa)?)
    } else {
        None
    };

    Ok(XmlDeclaration {
        version,
        encoding,
        standalone,
    })
}
