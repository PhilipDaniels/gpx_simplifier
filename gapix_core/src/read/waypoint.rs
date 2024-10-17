use anyhow::{bail, Result};
use quick_xml::{events::Event, Reader};

use crate::model::Waypoint;

use super::{
    attributes::Attributes, bytes_to_string, extensions::parse_extensions, link::parse_link,
    read_inner_as, read_inner_as_time
};

/// Parses a waypoint. Waypoints can appear under the 'gpx' tag, as part of a
/// route or as part of a track.
pub(crate) fn parse_waypoint(
    mut attributes: Attributes,
    buf: &mut Vec<u8>,
    xml_reader: &mut Reader<&[u8]>,
    expected_end_tag: &[u8],   // Possible ending tags: wpt, rtept, trkpt
) -> Result<Waypoint> {
    let lat = attributes.get("lat")?;
    let lon = attributes.get("lon")?;
    if !attributes.is_empty() {
        bail!(
            "Found extra attributes while parsing waypoint {:?}",
            attributes
        );
    }

    let mut wp = Waypoint::with_lat_lon(lat, lon);

    loop {
        match xml_reader.read_event_into(buf) {
            Ok(Event::Start(e)) => match e.name().as_ref() {
                b"ele" => {
                    wp.ele = Some(read_inner_as(buf, xml_reader)?);
                }
                b"time" => {
                    wp.time = Some(read_inner_as_time(buf, xml_reader)?);
                }
                b"magvar" => {
                    wp.magvar = Some(read_inner_as(buf, xml_reader)?);
                }
                b"geoidheight" => {
                    wp.geoid_height = Some(read_inner_as(buf, xml_reader)?);
                }
                b"name" => {
                    wp.name = Some(read_inner_as(buf, xml_reader)?);
                }
                b"cmt" => {
                    wp.comment = Some(read_inner_as(buf, xml_reader)?);
                }
                b"desc" => {
                    wp.description = Some(read_inner_as(buf, xml_reader)?);
                }
                b"src" => {
                    wp.source = Some(read_inner_as(buf, xml_reader)?);
                }
                b"link" => {
                    let link = parse_link(Attributes::new(&e)?, buf, xml_reader)?;
                    wp.links.push(link);
                }
                b"sym" => {
                    wp.source = Some(read_inner_as(buf, xml_reader)?);
                }
                b"type" => {
                    wp.r#type = Some(read_inner_as(buf, xml_reader)?);
                }
                b"fix" => {
                    let fix: String = read_inner_as(buf, xml_reader)?;
                    wp.fix = Some(fix.try_into()?);
                }
                b"sat" => {
                    wp.num_satellites = Some(read_inner_as(buf, xml_reader)?);
                }
                b"hdop" => {
                    wp.hdop = Some(read_inner_as(buf, xml_reader)?);
                }
                b"vdop" => {
                    wp.vdop = Some(read_inner_as(buf, xml_reader)?);
                }
                b"pdop" => {
                    wp.pdop = Some(read_inner_as(buf, xml_reader)?);
                }
                b"ageofdgpsdata" => {
                    wp.age_of_dgps_data = Some(read_inner_as(buf, xml_reader)?);
                }
                b"dgpsid" => {
                    wp.dgps_id = Some(read_inner_as(buf, xml_reader)?);
                }
                b"extensions" => {
                    wp.extensions = Some(parse_extensions(xml_reader)?);
                }
                e => bail!("Unexpected element {:?}", bytes_to_string(e)),
            },
            Ok(Event::End(e)) => {
                if e.name().as_ref() == expected_end_tag {
                    // Only waypoints appear in quantity, by clearing the buffer after each one
                    // we can keep memory usage to a minimum.
                    buf.clear();
                    return Ok(wp);
                } else {
                    // TODO: Check for all valid ends.
                }
            }
            // Ignore spurious Event::Text, I think they are newlines.
            Ok(Event::Text(_)) => {}
            e => bail!("Unexpected element {:?}", e),
        }
    }
}
