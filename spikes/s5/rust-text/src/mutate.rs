//! Deterministic malformed variants of valid font files (S5-Q3).
//!
//! Most mutations land inside one table's bytes, so that they pass the
//! container's checks and reach the transforms and the parsers behind them:
//! - WOFF 1.0 files are re-encoded with every table stored uncompressed;
//! - WOFF2 files are changed in their decompressed table stream and rewrapped
//!   with a stored Brotli stream (which also reaches the `glyf` transform);
//! - bare fonts are changed in place.
//!
//! The rest change the file anywhere: bit flips, overwrites with boundary
//! values, truncation, and deleted or duplicated chunks.

use crate::sfnt;
use crate::webfont::{Kind, sniff};
use crate::{Limits, woff, woff2};

/// xorshift64*: small, fast and reproducible from its seed.
#[derive(Clone, Debug)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed.max(1))
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// A number in `0..n`, for `n > 0`.
    pub fn below(&mut self, n: usize) -> usize {
        // The modulo bias is irrelevant here.
        usize::try_from(self.next_u64() % n as u64).unwrap_or(0)
    }
}

/// One variant and how it was made.
#[derive(Clone, Debug)]
pub struct Mutant {
    pub strategy: &'static str,
    pub data: Vec<u8>,
}

/// Bytes that often sit on a boundary of some field.
const BOUNDARY: [u8; 6] = [0x00, 0x01, 0x7F, 0x80, 0xFE, 0xFF];

/// `count` variants of `original`, reproducible from `seed`.
pub fn mutants(original: &[u8], seed: u64, count: usize, limits: &Limits) -> Vec<Mutant> {
    let mut rng = Rng::new(seed);
    let target = TableTarget::new(original, limits);
    (0..count)
        .map(|_| {
            if let Some(target) = &target
                && rng.below(10) < 6
            {
                target.mutate(&mut rng)
            } else {
                whole_file(original, &mut rng)
            }
        })
        .collect()
}

/// A file whose tables can be changed in their decoded form.
enum TableTarget {
    /// A container (or bare font) with every table stored as is, and the
    /// byte ranges of its tables.
    Stored {
        file: Vec<u8>,
        ranges: Vec<(usize, usize)>,
        strategy: &'static str,
    },
    /// A WOFF2 file, its directory and its decompressed stream.
    Woff2 {
        original: Vec<u8>,
        directory: woff2::Directory,
        stream: Vec<u8>,
    },
}

impl TableTarget {
    fn new(original: &[u8], limits: &Limits) -> Option<Self> {
        match sniff(original)? {
            Kind::Woff => {
                let font = woff::decode(original, limits).ok()?;
                let file = woff::encode(&font, false).ok()?;
                let ranges = woff_ranges(&file)?;
                Some(Self::Stored {
                    file,
                    ranges,
                    strategy: "woff-table",
                })
            }
            Kind::Woff2 => {
                let (directory, stream) = woff2::split(original, limits).ok()?;
                Some(Self::Woff2 {
                    original: original.to_vec(),
                    directory,
                    stream,
                })
            }
            Kind::Sfnt => {
                let (_, records) = sfnt::read(original).ok()?;
                let ranges = records.iter().map(|r| (r.offset, r.len)).collect();
                Some(Self::Stored {
                    file: original.to_vec(),
                    ranges,
                    strategy: "sfnt-table",
                })
            }
            Kind::Collection => None,
        }
    }

    fn mutate(&self, rng: &mut Rng) -> Mutant {
        match self {
            Self::Stored {
                file,
                ranges,
                strategy,
            } => {
                let mut data = file.clone();
                if let Some(&(offset, len)) = pick(ranges, rng) {
                    corrupt(&mut data[offset..offset + len], rng);
                }
                Mutant { strategy, data }
            }
            Self::Woff2 {
                original,
                directory,
                stream,
            } => {
                let mut stream = stream.clone();
                let ranges: Vec<(usize, usize)> = directory
                    .entries
                    .iter()
                    .map(|e| (e.stream_offset, e.stream_len as usize))
                    .collect();
                if let Some(&(offset, len)) = pick(&ranges, rng) {
                    corrupt(&mut stream[offset..offset + len], rng);
                }
                Mutant {
                    strategy: "woff2-stream",
                    data: woff2::rewrap(original, directory, &stream),
                }
            }
        }
    }
}

/// The byte ranges of a stored WOFF 1.0 file's tables, from its directory.
fn woff_ranges(file: &[u8]) -> Option<Vec<(usize, usize)>> {
    let mut r = crate::bytes::Reader::new(file);
    r.skip(12).ok()?;
    let count = r.u16().ok()?;
    r.skip(30).ok()?;
    let mut ranges = Vec::new();
    for _ in 0..count {
        r.skip(4).ok()?;
        let offset = r.u32().ok()? as usize;
        let comp_len = r.u32().ok()? as usize;
        r.skip(8).ok()?;
        ranges.push((offset, comp_len));
    }
    Some(ranges)
}

fn pick<'a>(ranges: &'a [(usize, usize)], rng: &mut Rng) -> Option<&'a (usize, usize)> {
    let nonempty: Vec<&(usize, usize)> = ranges.iter().filter(|r| r.1 > 0).collect();
    if nonempty.is_empty() {
        None
    } else {
        Some(nonempty[rng.below(nonempty.len())])
    }
}

/// Change a few bytes of `bytes`, keeping its length.
fn corrupt(bytes: &mut [u8], rng: &mut Rng) {
    let edits = 1 + rng.below(8);
    for _ in 0..edits {
        let at = rng.below(bytes.len());
        bytes[at] = match rng.below(3) {
            0 => bytes[at] ^ (1 << rng.below(8)),
            1 => BOUNDARY[rng.below(BOUNDARY.len())],
            _ => rng.next_u64().to_le_bytes()[0],
        };
    }
}

fn whole_file(original: &[u8], rng: &mut Rng) -> Mutant {
    let mut data = original.to_vec();
    if data.is_empty() {
        return Mutant {
            strategy: "empty",
            data,
        };
    }
    let strategy = match rng.below(5) {
        0 => {
            corrupt(&mut data, rng);
            "file-bytes"
        }
        1 => {
            let at = rng.below(data.len());
            data.truncate(at);
            "truncate"
        }
        2 => {
            let at = rng.below(data.len());
            let len = 1 + rng.below((data.len() - at).min(64));
            data.drain(at..at + len);
            "delete-chunk"
        }
        3 => {
            let at = rng.below(data.len());
            let len = 1 + rng.below((data.len() - at).min(64));
            let chunk: Vec<u8> = data[at..at + len].to_vec();
            data.splice(at..at, chunk);
            "duplicate-chunk"
        }
        _ => {
            // A header field: one of the first 64 bytes' 16-bit words.
            let words = data.len().min(64) / 2;
            if words > 0 {
                let at = 2 * rng.below(words);
                let value = BOUNDARY[rng.below(BOUNDARY.len())];
                data[at] = value;
                data[at + 1] = value;
            }
            "header-field"
        }
    };
    Mutant { strategy, data }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testfont;
    use crate::webfont::check;

    #[test]
    fn mutants_are_reproducible() {
        let font = testfont::square_font();
        let woff2 = woff2::encode_stored(&font).expect("encodes");
        let a = mutants(&woff2, 7, 20, &Limits::default());
        let b = mutants(&woff2, 7, 20, &Limits::default());
        assert!(a.iter().zip(&b).all(|(x, y)| x.data == y.data));
    }

    /// A miniature of S5-Q3's mutation run: every variant of every container
    /// ends as an acceptance or a rejection, never a panic.
    #[test]
    fn every_mutant_is_accepted_or_rejected() {
        let font = testfont::square_font();
        let limits = Limits::default();
        for file in [
            font.clone(),
            woff::encode(&font, true).expect("encodes"),
            woff2::encode_stored(&font).expect("encodes"),
        ] {
            for mutant in mutants(&file, 42, 300, &limits) {
                let _ = check(&mutant.data, &limits);
            }
        }
    }
}
