use anyhow::{bail, Result};
use quick_xml::{events::Event, Reader};

use crate::model::Track;

use super::{
    extensions::parse_extensions, link::parse_link, track_segment::parse_track_segment,
    XmlReaderConversions, XmlReaderExtensions,
};

pub(crate) fn parse_track(xml_reader: &mut Reader<&[u8]>) -> Result<Track> {
    let mut track = Track::default();

    loop {
        match xml_reader.read_event() {
            Ok(Event::Start(e)) => match e.name().as_ref() {
                b"name" => {
                    track.name = Some(xml_reader.read_inner_as()?);
                }
                b"cmt" => {
                    track.comment = Some(xml_reader.read_inner_as()?);
                }
                b"desc" => {
                    track.description = Some(xml_reader.read_inner_as()?);
                }
                b"src" => {
                    track.source = Some(xml_reader.read_inner_as()?);
                }
                b"link" => {
                    let link = parse_link(&e, xml_reader)?;
                    track.links.push(link);
                }
                b"number" => {
                    track.number = Some(xml_reader.read_inner_as()?);
                }
                b"type" => {
                    track.r#type = Some(xml_reader.read_inner_as_string()?);
                }
                b"extensions" => {
                    track.extensions = Some(parse_extensions(xml_reader)?);
                }
                b"trkseg" => {
                    track.segments.push(parse_track_segment(xml_reader)?);
                }
                e => bail!("Unexpected Start element {:?}", xml_reader.bytes_to_cow(e)),
            },
            Ok(Event::End(e)) => match e.name().as_ref() {
                b"trk" => {
                    return Ok(track);
                }
                _ => {}
            },
            // Ignore spurious Event::Text, I think they are newlines.
            Ok(Event::Text(_)) => {}
            e => bail!("Unexpected element {:?}", e),
        }
    }
}
