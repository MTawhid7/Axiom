//! S1's benchmark harness, generalized from two patterns to any number of
//! variants.
//!
//! - Warm-up rounds run first, for a minimum *time*, and are discarded:
//!   Apple Silicon raises a core's clock only after sustained load.
//! - Samples rotate through the variants, reversing the order every other
//!   round (for two variants, S1's ABBA), so slow drift falls on all of them.
//! - A sample times one call. Setup and teardown are excluded.
//! - Reports use the median, with the minimum and maximum as the spread.
//!   Across runs, [`aggregate`] reports the median of the run medians and
//!   their range.

use core::fmt::{self, Write as _};
use core::hint::black_box;
use std::time::{Duration, Instant};

/// How long to warm up, and how many samples to take.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sampling {
    pub warmup: Duration,
    pub samples: usize,
}

/// The durations of one measurement's samples, sorted ascending.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Samples(Vec<Duration>);

impl Samples {
    /// # Panics
    /// If `samples` is empty.
    #[must_use]
    pub fn new(mut samples: Vec<Duration>) -> Self {
        assert!(!samples.is_empty(), "a measurement needs a sample");
        samples.sort_unstable();
        Self(samples)
    }

    /// The middle sample (the upper middle for an even count).
    #[must_use]
    pub fn median(&self) -> Duration {
        self.0[self.0.len() / 2]
    }

    #[must_use]
    pub fn min(&self) -> Duration {
        self.0[0]
    }

    #[must_use]
    pub fn max(&self) -> Duration {
        self.0[self.0.len() - 1]
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// Run `f` and return how long it took. Its result is dropped after the
/// clock stops.
pub fn timed<R>(f: impl FnOnce() -> R) -> Duration {
    let start = Instant::now();
    let result = black_box(f());
    let elapsed = start.elapsed();
    drop(result);
    elapsed
}

/// Sample each of `variants` in rotation, after discarded warm-up rounds.
pub fn rotate(sampling: Sampling, variants: &mut [&mut dyn FnMut() -> Duration]) -> Vec<Samples> {
    let warmup = Instant::now();
    loop {
        for v in variants.iter_mut() {
            v();
        }
        if warmup.elapsed() >= sampling.warmup {
            break;
        }
    }
    let n = variants.len();
    let mut samples = vec![Vec::with_capacity(sampling.samples); n];
    for round in 0..sampling.samples {
        for k in 0..n {
            let k = if round % 2 == 0 { k } else { n - 1 - k };
            samples[k].push(variants[k]());
        }
    }
    samples.into_iter().map(Samples::new).collect()
}

/// One benchmark result: a summary of its samples, plus extra facts.
#[derive(Clone, Debug, PartialEq)]
pub struct Measurement {
    pub suite: String,
    pub case: String,
    pub variant: String,
    /// Work items per timed call (operations, dispatches, nodes …).
    pub items: u64,
    pub unit: String,
    pub samples: usize,
    pub median: Duration,
    pub min: Duration,
    pub max: Duration,
    /// Facts that are not timings, as `key=value` pairs.
    pub extra: Vec<(String, String)>,
}

impl Measurement {
    pub const TSV_HEADER: &'static str =
        "suite\tcase\tvariant\titems\tunit\tsamples\tmedian_ns\tmin_ns\tmax_ns\tper_item_ns\textra";

    #[must_use]
    pub fn new(
        suite: &str,
        case: &str,
        variant: &str,
        items: u64,
        unit: &str,
        samples: &Samples,
    ) -> Self {
        Self {
            suite: suite.to_owned(),
            case: case.to_owned(),
            variant: variant.to_owned(),
            items,
            unit: unit.to_owned(),
            samples: samples.len(),
            median: samples.median(),
            min: samples.min(),
            max: samples.max(),
            extra: Vec::new(),
        }
    }

    /// Add an extra fact.
    #[must_use]
    pub fn with(mut self, key: &str, value: impl fmt::Display) -> Self {
        self.extra.push((key.to_owned(), value.to_string()));
        self
    }

    /// The median time per work item, in nanoseconds.
    #[must_use]
    #[expect(clippy::cast_precision_loss, reason = "item counts are far below 2^52")]
    pub fn per_item_ns(&self) -> f64 {
        self.median.as_secs_f64() * 1e9 / self.items.max(1) as f64
    }

    #[must_use]
    pub fn to_tsv(&self) -> String {
        let extra = self
            .extra
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect::<Vec<_>>()
            .join(" ");
        format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{:.3}\t{}",
            self.suite,
            self.case,
            self.variant,
            self.items,
            self.unit,
            self.samples,
            self.median.as_nanos(),
            self.min.as_nanos(),
            self.max.as_nanos(),
            self.per_item_ns(),
            extra,
        )
    }

    /// Parse one line written by [`to_tsv`](Self::to_tsv).
    pub fn from_tsv(line: &str) -> Result<Self, String> {
        let fields: Vec<&str> = line.split('\t').collect();
        let [
            suite,
            case,
            variant,
            items,
            unit,
            samples,
            median,
            min,
            max,
            _per_item,
            extra,
        ] = fields[..]
        else {
            return Err(format!("expected 11 tab-separated fields: {line:?}"));
        };
        let int = |s: &str| s.parse::<u64>().map_err(|e| format!("{s:?}: {e}"));
        let nanos = |s: &str| int(s).map(Duration::from_nanos);
        Ok(Self {
            suite: suite.to_owned(),
            case: case.to_owned(),
            variant: variant.to_owned(),
            items: int(items)?,
            unit: unit.to_owned(),
            samples: usize::try_from(int(samples)?).map_err(|e| e.to_string())?,
            median: nanos(median)?,
            min: nanos(min)?,
            max: nanos(max)?,
            extra: extra
                .split(' ')
                .filter(|kv| !kv.is_empty())
                .map(|kv| {
                    kv.split_once('=')
                        .map(|(k, v)| (k.to_owned(), v.to_owned()))
                        .ok_or_else(|| format!("bad extra {kv:?}"))
                })
                .collect::<Result<_, _>>()?,
        })
    }
}

/// A duration printed with a unit that suits its size.
#[derive(Clone, Copy, Debug)]
pub struct Human(pub Duration);

impl fmt::Display for Human {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ns = self.0.as_secs_f64() * 1e9;
        if ns < 1e3 {
            write!(f, "{ns:.0} ns")
        } else if ns < 1e6 {
            write!(f, "{:.1} µs", ns / 1e3)
        } else if ns < 1e9 {
            write!(f, "{:.2} ms", ns / 1e6)
        } else {
            write!(f, "{:.3} s", ns / 1e9)
        }
    }
}

/// Human-readable lines for one case's variants, each with its ratio to the
/// first (the baseline).
#[must_use]
pub fn format_case(measurements: &[Measurement]) -> String {
    let Some(first) = measurements.first() else {
        return String::new();
    };
    let mut out = format!(
        "{}/{}  ({} {} per call)\n",
        first.suite, first.case, first.items, first.unit
    );
    for m in measurements {
        let _ = writeln!(
            out,
            "  {:<22} median {:>10}  [{:>10} … {:>10}]  {:>9.2} ns/{}  ×{:.2}  {}",
            m.variant,
            Human(m.median).to_string(),
            Human(m.min).to_string(),
            Human(m.max).to_string(),
            m.per_item_ns(),
            m.unit,
            m.median.as_secs_f64() / first.median.as_secs_f64(),
            m.extra
                .iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect::<Vec<_>>()
                .join(" "),
        );
    }
    out
}

/// One row of [`aggregate`]: a measurement across several runs.
#[derive(Clone, Debug)]
pub struct Aggregated {
    pub suite: String,
    pub case: String,
    pub variant: String,
    pub items: u64,
    pub unit: String,
    /// Each run's median, sorted ascending.
    pub run_medians: Vec<Duration>,
    /// Extra facts, from the first run.
    pub extra: Vec<(String, String)>,
}

impl Aggregated {
    /// The median of the run medians.
    #[must_use]
    pub fn median(&self) -> Duration {
        self.run_medians[self.run_medians.len() / 2]
    }

    /// Half the range of the run medians, as a fraction of their median.
    #[must_use]
    pub fn half_spread(&self) -> f64 {
        let (lo, hi) = (
            self.run_medians[0],
            self.run_medians[self.run_medians.len() - 1],
        );
        hi.saturating_sub(lo).as_secs_f64() / 2.0 / self.median().as_secs_f64()
    }

    #[must_use]
    #[expect(clippy::cast_precision_loss, reason = "item counts are far below 2^52")]
    pub fn per_item_ns(&self) -> f64 {
        self.median().as_secs_f64() * 1e9 / self.items.max(1) as f64
    }
}

/// Group measurements from several runs by (suite, case, variant), in order
/// of first appearance.
///
/// # Errors
/// If runs disagree on a case's work-item count: they measured different work.
pub fn aggregate(runs: &[Vec<Measurement>]) -> Result<Vec<Aggregated>, String> {
    let mut rows: Vec<Aggregated> = Vec::new();
    for m in runs.iter().flatten() {
        let found = rows
            .iter_mut()
            .find(|r| r.suite == m.suite && r.case == m.case && r.variant == m.variant);
        match found {
            Some(row) if row.items != m.items => {
                return Err(format!(
                    "{}/{}/{}: runs disagree on items ({} vs {})",
                    m.suite, m.case, m.variant, row.items, m.items
                ));
            }
            Some(row) => row.run_medians.push(m.median),
            None => rows.push(Aggregated {
                suite: m.suite.clone(),
                case: m.case.clone(),
                variant: m.variant.clone(),
                items: m.items,
                unit: m.unit.clone(),
                run_medians: vec![m.median],
                extra: m.extra.clone(),
            }),
        }
    }
    for row in &mut rows {
        row.run_medians.sort_unstable();
    }
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use core::cell::RefCell;

    use super::*;

    #[test]
    fn tsv_round_trips() {
        let m = Measurement::new(
            "vm",
            "w1",
            "match/raw",
            83_000,
            "op",
            &Samples::new(vec![
                Duration::from_micros(30),
                Duration::from_micros(10),
                Duration::from_micros(20),
            ]),
        )
        .with("instructions", 83_000);
        assert_eq!(m.median, Duration::from_micros(20));
        let parsed = Measurement::from_tsv(&m.to_tsv()).expect("parses");
        assert_eq!(parsed, m);
    }

    #[test]
    fn rotation_reverses_every_other_round() {
        let order = RefCell::new(String::new());
        let mut a = || {
            order.borrow_mut().push('A');
            Duration::from_nanos(1)
        };
        let mut b = || {
            order.borrow_mut().push('B');
            Duration::from_nanos(2)
        };
        let mut c = || {
            order.borrow_mut().push('C');
            Duration::from_nanos(3)
        };
        let samples = rotate(
            Sampling {
                warmup: Duration::ZERO,
                samples: 3,
            },
            &mut [&mut a, &mut b, &mut c],
        );
        assert_eq!(
            order.into_inner(),
            "ABC" /* warm-up */
                .to_owned()
                + "ABCCBAABC"
        );
        assert_eq!(samples[2].median(), Duration::from_nanos(3));
    }

    #[test]
    fn aggregate_takes_the_median_of_run_medians() {
        let run = |micros| {
            vec![Measurement::new(
                "s",
                "c",
                "v",
                10,
                "op",
                &Samples::new(vec![Duration::from_micros(micros)]),
            )]
        };
        let rows = aggregate(&[run(30), run(10), run(20)]).expect("consistent runs");
        assert_eq!(rows[0].median(), Duration::from_micros(20));
        assert!((rows[0].half_spread() - 0.5).abs() < 1e-9);
        assert!((rows[0].per_item_ns() - 2_000.0).abs() < 1e-6);
    }
}
