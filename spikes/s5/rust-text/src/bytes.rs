//! Big-endian reading and writing for font containers. Every read is bounds
//! checked and fails with an [`Error`] instead of panicking, because the
//! input is untrusted (TX7).

use std::fmt;

/// Why a container or a font was rejected. The reason is a fixed phrase, so
/// reports from many malformed inputs can be grouped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Error(pub &'static str);

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;

/// A cursor over untrusted bytes.
#[derive(Clone, Debug)]
pub struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    pub fn position(&self) -> usize {
        self.pos
    }

    pub fn remaining(&self) -> usize {
        self.data.len() - self.pos
    }

    /// The next `len` bytes.
    pub fn take(&mut self, len: usize) -> Result<&'a [u8]> {
        let end = self
            .pos
            .checked_add(len)
            .filter(|&end| end <= self.data.len())
            .ok_or(Error("read past the end of the data"))?;
        let bytes = &self.data[self.pos..end];
        self.pos = end;
        Ok(bytes)
    }

    pub fn skip(&mut self, len: usize) -> Result<()> {
        self.take(len).map(|_| ())
    }

    pub fn u8(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }

    pub fn u16(&mut self) -> Result<u16> {
        let b = self.take(2)?;
        Ok(u16::from_be_bytes([b[0], b[1]]))
    }

    pub fn i16(&mut self) -> Result<i16> {
        let b = self.take(2)?;
        Ok(i16::from_be_bytes([b[0], b[1]]))
    }

    pub fn u32(&mut self) -> Result<u32> {
        let b = self.take(4)?;
        Ok(u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }

    pub fn tag(&mut self) -> Result<[u8; 4]> {
        let b = self.take(4)?;
        Ok([b[0], b[1], b[2], b[3]])
    }

    /// `UIntBase128` (WOFF2 §4.1): at most five bytes, no leading zeros, and
    /// no value above `u32::MAX`; anything else is rejected.
    pub fn base128(&mut self) -> Result<u32> {
        let mut value: u32 = 0;
        for i in 0..5 {
            let byte = self.u8()?;
            if i == 0 && byte == 0x80 {
                return Err(Error("UIntBase128 with a leading zero"));
            }
            if value & 0xFE00_0000 != 0 {
                return Err(Error("UIntBase128 above 2^32 - 1"));
            }
            value = (value << 7) | u32::from(byte & 0x7F);
            if byte & 0x80 == 0 {
                return Ok(value);
            }
        }
        Err(Error("UIntBase128 longer than five bytes"))
    }

    /// `255UInt16` (WOFF2 §4.2).
    pub fn u255(&mut self) -> Result<u16> {
        const WORD_CODE: u8 = 253;
        const ONE_MORE_BYTE_CODE_2: u8 = 254;
        const ONE_MORE_BYTE_CODE_1: u8 = 255;
        const LOWEST_U_CODE: u16 = 253;
        match self.u8()? {
            WORD_CODE => self.u16(),
            ONE_MORE_BYTE_CODE_1 => Ok(u16::from(self.u8()?) + LOWEST_U_CODE),
            ONE_MORE_BYTE_CODE_2 => Ok(u16::from(self.u8()?) + LOWEST_U_CODE * 2),
            code => Ok(u16::from(code)),
        }
    }
}

/// `len` rounded up to a multiple of four, or `None` on overflow.
pub fn pad4(len: usize) -> Option<usize> {
    len.checked_add(3).map(|n| n & !3)
}

pub fn put_u16(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_be_bytes());
}

pub fn put_i16(out: &mut Vec<u8>, value: i16) {
    out.extend_from_slice(&value.to_be_bytes());
}

pub fn put_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_be_bytes());
}

/// Append zero bytes until `out`'s length is a multiple of four.
pub fn pad_to_4(out: &mut Vec<u8>) {
    while !out.len().is_multiple_of(4) {
        out.push(0);
    }
}

/// The OpenType table checksum: the wrapping sum of big-endian `u32`s, with
/// the last word zero-padded.
pub fn checksum(data: &[u8]) -> u32 {
    let mut sum: u32 = 0;
    let mut words = data.chunks_exact(4);
    for word in &mut words {
        sum = sum.wrapping_add(u32::from_be_bytes([word[0], word[1], word[2], word[3]]));
    }
    let tail = words.remainder();
    if !tail.is_empty() {
        let mut last = [0_u8; 4];
        last[..tail.len()].copy_from_slice(tail);
        sum = sum.wrapping_add(u32::from_be_bytes(last));
    }
    sum
}

/// `UIntBase128`, for the test encoders.
pub fn put_base128(out: &mut Vec<u8>, value: u32) {
    let mut groups = [0_u8; 5];
    let mut count = 0;
    let mut rest = value;
    loop {
        groups[count] = (rest & 0x7F).to_le_bytes()[0];
        count += 1;
        rest >>= 7;
        if rest == 0 {
            break;
        }
    }
    for i in (0..count).rev() {
        let more = if i == 0 { 0 } else { 0x80 };
        out.push(groups[i] | more);
    }
}

/// `255UInt16`, for the test encoders.
pub fn put_u255(out: &mut Vec<u8>, value: u16) {
    match value {
        0..253 => out.push(value.to_be_bytes()[1]),
        253..506 => {
            out.push(255);
            out.push((value - 253).to_be_bytes()[1]);
        }
        506..762 => {
            out.push(254);
            out.push((value - 506).to_be_bytes()[1]);
        }
        _ => {
            out.push(253);
            put_u16(out, value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base128_round_trips_and_rejects_what_woff2_forbids() {
        for value in [0, 1, 127, 128, 16_383, 16_384, 1 << 28, u32::MAX] {
            let mut out = Vec::new();
            put_base128(&mut out, value);
            assert_eq!(Reader::new(&out).base128(), Ok(value), "{value}");
        }
        assert!(
            Reader::new(&[0x80, 0x01]).base128().is_err(),
            "leading zero"
        );
        assert!(
            Reader::new(&[0x90, 0x80, 0x80, 0x80, 0x00])
                .base128()
                .is_err(),
            "above 2^32 - 1"
        );
        assert!(
            Reader::new(&[0x81, 0x81, 0x81, 0x81, 0x81, 0x01])
                .base128()
                .is_err(),
            "six bytes"
        );
    }

    #[test]
    fn u255_round_trips_every_form() {
        for value in [0, 252, 253, 505, 506, 761, 762, 65_535] {
            let mut out = Vec::new();
            put_u255(&mut out, value);
            assert_eq!(Reader::new(&out).u255(), Ok(value), "{value}");
        }
        // The same number in each of its valid representations (WPT's
        // datatypes-alt-255uint16-001 tests this).
        assert_eq!(Reader::new(&[255, 253]).u255(), Ok(506));
        assert_eq!(Reader::new(&[254, 0]).u255(), Ok(506));
        assert_eq!(Reader::new(&[253, 1, 250]).u255(), Ok(506));
    }

    #[test]
    fn reads_past_the_end_fail() {
        let mut r = Reader::new(&[1, 2, 3]);
        assert_eq!(r.u16(), Ok(0x0102));
        assert!(r.u16().is_err());
        assert_eq!(r.u8(), Ok(3));
        assert!(r.take(usize::MAX).is_err());
    }

    #[test]
    fn checksum_pads_the_last_word() {
        assert_eq!(checksum(&[0, 0, 0, 1, 0, 0, 0, 2]), 3);
        assert_eq!(checksum(&[1]), 0x0100_0000);
    }
}
