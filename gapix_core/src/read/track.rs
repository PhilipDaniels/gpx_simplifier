use anyhow::{bail, Result};
use quick_xml::{events::Event, Reader};

use crate::model::Track;

use super::{
    attributes::Attributes, bytes_to_string, extensions::parse_extensions, link::parse_link,
    read_inner_as, read_inner_as_string, track_segment::parse_track_segment,
};

pub(crate) fn parse_track(
    buf: &mut Vec<u8>,
    xml_reader: &mut Reader<&[u8]>,
) -> Result<Track> {
    let mut track = Track::default();

    loop {
        match xml_reader.read_event_into(buf) {
            Ok(Event::Start(e)) => match e.name().as_ref() {
                b"name" => {
                    track.name = Some(read_inner_as_string(buf, xml_reader)?);
                }
                b"cmt" => {
                    track.comment = Some(read_inner_as(buf, xml_reader)?);
                }
                b"desc" => {
                    track.description = Some(read_inner_as(buf, xml_reader)?);
                }
                b"src" => {
                    track.source = Some(read_inner_as(buf, xml_reader)?);
                }
                b"link" => {
                    let link = parse_link(Attributes::new(&e)?, buf, xml_reader)?;
                    track.links.push(link);
                }
                b"number" => {
                    track.number = Some(read_inner_as(buf, xml_reader)?);
                }
                b"type" => {
                    track.r#type = Some(read_inner_as_string(buf, xml_reader)?);
                }
                b"extensions" => {
                    track.extensions = Some(parse_extensions(xml_reader)?);
                }
                b"trkseg" => {
                    track.segments.push(parse_track_segment(buf, xml_reader)?);
                }
                e => bail!("Unexpected element {:?}", bytes_to_string(e)?),
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
