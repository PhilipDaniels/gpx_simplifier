use std::io::Write;

use time::{format_description::{self, well_known}, OffsetDateTime, UtcOffset};

pub fn format_utc_date(date: OffsetDateTime) -> String {
    let mut buf = Vec::with_capacity(32);
    write_utc_date(&mut buf, date);
    String::from_utf8(buf).unwrap()
}

pub fn to_local_date(date: OffsetDateTime) -> OffsetDateTime {
    let local_offset = UtcOffset::local_offset_at(date).unwrap();
    date.to_offset(local_offset)
}

pub fn format_utc_and_local_date(date: OffsetDateTime, sep: &str) -> String {
    let local_fmt = format_description::parse("[year]-[month]-[day] [hour repr:24]:[minute]:[second][end] (local time)").unwrap();
    let mut buf = Vec::with_capacity(64);
    write_utc_date(&mut buf, date);
    write!(buf, "{}", sep).unwrap();
    let d = to_local_date(date);
    d.format_into(&mut buf, &local_fmt).unwrap();
    String::from_utf8(buf).unwrap()
}

pub fn write_utc_date<W: Write>(w: &mut W, date: OffsetDateTime) {
    date.format_into(w, &well_known::Rfc3339).unwrap();
}