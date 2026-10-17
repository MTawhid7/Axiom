//! S3-Q1: the unit costs of the root stacks, measured exactly as
//! `s1-probe roots` measured S1's (`s1_bench::roots`):
//! - opening and closing an empty frame;
//! - rooting a handle, in frames of 64 roots, as a dispatch roots a listener
//!   snapshot (so the per-root figure includes 1/64 of a frame, as S1's did);
//! - reading a rooted handle back.
//!
//! The same loops, warm-up and sample count are run on S1's own stack, with
//! S1's own types, in the same process, as the baseline.
//!
//! **One supplementary measurement**, beyond S1's method: `push-run`, 64
//! pushes back to back per frame with one `black_box` per frame instead of
//! one per push. S1's method puts a compiler barrier after every push, and
//! the stack is reachable from escaped pointers, so every push must reload
//! the stack length that the previous push stored: it measures the store-to-
//! load forwarding of that length as much as the push. `push-run` is the
//! shape of a builtin rooting its arguments on entry, where consecutive pushes
//! keep the length in a register.

use core::hint::black_box;
use std::time::{Duration, Instant};

use s3_gc::{CellStack, Pacing, RawStack, RefCellStack, RootStack};

use crate::dom::{Agent, NodeRef};
use crate::harness::Samples;

/// Operations timed per sample (S1's).
pub const OPS: usize = 1 << 16;
/// Roots pushed per frame in the push measurement (S1's).
const PER_FRAME: usize = 64;

/// The primitives, in report order. `push-run` is supplementary (see the
/// module docs).
pub const PRIMITIVES: [&str; 4] = ["frame", "push", "get", "push-run"];
/// The stacks, in report order: S1's first, as the baseline.
pub const STACKS: [&str; 4] = ["s1", "refcell", "cell", "raw"];

/// Samples of each primitive, in [`PRIMITIVES`] order, for the named stack.
///
/// # Panics
/// If `stack` is not one of [`STACKS`].
#[must_use]
pub fn measure(stack: &str, warmup: Duration, samples: usize) -> [Samples; 4] {
    match stack {
        "s1" => measure_s1(warmup, samples),
        "refcell" => measure_s3::<RefCellStack>(warmup, samples),
        "cell" => measure_s3::<CellStack>(warmup, samples),
        "raw" => measure_s3::<RawStack>(warmup, samples),
        _ => panic!("unknown stack {stack:?}"),
    }
}

/// S1's probe (`s1_bench::roots::measure`), returning the samples.
fn measure_s1(warmup: Duration, samples: usize) -> [Samples; 4] {
    let agent = s1_b_dom::dom::Agent::new(1, 0, s1_b_gc::Pacing::default());
    let core = &agent.heap.core;
    let document = agent.globals.document;

    let frames = || {
        for _ in 0..OPS {
            drop(black_box(core.frame()));
        }
    };
    let pushes = || {
        for _ in 0..OPS / PER_FRAME {
            let frame = core.frame();
            for _ in 0..PER_FRAME {
                black_box(black_box(document).scope(&frame));
            }
        }
    };
    let runs = || {
        for _ in 0..OPS / PER_FRAME {
            let frame = core.frame();
            let mut last = None;
            for _ in 0..PER_FRAME {
                last = Some(document.scope(&frame));
            }
            black_box(last);
        }
    };
    let frame = core.frame();
    let scoped = document.scope(&frame);
    let gets = || {
        for _ in 0..OPS {
            black_box(black_box(&scoped).get());
        }
    };
    [
        sampled(warmup, samples, frames),
        sampled(warmup, samples, pushes),
        sampled(warmup, samples, gets),
        sampled(warmup, samples, runs),
    ]
}

/// The same loops on one of `s3-gc`'s stacks, on the DOM port's heap.
fn measure_s3<S: RootStack + Default>(warmup: Duration, samples: usize) -> [Samples; 4] {
    let stack = S::default();
    let agent: Agent<'_, S> = Agent::new(&stack, 1, Pacing::default());
    let document: NodeRef<'static> = agent.globals.document;
    let core = &agent.heap.core;

    let frames = || {
        for _ in 0..OPS {
            drop(black_box(core.frame()));
        }
    };
    let pushes = || {
        for _ in 0..OPS / PER_FRAME {
            let frame = core.frame();
            for _ in 0..PER_FRAME {
                black_box(black_box(document).scope(&frame));
            }
        }
    };
    let runs = || {
        for _ in 0..OPS / PER_FRAME {
            let frame = core.frame();
            let mut last = None;
            for _ in 0..PER_FRAME {
                last = Some(document.scope(&frame));
            }
            black_box(last);
        }
    };
    let frame = core.frame();
    let scoped = document.scope(&frame);
    let gets = || {
        for _ in 0..OPS {
            black_box(black_box(&scoped).get());
        }
    };
    [
        sampled(warmup, samples, frames),
        sampled(warmup, samples, pushes),
        sampled(warmup, samples, gets),
        sampled(warmup, samples, runs),
    ]
}

/// S1's `per_op`: a time-based warm-up, then `samples` timed calls.
fn sampled(warmup: Duration, samples: usize, mut f: impl FnMut()) -> Samples {
    let start = Instant::now();
    loop {
        f();
        if start.elapsed() >= warmup {
            break;
        }
    }
    Samples::new(
        (0..samples.max(1))
            .map(|_| {
                let start = Instant::now();
                f();
                start.elapsed()
            })
            .collect(),
    )
}

/// Nanoseconds per operation, from a sample set of [`OPS`] operations each.
#[must_use]
#[expect(
    clippy::cast_precision_loss,
    reason = "OPS is a small power of two, exactly representable"
)]
pub fn per_op_ns(samples: &Samples) -> f64 {
    samples.median().as_secs_f64() * 1e9 / OPS as f64
}
