//! What recording costs, in instructions retired and cycles (OB25, OB27):
//! PF5's first tier, run optimized by `tools/check.sh` with
//! `cargo test --release -p trace --test cost`. One test in its own binary,
//! since the counts cover the whole process.
//!
//! The clock here is a relaxed load, so the counts are the recorder's own;
//! `mach_continuous_time`'s cost joins at M1-CP3 with `platform-event`.
#![forbid(unsafe_code)]

use std::hint::black_box;
use std::sync::atomic::{AtomicU64, Ordering};

use test_support::{Cpu, count_cpu};
use trace::{Category, Name, OwnedRing, Reader};

static CLOCK: AtomicU64 = AtomicU64::new(1);
const ITERATIONS: u64 = 100_000;

fn clock() -> u64 {
    CLOCK.load(Ordering::Relaxed)
}

/// The cheapest of five runs of `ITERATIONS` iterations, per iteration.
fn per_iteration(ring: &OwnedRing, mut body: impl FnMut(u64)) -> (f64, f64) {
    let mut best = Cpu {
        instructions: u64::MAX,
        cycles: u64::MAX,
    };
    let mut drained = Vec::new();
    for _ in 0..5 {
        let ((), cpu) = count_cpu(|| (0..ITERATIONS).for_each(|i| body(black_box(i))))
            .expect("proc_pid_rusage");
        best.instructions = best.instructions.min(cpu.instructions);
        best.cycles = best.cycles.min(cpu.cycles);
        drained.clear();
        Reader::new(ring.storage().expect("a valid ring"))
            .drain(&mut drained)
            .expect("a correct writer");
    }
    #[expect(clippy::cast_precision_loss, reason = "counts far below 2^52")]
    (
        best.instructions as f64 / ITERATIONS as f64,
        best.cycles as f64 / ITERATIONS as f64,
    )
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "costs are measured optimized: cargo test --release -p trace --test cost"
)]
#[cfg_attr(miri, ignore = "Miri can't call proc_pid_rusage")]
fn recording_costs_stay_within_their_measured_ceilings() {
    trace::set_clock(clock).expect("this binary's only clock");
    // 2^20 words hold more than one run's records, drained between runs.
    let ring: &'static OwnedRing =
        Box::leak(Box::new(OwnedRing::new(1 << 20).expect("a power of two")));
    trace::install(ring.storage().expect("a valid ring"));
    let name = Name::new(1);

    let empty = per_iteration(ring, |i| {
        black_box(i);
    });
    let disabled = per_iteration(ring, |i| {
        if trace::enabled(Category::Style) {
            trace::span_begin(Category::Style, name, &[i]);
        }
    });
    let span = per_iteration(ring, |i| {
        trace::span_begin(Category::Style, name, &[i, 2]);
        trace::span_end(Category::Style, name);
    });
    let counter = per_iteration(ring, |i| trace::counter(Category::Health, name, i));

    let net = |(instructions, cycles): (f64, f64), records: f64| {
        (
            (instructions - empty.0) / records,
            (cycles - empty.1) / records,
        )
    };
    let (disabled, span, counter) = (net(disabled, 1.0), net(span, 2.0), net(counter, 1.0));
    println!("per record, net of the loop: instructions, cycles");
    println!(
        "  a disabled detailed check: {:.1}, {:.1}",
        disabled.0, disabled.1
    );
    println!(
        "  a span record (begin or end): {:.1}, {:.1}",
        span.0, span.1
    );
    println!("  a counter record: {:.1}, {:.1}", counter.0, counter.1);

    // Ceilings: the measured values with about 20% headroom (OB25, as the
    // owner asked: "minimal performance overhead"), not OB25's 2 ns and 50 ns,
    // which these are far below. Measured on 2026-10-23 with Rust 1.94.1:
    // 1.0, 130.5 and 103.0 instructions, steady from run to run; 0, 29-30
    // and 19-20 cycles. A change that passes a ceiling updates it here, with
    // the reason in its commit (ES23).
    assert!(
        disabled.0 <= CEILING_DISABLED,
        "a disabled check: {:.1} instructions",
        disabled.0
    );
    assert!(
        span.0 <= CEILING_SPAN,
        "a span record: {:.1} instructions",
        span.0
    );
    assert!(
        counter.0 <= CEILING_COUNTER,
        "a counter record: {:.1} instructions",
        counter.0
    );
}

const CEILING_DISABLED: f64 = 2.0;
const CEILING_SPAN: f64 = 160.0;
const CEILING_COUNTER: f64 = 125.0;
