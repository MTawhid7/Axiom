//! The benchmark suites, the loops that `s3-probe profile` records, and the
//! counts that `s3-count` reports.
//!
//! | Suite | Question | Variants |
//! |---|---|---|
//! | `roots` | S3-Q1: frame, push and read costs | S1's stack, then `RefCellStack`, `CellStack`, `RawStack`, `SliceStackLenLast`, `SliceStack` |
//! | `dispatch` | S3-Q1: S1's listener-dispatch case | S1's Pattern B, then the port on each stack |
//! | `vm` | S3-Q2, S3-Q3: W1–W4 | every technique on every stack but the field-order control |
//! | `traverse` | S3-Q4: S1's traversal cases | S1's Pattern B, the port, then four layouts |
//! | `mutate` | S3-Q5: S1's mutation case | S1's plain writes, then the port's setters without and with card marking |
//! | `barrier` | S3-Q5: one handle write | the same three |
//!
//! Every input is verified before it is timed: logs, digests, folds and
//! results must match S1's reference interpreter or the VM's reference
//! evaluator and native models.

use core::hint::black_box;
use std::time::{Duration, Instant};

use s1_bench::PatternB;
use s1_bench::dom::{Config, Dom};
use s1_bench::plan::{self, Order, Plan};
use s1_scenarios::Tag;
use s3_gc::count::{self as root_count, Context, RootCounts};
use s3_gc::{
    Barrier, CardBarrier, CellStack, DEFAULT_CAPACITY, GcScope, GcToken, NoBarrier, Pacing,
    RawStack, RefCellStack, RootStack, SliceStack, SliceStackLenLast, slot_buffer,
};
use s3_vm::workloads::{self, Size, Workload};
use s3_vm::{Loaded, Stats, Technique, Vm};

use crate::dom::{Agent, NodeKind};
use crate::harness::{Measurement, Sampling, rotate, timed};
use crate::layout::{self, Layouts};
use crate::port::Port;
use crate::roots;

/// The seed of every generated input (S1's).
pub const SEED: u64 = 1;

/// The suites, in the order [`run`] runs them.
pub const SUITES: [&str; 6] = ["roots", "dispatch", "vm", "traverse", "mutate", "barrier"];

/// The root stacks of `s3-gc` that the `vm` suite runs, by name.
pub const STACKS: [&str; 4] = ["refcell", "cell", "raw", "slice"];

/// What to run.
#[derive(Clone, Debug, Default)]
pub struct Options {
    /// Small inputs and one sample per case: a fast, verifying smoke run.
    pub quick: bool,
    /// Run only cases whose `suite/case` name contains this.
    pub filter: Option<String>,
}

impl Options {
    fn wants(&self, suite: &str, case: &str) -> bool {
        self.filter
            .as_deref()
            .is_none_or(|filter| format!("{suite}/{case}").contains(filter))
    }

    /// S1's sampling for micro-benchmarks.
    const fn micro(&self) -> Sampling {
        if self.quick {
            Sampling {
                warmup: Duration::ZERO,
                samples: 1,
            }
        } else {
            Sampling {
                warmup: Duration::from_millis(300),
                samples: 11,
            }
        }
    }

    /// For whole workload runs, which take tens of milliseconds.
    const fn runs(&self) -> Sampling {
        if self.quick {
            Sampling {
                warmup: Duration::ZERO,
                samples: 1,
            }
        } else {
            Sampling {
                warmup: Duration::from_millis(500),
                samples: 11,
            }
        }
    }
}

/// Receives each case's measurements as soon as they are taken.
pub type Report<'a> = dyn FnMut(&[Measurement]) + 'a;

/// Run every suite.
pub fn run(options: &Options, report: &mut Report<'_>) {
    root_costs(options, report);
    dispatch(options, report);
    vm(options, report);
    traverse(options, report);
    mutate(options, report);
    barrier(options, report);
}

fn short(n: u32) -> String {
    if n >= 1_000 && n.is_multiple_of(1_000) {
        format!("{}k", n / 1_000)
    } else {
        n.to_string()
    }
}

/// S1's Pattern B holding `plan`'s setup state, without automatic collection.
fn built_s1(plan: &Plan) -> PatternB {
    let mut dom = PatternB::new(plan.node_slots, Config::MUTATOR);
    dom.apply(&plan.setup);
    dom
}

/// The port holding `plan`'s setup state, without automatic collection.
fn built_port<'r, S: RootStack, B: Barrier>(stack: &'r S, plan: &Plan) -> Port<'r, S, B> {
    let mut port = Port::new(stack, plan.node_slots, false);
    port.apply(&plan.setup);
    port
}

// ----- roots (S3-Q1) -----

fn root_costs(options: &Options, report: &mut Report<'_>) {
    if !roots::PRIMITIVES
        .iter()
        .any(|primitive| options.wants("roots", primitive))
    {
        return;
    }
    let sampling = options.micro();
    let measured: Vec<_> = roots::STACKS
        .iter()
        .map(|&stack| {
            (
                stack,
                roots::measure(stack, sampling.warmup, sampling.samples),
            )
        })
        .collect();
    for (k, primitive) in roots::PRIMITIVES.iter().enumerate() {
        if !options.wants("roots", primitive) {
            continue;
        }
        let row: Vec<_> = measured
            .iter()
            .map(|(stack, samples)| {
                Measurement::new(
                    "roots",
                    primitive,
                    stack,
                    roots::OPS as u64,
                    "op",
                    &samples[k],
                )
            })
            .collect();
        report(&row);
    }
}

// ----- dispatch (S3-Q1) -----

fn dispatch(options: &Options, report: &mut Report<'_>) {
    let (elements, listeners, dispatches) = if options.quick {
        (200, 600, 400)
    } else {
        (20_000, 60_000, 50_000)
    };
    let case = format!("tree-{}-listeners-{}", short(elements), short(listeners));
    if !options.wants("dispatch", &case) {
        return;
    }
    let plan = plan::dispatch(elements, listeners, dispatches, SEED);
    let (refcell, cell, raw) = (
        RefCellStack::default(),
        CellStack::default(),
        RawStack::default(),
    );
    let (mut buffer_len_last, mut buffer) =
        (slot_buffer(DEFAULT_CAPACITY), slot_buffer(DEFAULT_CAPACITY));
    let slice_len_last = SliceStackLenLast::new(&mut buffer_len_last);
    let slice = SliceStack::new(&mut buffer);
    let mut s1 = built_s1(&plan);
    let mut p_refcell: Port<'_, _> = built_port(&refcell, &plan);
    let mut p_cell: Port<'_, _> = built_port(&cell, &plan);
    let mut p_raw: Port<'_, _> = built_port(&raw, &plan);
    let mut p_slice_len_last: Port<'_, _> = built_port(&slice_len_last, &plan);
    let mut p_slice: Port<'_, _> = built_port(&slice, &plan);
    // The first pass must match the reference. Later passes repeat it
    // exactly: listeners only log, stop, or rewrite an attribute with the
    // value it already has.
    s1.apply(&plan.work);
    p_refcell.apply(&plan.work);
    p_cell.apply(&plan.work);
    p_raw.apply(&plan.work);
    p_slice_len_last.apply(&plan.work);
    p_slice.apply(&plan.work);
    assert!(s1.log() == plan.log, "{case}: S1 logged differently");
    assert_eq!(s1.tree_digest(), plan.final_digest, "{case}: S1's tree");
    for (name, log, digest) in [
        ("refcell", p_refcell.log(), p_refcell.tree_digest()),
        ("cell", p_cell.log(), p_cell.tree_digest()),
        ("raw", p_raw.log(), p_raw.tree_digest()),
        (
            "slice-len-last",
            p_slice_len_last.log(),
            p_slice_len_last.tree_digest(),
        ),
        ("slice", p_slice.log(), p_slice.tree_digest()),
    ] {
        assert!(
            log == plan.log,
            "{case}: the port on {name} logged differently"
        );
        assert_eq!(
            digest, plan.final_digest,
            "{case}: the port on {name}'s tree"
        );
    }

    let samples = rotate(
        options.micro(),
        &mut [
            &mut || {
                s1.clear_log();
                timed(|| s1.apply(&plan.work))
            },
            &mut || {
                p_refcell.clear_log();
                timed(|| p_refcell.apply(&plan.work))
            },
            &mut || {
                p_cell.clear_log();
                timed(|| p_cell.apply(&plan.work))
            },
            &mut || {
                p_raw.clear_log();
                timed(|| p_raw.apply(&plan.work))
            },
            &mut || {
                p_slice_len_last.clear_log();
                timed(|| p_slice_len_last.apply(&plan.work))
            },
            &mut || {
                p_slice.clear_log();
                timed(|| p_slice.apply(&plan.work))
            },
        ],
    );
    assert!(s1.log() == plan.log && p_slice.log() == plan.log);
    let items = plan.work.len() as u64;
    let row: Vec<_> = ["s1", "refcell", "cell", "raw", "slice-len-last", "slice"]
        .iter()
        .zip(&samples)
        .map(|(variant, s)| {
            Measurement::new("dispatch", &case, variant, items, "dispatch", s)
                .with("invocations", plan.log.len())
        })
        .collect();
    report(&row);
}

// ----- vm (S3-Q2, S3-Q3) -----

/// One timed run of `w` on a fresh machine. The stack is the workload's own,
/// reused from run to run as an agent keeps its stack from task to task; every
/// run leaves it empty. The result is checked after the clock stops.
fn timed_run<S: RootStack>(
    stack: &S,
    loaded: &Loaded<S>,
    w: &Workload,
    technique: Technique,
) -> Duration {
    let mut vm = Vm::new(stack, Pacing::default());
    let start = Instant::now();
    let result = vm.run(loaded, technique, &w.args);
    let elapsed = start.elapsed();
    let value = result.unwrap_or_else(|e| panic!("{}: {e}", w.name));
    assert_eq!(vm.digest(value), w.expected, "{} on {technique}", w.name);
    assert_eq!(stack.height(), 0, "{}: every root frame was popped", w.name);
    elapsed
}

fn vm(options: &Options, report: &mut Report<'_>) {
    let size = if options.quick {
        Size::Test
    } else {
        Size::Bench
    };
    for w in workloads::all(size) {
        if !options.wants("vm", w.name) {
            continue;
        }
        let reference = s3_vm::reference::run(&w.program, &w.args);
        assert_eq!(
            reference.result,
            Ok(w.expected),
            "{}: the reference evaluator disagrees with the native model",
            w.name
        );
        let (refcell_stack, cell_stack, raw_stack) = (
            RefCellStack::default(),
            CellStack::default(),
            RawStack::default(),
        );
        let mut buffer = slot_buffer(DEFAULT_CAPACITY);
        let slice_stack = SliceStack::new(&mut buffer);
        let refcell = Loaded::<RefCellStack>::new(w.program.clone()).expect("verified");
        let cell = Loaded::<CellStack>::new(w.program.clone()).expect("verified");
        let raw = Loaded::<RawStack>::new(w.program.clone()).expect("verified");
        let slice = Loaded::<SliceStack<'_>>::new(w.program.clone()).expect("verified");

        // Heap facts, from one run outside the timing.
        let mut probe = Vm::new(&slice_stack, Pacing::default());
        probe.run(&slice, Technique::Match, &w.args).expect("runs");
        let metrics = probe.metrics();
        drop(probe);

        let (refcell, cell, raw, slice, workload) = (&refcell, &cell, &raw, &slice, &w);
        let (refcell_stack, cell_stack, raw_stack, slice_stack) =
            (&refcell_stack, &cell_stack, &raw_stack, &slice_stack);
        let mut names = Vec::new();
        let mut runners: Vec<Box<dyn FnMut() -> Duration + '_>> = Vec::new();
        for &technique in Technique::ALL {
            for stack in STACKS {
                names.push(format!("{technique}/{stack}"));
                runners.push(match stack {
                    "refcell" => {
                        Box::new(move || timed_run(refcell_stack, refcell, workload, technique))
                    }
                    "cell" => Box::new(move || timed_run(cell_stack, cell, workload, technique)),
                    "raw" => Box::new(move || timed_run(raw_stack, raw, workload, technique)),
                    _ => Box::new(move || timed_run(slice_stack, slice, workload, technique)),
                });
            }
        }
        let mut refs: Vec<&mut dyn FnMut() -> Duration> = runners
            .iter_mut()
            .map(|r| &mut **r as &mut dyn FnMut() -> Duration)
            .collect();
        let samples = rotate(options.runs(), &mut refs);
        let row: Vec<_> = names
            .iter()
            .zip(&samples)
            .map(|(name, s)| {
                Measurement::new("vm", w.name, name, reference.instructions, "instr", s)
                    .with("collections", metrics.collections)
                    .with("allocated", metrics.total_allocated_objects)
            })
            .collect();
        report(&row);
    }
}

// ----- traverse (S3-Q4) -----

fn traverse(options: &Options, report: &mut Report<'_>) {
    let large = if options.quick { 200 } else { 100_000 };
    let documents: &[u32] = if options.quick {
        &[200]
    } else {
        &[10_000, 100_000]
    };
    for &elements in documents {
        let case = format!("tree-{}", short(elements));
        if options.wants("traverse", &case) {
            let plan = plan::tree(elements, Order::Document, SEED);
            traverse_case(options, report, &case, &plan, false);
        }
    }
    let case = format!("tree-{}-breadth-order", short(large));
    if options.wants("traverse", &case) {
        let plan = plan::tree(large, Order::Breadth, SEED);
        traverse_case(options, report, &case, &plan, false);
    }
    let case = format!("tree-{}-shuffled", short(large));
    if options.wants("traverse", &case) {
        let plan = plan::shuffle(large, large, SEED);
        traverse_case(options, report, &case, &plan, true);
    }
}

fn traverse_case(
    options: &Options,
    report: &mut Report<'_>,
    case: &str,
    plan: &Plan,
    with_work: bool,
) {
    let stack = RawStack::default();
    let mut s1 = built_s1(plan);
    let mut port: Port<'_, _> = built_port(&stack, plan);
    let expected = if with_work {
        s1.apply(&plan.work);
        port.apply(&plan.work);
        plan.final_digest
    } else {
        plan.setup_digest
    };
    assert_eq!(s1.tree_digest(), expected, "{case}: S1's tree");
    assert_eq!(port.tree_digest(), expected, "{case}: the port's tree");
    let fold = s1.traverse();
    assert_eq!(
        port.traverse(),
        fold,
        "{case}: the port walked another tree"
    );
    assert_eq!(fold.nodes, plan.nodes, "{case}: every node is attached");
    let layouts = Layouts::from_port(&port);
    for variant in layout::VARIANTS {
        assert_eq!(layouts.traverse(variant), fold, "{case}: layout {variant}");
    }

    let [a, b, c, d] = layout::VARIANTS;
    let samples = rotate(
        options.micro(),
        &mut [
            &mut || timed(|| s1.traverse()),
            &mut || timed(|| port.traverse()),
            &mut || timed(|| layouts.traverse(a)),
            &mut || timed(|| layouts.traverse(b)),
            &mut || timed(|| layouts.traverse(c)),
            &mut || timed(|| layouts.traverse(d)),
        ],
    );
    let row: Vec<_> = ["s1", "port", a, b, c, d]
        .iter()
        .zip(&samples)
        .map(|(variant, s)| Measurement::new("traverse", case, variant, fold.nodes, "node", s))
        .collect();
    report(&row);
}

// ----- mutate and barrier (S3-Q5) -----

fn mutate(options: &Options, report: &mut Report<'_>) {
    let (elements, ops) = if options.quick {
        (200, 400)
    } else {
        (100_000, 100_000)
    };
    let case = format!("tree-{}-ops-{}", short(elements), short(ops));
    if !options.wants("mutate", &case) {
        return;
    }
    let plan = plan::mutate(elements, ops, SEED);
    let stack = RawStack::default();
    let mut s1 = built_s1(&plan);
    s1.apply(&plan.work);
    assert_eq!(s1.tree_digest(), plan.final_digest, "{case}: S1's tree");
    let mut none: Port<'_, _, NoBarrier> = built_port(&stack, &plan);
    none.apply(&plan.work);
    assert_eq!(none.tree_digest(), plan.final_digest, "{case}: no barrier");
    let mut cards: Port<'_, _, CardBarrier> = built_port(&stack, &plan);
    cards.apply(&plan.work);
    assert_eq!(cards.tree_digest(), plan.final_digest, "{case}: cards");
    let dirty = cards.agent().heap.core.barrier.dirty_cards();
    drop((s1, none, cards));

    let samples = rotate(
        options.micro(),
        &mut [
            &mut || {
                let mut dom = built_s1(&plan);
                timed(|| dom.apply(&plan.work))
            },
            &mut || {
                let stack = RawStack::default();
                let mut port: Port<'_, _, NoBarrier> = built_port(&stack, &plan);
                timed(|| port.apply(&plan.work))
            },
            &mut || {
                let stack = RawStack::default();
                let mut port: Port<'_, _, CardBarrier> = built_port(&stack, &plan);
                timed(|| port.apply(&plan.work))
            },
        ],
    );
    let items = plan.work.len() as u64;
    report(&[
        Measurement::new("mutate", &case, "s1-plain", items, "op", &samples[0]),
        Measurement::new("mutate", &case, "setter-none", items, "op", &samples[1]),
        Measurement::new("mutate", &case, "setter-cards", items, "op", &samples[2])
            .with("dirty_cards", dirty),
    ]);
}

/// Handle writes per barrier sample, over a ring of nodes.
const WRITES: usize = 1 << 16;
const RING: usize = 4096;

fn port_ring<S: RootStack, B: Barrier>(
    agent: &mut Agent<'_, S, B>,
) -> Vec<crate::dom::NodeRef<'static>> {
    let mut token = GcToken::new();
    let mut gc = GcScope::new(&mut token);
    (0..RING)
        .map(|_| {
            agent
                .create_node(NodeKind::Element(Tag::Div), None, gc.reborrow())
                .unbind()
        })
        .collect()
}

fn barrier(options: &Options, report: &mut Report<'_>) {
    if !options.wants("barrier", "write") {
        return;
    }
    // S1: a plain store through the arena index.
    let mut s1 = s1_b_dom::dom::Agent::new(
        1,
        0,
        s1_b_gc::Pacing {
            min_bytes: usize::MAX,
            growth_percent: 0,
        },
    );
    let mut token = s1_b_gc::GcToken::new();
    let mut gc = s1_b_gc::GcScope::new(&mut token);
    let s1_ring: Vec<_> = (0..RING)
        .map(|_| {
            s1.create_node(
                s1_b_dom::dom::NodeKind::Element(Tag::Div),
                None,
                gc.reborrow(),
            )
            .unbind()
        })
        .collect();
    let (stack_none, stack_cards) = (RawStack::default(), RawStack::default());
    let mut none: Agent<'_, RawStack, NoBarrier> = Agent::new(&stack_none, 1, Pacing::NEVER);
    let none_ring = port_ring(&mut none);
    let mut cards: Agent<'_, RawStack, CardBarrier> = Agent::new(&stack_cards, 1, Pacing::NEVER);
    let cards_ring = port_ring(&mut cards);

    let samples = rotate(
        options.micro(),
        &mut [
            &mut || {
                timed(|| {
                    for k in 0..WRITES {
                        let (a, b) = (s1_ring[k % RING], s1_ring[(k + 1) % RING]);
                        s1.heap.nodes[black_box(a)].next_sibling = Some(b);
                    }
                })
            },
            &mut || {
                timed(|| {
                    for k in 0..WRITES {
                        let (a, b) = (none_ring[k % RING], none_ring[(k + 1) % RING]);
                        none.heap.set_next_sibling(black_box(a), Some(b));
                    }
                })
            },
            &mut || {
                timed(|| {
                    for k in 0..WRITES {
                        let (a, b) = (cards_ring[k % RING], cards_ring[(k + 1) % RING]);
                        cards.heap.set_next_sibling(black_box(a), Some(b));
                    }
                })
            },
        ],
    );
    let items = WRITES as u64;
    report(&[
        Measurement::new("barrier", "write", "s1-plain", items, "write", &samples[0]),
        Measurement::new(
            "barrier",
            "write",
            "setter-none",
            items,
            "write",
            &samples[1],
        ),
        Measurement::new(
            "barrier",
            "write",
            "setter-cards",
            items,
            "write",
            &samples[2],
        )
        .with("dirty_cards", cards.heap.core.barrier.dirty_cards()),
    ]);
}

// ----- profiles -----

/// What `s3-probe profile` can loop: `w1`…`w4` with a technique and stack
/// (`w3/match/slice`), or S1's dispatch case on a stack (`dispatch/slice`).
pub const PROFILE_CASES: &str = "w1..w4/<technique>/<refcell|cell|raw|slice>, \
     dispatch/<refcell|cell|raw|slice-len-last|slice>";

/// The function every profiled iteration runs under, so that profiles can be
/// narrowed to the timed work (`samply-summary.py --match profiled_iteration`).
#[inline(never)]
fn profiled_iteration(f: &mut dyn FnMut(), iterations: &mut u64) {
    f();
    *iterations += 1;
}

/// Loop `case` for `seconds`; returns the number of iterations.
///
/// # Errors
/// If the case is unknown.
pub fn profile(case: &str, seconds: f64) -> Result<u64, String> {
    let deadline = Instant::now() + Duration::from_secs_f64(seconds);
    let parts: Vec<&str> = case.split('/').collect();
    match parts[..] {
        ["dispatch", stack] => {
            let mut buffer = slot_buffer(DEFAULT_CAPACITY);
            match stack {
                "refcell" => Ok(profile_dispatch(&RefCellStack::default(), deadline)),
                "cell" => Ok(profile_dispatch(&CellStack::default(), deadline)),
                "raw" => Ok(profile_dispatch(&RawStack::default(), deadline)),
                "slice-len-last" => Ok(profile_dispatch(
                    &SliceStackLenLast::new(&mut buffer),
                    deadline,
                )),
                "slice" => Ok(profile_dispatch(&SliceStack::new(&mut buffer), deadline)),
                _ => Err(format!("unknown stack {stack:?}")),
            }
        }
        [workload, technique, stack] => {
            let w = workloads::named(workload, Size::Bench)
                .ok_or_else(|| format!("unknown workload {workload:?}"))?;
            let technique = Technique::parse(technique)
                .ok_or_else(|| format!("unknown technique {technique:?}"))?;
            let mut buffer = slot_buffer(DEFAULT_CAPACITY);
            match stack {
                "refcell" => Ok(profile_vm(
                    &RefCellStack::default(),
                    &w,
                    technique,
                    deadline,
                )),
                "cell" => Ok(profile_vm(&CellStack::default(), &w, technique, deadline)),
                "raw" => Ok(profile_vm(&RawStack::default(), &w, technique, deadline)),
                "slice" => Ok(profile_vm(
                    &SliceStack::new(&mut buffer),
                    &w,
                    technique,
                    deadline,
                )),
                _ => Err(format!("unknown stack {stack:?}")),
            }
        }
        _ => Err(format!("unknown case {case:?}; expected {PROFILE_CASES}")),
    }
}

fn profile_vm<S: RootStack>(
    stack: &S,
    w: &Workload,
    technique: Technique,
    deadline: Instant,
) -> u64 {
    let loaded = Loaded::<S>::new(w.program.clone()).expect("verified");
    let mut vm = Vm::new(stack, Pacing::default());
    let mut iterations = 0;
    while Instant::now() < deadline {
        let mut result = None;
        profiled_iteration(
            &mut || result = Some(vm.run(&loaded, technique, &w.args)),
            &mut iterations,
        );
        let value = result.expect("ran").expect("runs");
        assert_eq!(vm.digest(value), w.expected);
        // Free the run's heap outside the profiled work.
        vm.clear_registers();
        vm.collect();
    }
    iterations
}

fn profile_dispatch<S: RootStack>(stack: &S, deadline: Instant) -> u64 {
    let plan = plan::dispatch(20_000, 60_000, 50_000, SEED);
    let mut port: Port<'_, S> = built_port(stack, &plan);
    let mut iterations = 0;
    while Instant::now() < deadline {
        port.clear_log();
        profiled_iteration(&mut || port.apply(&plan.work), &mut iterations);
    }
    assert!(port.log() == plan.log);
    iterations
}

// ----- counts (S3-Q2) -----

/// One workload's counts, from a `count` build.
#[derive(Clone, Debug)]
pub struct Counts {
    pub workload: &'static str,
    pub technique: Technique,
    pub stats: Stats,
    pub roots: RootCounts,
    pub collections: u64,
}

impl Counts {
    /// Root operations per executed instruction.
    #[must_use]
    #[expect(clippy::cast_precision_loss, reason = "counts far below 2^52")]
    pub fn roots_per_instruction(&self) -> f64 {
        self.roots.total() as f64 / self.stats.instructions().max(1) as f64
    }

    /// Root operations in one context.
    #[must_use]
    pub fn in_context(&self, context: Context) -> u64 {
        self.roots.in_context(context)
    }
}

/// Count every workload at `size` with every technique, on C1's `SliceStack`
/// with default pacing. The counts are zero unless built with `count`; they
/// count the operations of frames and scoped values, which are the same on
/// every stack.
///
/// # Panics
/// If a run fails or disagrees with its native model, or (in a `count` build)
/// executes other instructions than the reference evaluator.
#[must_use]
pub fn counts(size: Size) -> Vec<Counts> {
    let mut rows = Vec::new();
    for w in workloads::all(size) {
        let reference = s3_vm::reference::run(&w.program, &w.args);
        let mut buffer = slot_buffer(DEFAULT_CAPACITY);
        let stack = SliceStack::new(&mut buffer);
        let loaded = Loaded::<SliceStack<'_>>::new(w.program.clone()).expect("verified");
        for &technique in Technique::ALL {
            let mut vm = Vm::new(&stack, Pacing::default());
            let _ = root_count::take();
            let _ = vm.take_stats();
            let value = vm.run(&loaded, technique, &w.args).expect("runs");
            let roots = root_count::take();
            let stats = vm.take_stats();
            assert_eq!(vm.digest(value), w.expected, "{} on {technique}", w.name);
            if root_count::enabled() {
                assert_eq!(
                    stats.instructions(),
                    reference.instructions,
                    "{}: the interpreter and the reference executed different instructions",
                    w.name
                );
            }
            rows.push(Counts {
                workload: w.name,
                technique,
                stats,
                roots,
                collections: vm.metrics().collections,
            });
        }
    }
    rows
}
