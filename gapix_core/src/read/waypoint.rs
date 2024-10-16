use std::io::BufRead;

use anyhow::{bail, Context, Result};
use quick_xml::{events::Event, Reader};

use crate::model::Waypoint;

use super::{
    bytes_to_string, read_attribute_as_f64, read_inner_as,
    read_inner_as_time, trackpoint_extensions::parse_garmin_trackpoint_extensions,
};

pub(crate) fn parse_waypoint<R: BufRead>(
    buf: &mut Vec<u8>,
    reader: &mut Reader<R>,
) -> Result<Option<Waypoint>> {
    let mut lat = None;
    let mut lon = None;
    let mut ele = None;
    let mut time = None;
    let mut tp_extensions = None;

    loop {
        match reader.read_event_into(buf) {
            Ok(Event::Start(e)) => match e.name().as_ref() {
                b"trkpt" => {
                    lat = Some(read_attribute_as_f64(&e, "lat")?);
                    lon = Some(read_attribute_as_f64(&e, "lon")?);
                }
                b"ele" => {
                    ele = Some(read_inner_as::<R, f64>(buf, reader)?);
                }
                b"time" => {
                    time = Some(read_inner_as_time(buf, reader)?);
                }
                b"extensions" => {
                    tp_extensions = Some(parse_garmin_trackpoint_extensions(buf, reader)?);
                }
                e => bail!("Unexpected element {:?}", bytes_to_string(e)),
            },
            Ok(Event::End(e)) => match e.name().as_ref() {
                b"trkpt" => {
                    let mut tp = Waypoint::with_lat_lon(
                        lat.context("lat attribute not found")?,
                        lon.context("lon attribute not found")?,
                    );

                    tp.ele = ele;
                    tp.time = time;
                    tp.tp_extensions = tp_extensions;
                    return Ok(Some(tp));
                }
                b"trkseg" => {
                    // Reached the end of the trackpoints for this segment.
                    return Ok(None);
                }
                _ => {}
            },
            // Ignore spurious Event::Text, I think they are newlines.
            Ok(Event::Text(_)) => {}
            e => bail!("Unexpected element {:?}", e),
        }
    }
}
