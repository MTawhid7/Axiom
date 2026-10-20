//! Spike S9-Q1's fuzz subject: a small decoder of length-prefixed records, the
//! shape of an IPC message (docs/testing.md §9).
//!
//! A message is the magic `AXM1`, then records until the input ends. A record
//! is a tag byte, a LEB128 length and that many bytes of payload:
//! - `0x01`, text: UTF-8;
//! - `0x02`, numbers: little-endian `u32`s;
//! - `0x03`, a nested list of records, at most [`MAX_DEPTH`] deep.
//!
//! [`decode`] checks every length. [`decode_planted`] is the same decoder with
//! S9-Q1's planted bug: inside a list nested at least two deep, it trusts a
//! numbers record's length, so a record that claims more bytes than remain
//! panics. A fuzzer has to build that nesting to reach it.
#![forbid(unsafe_code)]

/// The four bytes every message starts with.
pub const MAGIC: &[u8; 4] = b"AXM1";
/// The deepest nesting of lists a message may have.
pub const MAX_DEPTH: usize = 8;

const TEXT: u8 = 0x01;
const NUMBERS: u8 = 0x02;
const LIST: u8 = 0x03;

/// One decoded record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Record {
    Text(String),
    Numbers(Vec<u32>),
    List(Vec<Record>),
}

/// Why a message was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// The input doesn't start with [`MAGIC`].
    Magic,
    /// A length or a payload runs past the end of the input.
    Truncated,
    /// A length takes more than ten bytes, or doesn't fit in `usize`.
    Length,
    /// An unknown tag.
    Tag(u8),
    /// A text record isn't UTF-8.
    Utf8,
    /// A numbers record's length isn't a multiple of four.
    Alignment,
    /// Lists nest deeper than [`MAX_DEPTH`].
    Depth,
}

/// Decodes a message, checking every length.
pub fn decode(input: &[u8]) -> Result<Vec<Record>, Error> {
    Decoder { planted: false }.message(input)
}

/// Decodes a message with S9-Q1's planted bug: it panics on a numbers record,
/// at least two lists deep, whose length runs past the end of the input.
pub fn decode_planted(input: &[u8]) -> Result<Vec<Record>, Error> {
    Decoder { planted: true }.message(input)
}

/// Encodes records as a message that [`decode`] accepts.
#[must_use]
pub fn encode(records: &[Record]) -> Vec<u8> {
    let mut out = MAGIC.to_vec();
    encode_records(records, &mut out);
    out
}

fn encode_records(records: &[Record], out: &mut Vec<u8>) {
    for record in records {
        let (tag, payload) = match record {
            Record::Text(text) => (TEXT, text.as_bytes().to_vec()),
            Record::Numbers(numbers) => (
                NUMBERS,
                numbers.iter().flat_map(|n| n.to_le_bytes()).collect(),
            ),
            Record::List(list) => {
                let mut nested = Vec::new();
                encode_records(list, &mut nested);
                (LIST, nested)
            }
        };
        out.push(tag);
        let mut len = payload.len() as u64;
        loop {
            let low = (len & 0x7f) as u8;
            len >>= 7;
            if len == 0 {
                out.push(low);
                break;
            }
            out.push(low | 0x80);
        }
        out.extend_from_slice(&payload);
    }
}

struct Decoder {
    planted: bool,
}

impl Decoder {
    fn message(&self, input: &[u8]) -> Result<Vec<Record>, Error> {
        let body = input.strip_prefix(MAGIC.as_slice()).ok_or(Error::Magic)?;
        self.records(body, 0)
    }

    fn records(&self, mut rest: &[u8], depth: usize) -> Result<Vec<Record>, Error> {
        if depth > MAX_DEPTH {
            return Err(Error::Depth);
        }
        let mut records = Vec::new();
        while let Some((&tag, after_tag)) = rest.split_first() {
            let (len, after_len) = varint(after_tag)?;
            let len = usize::try_from(len).map_err(|_| Error::Length)?;
            let (payload, after) = if self.planted && tag == NUMBERS && depth >= 2 {
                // S9-Q1's planted bug: this length isn't checked against the input.
                (&after_len[..len], &after_len[len..])
            } else if len <= after_len.len() {
                after_len.split_at(len)
            } else {
                return Err(Error::Truncated);
            };
            records.push(self.record(tag, payload, depth)?);
            rest = after;
        }
        Ok(records)
    }

    fn record(&self, tag: u8, payload: &[u8], depth: usize) -> Result<Record, Error> {
        match tag {
            TEXT => std::str::from_utf8(payload)
                .map(|text| Record::Text(text.to_owned()))
                .map_err(|_| Error::Utf8),
            NUMBERS => {
                if !payload.len().is_multiple_of(4) {
                    return Err(Error::Alignment);
                }
                Ok(Record::Numbers(
                    payload
                        .chunks_exact(4)
                        .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                        .collect(),
                ))
            }
            LIST => self.records(payload, depth + 1).map(Record::List),
            other => Err(Error::Tag(other)),
        }
    }
}

/// A LEB128 length of at most ten bytes, and the input after it.
fn varint(input: &[u8]) -> Result<(u64, &[u8]), Error> {
    let mut value = 0u64;
    for (i, &byte) in input.iter().enumerate().take(10) {
        value |= u64::from(byte & 0x7f) << (7 * i);
        if byte & 0x80 == 0 {
            return Ok((value, &input[i + 1..]));
        }
    }
    Err(if input.len() < 10 {
        Error::Truncated
    } else {
        Error::Length
    })
}

#[cfg(test)]
mod tests {
    use super::{Error, MAGIC, Record, decode, decode_planted, encode};

    fn sample() -> Vec<Record> {
        vec![
            Record::Text("Axiom".to_owned()),
            Record::Numbers(vec![1, 2, u32::MAX]),
            Record::List(vec![
                Record::Text(String::new()),
                Record::List(vec![Record::Numbers(vec![7])]),
            ]),
        ]
    }

    #[test]
    fn round_trips() {
        let message = encode(&sample());
        assert_eq!(decode(&message), Ok(sample()));
        assert_eq!(decode_planted(&message), Ok(sample()));
        assert_eq!(decode(MAGIC), Ok(Vec::new()));
    }

    #[test]
    fn rejects_what_it_should() {
        assert_eq!(decode(b"AXM2"), Err(Error::Magic));
        assert_eq!(decode(b"AXM1\x01\x05abc"), Err(Error::Truncated));
        assert_eq!(decode(b"AXM1\x01\x80"), Err(Error::Truncated));
        assert_eq!(
            decode(b"AXM1\x01\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff"),
            Err(Error::Length)
        );
        assert_eq!(decode(b"AXM1\x09\x00"), Err(Error::Tag(9)));
        assert_eq!(decode(b"AXM1\x01\x01\xff"), Err(Error::Utf8));
        assert_eq!(decode(b"AXM1\x02\x03abc"), Err(Error::Alignment));
        let mut deep = vec![Record::Text("x".to_owned())];
        for _ in 0..10 {
            deep = vec![Record::List(deep)];
        }
        assert_eq!(decode(&encode(&deep)), Err(Error::Depth));
    }

    #[test]
    fn the_checked_decoder_rejects_the_planted_input() {
        // Two lists deep, a numbers record claims 8 bytes; 4 remain.
        let input = b"AXM1\x03\x08\x03\x06\x02\x08\x01\x02\x03\x04";
        assert_eq!(decode(input), Err(Error::Truncated));
    }

    #[test]
    #[should_panic(expected = "out of range")]
    fn the_planted_bug_panics() {
        let input = b"AXM1\x03\x08\x03\x06\x02\x08\x01\x02\x03\x04";
        let _ = decode_planted(input);
    }
}
