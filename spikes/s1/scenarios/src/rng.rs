//! Small, fast, deterministic PRNG for workload generation. It is not cryptographic.
//!
//! The generator is xoshiro256++ (Blackman & Vigna), seeded through `SplitMix64`.
//! Bounded sampling uses Lemire's nearly-divisionless method, which is unbiased.
//!
//! It is written here instead of depending on `rand`, so that generated scripts
//! (and therefore benchmark workloads) stay bit-identical across dependency
//! upgrades.

use core::ops::Range;

/// xoshiro256++ generator.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rng {
    s: [u64; 4],
}

impl Rng {
    /// Seeds the generator. The state is expanded with `SplitMix64`, so similar
    /// seeds still give unrelated streams.
    #[must_use]
    pub fn seed_from_u64(seed: u64) -> Self {
        let mut sm = SplitMix64(seed);
        Self {
            s: [sm.next(), sm.next(), sm.next(), sm.next()],
        }
    }

    /// Next 64 uniformly distributed bits.
    #[inline]
    pub fn next_u64(&mut self) -> u64 {
        let s = &mut self.s;
        let result = s[0].wrapping_add(s[3]).rotate_left(23).wrapping_add(s[0]);
        let t = s[1] << 17;
        s[2] ^= s[0];
        s[3] ^= s[1];
        s[1] ^= s[2];
        s[0] ^= s[3];
        s[2] ^= t;
        s[3] = s[3].rotate_left(45);
        result
    }

    /// Next 32 uniformly distributed bits, taken from the high half (the strongest
    /// bits of xoshiro256++).
    #[inline]
    pub fn next_u32(&mut self) -> u32 {
        high_u32(self.next_u64())
    }

    /// Uniform sample from `0..n`.
    ///
    /// # Panics
    ///
    /// Panics if `n == 0`.
    #[inline]
    pub fn below(&mut self, n: u32) -> u32 {
        assert!(n > 0, "Rng::below requires a non-empty range");
        // Lemire, "Fast Random Integer Generation in an Interval" (2019).
        let mut m = u64::from(self.next_u32()) * u64::from(n);
        if low_u32(m) < n {
            let threshold = n.wrapping_neg() % n;
            while low_u32(m) < threshold {
                m = u64::from(self.next_u32()) * u64::from(n);
            }
        }
        high_u32(m)
    }

    /// Uniform sample from a half-open range, such as `rng.range(2..9)`.
    ///
    /// # Panics
    ///
    /// Panics if the range is empty.
    #[inline]
    pub fn range(&mut self, range: Range<u32>) -> u32 {
        assert!(
            range.start < range.end,
            "Rng::range requires a non-empty range"
        );
        range.start + self.below(range.end - range.start)
    }

    /// `true` with probability `numerator / denominator`.
    ///
    /// # Panics
    ///
    /// Panics if `denominator == 0`.
    #[inline]
    pub fn ratio(&mut self, numerator: u32, denominator: u32) -> bool {
        debug_assert!(numerator <= denominator, "probability above 1");
        self.below(denominator) < numerator
    }

    /// A uniformly chosen element of a non-empty slice.
    ///
    /// # Panics
    ///
    /// Panics if `items` is empty or has more than `u32::MAX` elements.
    #[inline]
    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        let len = u32::try_from(items.len()).expect("slice too long for Rng::pick");
        &items[self.below(len) as usize]
    }
}

#[inline]
#[expect(
    clippy::cast_possible_truncation,
    reason = "deliberately keeps the low 32 bits"
)]
const fn low_u32(x: u64) -> u32 {
    x as u32
}

#[inline]
const fn high_u32(x: u64) -> u32 {
    (x >> 32) as u32
}

/// `SplitMix64` (Steele, Lea & Flood), used only to expand seeds.
struct SplitMix64(u64);

impl SplitMix64 {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splitmix64_matches_reference_vector() {
        // First output of the reference SplitMix64 implementation seeded with 0.
        assert_eq!(SplitMix64(0).next(), 0xE220_A839_7B1D_CDAF);
    }

    #[test]
    fn same_seed_same_stream_different_seed_different_stream() {
        let mut a = Rng::seed_from_u64(42);
        let mut b = Rng::seed_from_u64(42);
        let mut c = Rng::seed_from_u64(43);
        let xs: Vec<u64> = (0..64).map(|_| a.next_u64()).collect();
        let ys: Vec<u64> = (0..64).map(|_| b.next_u64()).collect();
        let zs: Vec<u64> = (0..64).map(|_| c.next_u64()).collect();
        assert_eq!(xs, ys);
        assert_ne!(xs, zs);
    }

    #[test]
    fn below_respects_bounds_including_extremes() {
        let mut rng = Rng::seed_from_u64(7);
        for n in [1, 2, 3, 7, 10, 1000, u32::MAX - 1, u32::MAX] {
            for _ in 0..1000 {
                assert!(rng.below(n) < n);
            }
        }
        assert_eq!(rng.below(1), 0);
    }

    #[test]
    fn below_is_roughly_uniform() {
        // Deterministic (fixed seed), so this test cannot flake.
        const BUCKETS: u32 = 10;
        const SAMPLES: u32 = 200_000;
        let mut rng = Rng::seed_from_u64(0xA5B0);
        let mut counts = [0u32; BUCKETS as usize];
        for _ in 0..SAMPLES {
            counts[rng.below(BUCKETS) as usize] += 1;
        }
        let expected = f64::from(SAMPLES / BUCKETS);
        for c in counts {
            let deviation = (f64::from(c) - expected).abs() / expected;
            assert!(
                deviation < 0.02,
                "bucket deviates {deviation:.4} from uniform: {counts:?}"
            );
        }
    }

    #[test]
    fn range_and_pick_stay_in_bounds() {
        let mut rng = Rng::seed_from_u64(1);
        let items = ["a", "b", "c"];
        for _ in 0..1000 {
            let r = rng.range(5..9);
            assert!((5..9).contains(&r));
            assert!(items.contains(rng.pick(&items)));
        }
    }

    #[test]
    #[should_panic(expected = "non-empty range")]
    fn below_zero_panics() {
        Rng::seed_from_u64(0).below(0);
    }
}
