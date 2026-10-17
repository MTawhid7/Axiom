//! The benchmark suites (see the crate docs for what each one attributes),
//! plus the long-running loops that `s1-probe profile` records.

use core::hint::black_box;
use std::time::{Duration, Instant};

use s1_scenarios::{Size, Workload};

use crate::dom::{Config, Dom, ScenarioRun};
use crate::harness::{Measurement, Pattern, Samples, Sampling, alternate, timed, timed_with};
use crate::plan::{self, Order, Plan};
use crate::{PatternA, PatternB};

/// The seed of every generated input.
pub const SEED: u64 = 1;

/// The suites, in the order [`run`] runs them.
pub const SUITES: [&str; 6] = [
    "build", "traverse", "mutate", "dispatch", "collect", "scenario",
];

/// What to run.
#[derive(Clone, Debug, Default)]
pub struct Options {
    /// Tiny inputs and one sample per case: a fast, verifying smoke run.
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

    const fn scenario(&self) -> Sampling {
        if self.quick {
            Sampling {
                warmup: Duration::ZERO,
                samples: 1,
            }
        } else {
            Sampling {
                warmup: Duration::from_millis(500),
                samples: 5,
            }
        }
    }

    fn pick<T: Copy>(&self, full: &[T], quick: &[T]) -> Vec<T> {
        if self.quick { quick } else { full }.to_vec()
    }
}

/// Receives each A/B pair as soon as it is measured.
pub type Report<'a> = dyn FnMut([Measurement; 2]) + 'a;

/// Run every suite.
pub fn run(options: &Options, report: &mut Report<'_>) {
    build(options, report);
    traverse(options, report);
    mutate(options, report);
    dispatch(options, report);
    collect(options, report);
    scenarios(options, report);
}

/// `12_000` → `12k`.
fn short(n: u32) -> String {
    if n >= 1_000 && n.is_multiple_of(1_000) {
        format!("{}k", n / 1_000)
    } else {
        n.to_string()
    }
}

/// A heap holding `plan`'s setup state.
fn built<D: Dom>(plan: &Plan, config: Config) -> D {
    let mut dom = D::new(plan.node_slots, config);
    dom.apply(&plan.setup);
    dom
}

fn expect_digest<D: Dom>(dom: &D, expected: u64, case: &str) {
    assert_eq!(
        dom.tree_digest(),
        expected,
        "{case}: pattern {} reached a different tree than the reference",
        D::PATTERN,
    );
}

fn expect_objects<D: Dom>(dom: &D, expected: u64, case: &str) {
    assert_eq!(
        dom.heap().objects as u64,
        expected,
        "{case}: pattern {} holds the wrong number of objects",
        D::PATTERN,
    );
}

// ----- build -----

fn build(options: &Options, report: &mut Report<'_>) {
    for elements in options.pick(&[10_000, 100_000], &[200]) {
        let case = format!("tree-{}", short(elements));
        if !options.wants("build", &case) {
            continue;
        }
        let plan = plan::tree(elements, Order::Document, SEED);
        let a: PatternA = built(&plan, Config::MUTATOR);
        let b: PatternB = built(&plan, Config::MUTATOR);
        expect_digest(&a, plan.setup_digest, &case);
        expect_digest(&b, plan.setup_digest, &case);
        let (heap_a, heap_b) = (a.heap(), b.heap());
        drop((a, b));

        let [sa, sb] = alternate(
            options.micro(),
            || timed(|| built::<PatternA>(&plan, Config::MUTATOR)),
            || timed(|| built::<PatternB>(&plan, Config::MUTATOR)),
        );
        report([
            Measurement::new("build", &case, Pattern::A, plan.nodes, "node", &sa)
                .with("ops", plan.setup.len())
                .with("heap_bytes", heap_a.bytes),
            Measurement::new("build", &case, Pattern::B, plan.nodes, "node", &sb)
                .with("ops", plan.setup.len())
                .with("heap_bytes", heap_b.bytes),
        ]);
    }
}

// ----- traverse -----

fn traverse(options: &Options, report: &mut Report<'_>) {
    let large = if options.quick { 200 } else { 100_000 };
    for elements in options.pick(&[10_000, 100_000], &[200]) {
        let case = format!("tree-{}", short(elements));
        if options.wants("traverse", &case) {
            let plan = plan::tree(elements, Order::Document, SEED);
            traverse_case(options, report, &case, &plan, false);
        }
    }
    // The same tree, allocated level by level as the CP2 generator grows it.
    let case = format!("tree-{}-breadth-order", short(large));
    if options.wants("traverse", &case) {
        let plan = plan::tree(large, Order::Breadth, SEED);
        traverse_case(options, report, &case, &plan, false);
    }
    // The same tree after random moves: tree order no longer follows
    // allocation order, in either pattern.
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
    let mut a: PatternA = built(plan, Config::MUTATOR);
    let mut b: PatternB = built(plan, Config::MUTATOR);
    let expected = if with_work {
        a.apply(&plan.work);
        b.apply(&plan.work);
        plan.final_digest
    } else {
        plan.setup_digest
    };
    expect_digest(&a, expected, case);
    expect_digest(&b, expected, case);
    let fold = a.traverse();
    assert_eq!(
        fold,
        b.traverse(),
        "{case}: the patterns walked different trees"
    );
    assert_eq!(fold.nodes, plan.nodes, "{case}: every node is attached");

    let [sa, sb] = alternate(
        options.micro(),
        || timed(|| a.traverse()),
        || timed(|| b.traverse()),
    );
    report([
        Measurement::new("traverse", case, Pattern::A, fold.nodes, "node", &sa),
        Measurement::new("traverse", case, Pattern::B, fold.nodes, "node", &sb),
    ]);
}

// ----- mutate -----

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
    let mut a: PatternA = built(&plan, Config::MUTATOR);
    let mut b: PatternB = built(&plan, Config::MUTATOR);
    a.apply(&plan.work);
    b.apply(&plan.work);
    expect_digest(&a, plan.final_digest, &case);
    expect_digest(&b, plan.final_digest, &case);
    drop((a, b));

    let [sa, sb] = alternate(
        options.micro(),
        || {
            timed_with(
                || built::<PatternA>(&plan, Config::MUTATOR),
                |dom| dom.apply(&plan.work),
            )
        },
        || {
            timed_with(
                || built::<PatternB>(&plan, Config::MUTATOR),
                |dom| dom.apply(&plan.work),
            )
        },
    );
    let ops = plan.work.len() as u64;
    report([
        Measurement::new("mutate", &case, Pattern::A, ops, "op", &sa),
        Measurement::new("mutate", &case, Pattern::B, ops, "op", &sb),
    ]);
}

// ----- dispatch -----

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
    let mut a: PatternA = built(&plan, Config::MUTATOR);
    let mut b: PatternB = built(&plan, Config::MUTATOR);
    // The first pass must match the reference. Later passes repeat it
    // exactly: listeners only log, stop, or rewrite an attribute with the
    // value it already has.
    a.apply(&plan.work);
    b.apply(&plan.work);
    assert!(a.log() == plan.log, "{case}: pattern A logged differently");
    assert!(b.log() == plan.log, "{case}: pattern B logged differently");
    expect_digest(&a, plan.final_digest, &case);
    expect_digest(&b, plan.final_digest, &case);

    let [sa, sb] = alternate(
        options.micro(),
        || {
            a.clear_log();
            timed(|| a.apply(&plan.work))
        },
        || {
            b.clear_log();
            timed(|| b.apply(&plan.work))
        },
    );
    assert!(
        a.log() == plan.log,
        "{case}: pattern A's last pass differed"
    );
    assert!(
        b.log() == plan.log,
        "{case}: pattern B's last pass differed"
    );

    let invocations = plan.log.len();
    report([
        Measurement::new(
            "dispatch",
            &case,
            Pattern::A,
            plan.work.len() as u64,
            "dispatch",
            &sa,
        )
        .with("invocations", invocations),
        Measurement::new(
            "dispatch",
            &case,
            Pattern::B,
            plan.work.len() as u64,
            "dispatch",
            &sb,
        )
        .with("invocations", invocations),
    ]);
}

// ----- collect -----

fn collect(options: &Options, report: &mut Report<'_>) {
    for elements in options.pick(&[1_000, 4_000, 16_000, 64_000, 256_000], &[200]) {
        let live = format!("live-{}", short(elements));
        let garbage = format!("half-garbage-{}", short(elements));
        let (wants_live, wants_garbage) = (
            options.wants("collect", &live),
            options.wants("collect", &garbage),
        );
        if !wants_live && !wants_garbage {
            continue;
        }
        let plan = plan::tree(elements, Order::Document, SEED);
        if wants_live {
            collect_live(options, report, &live, &plan);
        }
        if wants_garbage {
            collect_garbage(options, report, &garbage, &plan);
        }
    }
}

/// Repeated full collections of an all-live heap: the cost of tracing.
fn collect_live(options: &Options, report: &mut Report<'_>, case: &str, plan: &Plan) {
    let mut a: PatternA = built(plan, Config::MUTATOR);
    let mut b: PatternB = built(plan, Config::MUTATOR);
    a.collect();
    b.collect();
    expect_objects(&a, plan.nodes, case);
    expect_objects(&b, plan.nodes, case);

    let [sa, sb] = alternate(
        options.micro(),
        || timed(|| a.collect()),
        || timed(|| b.collect()),
    );
    expect_objects(&a, plan.nodes, case);
    expect_objects(&b, plan.nodes, case);
    expect_digest(&a, plan.setup_digest, case);
    expect_digest(&b, plan.setup_digest, case);
    report([
        Measurement::new("collect", case, Pattern::A, plan.nodes, "object", &sa)
            .with("heap_bytes", a.heap().bytes),
        Measurement::new("collect", case, Pattern::B, plan.nodes, "object", &sb)
            .with("heap_bytes", b.heap().bytes),
    ]);
}

/// One full collection of a heap where every live node is followed by one
/// garbage node in allocation order: tracing plus freeing (A) or compacting (B).
fn collect_garbage(options: &Options, report: &mut Report<'_>, case: &str, plan: &Plan) {
    let config = Config {
        auto_collect: false,
        garbage_per_node: 1,
    };
    // Every node but the document gets one garbage neighbour.
    let before = 2 * plan.nodes - 1;
    let mut a: PatternA = built(plan, config);
    let mut b: PatternB = built(plan, config);
    expect_objects(&a, before, case);
    expect_objects(&b, before, case);
    a.collect();
    b.collect();
    expect_objects(&a, plan.nodes, case);
    expect_objects(&b, plan.nodes, case);
    expect_digest(&a, plan.setup_digest, case);
    expect_digest(&b, plan.setup_digest, case);
    drop((a, b));

    let [sa, sb] = alternate(
        options.micro(),
        || timed_with(|| built::<PatternA>(plan, config), Dom::collect),
        || timed_with(|| built::<PatternB>(plan, config), Dom::collect),
    );
    let freed = before - plan.nodes;
    report([
        Measurement::new("collect", case, Pattern::A, before, "object", &sa).with("freed", freed),
        Measurement::new("collect", case, Pattern::B, before, "object", &sb).with("freed", freed),
    ]);
}

// ----- scenario -----

fn scenarios(options: &Options, report: &mut Report<'_>) {
    let size = if options.quick {
        Size::Small
    } else {
        Size::Large
    };
    for workload in Workload::ALL {
        let case = workload.name();
        if !options.wants("scenario", case) {
            continue;
        }
        let generated = workload.generate(size, SEED);
        let script = &generated.script;
        let run_a = PatternA::run_script(script);
        let run_b = PatternB::run_script(script);
        for (pattern, run) in [(Pattern::A, &run_a), (Pattern::B, &run_b)] {
            if let Some(difference) = run.outcome.first_difference(&generated.expected) {
                panic!("{case}: pattern {pattern} differs from the reference: {difference}");
            }
        }

        let [sa, sb] = alternate(
            options.scenario(),
            || timed(|| PatternA::run_script(script)),
            || timed(|| PatternB::run_script(script)),
        );
        let ops = script.ops.len() as u64;
        report([
            scenario_measurement(case, Pattern::A, ops, &sa, &run_a),
            scenario_measurement(case, Pattern::B, ops, &sb, &run_b),
        ]);
    }
}

fn micros(d: Duration) -> String {
    format!("{:.1}", d.as_secs_f64() * 1e6)
}

fn scenario_measurement(
    case: &str,
    pattern: Pattern,
    ops: u64,
    samples: &Samples,
    run: &ScenarioRun,
) -> Measurement {
    let heap = &run.heap;
    let mean_pause = u32::try_from(heap.collections)
        .ok()
        .filter(|&n| n > 0)
        .map_or(Duration::ZERO, |n| heap.total_pause / n);
    let m = Measurement::new("scenario", case, pattern, ops, "op", samples)
        .with("collections", heap.collections)
        .with("max_pause_us", micros(heap.max_pause))
        .with("mean_pause_us", micros(mean_pause))
        .with("peak_live_nodes", run.peak_live_nodes)
        .with("peak_objects", heap.peak_objects)
        .with("peak_heap_bytes", heap.peak_bytes)
        .with("allocated_objects", heap.total_allocated_objects)
        .with("allocated_bytes", heap.total_allocated_bytes);
    match run.dispatch_yields {
        Some(yields) => m.with("dispatch_yields", yields),
        None => m,
    }
}

// ----- profiling loops -----

/// Every iteration of a profiling loop runs inside this function. It is never
/// inlined, and the count after the call keeps the call from becoming a
/// tail jump, so the function stays on the stack and a profile can be narrowed
/// to the loop: `tools/samply-summary.py --match profiled_iteration`.
#[inline(never)]
fn profiled_iteration(f: &mut dyn FnMut(), iterations: &mut u64) {
    f();
    *iterations += 1;
}

/// The cases `s1-probe profile` can loop.
pub const PROFILE_CASES: [&str; 9] = [
    "build",
    "traverse",
    "mutate",
    "dispatch",
    "collect-live",
    "scenario-tree",
    "scenario-listeners",
    "scenario-churn",
    "scenario-pressure",
];

/// Run one case's timed work over and over for about `seconds`, at the
/// full-size inputs, so a sampling profiler sees mostly that work. Returns
/// the number of iterations.
///
/// # Errors
/// If `case` is not one of [`PROFILE_CASES`].
pub fn profile<D: Dom>(case: &str, seconds: f64) -> Result<u64, String> {
    let deadline = Instant::now() + Duration::from_secs_f64(seconds);
    let mut iterations = 0;
    let mut until_deadline = |f: &mut dyn FnMut()| {
        while Instant::now() < deadline {
            profiled_iteration(f, &mut iterations);
        }
    };
    match case {
        "build" => {
            let plan = plan::tree(100_000, Order::Document, SEED);
            until_deadline(&mut || drop(black_box(built::<D>(&plan, Config::MUTATOR))));
        }
        "traverse" => {
            let plan = plan::tree(100_000, Order::Document, SEED);
            let dom: D = built(&plan, Config::MUTATOR);
            until_deadline(&mut || {
                black_box(dom.traverse());
            });
        }
        "mutate" => {
            // Ten times more mutations than setup ops, so they dominate.
            let plan = plan::mutate(20_000, 1_000_000, SEED);
            until_deadline(&mut || {
                let mut dom: D = built(&plan, Config::MUTATOR);
                dom.apply(&plan.work);
                black_box(dom);
            });
        }
        "dispatch" => {
            let plan = plan::dispatch(20_000, 60_000, 50_000, SEED);
            let mut dom: D = built(&plan, Config::MUTATOR);
            until_deadline(&mut || {
                dom.clear_log();
                dom.apply(&plan.work);
            });
        }
        "collect-live" => {
            let plan = plan::tree(256_000, Order::Document, SEED);
            let mut dom: D = built(&plan, Config::MUTATOR);
            until_deadline(&mut || dom.collect());
        }
        scenario if scenario.starts_with("scenario-") => {
            let name = &scenario["scenario-".len()..];
            let workload = Workload::ALL
                .into_iter()
                .find(|w| w.name() == name)
                .ok_or_else(|| format!("unknown workload {name:?}"))?;
            let generated = workload.generate(Size::Large, SEED);
            until_deadline(&mut || drop(black_box(D::run_script(&generated.script))));
        }
        other => {
            return Err(format!(
                "unknown profile case {other:?}; expected one of {PROFILE_CASES:?}"
            ));
        }
    }
    Ok(iterations)
}
