//! Histograms' fixed log-linear buckets (OB8): four buckets per power of two,
//! so a sample's bucket is within 25% of it, and a `u64` needs 252 of them.

/// The number of buckets.
pub const BUCKETS: usize = 252;

/// Sub-buckets per power of two, as a shift: 2² = 4.
const SUB_BITS: u32 = 2;

/// The bucket a sample falls in: samples 0 to 3 have their own; above them,
/// each power of two splits into four.
#[must_use]
pub fn bucket(sample: u64) -> u8 {
    if sample < 4 {
        return u8::try_from(sample).unwrap_or(u8::MAX);
    }
    // The exponent is at least 2 here, and at most 63.
    let exponent = 63_u32.wrapping_sub(sample.leading_zeros());
    let mantissa = sample.wrapping_shr(exponent.wrapping_sub(SUB_BITS)) & 3;
    let index = u64::from(exponent.wrapping_sub(1)).wrapping_shl(SUB_BITS) | mantissa;
    u8::try_from(index).unwrap_or(u8::MAX)
}

/// The smallest sample in `bucket`.
#[must_use]
pub fn bucket_floor(bucket: u8) -> u64 {
    if bucket < 4 {
        return u64::from(bucket);
    }
    let exponent = u32::from(bucket).wrapping_shr(SUB_BITS).wrapping_add(1);
    let mantissa = u64::from(bucket) & 3;
    (4 | mantissa).wrapping_shl(exponent.wrapping_sub(SUB_BITS))
}

#[cfg(test)]
mod tests {
    use super::{BUCKETS, bucket, bucket_floor};

    #[test]
    fn small_samples_have_their_own_buckets() {
        for sample in 0..8 {
            assert_eq!(u64::from(bucket(sample)), sample);
        }
    }

    #[test]
    fn every_bucket_starts_at_its_floor_and_holds_samples_up_to_the_next() {
        let mut previous = None;
        for index in 0..u8::try_from(BUCKETS).unwrap() {
            let floor = bucket_floor(index);
            assert_eq!(bucket(floor), index, "bucket {index} starts at {floor}");
            if let Some(previous) = previous {
                assert!(floor > previous);
                assert_eq!(bucket(floor - 1), index - 1);
            }
            previous = Some(floor);
        }
        assert_eq!(usize::from(bucket(u64::MAX)), BUCKETS - 1);
    }

    #[test]
    fn a_bucket_is_within_a_quarter_of_its_samples() {
        for sample in [5_u64, 99, 1_000, 123_456_789, u64::MAX / 3] {
            let floor = bucket_floor(bucket(sample));
            assert!(
                floor <= sample && sample - floor <= floor / 4,
                "{sample} in [{floor}, …)"
            );
        }
    }
}
