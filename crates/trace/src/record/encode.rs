//! Encoding records into words, on the recording path: into arrays on the
//! stack, with no allocation (OB5).

use super::text::{pack, packed_words, truncate};
use super::{Kind, MAX_FIELDS, MAX_FILE, MAX_TEXT, MAX_WORDS, Name, header_word};
use crate::category::{Category, Level};

/// The words of a span or event record: the header, the timestamp, the fields.
pub(crate) const FIELD_WORDS: usize = 2 + MAX_FIELDS;
/// The words of a metric, flow or drop record.
pub(crate) const VALUE_WORDS: usize = 3;
/// The words of a drop record.
pub(crate) const DROPPED_WORDS: usize = VALUE_WORDS;

/// A span or event: `kind` with up to [`MAX_FIELDS`] of `fields`; returns the
/// record's length in `out`.
pub(crate) fn fields(
    out: &mut [u64; FIELD_WORDS],
    kind: Kind,
    category: Category,
    name: Name,
    timestamp: u64,
    fields: &[u64],
) -> usize {
    let mut len = 2_usize;
    for (slot, field) in out.iter_mut().skip(2).zip(fields) {
        *slot = *field;
        len = len.saturating_add(1);
    }
    out[0] = header_word(kind, category, name, len, 0);
    out[1] = timestamp;
    len
}

/// A metric, flow or drop: `kind` with one value.
pub(crate) fn value(
    kind: Kind,
    category: Category,
    name: Name,
    timestamp: u64,
    value: u64,
) -> [u64; VALUE_WORDS] {
    [
        header_word(kind, category, name, VALUE_WORDS, 0),
        timestamp,
        value,
    ]
}

/// The drop record that precedes the next record a full ring accepts.
pub(crate) fn dropped(timestamp: u64, count: u64) -> [u64; DROPPED_WORDS] {
    value(
        Kind::Dropped,
        Category::Health,
        Name::RECORDER,
        timestamp,
        count,
    )
}

/// A log, its text cut at [`MAX_TEXT`] bytes on a character boundary.
pub(crate) fn log(
    out: &mut [u64; MAX_WORDS],
    level: Level,
    category: Category,
    name: Name,
    timestamp: u64,
    text: &str,
) -> usize {
    let text = truncate(text, MAX_TEXT);
    let len = 2_usize
        .saturating_add(packed_words(text.len()))
        .saturating_add(1);
    out[0] = header_word(Kind::Log, category, name, len, level as u8);
    out[1] = timestamp;
    if let Some(rest) = out.get_mut(2..) {
        pack(text, rest);
    }
    len
}

/// A panic: its message, cut at [`MAX_TEXT`] bytes, then its file, cut at
/// [`MAX_FILE`], then its line and column in one word.
pub(crate) fn panic(
    out: &mut [u64; MAX_WORDS],
    timestamp: u64,
    message: &str,
    file: &str,
    line: u32,
    column: u32,
) -> usize {
    let (message, file) = (truncate(message, MAX_TEXT), truncate(file, MAX_FILE));
    let mut len = 2_usize;
    if let Some(rest) = out.get_mut(len..) {
        len = len.saturating_add(pack(message, rest));
    }
    if let Some(rest) = out.get_mut(len..) {
        len = len.saturating_add(pack(file, rest));
    }
    if let Some(slot) = out.get_mut(len) {
        *slot = u64::from(line) | u64::from(column).wrapping_shl(32);
        len = len.saturating_add(1);
    }
    out[0] = header_word(Kind::Panic, Category::Health, Name::RECORDER, len, 0);
    out[1] = timestamp;
    len
}
