use anyhow::Result;
use quick_xml::events::BytesDecl;

use crate::model::XmlDeclaration;

use super::cow_to_string;

/// Parses an XML declaration, i.e. the very first line of the file which is:
///     <?xml version="1.0" encoding="UTF-8"?>
pub(crate) fn parse_declaration(decl: &BytesDecl<'_>) -> Result<XmlDeclaration> {
    let version = cow_to_string(decl.version()?)?;

    let encoding = if let Some(enc) = decl.encoding() {
        let enc = enc?;
        Some(cow_to_string(enc)?)
    } else {
        None
    };

    let standalone = if let Some(sa) = decl.standalone() {
        let sa = sa?;
        Some(cow_to_string(sa)?)
    } else {
        None
    };

    Ok(XmlDeclaration {
        version,
        encoding,
        standalone,
    })
}

// fn rcow_to_string(v: Result<Cow<'_, [u8]>, quick_xml::Error>) -> Result<String> {
//     match v {
//         Ok(Cow::Borrowed(s)) => Ok(bytes_to_string(s)?),
//         Ok(Cow::Owned(s)) => Ok(String::from_utf8(s)?),
//         Err(err) => Err(err.into()),
//     }
// }

// fn orcow_to_string(v: Option<Result<Cow<'_, [u8]>, quick_xml::Error>>) -> Result<Option<String>> {
//     match v {
//         Some(Ok(Cow::Borrowed(s))) => Ok(Some(bytes_to_string(s)?)),
//         Some(Ok(Cow::Owned(s))) => Ok(Some(String::from_utf8(s)?)),
//         Some(Err(err)) => Err(err.into()),
//         None => Ok(None),
//     }
// }
