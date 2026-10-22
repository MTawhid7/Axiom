//! Percentiles of a sample, in the units given.

/// The median, 95th and 99th percentiles and the maximum, by nearest rank.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Summary {
    pub(crate) median: f64,
    pub(crate) p95: f64,
    pub(crate) p99: f64,
    pub(crate) max: f64,
}

pub(crate) fn summarize(values: &[f64]) -> Summary {
    if values.is_empty() {
        return Summary::default();
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let rank = |p: usize| {
        sorted[((sorted.len() * p).div_ceil(100))
            .saturating_sub(1)
            .min(sorted.len() - 1)]
    };
    Summary {
        median: rank(50),
        p95: rank(95),
        p99: rank(99),
        max: sorted[sorted.len() - 1],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nearest_rank() {
        let values: Vec<f64> = (1..=100).map(f64::from).collect();
        let s = summarize(&values);
        assert_eq!((s.median, s.p95, s.p99, s.max), (50.0, 95.0, 99.0, 100.0));
    }
}
