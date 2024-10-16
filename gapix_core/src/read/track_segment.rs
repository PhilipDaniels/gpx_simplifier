use std::io::BufRead;

use anyhow::Result;
use quick_xml::Reader;

use crate::model::TrackSegment;

use super::waypoint::parse_waypoint;

pub(crate) fn parse_track_segment<R: BufRead>(
    buf: &mut Vec<u8>,
    reader: &mut Reader<R>,
) -> Result<TrackSegment> {
    let mut segment = TrackSegment::default();

    while let Some(point) = parse_waypoint(buf, reader)? {
        segment.points.push(point);
    }

    Ok(segment)
}
