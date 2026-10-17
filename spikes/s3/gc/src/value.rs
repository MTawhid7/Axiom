//! NaN-boxed values: a JavaScript-shaped value in 64 bits, with `u32` handles
//! inside (ADR-0008 SH13).

use core::fmt;

/// Bits 63–51 all set: a negative quiet NaN. Arithmetic never produces one,
/// because [`Value::number`] canonicalizes NaN, so every boxed value is at or
/// above this.
const BOXED: u64 = 0xFFF8_0000_0000_0000;
const TAG_SHIFT: u32 = 48;
const TAG_INT: u64 = 1;
const TAG_MISC: u64 = 2;
const TAG_HEAP: u64 = 3;
const EPOCH_SHIFT: u32 = 32;
const EPOCH_MASK: u64 = 0xFFFF << EPOCH_SHIFT;
const CANONICAL_NAN: u64 = 0x7FF8_0000_0000_0000;
/// The upper 32 bits of every 32-bit integer.
const INT_PREFIX: u64 = (BOXED | (TAG_INT << TAG_SHIFT)) >> 32;
/// The upper 32 bits of every `TAG_MISC` value.
const MISC_PREFIX: u64 = (BOXED | (TAG_MISC << TAG_SHIFT)) >> 32;
/// `TAG_MISC` payloads from here on are builtin functions.
const BUILTIN_BASE: u32 = 256;

/// The most arenas one heap can have: tags 3–7.
pub const MAX_ARENAS: usize = 5;

/// A 64-bit value.
///
/// - **Doubles** are stored as their IEEE 754 bits. NaN has one canonical bit
///   pattern, so no double has its bits at or above the boxed range.
/// - **Everything else** is a NaN with the sign bit set: bits 63–51 are all
///   ones, bits 50–48 hold a tag, and bits 31–0 a payload.
///
/// | Tag | Meaning | Payload |
/// |---|---|---|
/// | 1 | a 32-bit integer | the `i32` |
/// | 2 | undefined, null, false, true; or a builtin function | 0–3; or 256 + the builtin's id |
/// | 3–7 | a reference into arena 0–4 of a heap | the handle's raw index (index + 1) |
///
/// Heap references carry the debug-build collection epoch of their handle in
/// bits 47–32 (always zero in release builds), so a stale value is caught like
/// a stale handle.
///
/// `==` compares bits: it is identity, not JavaScript equality.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct Value(u64);

impl Value {
    pub const UNDEFINED: Self = Self::misc(0);
    pub const NULL: Self = Self::misc(1);
    pub const FALSE: Self = Self::misc(2);
    pub const TRUE: Self = Self::misc(3);

    const fn misc(payload: u32) -> Self {
        Self(BOXED | (TAG_MISC << TAG_SHIFT) | payload as u64)
    }

    #[inline]
    #[must_use]
    pub const fn bool(b: bool) -> Self {
        if b { Self::TRUE } else { Self::FALSE }
    }

    #[inline]
    #[must_use]
    #[expect(
        clippy::cast_sign_loss,
        reason = "the i32's bits are the payload; as_int reverses this"
    )]
    pub const fn int(i: i32) -> Self {
        Self(BOXED | (TAG_INT << TAG_SHIFT) | i as u32 as u64)
    }

    /// A double, with NaN canonicalized.
    #[inline]
    #[must_use]
    pub fn number(f: f64) -> Self {
        if f.is_nan() {
            Self(CANONICAL_NAN)
        } else {
            Self(f.to_bits())
        }
    }

    /// A builtin function, by id.
    #[inline]
    #[must_use]
    pub const fn builtin(id: u8) -> Self {
        Self::misc(BUILTIN_BASE + id as u32)
    }

    #[inline]
    #[must_use]
    pub const fn from_bits(bits: u64) -> Self {
        Self(bits)
    }

    #[inline]
    #[must_use]
    pub const fn to_bits(self) -> u64 {
        self.0
    }

    /// Whether this is a double (not an integer: see [`as_f64`](Self::as_f64)).
    #[inline]
    #[must_use]
    pub const fn is_double(self) -> bool {
        self.0 < BOXED
    }

    #[inline]
    #[must_use]
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_possible_wrap,
        reason = "the payload is the i32's bits"
    )]
    pub const fn as_int(self) -> Option<i32> {
        if self.0 >> 32 == INT_PREFIX {
            Some(self.0 as u32 as i32)
        } else {
            None
        }
    }

    /// The numeric value of a double or an integer.
    #[inline]
    #[must_use]
    pub fn as_f64(self) -> Option<f64> {
        if self.is_double() {
            Some(f64::from_bits(self.0))
        } else {
            self.as_int().map(f64::from)
        }
    }

    #[inline]
    #[must_use]
    pub const fn as_bool(self) -> Option<bool> {
        match self.0 {
            x if x == Self::TRUE.0 => Some(true),
            x if x == Self::FALSE.0 => Some(false),
            _ => None,
        }
    }

    #[inline]
    #[must_use]
    #[expect(
        clippy::cast_possible_truncation,
        reason = "builtin payloads are 256 + a u8 id"
    )]
    pub const fn as_builtin(self) -> Option<u8> {
        let payload = self.0 as u32;
        if self.0 >> 32 == MISC_PREFIX && payload >= BUILTIN_BASE {
            Some((payload - BUILTIN_BASE) as u8)
        } else {
            None
        }
    }

    /// A reference into a heap: `(arena, raw index, epoch)`. The raw index is
    /// the handle's index plus one.
    #[inline]
    #[must_use]
    #[expect(
        clippy::cast_possible_truncation,
        reason = "payload and epoch are bit fields"
    )]
    pub const fn heap_ref(self) -> Option<(usize, u32, u16)> {
        let tag = (self.0 >> TAG_SHIFT) & 7;
        if self.0 >= BOXED && tag >= TAG_HEAP {
            Some((
                (tag - TAG_HEAP) as usize,
                self.0 as u32,
                (self.0 >> EPOCH_SHIFT) as u16,
            ))
        } else {
            None
        }
    }

    /// Release builds drop the epoch, so that equal references have equal bits.
    #[inline]
    pub(crate) const fn from_heap(arena: usize, raw: u32, epoch: u16) -> Self {
        debug_assert!(arena < MAX_ARENAS);
        let epoch = if cfg!(debug_assertions) { epoch } else { 0 };
        Self(
            BOXED
                | ((TAG_HEAP + arena as u64) << TAG_SHIFT)
                | ((epoch as u64) << EPOCH_SHIFT)
                | raw as u64,
        )
    }

    /// The bits without a heap reference's debug epoch: equal for two values
    /// that refer to the same object.
    #[inline]
    #[must_use]
    pub const fn identity(self) -> u64 {
        if self.heap_ref().is_some() {
            self.0 & !EPOCH_MASK
        } else {
            self.0
        }
    }
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(n) = self.as_int() {
            write!(f, "{n}")
        } else if self.is_double() {
            write!(f, "{:?}", f64::from_bits(self.0))
        } else if let Some((arena, raw, _)) = self.heap_ref() {
            write!(f, "@{arena}:{}", raw - 1)
        } else if let Some(id) = self.as_builtin() {
            write!(f, "builtin#{id}")
        } else {
            match *self {
                Self::UNDEFINED => f.write_str("undefined"),
                Self::NULL => f.write_str("null"),
                Self::FALSE => f.write_str("false"),
                Self::TRUE => f.write_str("true"),
                _ => write!(f, "Value({:#x})", self.0),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodings_round_trip_and_stay_distinct() {
        for i in [0, 1, -1, i32::MIN, i32::MAX] {
            assert_eq!(Value::int(i).as_int(), Some(i));
            assert!(!Value::int(i).is_double());
        }
        for x in [0.0, -0.0, 1.5, f64::INFINITY, f64::NEG_INFINITY, f64::MIN] {
            let v = Value::number(x);
            assert!(v.is_double());
            assert_eq!(v.as_f64().map(f64::to_bits), Some(x.to_bits()));
            assert_eq!(v.as_int(), None);
        }
        let nan = Value::number(-f64::NAN);
        assert!(nan.is_double() && nan.as_f64().is_some_and(f64::is_nan));
        assert_eq!(nan, Value::number(f64::NAN), "NaN is canonical");
        assert_eq!(Value::builtin(7).as_builtin(), Some(7));
        assert_eq!(Value::TRUE.as_builtin(), None);
        assert_eq!(Value::TRUE.as_bool(), Some(true));
        assert_eq!(Value::int(1).as_bool(), None);
        let r = Value::from_heap(4, 9, 3);
        assert_eq!(r.heap_ref(), Some((4, 9, 3)));
        assert_eq!(r.identity(), Value::from_heap(4, 9, 0).identity());
        for v in [
            Value::UNDEFINED,
            Value::NULL,
            Value::int(5),
            Value::number(2.0),
        ] {
            assert_eq!(v.heap_ref(), None);
        }
    }
}
