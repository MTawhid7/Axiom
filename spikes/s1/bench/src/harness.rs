//! A minimal benchmark harness.
//!
//! - Warm-up rounds run first, for a minimum *time*, and are discarded.
//!   Apple Silicon raises a core's clock only after it has been busy for a
//!   while, so a few iterations of a short case would be measured at a low
//!   frequency.
//! - Samples alternate between the two patterns in ABBA order, so slow drift
//!   (thermal throttling on this fanless machine, background load) falls on
//!   both equally.
//! - A sample times one call. Setup and teardown are excluded.
//! - Reports use the median, with the minimum and maximum as the spread.
//!   Across separate runs, [`aggregate`] reports the median of the run
//!   medians and their range.

use core::fmt::{self, Write as _};
use core::hint::black_box;
use std::time::{Duration, Instant};

/// Which implementation a measurement belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Pattern {
    A,
    B,
}

impl Pattern {
    pub const BOTH: [Self; 2] = [Self::A, Self::B];

    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::A => "A",
            Self::B => "B",
        }
    }

    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "A" | "a" => Some(Self::A),
            "B" | "b" => Some(Self::B),
            _ => None,
        }
    }
}

impl fmt::Display for Pattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// How long to warm up, and how many samples to take.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sampling {
    /// Discarded rounds run until at least this much time has passed (and
    /// at least one round has run).
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
        assert!(
            !samples.is_empty(),
            "a measurement needs at least one sample"
        );
        samples.sort_unstable();
        Self(samples)
    }

    /// The middle sample (the upper middle for an even count; suites take odd counts).
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

/// Time `routine` on state made by `setup`. Neither the setup nor dropping
/// the state or the result is timed.
pub fn timed_with<S, R>(setup: impl FnOnce() -> S, routine: impl FnOnce(&mut S) -> R) -> Duration {
    let mut state = setup();
    let elapsed = timed(|| routine(black_box(&mut state)));
    drop(state);
    elapsed
}

/// Sample `a` and `b` alternately, in ABBA order, after discarded warm-up
/// rounds.
pub fn alternate(
    sampling: Sampling,
    mut a: impl FnMut() -> Duration,
    mut b: impl FnMut() -> Duration,
) -> [Samples; 2] {
    let warmup = Instant::now();
    loop {
        a();
        b();
        if warmup.elapsed() >= sampling.warmup {
            break;
        }
    }
    let mut sa = Vec::with_capacity(sampling.samples);
    let mut sb = Vec::with_capacity(sampling.samples);
    for round in 0..sampling.samples {
        if round.is_multiple_of(2) {
            sa.push(a());
            sb.push(b());
        } else {
            sb.push(b());
            sa.push(a());
        }
    }
    [Samples::new(sa), Samples::new(sb)]
}

/// One benchmark result: a summary of its samples, plus extra facts.
#[derive(Clone, Debug, PartialEq)]
pub struct Measurement {
    pub suite: String,
    pub case: String,
    pub pattern: Pattern,
    /// Work items per timed call (nodes, ops, dispatches, objects …).
    pub items: u64,
    pub unit: String,
    pub samples: usize,
    pub median: Duration,
    pub min: Duration,
    pub max: Duration,
    /// Facts that are not timings: heap metrics, counts. `key=value` pairs.
    pub extra: Vec<(String, String)>,
}

impl Measurement {
    pub const TSV_HEADER: &'static str =
        "suite\tcase\tpattern\titems\tunit\tsamples\tmedian_ns\tmin_ns\tmax_ns\tper_item_ns\textra";

    #[must_use]
    pub fn new(
        suite: &str,
        case: &str,
        pattern: Pattern,
        items: u64,
        unit: &str,
        samples: &Samples,
    ) -> Self {
        Self {
            suite: suite.to_owned(),
            case: case.to_owned(),
            pattern,
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

    /// An extra fact, by key.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&str> {
        self.extra
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }

    /// The median time per work item, in nanoseconds.
    #[must_use]
    #[expect(
        clippy::cast_precision_loss,
        reason = "item counts are far below 2^52, so the division is exact enough"
    )]
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
            self.pattern,
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
            pattern,
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
            pattern: Pattern::parse(pattern).ok_or_else(|| format!("bad pattern {pattern:?}"))?,
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

/// Human-readable lines for one A/B pair, with the B/A ratio of medians.
#[must_use]
pub fn format_pair(a: &Measurement, b: &Measurement) -> String {
    let mut out = format!(
        "{}/{}  ({} {} per call)\n",
        a.suite, a.case, a.items, a.unit
    );
    for m in [a, b] {
        let _ = writeln!(
            out,
            "  {}  median {:>10}  [{:>10} … {:>10}]  {:>9.1} ns/{}  {}",
            m.pattern,
            Human(m.median).to_string(),
            Human(m.min).to_string(),
            Human(m.max).to_string(),
            m.per_item_ns(),
            m.unit,
            m.extra
                .iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect::<Vec<_>>()
                .join(" "),
        );
    }
    let _ = writeln!(out, "  B/A {:.2}", ratio(b.median, a.median));
    out
}

/// `numerator / denominator`, as a float.
#[must_use]
pub fn ratio(numerator: Duration, denominator: Duration) -> f64 {
    numerator.as_secs_f64() / denominator.as_secs_f64()
}

/// One row of [`aggregate`]: a measurement across several runs.
#[derive(Clone, Debug)]
pub struct Aggregated {
    pub suite: String,
    pub case: String,
    pub pattern: Pattern,
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

    /// The range of the run medians, as a fraction of their median.
    #[must_use]
    pub fn spread(&self) -> f64 {
        let (lo, hi) = (
            self.run_medians[0],
            self.run_medians[self.run_medians.len() - 1],
        );
        ratio(hi.saturating_sub(lo), self.median())
    }

    #[must_use]
    #[expect(
        clippy::cast_precision_loss,
        reason = "item counts are far below 2^52, so the division is exact enough"
    )]
    pub fn per_item_ns(&self) -> f64 {
        self.median().as_secs_f64() * 1e9 / self.items.max(1) as f64
    }
}

/// Group measurements from several runs by (suite, case, pattern), in the
/// order of first appearance.
///
/// # Errors
/// If runs disagree on a case's work-item count, they measured different work.
pub fn aggregate(runs: &[Vec<Measurement>]) -> Result<Vec<Aggregated>, String> {
    let mut rows: Vec<Aggregated> = Vec::new();
    for m in runs.iter().flatten() {
        let found = rows
            .iter_mut()
            .find(|r| r.suite == m.suite && r.case == m.case && r.pattern == m.pattern);
        match found {
            Some(row) if row.items != m.items => {
                return Err(format!(
                    "{}/{}/{}: runs disagree on items ({} vs {})",
                    m.suite, m.case, m.pattern, row.items, m.items
                ));
            }
            Some(row) => row.run_medians.push(m.median),
            None => rows.push(Aggregated {
                suite: m.suite.clone(),
                case: m.case.clone(),
                pattern: m.pattern,
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
    use super::*;

    #[test]
    fn tsv_round_trips() {
        let m = Measurement::new(
            "collect",
            "live-64k",
            Pattern::B,
            83_000,
            "object",
            &Samples::new(vec![
                Duration::from_micros(30),
                Duration::from_micros(10),
                Duration::from_micros(20),
            ]),
        )
        .with("collections", 3)
        .with("max_pause_us", 30);
        assert_eq!(m.median, Duration::from_micros(20));
        assert_eq!(m.min, Duration::from_micros(10));
        let parsed = Measurement::from_tsv(&m.to_tsv()).expect("parses");
        assert_eq!(parsed, m);
        assert_eq!(parsed.get("max_pause_us"), Some("30"));
    }

    #[test]
    fn aggregate_takes_the_median_of_run_medians() {
        let run = |micros| {
            vec![Measurement::new(
                "s",
                "c",
                Pattern::A,
                10,
                "op",
                &Samples::new(vec![Duration::from_micros(micros)]),
            )]
        };
        let rows = aggregate(&[run(30), run(10), run(20)]).expect("consistent runs");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].median(), Duration::from_micros(20));
        assert!((rows[0].spread() - 1.0).abs() < 1e-9);
        assert!((rows[0].per_item_ns() - 2_000.0).abs() < 1e-6);
    }

    #[test]
    fn alternate_runs_both_sides_in_abba_order() {
        let order = core::cell::RefCell::new(String::new());
        let [a, b] = alternate(
            Sampling {
                warmup: Duration::ZERO,
                samples: 3,
            },
            || {
                order.borrow_mut().push('A');
                Duration::from_nanos(1)
            },
            || {
                order.borrow_mut().push('B');
                Duration::from_nanos(2)
            },
        );
        assert_eq!(
            order.into_inner(),
            "ABABBAAB",
            "at least one warm-up round, then AB, BA, AB"
        );
        assert_eq!((a.len(), b.len()), (3, 3));
    }
}
