use anyhow::{bail, Result};
use quick_xml::events::BytesStart;

use crate::model::Bounds;

use super::{attributes::Attributes, XmlReaderConversions};

pub(crate) fn parse_bounds<C: XmlReaderConversions>(
    start_element: &BytesStart<'_>,
    converter: &C,
) -> Result<Bounds> {
    let mut attributes = Attributes::new(start_element, converter)?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::read::start_parse;
    use quick_xml::Reader;

    #[test]
    fn valid_bounds() {
        let mut xml_reader = Reader::from_str(
            r#"<bounds minlat="-1.1" maxlat="1.1" minlon="-53.1111" maxlon="88.88">"#,
        );
        let start = start_parse(&mut xml_reader).unwrap();
        let result = parse_bounds(&start, &xml_reader).unwrap();
        assert_eq!(result.min_lat, -1.1);
        assert_eq!(result.max_lat, 1.1);
        assert_eq!(result.min_lon, -53.1111);
        assert_eq!(result.max_lon, 88.88);
    }

    #[test]
    fn missing_min_lat() {
        let mut xml_reader =
            Reader::from_str(r#"<bounds maxlat="1.1" minlon="-53.1111" maxlon="88.88">"#);
        let start = start_parse(&mut xml_reader).unwrap();
        let result = parse_bounds(&start, &xml_reader);
        assert!(result.is_err());
    }

    #[test]
    fn missing_max_lat() {
        let mut xml_reader =
            Reader::from_str(r#"<bounds minlat="-1.1" minlon="-53.1111" maxlon="88.88">"#);
        let start = start_parse(&mut xml_reader).unwrap();
        let result = parse_bounds(&start, &xml_reader);
        assert!(result.is_err());
    }

    #[test]
    fn missing_min_lon() {
        let mut xml_reader =
            Reader::from_str(r#"<bounds minlat="-1.1" maxlat="1.1" maxlon="88.88">"#);
        let start = start_parse(&mut xml_reader).unwrap();
        let result = parse_bounds(&start, &xml_reader);
        assert!(result.is_err());
    }

    #[test]
    fn missing_max_lon() {
        let mut xml_reader =
            Reader::from_str(r#"<bounds minlat="-1.1" maxlat="1.1" minlon="-53.1111">"#);
        let start = start_parse(&mut xml_reader).unwrap();
        let result = parse_bounds(&start, &xml_reader);
        assert!(result.is_err());
    }

    #[test]
    fn missing_all() {
        let mut xml_reader = Reader::from_str(r#"<bounds>"#);
        let start = start_parse(&mut xml_reader).unwrap();
        let result = parse_bounds(&start, &xml_reader);
        assert!(result.is_err());
    }

    #[test]
    fn extras() {
        let mut xml_reader =
            Reader::from_str(r#"<bounds maxlat="1.1" minlon="-53.1111" maxlon="88.88" foo="bar">"#);
        let start = start_parse(&mut xml_reader).unwrap();
        let result = parse_bounds(&start, &xml_reader);
        assert!(result.is_err());
    }
}
