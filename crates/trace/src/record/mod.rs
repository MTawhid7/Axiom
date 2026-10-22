//! The record format (OB1–OB3, OB5): little-endian 64-bit words, the layout a
//! ring stores and a region shares.
//!
//! Every record starts with two words:
//!
//! | Word | Bits | Field |
//! |---|---|---|
//! | 0 | 0–7 | [`Kind`] (0 is never a record) |
//! | 0 | 8–15 | [`Category`] |
//! | 0 | 16–31 | [`Name`] |
//! | 0 | 32–39 | the record's length in words, these two included |
//! | 0 | 40–47 | [`Level`] for a log; 0 otherwise |
//! | 0 | 48–63 | 0 |
//! | 1 | 0–63 | the timestamp, from the process's clock |
//!
//! Then the payload: up to [`MAX_FIELDS`] fields for spans and events; one
//! value for metrics, flows and drops; text for logs and panics, each text a
//! length word and its UTF-8 bytes packed eight to a word, zero-padded.
//! Encoding is canonical, so [`decode`] refuses every other arrangement.

mod decode;
mod encode;
mod text;

use crate::category::{Category, Level};

pub use decode::{DecodeError, Records, decode, records};
pub(crate) use encode::{DROPPED_WORDS, FIELD_WORDS, dropped, fields, log, panic, value};
pub use text::Text;

/// The longest record, in words.
pub const MAX_WORDS: usize = 64;
/// The most fields a span or an event carries.
pub const MAX_FIELDS: usize = 4;
/// The longest text a log carries, and a panic's message, in bytes (OB3).
pub const MAX_TEXT: usize = 255;
/// The longest file name a panic's location carries, in bytes.
pub const MAX_FILE: usize = 128;

/// What a record is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Kind {
    /// Work begins on this thread; spans nest (OB1).
    SpanBegin = 1,
    /// The innermost open span ends.
    SpanEnd = 2,
    /// A moment.
    Event = 3,
    /// A counter grows by a value (OB8).
    Counter = 4,
    /// A gauge holds a value (OB8).
    Gauge = 5,
    /// A histogram's sample (OB8).
    Histogram = 6,
    /// A flow begins, keyed by an IPC message's channel and sequence (OB4).
    FlowBegin = 7,
    /// A flow ends, where the message is received.
    FlowEnd = 8,
    /// A log, with its level and text (OB1).
    Log = 9,
    /// Records this ring dropped since its last drop record (OB5).
    Dropped = 10,
    /// A panic's message and location (OB9).
    Panic = 11,
}

impl Kind {
    /// The kind with this ID, if there is one.
    #[must_use]
    pub const fn from_id(id: u8) -> Option<Self> {
        match id {
            1 => Some(Self::SpanBegin),
            2 => Some(Self::SpanEnd),
            3 => Some(Self::Event),
            4 => Some(Self::Counter),
            5 => Some(Self::Gauge),
            6 => Some(Self::Histogram),
            7 => Some(Self::FlowBegin),
            8 => Some(Self::FlowEnd),
            9 => Some(Self::Log),
            10 => Some(Self::Dropped),
            11 => Some(Self::Panic),
            _ => None,
        }
    }
}

/// A span's, event's, metric's or log's name: an ID into a table compiled
/// into every executable (OB3). How names are declared is M1-CP3's
/// interface note; 0 is the recorder's own, for drops and panics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Name(u16);

impl Name {
    /// The recorder's own name, for drop and panic records.
    pub const RECORDER: Self = Self(0);

    /// The name with this ID.
    #[must_use]
    pub const fn new(id: u16) -> Self {
        Self(id)
    }

    /// The name's ID.
    #[must_use]
    pub const fn id(self) -> u16 {
        self.0
    }
}

/// What every record but a drop or a panic carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Header {
    /// The record's category.
    pub category: Category,
    /// The record's name.
    pub name: Name,
    /// When it was recorded, on the process's clock (OB2).
    pub timestamp: u64,
}

/// A span's or event's fields: integers, durations, IDs and enumerations
/// (OB3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fields {
    values: [u64; MAX_FIELDS],
    len: usize,
}

impl Fields {
    pub(crate) fn from_words(words: &[u64]) -> Self {
        let mut values = [0; MAX_FIELDS];
        let mut len = 0_usize;
        for (slot, word) in values.iter_mut().zip(words) {
            *slot = *word;
            len = len.saturating_add(1);
        }
        Self { values, len }
    }

    /// The fields, in order.
    #[must_use]
    pub fn as_slice(&self) -> &[u64] {
        self.values.get(..self.len).unwrap_or(&[])
    }
}

/// One decoded record.
#[derive(Debug, Clone, PartialEq, Eq)]
#[expect(
    clippy::large_enum_variant,
    reason = "texts stay inline so that decoding never allocates; a decoded record is consumed at once, and the flight recorder keeps raw words, not records"
)]
pub enum Record {
    /// [`Kind::SpanBegin`].
    SpanBegin(Header, Fields),
    /// [`Kind::SpanEnd`].
    SpanEnd(Header),
    /// [`Kind::Event`].
    Event(Header, Fields),
    /// [`Kind::Counter`], with its increment.
    Counter(Header, u64),
    /// [`Kind::Gauge`], with its value.
    Gauge(Header, u64),
    /// [`Kind::Histogram`], with its sample.
    Histogram(Header, u64),
    /// [`Kind::FlowBegin`], with its key.
    FlowBegin(Header, u64),
    /// [`Kind::FlowEnd`], with its key.
    FlowEnd(Header, u64),
    /// [`Kind::Log`].
    Log(Header, Level, Text),
    /// [`Kind::Dropped`]: when the next record was written, and how many were
    /// dropped before it.
    Dropped {
        /// The next record's timestamp.
        timestamp: u64,
        /// The records dropped.
        count: u64,
    },
    /// [`Kind::Panic`].
    Panic {
        /// When the panic was recorded.
        timestamp: u64,
        /// The panic's message, cut at [`MAX_TEXT`] bytes.
        message: Text,
        /// The source file, cut at [`MAX_FILE`] bytes.
        file: Text,
        /// The line.
        line: u32,
        /// The column.
        column: u32,
    },
}

/// A record's first word.
pub(crate) fn header_word(
    kind: Kind,
    category: Category,
    name: Name,
    words: usize,
    level: u8,
) -> u64 {
    let words = u64::try_from(words).unwrap_or(0) & 0xff;
    u64::from(kind as u8)
        | u64::from(category.id()).wrapping_shl(8)
        | u64::from(name.id()).wrapping_shl(16)
        | words.wrapping_shl(32)
        | u64::from(level).wrapping_shl(40)
}

#[cfg(test)]
mod tests;
