use std::io::BufRead;

use anyhow::Result;
use quick_xml::{events::{BytesStart, Event}, Reader};

use crate::{model::Extensions, read::cow_to_string};

use super::bytes_to_string;

pub(crate) fn parse_extensions<R: BufRead>(
    buf: &mut Vec<u8>,
    xml_reader: &mut Reader<R>,
) -> Result<Extensions> {
    buf.clear();

    loop {
        match xml_reader.read_event_into(buf) {
            Ok(Event::End(e)) => match e.name().as_ref() {
                b"extensions" => {
                    let text = xml_reader.decoder().decode(buf)?;
                    let ext = Extensions::new(text);
                    //dbg!(&ext);
                    return Ok(ext);
                }
                _ => {}
            },
            _ => {}
        }

    }

    // let start = BytesStart::new("extensions");
    // let end = start.to_end();


    // // let start_pos = xml_reader.buffer_position();
    // // dbg!(bytes_to_string(buf)?);
    // // // This will eat the ending </extensions> tag too.
    // let span = xml_reader.read_to_end_into(end.name(), buf)?;
    // let len = span.end - span.start;
    
    // // //let ext = bytes_to_string(&buf[span.start as usize..span.end as usize])?;
    // // dbg!(bytes_to_string(buf)?);
    // // dbg!("start_pos={}, span={}", start_pos, &span);

    // // let end_pos = start_pos + (span.end - span.start);
    // // let ext = bytes_to_string(&buf[start_pos as usize..end_pos as usize])?;


    // dbg!(&ext);
    // Ok(Extensions::new(ext))
}
