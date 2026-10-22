//! Bounded UTF-8 text in records (OB3): packed eight bytes to a little-endian
//! word after a length word, and decoded into a fixed buffer, so neither side
//! allocates.

use std::fmt;

use super::MAX_TEXT;

/// Decoded text of at most [`MAX_TEXT`] bytes, valid UTF-8.
#[derive(Clone, PartialEq, Eq)]
pub struct Text {
    bytes: [u8; MAX_TEXT],
    len: usize,
}

impl Text {
    /// Copies `bytes` if they are valid UTF-8 and fit; the decoder's check.
    pub(crate) fn from_utf8(bytes: &[u8]) -> Option<Self> {
        if bytes.len() > MAX_TEXT || std::str::from_utf8(bytes).is_err() {
            return None;
        }
        let mut text = Self {
            bytes: [0; MAX_TEXT],
            len: bytes.len(),
        };
        for (slot, byte) in text.bytes.iter_mut().zip(bytes) {
            *slot = *byte;
        }
        Some(text)
    }

    /// The text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.bytes
            .get(..self.len)
            .and_then(|bytes| std::str::from_utf8(bytes).ok())
            .unwrap_or("")
    }
}

impl fmt::Debug for Text {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.as_str(), f)
    }
}

/// `text`, cut to at most `max` bytes on a character boundary.
pub(crate) fn truncate(text: &str, max: usize) -> &str {
    let mut end = text.len().min(max);
    while !text.is_char_boundary(end) {
        end = end.saturating_sub(1);
    }
    text.get(..end).unwrap_or("")
}

/// The words `bytes` occupies after its length word.
pub(crate) fn packed_words(bytes: usize) -> usize {
    bytes.div_ceil(8)
}

/// Writes `text`'s length and its packed bytes into `out`, returning how many
/// words they took. `out` must have room; the encoders size it.
pub(crate) fn pack(text: &str, out: &mut [u64]) -> usize {
    let bytes = text.as_bytes();
    let mut slots = out.iter_mut();
    if let Some(slot) = slots.next() {
        *slot = u64::try_from(bytes.len()).unwrap_or(0);
    }
    for (slot, chunk) in slots.zip(bytes.chunks(8)) {
        let mut word = [0_u8; 8];
        for (byte, value) in word.iter_mut().zip(chunk) {
            *byte = *value;
        }
        *slot = u64::from_le_bytes(word);
    }
    packed_words(bytes.len()).saturating_add(1)
}

/// Reads one packed text from the front of `words`, at most `max` bytes, and
/// returns it with the words it took; `None` if it is too long, runs past the
/// words, has nonzero padding or isn't UTF-8.
pub(crate) fn unpack(words: &[u64], max: usize) -> Option<(Text, usize)> {
    let (&len_word, rest) = words.split_first()?;
    let len = usize::try_from(len_word).ok().filter(|len| *len <= max)?;
    let used = packed_words(len);
    let packed = rest.get(..used)?;
    let mut bytes = [0_u8; MAX_TEXT];
    let mut filled = 0_usize;
    for word in packed {
        for byte in word.to_le_bytes() {
            if filled < len {
                *bytes.get_mut(filled)? = byte;
            } else if byte != 0 {
                return None; // Padding is zero: one encoding per text.
            }
            filled = filled.saturating_add(1);
        }
    }
    let text = Text::from_utf8(bytes.get(..len)?)?;
    Some((text, used.saturating_add(1)))
}
