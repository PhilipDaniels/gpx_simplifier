use std::io::BufRead;

use anyhow::{bail, Result};
use quick_xml::{events::Event, Reader};

use crate::model::Track;

use super::{bytes_to_string, read_inner_as_string, track_segment::parse_track_segment};

pub(crate) fn parse_track<R: BufRead>(buf: &mut Vec<u8>, reader: &mut Reader<R>) -> Result<Track> {
    // TODO: Make a track here instead of the individual fields.
    let mut name = None;
    let mut track_type = None;
    let mut segments = Vec::new();
    let mut desc = None;

    loop {
        match reader.read_event_into(buf) {
            Ok(Event::Start(e)) => match e.name().as_ref() {
                b"name" => {
                    name = Some(read_inner_as_string(buf, reader)?);
                }
                b"type" => {
                    track_type = Some(read_inner_as_string(buf, reader)?);
                }
                b"desc" => {
                    desc = Some(read_inner_as_string(buf, reader)?);
                }
                b"trkseg" => {
                    segments.push(parse_track_segment(buf, reader)?);
                }
                e => bail!("Unexpected element {:?}", bytes_to_string(e)?),
            },
            Ok(Event::End(e)) => match e.name().as_ref() {
                b"trk" => {
                    let mut track = Track::default();
                    track.name = name;
                    track.r#type = track_type;
                    track.description = desc;
                    track.segments = segments;
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
