use anyhow::Result;
use quick_xml::{events::BytesStart, Reader};

use crate::model::Extensions;

use super::check_no_attributes;

pub(crate) fn parse_extensions(
    start_element: &BytesStart<'_>,
    xml_reader: &mut Reader<&[u8]>,
) -> Result<Extensions> {
    check_no_attributes(start_element, xml_reader)?;

    let start = BytesStart::new("extensions");
    let end = start.to_end();
    let text = xml_reader.read_text(end.name())?;
    let ext = Extensions::new(text.trim());
    Ok(ext)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::read::start_parse;
    use quick_xml::Reader;

    #[test]
    fn valid_empty() {
        let mut xml_reader = Reader::from_str(
            r#"<extensions>  
               </extensions>"#,
        );

        let start = start_parse(&mut xml_reader).unwrap();
        let result = parse_extensions(&start, &mut xml_reader).unwrap();
        assert!(result.raw_xml.is_empty());
    }

    #[test]
    fn valid_extensions() {
        let mut xml_reader = Reader::from_str(
            r#"<extensions>  
                  <foo bar="42">inner text</foo>
               </extensions>"#,
        );

        let start = start_parse(&mut xml_reader).unwrap();
        let result = parse_extensions(&start, &mut xml_reader).unwrap();
        assert_eq!(result.raw_xml, r#"<foo bar="42">inner text</foo>"#);
    }
}
