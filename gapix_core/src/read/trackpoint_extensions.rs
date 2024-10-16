use std::io::BufRead;

use anyhow::{bail, Result};
use quick_xml::{events::Event, Reader};

use crate::model::GarminTrackpointExtensions;

use super::{bytes_to_string, read_inner_as};

pub(crate) fn parse_garmin_trackpoint_extensions<R: BufRead>(
    buf: &mut Vec<u8>,
    reader: &mut Reader<R>,
) -> Result<GarminTrackpointExtensions> {
    let mut air_temp = None;
    let mut water_temp = None;
    let mut depth = None;
    let mut heart_rate = None;
    let mut cadence = None;

    loop {
        match reader.read_event_into(buf) {
            Ok(Event::Start(e)) => match e.local_name().as_ref() {
                b"TrackPointExtension" => { /* ignore, just a container element */ }
                b"atemp" => {
                    air_temp = Some(read_inner_as::<R, f64>(buf, reader)?);
                }
                b"wtemp" => {
                    water_temp = Some(read_inner_as::<R, f64>(buf, reader)?);
                }
                b"depth" => {
                    depth = Some(read_inner_as::<R, f64>(buf, reader)?);
                }
                b"hr" => {
                    heart_rate = Some(read_inner_as::<R, u8>(buf, reader)?);
                }
                b"cad" => {
                    cadence = Some(read_inner_as::<R, u8>(buf, reader)?);
                }
                e => bail!("Unexpected element {:?}", bytes_to_string(e)),
            },
            Ok(Event::End(e)) => match e.local_name().as_ref() {
                b"TrackPointExtension" => { /* ignore, just a container element */ }
                b"extensions" => {
                    return Ok(GarminTrackpointExtensions {
                        air_temp,
                        water_temp,
                        depth,
                        heart_rate,
                        cadence,
                        extensions: None,
                    });
                }
                b"atemp" | b"wtemp" | b"depth" | b"hr" | b"cad" => { /* ignore, just the closing tags */
                }
                e => bail!("Unexpected element {:?}", bytes_to_string(e)),
            },
            // Ignore spurious Event::Text, I think they are newlines.
            Ok(Event::Text(_)) => {}
            e => bail!("Unexpected element {:?}", e),
        }
    }
}
