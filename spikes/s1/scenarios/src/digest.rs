//! Dependency-free digests: FNV-1a and the canonical tree encoding that every
//! driver uses to report its DOM tree.

use crate::script::{AttrName, NodeId, Tag};

/// 64-bit FNV-1a.
///
/// Chosen because it is stable across platforms and releases, has no
/// dependencies, and is adequate for equality digests of non-adversarial
/// data. It is not collision resistant and must never be used for anything
/// security-relevant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fnv1a(u64);

impl Fnv1a {
    const OFFSET_BASIS: u64 = 0xCBF2_9CE4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01B3;

    /// A fresh hasher.
    #[must_use]
    pub const fn new() -> Self {
        Self(Self::OFFSET_BASIS)
    }

    /// Feed raw bytes.
    #[inline]
    pub fn write_bytes(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.0 ^= u64::from(b);
            self.0 = self.0.wrapping_mul(Self::PRIME);
        }
    }

    /// Feed one byte.
    #[inline]
    pub fn write_u8(&mut self, x: u8) {
        self.write_bytes(&[x]);
    }

    /// Feed a `u32`, little-endian.
    #[inline]
    pub fn write_u32(&mut self, x: u32) {
        self.write_bytes(&x.to_le_bytes());
    }

    /// Feed a `u64`, little-endian.
    #[inline]
    pub fn write_u64(&mut self, x: u64) {
        self.write_bytes(&x.to_le_bytes());
    }

    /// Feed a string, length-prefixed so that concatenations can't collide.
    #[inline]
    pub fn write_str(&mut self, s: &str) {
        self.write_u64(s.len() as u64);
        self.write_bytes(s.as_bytes());
    }

    /// The digest.
    #[must_use]
    pub const fn finish(self) -> u64 {
        self.0
    }
}

impl Default for Fnv1a {
    fn default() -> Self {
        Self::new()
    }
}

impl core::hash::Hasher for Fnv1a {
    fn write(&mut self, bytes: &[u8]) {
        self.write_bytes(bytes);
    }

    fn finish(&self) -> u64 {
        self.0
    }
}

/// Canonical, unambiguous encoding of a DOM tree, fed in pre-order.
///
/// Each event writes a distinct marker byte, and variable-length data is
/// length-prefixed. Two drivers therefore produce equal digests exactly when
/// they report equal trees (up to FNV collisions). The things that are encoded:
/// - each node's kind, tag or text, and scenario id;
/// - attributes, in order;
/// - children, in order.
///
/// Every [`begin_element`](Self::begin_element) must be matched by an
/// [`end_element`](Self::end_element).
#[derive(Clone, Debug, Default)]
pub struct TreeDigest {
    hash: Fnv1a,
    open: u32,
}

impl TreeDigest {
    const ELEMENT: u8 = b'<';
    const ATTRIBUTE: u8 = b'@';
    const TEXT: u8 = b'"';
    const END: u8 = b'>';

    /// An empty digest.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            hash: Fnv1a::new(),
            open: 0,
        }
    }

    /// Open an element. Its attributes, then its children, follow.
    pub fn begin_element(&mut self, tag: Tag, id: Option<NodeId>) {
        self.hash.write_u8(Self::ELEMENT);
        self.hash.write_str(tag.as_str());
        self.write_id(id);
        self.open += 1;
    }

    /// An attribute of the innermost open element.
    pub fn attribute(&mut self, name: AttrName, value: &str) {
        debug_assert!(self.open > 0, "attribute outside an element");
        self.hash.write_u8(Self::ATTRIBUTE);
        self.hash.write_str(name.as_str());
        self.hash.write_str(value);
    }

    /// A text node (always a leaf).
    pub fn text(&mut self, id: Option<NodeId>, data: &str) {
        self.hash.write_u8(Self::TEXT);
        self.write_id(id);
        self.hash.write_str(data);
    }

    /// Close the innermost open element.
    ///
    /// # Panics
    ///
    /// Panics if no element is open.
    pub fn end_element(&mut self) {
        self.open = self
            .open
            .checked_sub(1)
            .expect("end_element without a matching begin_element");
        self.hash.write_u8(Self::END);
    }

    /// The digest of the whole tree.
    ///
    /// # Panics
    ///
    /// Panics if an element is still open.
    #[must_use]
    pub fn finish(self) -> u64 {
        assert_eq!(self.open, 0, "unbalanced tree digest");
        self.hash.finish()
    }

    fn write_id(&mut self, id: Option<NodeId>) {
        match id {
            Some(id) => {
                self.hash.write_u8(1);
                self.hash.write_u32(id.0);
            }
            None => self.hash.write_u8(0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fnv(s: &str) -> u64 {
        let mut h = Fnv1a::new();
        h.write_bytes(s.as_bytes());
        h.finish()
    }

    #[test]
    fn fnv1a_matches_reference_vectors() {
        // Test vectors from the FNV reference implementation (64-bit FNV-1a).
        assert_eq!(fnv(""), 0xCBF2_9CE4_8422_2325);
        assert_eq!(fnv("a"), 0xAF63_DC4C_8601_EC8C);
        assert_eq!(fnv("foobar"), 0x8594_4171_F739_67E8);
    }

    fn sample(swap_children: bool, attr_value: &str) -> u64 {
        let mut d = TreeDigest::new();
        d.begin_element(Tag::Document, Some(NodeId::DOCUMENT));
        d.begin_element(Tag::Div, Some(NodeId(1)));
        d.attribute(AttrName::Class, attr_value);
        if swap_children {
            d.text(None, "b");
            d.text(Some(NodeId(2)), "a");
        } else {
            d.text(Some(NodeId(2)), "a");
            d.text(None, "b");
        }
        d.end_element();
        d.end_element();
        d.finish()
    }

    #[test]
    fn digest_is_deterministic_and_order_sensitive() {
        assert_eq!(sample(false, "x"), sample(false, "x"));
        assert_ne!(sample(false, "x"), sample(true, "x"));
        assert_ne!(sample(false, "x"), sample(false, "y"));
    }

    #[test]
    fn length_prefixing_prevents_concatenation_collisions() {
        let mut a = TreeDigest::new();
        a.text(None, "ab");
        a.text(None, "c");
        let mut b = TreeDigest::new();
        b.text(None, "a");
        b.text(None, "bc");
        assert_ne!(a.finish(), b.finish());
    }

    #[test]
    #[should_panic(expected = "unbalanced")]
    fn unbalanced_digest_panics() {
        let mut d = TreeDigest::new();
        d.begin_element(Tag::Div, None);
        let _ = d.finish();
    }
}
