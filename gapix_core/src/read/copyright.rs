use anyhow::{bail, Result};
use quick_xml::{
    events::{BytesStart, Event},
    Reader,
};

use crate::model::Copyright;

use super::{check_no_attributes, XmlReaderConversions, XmlReaderExtensions};

pub(crate) fn parse_copyright(
    start_element: &BytesStart<'_>,
    xml_reader: &mut Reader<&[u8]>,
) -> Result<Copyright> {
    check_no_attributes(&start_element, xml_reader)?;

    let mut copyright = Copyright::default();

    loop {
        match xml_reader.read_event() {
            Ok(Event::Start(start)) => match start.name().as_ref() {
                b"year" => {
                    copyright.year = Some(xml_reader.read_inner_as()?);
                }
                b"license" => {
                    copyright.license = Some(xml_reader.read_inner_as()?);
                }
                b"author" => {
                    copyright.author = xml_reader.read_inner_as()?;
                }
                e => bail!("Unexpected Start element {:?}", xml_reader.bytes_to_cow(e)),
            },
            Ok(Event::End(e)) => match e.name().as_ref() {
                b"copyright" => {
                    if copyright.author.is_empty() {
                        bail!("Did not find the 'author' element");
                    }

                    return Ok(copyright);
                }
                _ => {}
            },
            // Ignore spurious Event::Text, I think they are newlines.
            Ok(Event::Text(_)) => {}
            e => bail!("Unexpected element {:?}", e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::read::start_parse;
    use quick_xml::Reader;

    #[test]
    fn valid_copyright_all_fields() {
        let mut xml_reader = Reader::from_str(
            r#"<copyright>
                 <year>2024</year>
                 <license>MIT</license>
                 <author>Homer Simpson</author>
               </copyright>"#,
        );

        let start = start_parse(&mut xml_reader).unwrap();
        let result = parse_copyright(&start, &mut xml_reader).unwrap();
        assert_eq!(result.year, Some(2024));
        assert_eq!(result.license, Some("MIT".to_string()));
        assert_eq!(result.author, "Homer Simpson");
    }

    #[test]
    fn valid_copyright_author_only() {
        let mut xml_reader = Reader::from_str(
            r#"<copyright>
                 <author>Homer Simpson</author>
               </copyright>"#,
        );

        let start = start_parse(&mut xml_reader).unwrap();
        let result = parse_copyright(&start, &mut xml_reader).unwrap();
        assert_eq!(result.year, None);
        assert_eq!(result.license, None);
        assert_eq!(result.author, "Homer Simpson");
    }

    #[test]
    fn valid_copyright_missing_license() {
        let mut xml_reader = Reader::from_str(
            r#"<copyright>
                 <year>2024</year>
                 <author>Homer Simpson</author>
               </copyright>"#,
        );

        let start = start_parse(&mut xml_reader).unwrap();
        let result = parse_copyright(&start, &mut xml_reader).unwrap();
        assert_eq!(result.year, Some(2024));
        assert_eq!(result.license, None);
        assert_eq!(result.author, "Homer Simpson");
    }

    #[test]
    fn missing_author() {
        let mut xml_reader = Reader::from_str(
            r#"<copyright>
                 <year>2024</year>
                 <license>MIT</license>
               </copyright>"#,
        );

        let start = start_parse(&mut xml_reader).unwrap();
        let result = parse_copyright(&start, &mut xml_reader);
        assert!(result.is_err());
    }

    #[test]
    fn extra_elements() {
        let mut xml_reader = Reader::from_str(
            r#"<copyright>
                 <author>Homer Simpson</author>
                 <foo>bar</foo>
               </copyright>"#,
        );

        let start = start_parse(&mut xml_reader).unwrap();
        let result = parse_copyright(&start, &mut xml_reader);
        assert!(result.is_err());
    }

    #[test]
    fn extra_attributes() {
        let mut xml_reader = Reader::from_str(
            r#"<copyright foo="bar">
                 <author>Homer Simpson</author>
               </copyright>"#,
        );

        let start = start_parse(&mut xml_reader).unwrap();
        let result = parse_copyright(&start, &mut xml_reader);
        assert!(result.is_err());
    }
}
