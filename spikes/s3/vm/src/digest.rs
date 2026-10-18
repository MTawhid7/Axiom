//! A canonical digest of a value and everything it reaches, so that the
//! interpreter's results can be compared with the reference evaluator's and
//! with native Rust models.
//!
//! The walk is depth-first in slot and element order. An object, array or
//! closure seen before is encoded as a back reference to the order in which it
//! was first seen, so cycles and sharing are part of the digest. Numbers are
//! encoded as doubles, so an integer and the equal double digest alike, while
//! `-0` and `+0` differ.

/// FNV-1a over a stream of tagged items.
#[derive(Clone, Copy, Debug)]
pub struct Digest(u64);

const NUMBER: u8 = 1;
const UNDEFINED: u8 = 2;
const NULL: u8 = 3;
const BOOLEAN: u8 = 4;
const BUILTIN: u8 = 5;
const BACKREF: u8 = 6;
const OBJECT: u8 = 7;
const ARRAY: u8 = 8;
const CLOSURE: u8 = 9;

impl Default for Digest {
    fn default() -> Self {
        Self(0xCBF2_9CE4_8422_2325)
    }
}

impl Digest {
    fn bytes(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.0 = (self.0 ^ u64::from(b)).wrapping_mul(0x0000_0100_0000_01B3);
        }
    }

    fn tagged(&mut self, tag: u8, payload: u64) {
        self.bytes(&[tag]);
        self.bytes(&payload.to_le_bytes());
    }

    pub fn number(&mut self, x: f64) {
        let bits = if x.is_nan() {
            f64::NAN.to_bits()
        } else {
            x.to_bits()
        };
        self.tagged(NUMBER, bits);
    }

    pub fn undefined(&mut self) {
        self.tagged(UNDEFINED, 0);
    }

    pub fn null(&mut self) {
        self.tagged(NULL, 0);
    }

    pub fn boolean(&mut self, b: bool) {
        self.tagged(BOOLEAN, u64::from(b));
    }

    pub fn builtin(&mut self, id: u8) {
        self.tagged(BUILTIN, u64::from(id));
    }

    /// An object, array or closure seen before: the `n`th first seen.
    pub fn backref(&mut self, n: usize) {
        self.tagged(BACKREF, n as u64);
    }

    /// An object; its four slots follow.
    pub fn object(&mut self) {
        self.tagged(OBJECT, 4);
    }

    /// An array of `len` elements, which follow.
    pub fn array(&mut self, len: usize) {
        self.tagged(ARRAY, len as u64);
    }

    /// A closure of `function`; its environment follows.
    pub fn closure(&mut self, function: u32) {
        self.tagged(CLOSURE, u64::from(function));
    }

    #[must_use]
    pub const fn finish(self) -> u64 {
        self.0
    }

    /// The digest of an array of numbers: how native models state their
    /// expected results.
    #[must_use]
    pub fn of_numbers(numbers: &[f64]) -> u64 {
        let mut d = Self::default();
        d.array(numbers.len());
        for &x in numbers {
            d.number(x);
        }
        d.finish()
    }

    /// The digest of one number.
    #[must_use]
    pub fn of_number(x: f64) -> u64 {
        let mut d = Self::default();
        d.number(x);
        d.finish()
    }
}
