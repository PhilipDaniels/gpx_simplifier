use time::format_description::well_known;
use time::{OffsetDateTime, UtcOffset};

/// Convert 'utc_date' to a local date by applying the current local offset of the
/// user at the specified time.
/// TODO: It would be better to determine the offset to apply based on the
/// lat-lon of the trackpoint. We need a time-zone database to do that.
pub fn to_local_date(utc_date: OffsetDateTime) -> OffsetDateTime {
    assert!(utc_date.offset().is_utc());

    let local_offset = UtcOffset::local_offset_at(utc_date).unwrap();
    utc_date.to_offset(local_offset)
}

/// Formats 'utc_date' into a string like "2024-09-01T05:10:44Z".
/// This is the format that GPX files contain.
pub fn format_utc_date(utc_date: &OffsetDateTime) -> String {
    assert!(utc_date.offset().is_utc());

    let mut buf = Vec::with_capacity(20);
    utc_date
        .format_into(&mut buf, &well_known::Rfc3339)
        .unwrap();
    String::from_utf8(buf).unwrap()
}
