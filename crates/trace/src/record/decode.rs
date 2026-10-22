//! Decoding records: total over any words, as a reader of untrusted memory
//! must be (OB11, OB24). A correct writer's records decode; every other
//! arrangement is refused with the reason, which the collector reports as a
//! violation (IR3).

use std::fmt;

use super::text::unpack;
use super::{Fields, Header, Kind, MAX_FIELDS, MAX_FILE, MAX_TEXT, MAX_WORDS, Name, Record};
use crate::category::{Category, Level};

/// Why words aren't a record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodeError {
    /// Fewer words than the header's length.
    Truncated,
    /// The kind byte names no kind.
    UnknownKind(u8),
    /// The category byte names no category.
    UnknownCategory(u8),
    /// The level byte names no level, or a record other than a log has one.
    BadLevel(u8),
    /// The header's reserved bits aren't zero.
    ReservedBits,
    /// The length doesn't fit the kind.
    BadLength(Kind, usize),
    /// A drop or panic record isn't in the recorder's category and name.
    NotTheRecorders(Kind),
    /// A text is too long, runs past the record, has nonzero padding or isn't
    /// UTF-8.
    BadText(Kind),
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Truncated => f.write_str("a record runs past the words given"),
            Self::UnknownKind(kind) => write!(f, "unknown record kind {kind}"),
            Self::UnknownCategory(category) => write!(f, "unknown category {category}"),
            Self::BadLevel(level) => write!(f, "level {level} is unknown or misplaced"),
            Self::ReservedBits => f.write_str("a header's reserved bits are set"),
            Self::BadLength(kind, words) => write!(f, "{words} words don't fit a {kind:?} record"),
            Self::NotTheRecorders(kind) => write!(
                f,
                "a {kind:?} record isn't in the recorder's category and name"
            ),
            Self::BadText(kind) => write!(
                f,
                "a {kind:?} record's text is too long, unpadded or not UTF-8"
            ),
        }
    }
}

impl std::error::Error for DecodeError {}

/// Decodes the record at the front of `words`, and returns it with its length
/// in words.
pub fn decode(words: &[u64]) -> Result<(Record, usize), DecodeError> {
    let &header = words.first().ok_or(DecodeError::Truncated)?;
    let [
        kind,
        category,
        name_low,
        name_high,
        len,
        level,
        reserved_low,
        reserved_high,
    ] = header.to_le_bytes();
    let kind = Kind::from_id(kind).ok_or(DecodeError::UnknownKind(kind))?;
    let category = Category::from_id(category).ok_or(DecodeError::UnknownCategory(category))?;
    if reserved_low != 0 || reserved_high != 0 {
        return Err(DecodeError::ReservedBits);
    }
    let len = usize::from(len);
    if !(2..=MAX_WORDS).contains(&len) {
        return Err(DecodeError::BadLength(kind, len));
    }
    let record = words.get(..len).ok_or(DecodeError::Truncated)?;
    let (timestamp, payload) = match record {
        [_, timestamp, payload @ ..] => (*timestamp, payload),
        _ => return Err(DecodeError::Truncated),
    };
    if level != 0 && kind != Kind::Log {
        return Err(DecodeError::BadLevel(level));
    }
    let header = Header {
        category,
        name: Name::new(u16::from_le_bytes([name_low, name_high])),
        timestamp,
    };
    Ok((decode_payload(kind, header, level, payload)?, len))
}

fn decode_payload(
    kind: Kind,
    header: Header,
    level: u8,
    payload: &[u64],
) -> Result<Record, DecodeError> {
    let length = || DecodeError::BadLength(kind, payload.len().saturating_add(2));
    let value = || match payload {
        [value] => Ok(*value),
        _ => Err(length()),
    };
    match kind {
        Kind::SpanBegin | Kind::Event if payload.len() > MAX_FIELDS => Err(length()),
        Kind::SpanBegin => Ok(Record::SpanBegin(header, Fields::from_words(payload))),
        Kind::Event => Ok(Record::Event(header, Fields::from_words(payload))),
        Kind::SpanEnd if payload.is_empty() => Ok(Record::SpanEnd(header)),
        Kind::SpanEnd => Err(length()),
        Kind::Counter => Ok(Record::Counter(header, value()?)),
        Kind::Gauge => Ok(Record::Gauge(header, value()?)),
        Kind::Histogram => Ok(Record::Histogram(header, value()?)),
        Kind::FlowBegin => Ok(Record::FlowBegin(header, value()?)),
        Kind::FlowEnd => Ok(Record::FlowEnd(header, value()?)),
        Kind::Log => {
            let level = Level::from_id(level).ok_or(DecodeError::BadLevel(level))?;
            match unpack(payload, MAX_TEXT) {
                Some((text, used)) if used == payload.len() => Ok(Record::Log(header, level, text)),
                Some(_) => Err(length()),
                None => Err(DecodeError::BadText(kind)),
            }
        }
        Kind::Dropped | Kind::Panic
            if (header.category, header.name) != (Category::Health, Name::RECORDER) =>
        {
            Err(DecodeError::NotTheRecorders(kind))
        }
        Kind::Dropped => Ok(Record::Dropped {
            timestamp: header.timestamp,
            count: value()?,
        }),
        Kind::Panic => decode_panic(header.timestamp, payload),
    }
}

fn decode_panic(timestamp: u64, payload: &[u64]) -> Result<Record, DecodeError> {
    let bad_text = DecodeError::BadText(Kind::Panic);
    let (message, used) = unpack(payload, MAX_TEXT).ok_or(bad_text)?;
    let rest = payload.get(used..).ok_or(bad_text)?;
    let (file, used) = unpack(rest, MAX_FILE).ok_or(bad_text)?;
    match rest.get(used..) {
        Some([location]) => Ok(Record::Panic {
            timestamp,
            message,
            file,
            line: u32::try_from(location & 0xffff_ffff).unwrap_or(0),
            column: u32::try_from(location.wrapping_shr(32)).unwrap_or(0),
        }),
        _ => Err(DecodeError::BadLength(
            Kind::Panic,
            payload.len().saturating_add(2),
        )),
    }
}

/// The records in `words`, in order. The iterator stops after the first
/// error, since nothing after a bad record can be trusted to be aligned.
pub fn records(words: &[u64]) -> Records<'_> {
    Records {
        words,
        failed: false,
    }
}

/// An iterator over the records in some words; see [`records`].
#[derive(Debug)]
pub struct Records<'a> {
    words: &'a [u64],
    failed: bool,
}

impl Iterator for Records<'_> {
    type Item = Result<Record, DecodeError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.failed || self.words.is_empty() {
            return None;
        }
        match decode(self.words) {
            Ok((record, len)) => {
                self.words = self.words.get(len..).unwrap_or(&[]);
                Some(Ok(record))
            }
            Err(error) => {
                self.failed = true;
                Some(Err(error))
            }
        }
    }
}
