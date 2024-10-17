use anyhow::{bail, Result};
use quick_xml::{events::Event, Reader};

use crate::model::TrackSegment;

use super::{extensions::parse_extensions, waypoint::parse_waypoint, XmlReaderConversions};

pub(crate) fn parse_track_segment(xml_reader: &mut Reader<&[u8]>) -> Result<TrackSegment> {
    let mut segment = TrackSegment::default();

    loop {
        match xml_reader.read_event() {
            Ok(Event::Start(e)) => match e.name().as_ref() {
                b"trkpt" => {
                    let point = parse_waypoint(&e, xml_reader, b"trkpt")?;
                    segment.points.push(point);
                }
                b"extensions" => {
                    segment.extensions = Some(parse_extensions(xml_reader)?);
                }
                e => bail!("Unexpected Start element {:?}", xml_reader.bytes_to_cow(e)),
            },
            Ok(Event::End(e)) => match e.name().as_ref() {
                b"trkseg" => {
                    return Ok(segment);
                }
                _ => {}
            },
            // Ignore spurious Event::Text, I think they are newlines.
            Ok(Event::Text(_)) => {}
            e => bail!("Unexpected element {:?}", e),
        }
    }
}
