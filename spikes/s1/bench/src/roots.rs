//! Unit costs of Pattern B's rooting primitives.
//!
//! CP7 profiles put about half of Pattern B's dispatch time in its root stack
//! (`RootFrame::push` and `Scoped::get`). This measures those operations on
//! their own, so the cost of rooting can be stated per operation:
//! - opening and closing an empty frame;
//! - rooting a handle (`Handle::scope`), in frames of 64 roots, as a
//!   dispatch roots a listener snapshot;
//! - reading a rooted handle back (`Scoped::get`).
//!
//! Pattern A has no counterpart: a `Gc` held inside a mutation scope is a
//! plain pointer copy.

use core::hint::black_box;
use std::time::{Duration, Instant};

use s1_b_dom::dom::Agent;
use s1_b_gc::Pacing;

use crate::harness::Samples;

/// Operations timed per sample.
const OPS: usize = 1 << 16;
/// Roots pushed per frame in the push measurement.
const PER_FRAME: usize = 64;

/// Median cost of each primitive, in nanoseconds per operation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RootCosts {
    pub frame_ns: f64,
    pub push_ns: f64,
    pub get_ns: f64,
}

/// Measure with a time-based warm-up, then `samples` samples per primitive.
#[must_use]
pub fn measure(warmup: Duration, samples: usize) -> RootCosts {
    let agent = Agent::new(1, 0, Pacing::default());
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
    let frame = core.frame();
    let scoped = document.scope(&frame);
    let gets = || {
        for _ in 0..OPS {
            black_box(black_box(&scoped).get());
        }
    };

    RootCosts {
        frame_ns: per_op(warmup, samples, frames),
        push_ns: per_op(warmup, samples, pushes),
        get_ns: per_op(warmup, samples, gets),
    }
}

#[expect(
    clippy::cast_precision_loss,
    reason = "OPS is a small power of two, exactly representable"
)]
fn per_op(warmup: Duration, samples: usize, mut f: impl FnMut()) -> f64 {
    let start = Instant::now();
    loop {
        f();
        if start.elapsed() >= warmup {
            break;
        }
    }
    let samples = Samples::new(
        (0..samples.max(1))
            .map(|_| {
                let start = Instant::now();
                f();
                start.elapsed()
            })
            .collect(),
    );
    samples.median().as_secs_f64() * 1e9 / OPS as f64
}
