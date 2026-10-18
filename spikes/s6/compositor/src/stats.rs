//! Summaries of per-frame samples.

/// The median, 95th percentile, maximum and mean of some samples.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Summary {
    pub(crate) n: usize,
    pub(crate) median: f64,
    pub(crate) p95: f64,
    pub(crate) max: f64,
    pub(crate) mean: f64,
}

impl Summary {
    pub(crate) fn of(samples: &[f64]) -> Self {
        if samples.is_empty() {
            return Self::default();
        }
        let mut sorted = samples.to_vec();
        sorted.sort_by(f64::total_cmp);
        let at = |q: f64| {
            let index = (q * to_f64(sorted.len() - 1)).round();
            sorted[to_index(index).min(sorted.len() - 1)]
        };
        Self {
            n: sorted.len(),
            median: at(0.5),
            p95: at(0.95),
            max: sorted[sorted.len() - 1],
            mean: sorted.iter().sum::<f64>() / to_f64(sorted.len()),
        }
    }

    /// `median/p95/max`, to `digits` decimals.
    pub(crate) fn show(&self, digits: usize) -> String {
        format!(
            "{:.d$}/{:.d$}/{:.d$}",
            self.median,
            self.p95,
            self.max,
            d = digits
        )
    }
}

fn to_f64(n: usize) -> f64 {
    f64::from(u32::try_from(n).unwrap_or(u32::MAX))
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "a rounded, non-negative index below the sample count"
)]
fn to_index(v: f64) -> usize {
    v.max(0.0) as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summaries_pick_the_right_order_statistics() {
        let samples: Vec<f64> = (1..=100).map(f64::from).collect();
        let s = Summary::of(&samples);
        assert_eq!((s.n, s.max), (100, 100.0));
        assert!((s.median - 51.0).abs() <= 1.0 && (s.p95 - 95.0).abs() <= 1.0);
        assert!((s.mean - 50.5).abs() < 1e-9);
        assert_eq!(Summary::of(&[]), Summary::default());
    }
}
