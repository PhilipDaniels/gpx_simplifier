use anyhow::{bail, Result};
use quick_xml::{events::Event, Reader};

use crate::model::GarminTrackpointExtensions;

use super::{bytes_to_string, XmlReaderExtensions};

pub(crate) fn parse_garmin_trackpoint_extensions(
    xml_reader: &mut Reader<&[u8]>,
) -> Result<GarminTrackpointExtensions> {
    let mut gext = GarminTrackpointExtensions::default();

    loop {
        match xml_reader.read_event() {
            Ok(Event::Start(e)) => match e.local_name().as_ref() {
                b"TrackPointExtension" => { /* ignore, just a container element */ }
                b"atemp" => {
                    gext.air_temp = Some(xml_reader.read_inner_as()?);
                }
                b"wtemp" => {
                    gext.water_temp = Some(xml_reader.read_inner_as()?);
                }
                b"depth" => {
                    gext.depth = Some(xml_reader.read_inner_as()?);
                }
                b"hr" => {
                    gext.heart_rate = Some(xml_reader.read_inner_as()?);
                }
                b"cad" => {
                    gext.cadence = Some(xml_reader.read_inner_as()?);
                }
                e => bail!("Unexpected element {:?}", bytes_to_string(e)),
            },
            Ok(Event::End(e)) => match e.local_name().as_ref() {
                b"TrackPointExtension" => { /* ignore, just a container element */ }
                b"extensions" => {
                    return Ok(gext);
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
