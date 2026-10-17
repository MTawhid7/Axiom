# Step 2b — Validation Spikes S1 and S2: Plan and Checkpoint Tracker

| | |
|---|---|
| **Status** | In progress |
| **Authorized** | 2026-10-16, by the owner |
| **Branch** | `step-2b`. It is merged into `main` only after owner review at CP9 |
| **Feeds** | [ADR-0002](../adr/0002-implementation-language.md) final acceptance; a future ADR-0003 on the GC handle/rooting pattern |
| **Last updated** | 2026-10-17 |

> **Resuming in a new session?**
> 1. Read §6 to find the first checkpoint that isn't ✅.
> 2. Run that checkpoint's verification commands to confirm the workspace state.
> 3. Continue from the checkpoint's deliverables list.
>
> Every completed checkpoint leaves the workspace building, tested and committed.

---

## 1. Purpose

Validate ADR-0002's central hypothesis before accepting it, and before any production engine code is written. The hypothesis: **a garbage-collected heap shared by JS objects and DOM nodes can be written in Rust with `unsafe` confined to the GC boundary, at acceptable ergonomic and performance cost on the target machine.**

S2 separately validates macOS platform integration and build cost.

## 2. Owner inputs (2026-10-16)

| Input | Effect |
|---|---|
| Step 2b authorized: spikes S1 and S2 | This plan |
| S1 compares Pattern A (gc-arena style) with Pattern B (Nova-style scoped handles with an explicit root context) | §4.2, §4.3 |
| Rust (stable, 2024 edition) provisionally accepted under evaluation §15.3 | ADR-0002 status updated |
| git initialized; baseline docs committed | `a223fe2` on `main` |
| Owner is fluent in Rust: write idiomatic, high-performance Rust without simplification | Resolves OQ-6 |
| Low memory use, CPU efficiency and power consumption are **primary** goals | Charter G4 updated; measured from CP1 on |
| Only CLI measurement tools: `samply`, `/usr/bin/time -l`, `cargo bench`, Miri | §7 |
| No calendar deadline or hourly budget; bite-sized, self-contained checkpoints | Resolves OQ-1; §6 |

## 3. Questions and pass criteria

| ID | Question | Measured by | Pass criterion |
|---|---|---|---|
| **S1-Q1** | Can the DOM layer be `#![forbid(unsafe_code)]`, with all `unsafe` inside the GC boundary? | Compiler; `tools/check-unsafe-policy.sh`; Miri | Both DOM crates compile under `forbid`. The policy check passes. Miri is clean on the GC crates and on the DOM scenario tests |
| **S1-Q2** | Is the ceremony for node ⇄ listener-closure cycles, and for dispatch that collects mid-way, acceptable? | Counts of GC-API "ceremony tokens", lines of code per operation, side-by-side excerpts | Owner judgment, informed by the data |
| **S1-Q3** | What does each pattern cost at run time? | Bench harness, GC statistics, `/usr/bin/time -l` | Reported, not pass/fail. Thresholds for the engine are set from this data |
| **S1-Q4** | What does the workspace cost to build on 8 GB? | `tools/measure-build.sh` | Reported. Thresholds for ADR-0002 revisit condition 3 are set here |
| **S2-Q5** | Can AppKit + `CAMetalLayer` + CoreText draw text with objc2, without Xcode, with `unsafe` confined to platform modules? | `--smoke` self-test; policy check | Smoke test passes; the owner confirms visually |
| **S2-Q6** | What do the objc2 crates cost to build, and what does the app cost to run? | `tools/measure-build.sh`; `/usr/bin/time -l` | Reported |

## 4. Experiment design

### 4.1 Workload model (`s1-scenarios`, independent of both patterns)

**Scripts.** A *script* is a deterministic, seed-generated sequence of DOM operations:
- create element or text;
- append, insert-before, remove;
- set attribute;
- add or remove a listener;
- dispatch an event;
- make a weak reference;
- forget a node;
- collect;
- checkpoint.

**The graph has real cycles.** Listener closures capture their own node (node → listener → closure → node). They may also capture a second node, which creates cross-subtree cycles.

**Listener behaviours:** log, set an attribute, append fresh children, allocate garbage, remove the captured node, stop propagation.

**Event dispatch:**
- capture, target and bubble phases;
- the listener list is snapshotted before invoking, with "removed" flags honoured;
- `stopPropagation`.

**Oracle (CP2).** A plain reference interpreter with no GC computes the expected `Outcome` for any script:
- the listener-invocation log;
- a tree digest;
- after each `Collect`, live node and closure counts derived from **reachability**, plus weak-reference liveness.

Each pattern must reproduce the reference outcome exactly (A ≡ reference ≡ B). An exact live count after a full collection proves the GC neither leaks nor frees early.

### 4.2 Pattern A — gc-arena style (crates `s1-a-gc`, `s1-a-gc-derive`, `s1-a-dom`)

- **Mutation scope.** All heap access happens inside `Arena::mutate(|mc: &Mutation<'gc>, root| …)`. `'gc` is an invariant, higher-ranked brand, so `Gc<'gc, T>` (a `Copy` pointer) cannot escape the closure.
- **Collection.** Collection happens only *between* `mutate` calls. Its only root is the arena root, so no stack scanning or rooting is needed.
- **Tracing.** `unsafe trait Trace` is implemented with `#[derive(Trace)]`.
- **Interior mutability.** `Lock` (Cell-like) and `RefLock` (RefCell-like) wrappers take `&Mutation` on writes, so write-barrier insertion points exist even though the first collector is non-incremental.
- **Weak references:** `GcWeak`.
- **Collector:** non-moving mark–sweep, triggered by an allocation-debt threshold. It is faithful to gc-arena: one global-allocator allocation per object, plus an intrusive list for sweeping.
- **Consequence:** a long dispatch that must be able to collect half-way is written as a **resumable task** (a stackless state machine) driven by a fuel-bounded executor. This is pattern A's main ergonomic cost, and S1 measures it.
- **Unsafe boundary:**
  - `s1-a-gc/src/raw/`;
  - the `unsafe impl Trace` emitted by `s1-a-gc-derive` (see §4.5).

### 4.3 Pattern B — Nova style (crates `s1-b-gc`, `s1-b-dom`)

- **Storage.** Typed arenas (`Vec`-backed) addressed by `u32` handles: `NodeRef<'a>`, `Value<'a>`, and so on.
- **GC tokens.** `GcScope<'a>` and `NoGcScope<'a>` are zero-sized tokens. A function that may collect takes `GcScope` (by value or via `reborrow()`). Handles are bound to a shared reborrow, so holding an unrooted handle across a possible collection is a **compile error**.
- **Escape hatch:** `bind` and `unbind`.
- **Explicit root context.** `RootFrame` and `Scoped<'f, T>` keep values alive across safepoints. The frame truncates its roots when dropped.
- **Collector.** Mark–compact. Forwarding is computed by rank, using popcount over mark bitsets, so no table is allocated per object. Every handle in live objects and roots is rewritten.
- **Stale handles.** A debug-build epoch inside each handle turns a stale handle into a deterministic panic, which makes misuse visible in tests.
- **Collection points:** any allocation (a safepoint). Dispatch needs no resumable rewrite; it needs rooting instead.
- **Unsafe target: none.** Index access is bounds-checked. If profiling justifies an unchecked-access variant, its `unsafe` stays in `s1-b-gc` and its benefit is measured.

### 4.4 Known confounds (stated up front)

Each pattern is built in its **canonical** form: gc-arena uses pointers and per-object allocation; Nova uses indices and typed arenas. So runtime differences mix two factors:
- the rooting discipline;
- the storage representation and allocator.

CP7 attributes differences carefully; for example, it compares allocation and traversal separately. Collector type (mark–sweep against mark–compact) is a further confound, and is reported as such.

### 4.5 Finding recorded before implementation (2026-10-16)

On rustc 1.94.1, `#![forbid(unsafe_code)]` **does not fire** on `unsafe impl` items generated by an external proc-macro derive or an external `macro_rules!` macro. It does reject hand-written `unsafe impl`.

The experiment was a minimal three-crate workspace, built in the scratchpad.

**Consequence:** `forbid(unsafe_code)` proves a crate has no *hand-written* `unsafe`, not that its expanded code has none. Macros that emit `unsafe` are part of the unsafe boundary. They must live in designated crates, be audited, and be registered in `tools/unsafe-policy.txt`. This has been added to evaluation §15.3 as rule 7.

## 5. Workspace structure

```
Arbor/
├── Cargo.toml              virtual workspace: shared package metadata, lints, profiles
├── Cargo.lock              committed (benchmarks and measurements must be reproducible)
├── rust-toolchain.toml     pinned stable 1.94.1 (+ rustfmt, clippy); Miri uses +nightly
├── rustfmt.toml · .gitignore · README.md
├── docs/                   source of truth (this file tracks Step 2b)
├── spikes/                 Step 2b validation code: throwaway, NOT production
│   ├── README.md
│   ├── s1/
│   │   ├── scenarios/      s1-scenarios     workload model, RNG, oracle (CP1–CP2)
│   │   ├── a-gc/           s1-a-gc          Pattern A heap; unsafe only in src/raw/ (CP3)
│   │   ├── a-gc-derive/    s1-a-gc-derive   #[derive(Trace)]; emits unsafe impl (CP3)
│   │   ├── a-dom/          s1-a-dom         DOM + driver on Pattern A; forbid(unsafe_code) (CP4)
│   │   ├── b-gc/           s1-b-gc          Pattern B heap; target: no unsafe (CP5)
│   │   ├── b-dom/          s1-b-dom         DOM + driver on Pattern B; forbid(unsafe_code) (CP6)
│   │   └── bench/          s1-bench         verified benchmark suites; s1-probe binary (CP7)
│   └── s2/window/          s2-window        AppKit + Metal + CoreText (CP8)
└── tools/
    ├── check-unsafe-policy.sh   + unsafe-policy.txt (allowlist)
    ├── measure-build.sh         clean / no-op / incremental builds
    ├── rustc-rss-wrapper.sh     RUSTC_WRAPPER: per-crate wall time and max RSS
    ├── linker-rss-wrapper.sh    -C linker=…: per-link wall time and max RSS
    ├── s1-bench.sh              runtime suites, N process runs, aggregated (CP7)
    ├── s1-memory.sh             per-process peak footprint, RSS, instructions (CP7)
    ├── s1-profile.sh            samply profiles per case (CP7)
    ├── samply-summary.py        profile → hot functions and source lines (CP7)
    └── s1-ceremony.sh           GC-API tokens and code size of the DOM crates (CP7)
```

**Engine crates** will live in `crates/` once the architecture is approved. The workspace manifest (lints, profiles, toolchain) is built to be reused unchanged when that happens.

**Spike results** go into notes in `docs/research/`. The spike code stays in git history and is deleted or archived after CP9.

## 6. Checkpoints

**Every checkpoint ends with all of the following green, then a commit on `step-2b` and an update to this tracker:**

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
tools/check-unsafe-policy.sh
```

| CP | Scope | Status |
|---|---|---|
| **CP1** | Foundation: workspace, toolchain pin, lints, policy checker, build-measurement tooling, scenario model types and RNG | ✅ 2026-10-16 (§8) |
| CP2 | Workload generators and reference-interpreter oracle | ✅ 2026-10-16 (§8) |
| CP3 | Pattern A heap and derive; Miri | ✅ 2026-10-16 (§8) |
| CP4 | Pattern A DOM and driver; oracle equality; Miri on small scenarios | ✅ 2026-10-16 (§8) |
| CP5 | Pattern B heap; Miri | ✅ 2026-10-16 (§8) |
| CP6 | Pattern B DOM and driver; oracle equality; A ≡ B differential | ✅ 2026-10-16 (§8) |
| CP7 | S1 measurements (runtime, ceremony, build) and S1 results note | ✅ 2026-10-17 (§8; [S1 results](s1-results.md)) |
| CP8 | S2 platform spike, measurements, and S2 results note | 🔄 8a ✅ (window, Metal, CoreText, smoke oracle, tests, measurement script); 8b measurements; 8c results note |
| CP9 | Synthesis: ADR-0002 final recommendation; ADR-0003 (GC pattern) proposed; docs | ⬜ |

### CP1 — Foundation

**Deliverables**
- Root workspace files, as in §5.
- `spikes/README.md`.
- `tools/check-unsafe-policy.sh`:
  - every crate root must declare `#![forbid(unsafe_code)]` unless it is on the allowlist;
  - allowlisted crates must declare `#![deny(unsafe_code)]`, and may contain `allow(unsafe_code)` only in their listed modules;
  - macro crates that emit `unsafe` must be listed;
  - prints an `unsafe` inventory;
  - has a `--self-test` mode that seeds violations and expects to catch them.
- `tools/measure-build.sh` and its wrappers.
- `s1-scenarios`:
  - the deterministic RNG (xoshiro256++ seeded by SplitMix64; Lemire bounded sampling);
  - script model types (`Op`, `Behavior`, identifiers, `Script`);
  - `Outcome` types;
  - FNV-1a digest;
  - tests.

**Verification**
- The four standard commands.
- `tools/check-unsafe-policy.sh --self-test`.
- `tools/measure-build.sh -p s1-scenarios`, with the baseline numbers recorded in §8.

### CP2 — Workload generators and oracle

**Deliverables**
- Generators:
  - `tree`: N nodes with a realistic depth and fan-out mix, plus attributes;
  - `listeners`: listener-heavy, with self captures and cross captures;
  - `churn`: random mutation and dispatch, plus forget and collect, which creates garbage cycles;
  - `pressure`: listeners that allocate garbage during long dispatches.
- `reference::run(&Script) -> Outcome`, including:
  - the dispatch semantics in §4.1;
  - a live count after each `Collect`, computed by BFS reachability from the document, the driver's id table, and listener captures;
  - weak-reference liveness.

**Verification**
- Golden tests: small hand-written scripts with hand-checked outcomes.
- 100 seeds per generator run on the reference without error.
- Same seed ⇒ identical outcome digest.

### CP3 — Pattern A heap

**Deliverables:** §4.2's GC core:
- `Arena<R: Rootable>`, `Mutation<'gc>`, `Gc`, `GcWeak`, `Lock`, `RefLock`, `Trace`, and the derive;
- mark–sweep with allocation debt;
- metrics: object and byte counts, collections, pauses.

**Verification**
- Unit tests:
  - unreachable cycles are freed and their `Drop` runs;
  - reachable objects are kept;
  - weak references are cleared;
  - randomized object graphs match a reachability oracle.
- `rustup component add miri --toolchain nightly` (one-time), then `cargo +nightly miri test -p s1-a-gc` (sizes reduced under `cfg(miri)`).
- Policy check: `unsafe` only in `src/raw/`.
- `unsafe` inventory recorded.

### CP4 — Pattern A DOM and driver

**Deliverables**
- `s1-a-dom` (`#![forbid(unsafe_code)]`):
  - nodes with intrusive parent and sibling links;
  - attributes;
  - listener and closure objects;
  - dispatch as a resumable task;
  - a script driver with a fuel-bounded executor that collects between steps when allocation debt is due.

**Verification**
- Outcome equals the reference on the golden tests and 50 seeds per generator.
- `pressure`: peak live nodes stays within the bound set in CP2.
- Miri passes on the small scripts.
- Policy check passes.

### CP5 — Pattern B heap

**Deliverables:** §4.3's GC core:
- typed arenas and handles;
- scopes, root frames and `Scoped`;
- mark–compact with rank-based forwarding;
- weak references;
- debug epochs;
- a `macro_rules!` macro that composes a heap.

**Verification**
- Unit tests:
  - compaction under randomized graphs matches the reachability oracle;
  - `#[should_panic]` tests prove stale-handle detection.
- Miri passes.
- Policy check: zero `unsafe` is the target.

### CP6 — Pattern B DOM and driver

**Deliverables:** `s1-b-dom` (`#![forbid(unsafe_code)]`), with the same semantics as CP4. Collection happens at allocation safepoints; rooting replaces the resumable task.

**Verification**
- Outcome equals the reference on the golden tests and 50 seeds per generator.
- A ≡ B on all generated seeds.
- `pressure` stays within the bound.
- Miri passes on the small scripts.
- Policy check passes.

### CP7 — S1 measurements and results

**Deliverables**
- `s1-bench` (`harness = false`), measuring:
  - build, traverse, mutate and dispatch throughput;
  - GC pause (median and max) against live-set size;
  - bytes per node;
  - peak RSS per scenario (`/usr/bin/time -l`);
  - `samply` profiles of hot paths (installs `samply` via `cargo install --locked samply`).
- Ceremony metrics per DOM crate.
- Build metrics for S1 crates in dev and release.
- `docs/research/s1-results.md`, containing the GC-pattern recommendation.

**Verification:** three runs per measurement, reporting the median and spread. The machine is fanless, so cool-down and thermal state are noted.

**Sub-steps** (each committed with the four standard checks green):

| Step | Content | Status |
|---|---|---|
| 7a | `s1-bench` crate and `s1-probe` binary; reference-validated inputs (`plan`); measurement scripts; shakedown and methodology fixes | ✅ |
| 7b | Formal runs: `tools/s1-bench.sh`, `tools/s1-memory.sh`, `tools/s1-profile.sh`, `tools/s1-ceremony.sh`, `tools/measure-build.sh` (dev and release) | ✅ |
| 7c | `docs/research/s1-results.md`: results, attribution, recommendation | ✅ |

### CP8 — S2 platform spike

**Deliverables**
- `s2-window`:
  - objc2 AppKit window;
  - `CAMetalLayer`;
  - a Metal pipeline whose MSL is compiled at run time (no Xcode);
  - CoreText line → `CGBitmapContext` → texture → quad;
  - `--smoke` mode: renders frames, reads back pixels, asserts that text pixels exist, then exits.
- Measurements:
  - build;
  - binary size;
  - time to first frame;
  - idle RSS and CPU.
- `docs/research/s2-results.md`.

**Verification**
- `cargo run -p s2-window -- --smoke` exits 0.
- The owner checks the window visually.
- Policy check passes, with `unsafe` confined to platform modules.

**Sub-steps** (each committed with the four standard checks green):

| Step | Content | Status |
|---|---|---|
| 8a | `s2-window`: AppKit window; `CAMetalLayer`; a Metal pipeline compiled from MSL at run time; CoreText → `CGBitmapContext` → texture → quad; `--smoke` with an exact pixel oracle; unit tests; `tools/s2-measure.sh` | ✅ |
| 8b | Formal runs: `tools/s2-measure.sh`; `tools/measure-build.sh` in dev and release, with minimal and full bindings | ⬜ |
| 8c | `docs/research/s2-results.md` | ⬜ |

### CP9 — Synthesis

**Deliverables**
- Updates to the language evaluation and ADR-0002, as a final recommendation for the owner.
- ADR-0003 (GC handle/rooting pattern), Proposed.
- Updates to the decision map, status and open questions.
- Branch ready for owner review and merge.

## 7. Measurement methods

| What | How |
|---|---|
| Build wall time | `/usr/bin/time -l cargo build …` in an isolated target directory (`target/measure`). Clean, no-op and incremental builds; for incremental, a chosen source file is touched |
| Peak RSS per `rustc` invocation | `RUSTC_WRAPPER=tools/rustc-rss-wrapper.sh` runs each `rustc` under `/usr/bin/time -l -o …`. Logged as crate, wall time, max RSS and peak memory footprint |
| Peak RSS per link | `-C linker=tools/linker-rss-wrapper.sh` wraps `cc` the same way |
| Aggregate concurrent build RSS | A `ps -axo rss=,comm=` sampler (200 ms) sums `rustc`, `cc`, `ld` and `clang` while cargo runs, and records the peak. This matters on 8 GB with parallel jobs |
| Runtime throughput and latency | `s1-bench` suites (`cargo bench -p s1-bench`, or `tools/s1-bench.sh` for N runs). Per case: warm-up for a fixed time (300 ms; 500 ms for whole scenarios), then 11 samples per pattern (5 for scenarios), alternating A and B in ABBA order. Each run reports the median, min and max. Across 3 separate process runs with 60 s cool-downs, the summary reports the median of the run medians and their range |
| Correctness of what is timed | Every case first checks both patterns against the reference results its plan carries (tree digests, dispatch logs, object counts, scenario outcomes), and the two traversal folds against each other. `cargo test` runs the same checks on small inputs |
| GC pauses, heap bytes, object counts | Each GC crate's metrics API; full-collection pauses are also timed directly in the `collect` suite |
| Peak memory and CPU work at run time | `tools/s1-memory.sh`. Each input is written once as a script file. Each measurement is a fresh `s1-probe run <none\|a\|b>` process under `/usr/bin/time -l`: peak memory footprint, max RSS, instructions retired. `none` only parses the script, and is subtracted |
| CPU profiles | `tools/s1-profile.sh`: `samply record` on the `profiling` profile (release plus line tables), 1 kHz. `tools/samply-summary.py` narrows each profile to the timed loop and resolves samples to source lines with `dsymutil` and `atos -i` (Command Line Tools). Interactive view: `samply load` (Firefox Profiler; no Xcode) |
| Ceremony | `tools/s1-ceremony.sh`: GC-API token counts and code size per file and per operation in the two DOM crates |
| Undefined behaviour | `cargo +nightly miri test` on GC crates and on small DOM scenarios |

**Caveats**
- The MacBook Air is fanless, so sustained load throttles. Runs are repeated, with cool-downs, and the thermal state is noted.
- Background load on the machine is not controlled. Medians of repeated runs are reported, not single runs.
- **Clock ramp-up.** Apple Silicon raises a core's frequency only after it has been busy for a while, so warm-up is measured in time, not iterations (found in CP7; see §8).
- **P-cores and E-cores.** Which core a benchmark runs on cannot be pinned from safe code. Instructions retired (`s1-memory.sh`) are a frequency-independent cross-check on wall-time ratios.

## 8. Results log

### CP1 — completed 2026-10-16

**Verification (all green)**
- `cargo fmt --all --check` passes.
- `cargo clippy --workspace --all-targets -- -D warnings` passes (pedantic enabled).
- `cargo test --workspace`: 17/17 tests pass. They include:
  - the SplitMix64 reference vector;
  - the FNV-1a 64 reference vectors;
  - a deterministic uniformity check for bounded sampling;
  - per-thread counter isolation.
- `tools/check-unsafe-policy.sh`: OK. The only crate (`s1-scenarios`) is `forbid`, with zero `unsafe`.
- `tools/check-unsafe-policy.sh --self-test`: accepts a compliant tree and reports all 5 seeded violations (R1 ×3, R2 ×2) plus the `emits-unsafe` registration.

**Baseline build cost.** Command: `tools/measure-build.sh -p s1-scenarios --runs 3 -- --all-targets`. Conditions: rustc 1.94.1, M1, 8 GB; load average about 3 with an editor running; no thermal warning recorded.

| Phase | Wall time, median (n=3) | rustc compilations | Max rustc RSS | Max linker RSS | Aggregate peak RSS |
|---|---|---|---|---|---|
| Clean | 0.96 s (range 0.80–2.47; the first run was a cold cache) | 3 | ~157–163 MiB | ~63 MiB | ~171–222 MiB |
| No-op | 0.05 s | 0 | — | — | — |
| Incremental (touch `lib.rs`) | 0.45 s | 2 | ~106 MiB | ~63 MiB | ~95–101 MiB |

**Notes**
- **The tools are validated.** The `rustc` and linker wrappers record per-process peaks, and the `ps` sampler records the aggregate. Interpreting the numbers waits until there is real code (CP3 onward).
- **The baseline is floor overhead.** These numbers are the fixed cost of the toolchain plus a small dependency-free crate. They are the reference point for every later measurement.

### CP2 — completed 2026-10-16

**Delivered** (`s1-scenarios`, still `forbid(unsafe_code)` and dependency-free)
- `reference`: the oracle interpreter, with an incremental API.
  - Its live counts come from a graph search over the documented roots and edges.
  - It validates scripts and rejects invalid ops with a position and a reason (11 error kinds).
- `generate`: the four workloads (`tree`, `listeners`, `churn`, `pressure`) at sizes Small, Medium and Large.
  - Generators drive the reference model while they record, so scripts are valid by construction and come with their expected outcome.
- `examples/workload_stats.rs` prints the volume of each workload.

**Verification (all green)**
- **Golden tests:** 10 hand-checked scripts covering:
  - all three phases, and `bubbles = false`;
  - `stopPropagation`, both mid-path and during capture at the target;
  - the snapshot's *removed* flag;
  - a self-capturing cycle that lives while rooted and dies when not;
  - an extra capture keeping a forgotten, detached node alive;
  - the event path staying fixed after the target detaches itself;
  - fresh children joining the tree;
  - garbage that is counted but never observable;
  - DOM insertion edge cases (insert before itself; appending moves the node);
  - error reporting.
- **100 seeds × 4 workloads (Small):** every generated script is valid, and re-running it reproduces the expected outcome and statistics.
- **Determinism:** the same seed gives an identical script and outcome digest; a different seed gives a different script.
- **Medium sizes** are valid too.
- **Coverage gate** across 100 Listeners and Churn seeds. Each of these occurs, so outcome equality is a strong oracle:
  - stopped dispatches;
  - removed-flag skips;
  - all three phases;
  - weak references that both survive and die;
  - reclaimed garbage.
- **Tree shape:** element count is exact, and depth sits within its bounds and is not unrealistically shallow.
- **Pressure invariant:** the two checkpoints are identical, and garbage equals `dispatches × path_depth × listeners_per_node × garbage_per_invocation`.
- The four standard checks pass (34 tests).

**Workload volumes** (seed 1; `cargo run --release -p s1-scenarios --example workload_stats`)

| Workload | Size | Ops | Nodes allocated | of which garbage | Listeners | Invocations | Max live nodes | Generation |
|---|---|---|---|---|---|---|---|---|
| tree | Medium | 23,662 | 5,218 | 0 | 0 | 0 | 5,218 | 7.5 ms |
| listeners | Medium | 11,236 | 5,486 | 3,090 | 3,000 | 8,859 | 2,376 | 8.7 ms |
| churn | Medium | 30,431 | 5,956 | 227 | 1,862 | 596 | 4,237 | 9.0 ms |
| pressure | Medium | 1,334 | 131,491 | 131,072 | 64 | 512 | 419 | 0.4 ms |
| tree | Large | 590,151 | 129,839 | 0 | 0 | 0 | 129,839 | 97 ms |
| listeners | Large | 214,158 | 196,128 | 147,952 | 60,000 | 707,719 | 47,741 | 205 ms |
| churn | Large | 452,979 | 90,738 | 5,369 | 28,100 | 15,544 | 61,360 | 119 ms |
| pressure | Large | 12,312 | 2,101,187 | 2,097,152 | 256 | 4,096 | 4,035 | 2.6 ms |

**Pressure bound (checked in CP4 and CP6).** Let:
- *L* be the live-node count at the pressure checkpoints (Medium: 419);
- *B* be the pattern's allocation budget between collection opportunities (set in CP3 and CP5);
- *s* be `garbage_per_invocation`.

A pattern that can collect during a dispatch must satisfy **peak live nodes ≤ L + B + s**. One that cannot collect mid-dispatch peaks near **L + total garbage** (Medium: about 131k; Large: about 2.1M). The workload is sized so that the two outcomes are far apart.

**Changes and findings during CP2**
- **Coverage gap closed.** No op or behaviour could exercise the snapshot's *removed* flag, because that needs an *earlier* listener to remove a *later* one mid-dispatch. Fixes:
  - added `Behavior::RemoveListener`;
  - added `ScriptBuilder::peek_next_listener`;
  - generators create "removal pairs" with it.
- **Generator bug caught by its own guard.** The model's `child_ids()` and `parent()` queries returned ids the driver had already forgotten, which scripts can no longer name. They now return live ids only. The panic-on-invalid-op guard caught this on the first run.
- **Realism fix.** Five percent of listeners on the document made `listeners` Large generate 11.7M invocations, almost all of them document listeners, since every path ends at the document. The share is now a parameter (`ListenerMix`): 5% / 1% / 0.2% for Small / Medium / Large. Large dropped to 708k invocations.

### CP3 — completed 2026-10-16

**Delivered**
- **`s1-a-gc`, a canonical gc-arena design:**
  - `Arena<R: Rootable>` with `mutate` and `mutate_root`, using higher-ranked closures branded by `'gc`;
  - `Gc` (pointer-sized and `Copy`) and `GcWeak`;
  - `Mutation` (allocation, plus `wants_collection` for cooperative yielding);
  - `unsafe trait Trace` and `Tracer`;
  - `Lock` and `RefLock`, writable only through `Gc::write` and the `Write` proof, via the `field!` and `unlock!` projection macros;
  - a non-moving, stop-the-world mark–sweep: tri-colour marks, an iterative gray stack, and per-type vtables built by const promotion;
  - weak references whose dead boxes keep their memory while still weakly referenced;
  - `Pacing` (allocation debt) and `Metrics`.
- **`s1-a-gc-derive`:** `#[derive(Trace)]` for structs, tuple structs, unit structs and enums, with type parameters and `#[trace(require_static)]`. Every derived type also gets a compile-time proof that it has no `Drop` impl.
- **Invariants I1–I6** are documented in `src/raw/mod.rs`, and every `SAFETY:` comment cites them.

**Verification (all green)**
- **Tests:**
  - 2 unit tests: pacing arithmetic, and the header size (pinned at 24 bytes).
  - Compile-time `NEEDS_TRACE` assertions.
  - 2 derive tests.
  - 9 heap tests. They cover cycle collection with `Drop` running, data integrity across collections, dead weak targets keeping their memory, weak upgrades, pacing, barrier conveniences and accounting, a 200,000-node chain (no stack overflow and no leak), and teardown.
  - A randomized oracle test: 15 rounds × 400 new nodes, plus random strong edges, weak edges and roots, checked against an independent reachability model. It checks live values, live boxes, edge integrity, and weak upgrades.
- **Doctests:** one runnable example, plus **6 compile-fail tests**:

  | Rule the compiler enforces | Error |
  |---|---|
  | No escaping a `Gc` from its scope | "lifetime may not live long enough" |
  | No mixing pointers from two arenas | E0521 |
  | No collection inside a mutation scope | E0502 |
  | No `Drop` on a derived type | E0119 |
  | No `Gc` hidden in a `require_static` field | E0521 |
  | No `Gc` in a plain `Cell`, which would bypass the barrier | E0521 |

  Each snippet was compiled on its own and confirmed to fail for its intended reason, and the codes are pinned in the doctests.
- **Miri** (nightly 2026-04-08), on the whole suite, two ways:
  - Stacked Borrows with `-Zmiri-strict-provenance`: **no undefined behaviour**;
  - Tree Borrows with `-Zmiri-strict-provenance`: **no undefined behaviour**.

  Miri's leak checker is on by default (confirmed by a deliberately leaking program), so every test path also frees all memory.
- Clippy (pedantic, `-D warnings`), rustfmt, and the policy check all pass.

**Unsafe inventory.** Everything sits under `src/raw/`; the arena, metrics and crate root have none.

| File | `unsafe {}` | `unsafe fn` (incl. fn-pointer types) | `unsafe impl` lines | `SAFETY:` comments |
|---|---|---|---|---|
| `raw/gc.rs` | 8 | 4 | 2 | 10 |
| `raw/heap.rs` | 10 | 0 | 0 | 10 |
| `raw/lock.rs` | 4 (one inside `field!`) | 1 | 2 | 6 |
| `raw/trace.rs` | 2 | 2 | 12 (macros generate the rest) | 14 |

Macro-emitted `unsafe` (rule 7) comes from exactly two audited templates:
- the derive's `unsafe impl Trace`;
- `field!`'s `Write::assume`.

**Build cost.** Command: `tools/measure-build.sh -p s1-a-gc --runs 3 -- --all-targets`. This includes `syn`, `quote` and `proc-macro2`, plus all test targets.

| Phase | Median wall (n=3) | rustc compilations | Max rustc RSS | Max linker RSS | Aggregate peak RSS |
|---|---|---|---|---|---|
| Clean | 2.57 s (2.57–3.38) | 13 | 252 MiB (`syn`, 0.78 s) | 69 MiB | 239–290 MiB |
| No-op | 0.04 s | 0 | — | — | — |
| Incremental (touch `s1-a-gc/src/lib.rs`) | 0.33 s | 4 | 95 MiB | 64 MiB | 155–170 MiB |

**Size:** 812 non-blank, non-comment lines in `s1-a-gc/src`, 144 in the derive, 395 in tests.

**Findings**
1. **The arena layer needs no `unsafe` at all.** The mutation closures are higher-ranked over `'gc`, so `Arena::new`, `mutate` and `mutate_root` can instantiate `'gc = 'static` with no lifetime transmutes. All the unsafety is concentrated in five concerns:
   - allocation and freeing;
   - dereferencing pointers;
   - sweeping;
   - vtable dispatch;
   - the `Write` projection and the `Trace` impls.
2. **`Drop` is a real soundness hazard in this design.** The sweep drops values in arbitrary order. The derive therefore *proves* that the deriving type has no `Drop` impl, and the compile-fail test confirms it. Fields such as `Live` counters may still have `Drop`, since they cannot reach a `Gc`.
3. **Barrier discipline is enforced by types.** `Cell` and `RefCell` are `Trace` only for `'static` contents, so a `Gc` can live only in `Lock`/`RefLock`, and those are writable only through `Gc::write`. This is checked by a compile-fail test. The cost is ceremony: `unlock!(Gc::write(mc, node), Node, field).set(..)`. CP4 measures it.
4. **The sweep is panic-safe.** A box is marked dead *before* its value is dropped, so a panicking `Drop` can never cause a double drop.
5. **The pressure bound's budget.** Pattern A's *B* is the `Pacing` budget: by default the larger of 256 KiB and the bytes that survived the last collection. Each box costs 24 bytes of header plus its value, before allocator rounding.

### CP4 — completed 2026-10-16

**Delivered**
- **`s1-a-dom`** (`#![forbid(unsafe_code)]`):
  - the DOM model: nodes with intrusive parent and sibling links in `Lock`s; attributes in a plain `RefCell`, since they hold no `Gc`; listeners; closures capturing their node (node ⇄ closure cycles);
  - DOM pre-insert and remove;
  - **dispatch as a resumable state machine stored in the arena root**;
  - a script driver that gives each op its own mutation scope and calls `collect_debt` between scopes. A dispatch yields whenever `Mutation::wants_collection()` says so.
- **`s1_scenarios::edge_cases`:** a corpus of 10 scripts, one per rarely hit semantic path. It mirrors the golden tests, and lets every pattern be compared on exactly those paths.

**Verification (all green)**
- **Outcome equals the reference** (log, digests, live node and closure counts after each full collection, weak liveness) on:
  - the 10 edge cases;
  - 100 seeds × 4 workloads (Small);
  - 3 seeds × 4 workloads (Medium).

  Each set passes both at default pacing and in **collect-always mode**. Collect-always is a 1-byte budget that forces a collection at nearly every op and listener boundary; the Medium set is run at default pacing.
- **Pressure bound** (Medium, seed 1): peak live nodes **2,211 ≤ bound 2,314** (L + B + s). Without mid-dispatch collection the peak would be about 131,491. The run made 73 dispatch yields and 75 collections, with a maximum pause of 145 µs.
- **The yield path is exercised.** In collect-always mode the pressure scripts must yield, and the test asserts it, under Miri too.
- **Miri**, Small sizes plus the edge cases (a shallower chain under Miri), at both pacings:
  - Stacked Borrows with strict provenance: no undefined behaviour;
  - Tree Borrows with strict provenance: no undefined behaviour.

  So the unsafe core is checked while it serves real DOM workloads, with collections during dispatch. The Medium and pressure tests are skipped under Miri for time, since the Small tests cover the same paths.
- **Policy:** `s1-a-dom` is `forbid` with zero `unsafe`.
- **Workspace:** 60 tests pass; clippy (pedantic), rustfmt and the policy check are green.

**S1-Q1 for Pattern A: answered yes, with one qualification.**
- **Yes:** a complete DOM with cycles, weak references, tree surgery and collection mid-dispatch compiles under `#![forbid(unsafe_code)]` with no hand-written `unsafe`. Its correctness is checked against the oracle and under Miri.
- **The qualification (rule 7):** its *expanded* code contains `unsafe` from exactly two audited templates in the GC boundary:
  - `#[derive(Trace)]`'s `unsafe impl`;
  - `unlock!`/`field!`'s `Write::assume`.

**Preliminary ceremony inventory** (`s1-a-dom/src`, 503 non-blank, non-comment lines; CP7 formalizes it):

| Ceremony item | Count |
|---|---|
| `Gc::write(...)` barrier calls | 12 |
| `unlock!(...)` projections | 16 |
| `#[trace(require_static)]` attributes, needed for foreign `'static` types (orphan rule) | 9 |
| `mc: &Mutation` parameters threaded through | 8 |
| Mutation scopes in the driver | 5 |
| Yield-protocol handling sites | 4 |

Beyond those counts, the whole dispatch state machine (`dispatch.rs`) exists only because collection cannot happen inside a call. Listener *code* written in this style would have to be stackless as well.

**Findings**
1. **The stackless requirement is the dominant cost.**
   - **What it costs:** dispatch had to become an explicit state machine: a flattened visit sequence, a cursor, a snapshot and a stop flag, all stored in the root.
   - **Why:** listener behaviours are atomic here. A real JS interpreter would have to be stackless too (as in piccolo) for an allocating loop to be collectible.
2. **Plain `Cell`/`RefCell` for `Gc`-free data avoids barrier ceremony.** That applies to attributes and flags. The barrier tax falls only on fields that hold `Gc` pointers.
3. **The orphan rule adds friction.** Types from other crates cannot implement `Trace`, so every `'static` foreign field needs `#[trace(require_static)]` (9 here). A real engine would want a `Static<T>` wrapper, or would co-locate its DOM data types with the GC traits.
4. **Edge cases caught an authoring mistake immediately.** An edge-case script that removed a node after forgetting its id was rejected by the reference oracle, which is the validation layering working as intended.

### CP5 — completed 2026-10-16

**Delivered: `s1-b-gc`, a canonical Nova design, `#![forbid(unsafe_code)]`**
- **Handles.** `Handle<'a, T>` is a typed index, stored plus one in a `NonZeroU32`: 4 bytes, and `Option<Handle>` is 4 bytes too. Debug builds add the collection epoch.
- **Scope tokens.** `GcScope`, `NoGcScope` and `GcToken`, with `bind`, `unbind`, `reborrow`, `nogc` and `into_nogc`.
- **Explicit root context.** `RootStack` (`Rc<RefCell<…>>`) with RAII `RootFrame`s and `Scoped<'f, T>`.
- **Arenas.** `Arena<T>`, indexed by handle, bounds-checked, with epoch checks in debug builds.
- **Tracing.** A *safe* `Trace` trait (`mark` and `relocate`, hand-written as in Nova), plus `Roots` for globals and `WeakHandle`.
- **Collector.** Mark–compact: a worklist mark; survivor ranks from per-word prefix popcounts (4 bytes per 64 slots, no table per object); order-preserving compaction with `retain`; relocation of every handle in heap data, globals and root slots.
- **`heap!`,** a `macro_rules!` that expands to safe code only: arenas, `Allocated`, `Store` and `HeapLayout` impls, `new`, `alloc` and `collect`.
- **`Pacing` and `Metrics`,** with the same policy as Pattern A, so the two are comparable.

**Verification (all green)**
- **9 heap tests:**
  - cycles collected and their `Drop` run;
  - compaction that *moves* every survivor, with every handle verified afterwards;
  - weak handles cleared or relocated;
  - scoped roots surviving a collection relocated, and released when the frame drops;
  - pacing;
  - a 200,000-node chain;
  - the randomized reachability-oracle test (15 rounds × 400 nodes, with edges verified by id after compaction).
- **Two debug-build detection tests, both passing as designed:**
  - using a stale handle panics with "stale handle";
  - a `Trace` impl that forgets to mark panics with "some Trace::mark missed it" when the collector relocates the dead target.
- **Doctests:** one runnable example, plus 2 compile-fail tests with pinned codes:
  - a handle bound to `gc.nogc()` and used after `gc.reborrow()` is passed to a collecting call: **E0502**;
  - a `Scoped` that outlives its frame: **E0597**.
- **Miri**, under Stacked Borrows and Tree Borrows with strict provenance: no undefined behaviour. This is expected, with zero `unsafe`, but it was run, not assumed.
- **Policy:** `s1-b-gc` is `forbid` with 0 `unsafe`, and the macro emits none.

**Build cost.** Command: `tools/measure-build.sh -p s1-b-gc --runs 3 -- --all-targets`. There are no proc-macro dependencies.

| Phase | Median wall | rustc compilations | Max rustc RSS | Max linker RSS | Aggregate peak |
|---|---|---|---|---|---|
| Clean | 1.13 s | 5 | 167–170 MiB | 65 MiB | 192–223 MiB |
| No-op | 0.05 s | 0 | — | — | — |
| Incremental | 0.45 s | 3 | 95 MiB | 65 MiB | 40–61 MiB |

**Size:** 708 non-blank, non-comment lines in `src`, 322 in tests.

**Findings**
1. **Pattern B needs no `unsafe` anywhere:** not in the GC, not in its macro, not in clients.
   - Bounds-checked indices turn every memory-safety question into a logic question.
   - The scope discipline and the debug epoch checks then catch the logic errors, at compile time and run time respectively.
   - Pattern A's soundness, by contrast, rests on branding (I2) and on complete tracing (I3).
2. **Its `Trace` is safe and hand-written, but mistakes are caught.** As in Nova, `mark` and `relocate` are hand-written. A missed `mark` is not undefined behaviour, and debug builds detect it at the next collection; the test proves it. A missed `relocate` trips the epoch check.
3. **There are two runtime costs to measure in CP7:**
   - the bounds and epoch checks on every access;
   - moving objects during compaction.

   They buy contiguous, dense arenas and 4-byte handles, which should show up in memory per node.

### CP6 — completed 2026-10-16

**Delivered: `s1-b-dom`, the same DOM in authentic Nova style, `#![forbid(unsafe_code)]`**
- **Data.** `NodeData`, `ListenerData` and `ClosureData` live in a `heap!`-declared `DomHeap`, with hand-written `Trace` impls.
- **Mutation.** Writes are plain field assignments (`heap.nodes[n].parent = …`), with no barriers.
- **Scopes.** Non-allocating operations take `&mut DomHeap` and no scope; they cannot collect. Allocating ones take `&mut Agent` plus a `GcScope`.
- **Safepoints.** `Agent::safepoint`, then GC-free allocation batches. `create_node` hits a safepoint per node.
- **Dispatch is an ordinary function.** It roots the event path and each listener snapshot in `RootFrame`s, and re-reads handles after every listener call.

**Verification (all green)**
- **Outcome equals the reference** on the 10 edge cases, 100 Small seeds × 4 workloads, and Medium seeds × 3. The edge cases and the Small seeds pass both at default pacing and with **collect-always** pacing, which collects at every allocation safepoint.
- **A ≡ B:** the outcomes of both patterns are equal on every workload, at Small and Medium.
- **Debug run with every handle access epoch-checked.** Under collect-always, every safepoint moves the whole heap, so any stale handle would panic. None did on any tested path.
- **Pressure bound** (Medium, seed 1): peak live nodes **2,940 ≤ bound 2,940** (L + B + 1). It is tight by construction: safepoints at every allocation overshoot by one node at most. Without mid-dispatch collection the peak would be about 131,491. The run made 53 collections, with a maximum pause of 71 µs.
- **Miri**, Small sizes plus the edge cases at both pacings, Stacked Borrows with strict provenance: no undefined behaviour. It took 356 s, against 99 s for Pattern A: under collect-always, Pattern B compacts the whole heap at every allocation, and Miri magnifies that cost. This says nothing about native speed.
- **Policy:** `s1-b-dom` is `forbid` with 0 `unsafe`. Its whole dependency chain (`s1-b-gc`, `s1-scenarios`) also has 0 `unsafe`, including macro expansions.

**S1-Q1 for Pattern B: answered yes, unconditionally.** There is no `unsafe` anywhere, in hand-written or generated code, in the GC or the DOM.

**Preliminary ceremony comparison** (DOM crates only; CP7 formalizes it):

| | Pattern A (`s1-a-dom`) | Pattern B (`s1-b-dom`) |
|---|---|---|
| Size (non-blank, non-comment lines) | 503 | 554 |
| Writes | 12 `Gc::write` + 16 `unlock!` | plain assignments |
| Tracing | derived (+9 `require_static`) | 31 hand-written `mark`/`relocate` lines |
| Collection-safety ceremony | 8 `mc` params, 5 mutation scopes, a resumable dispatch state machine | 9 `unbind`, 8 `reborrow`, 6 `GcScope` params, 5 `bind`, 5 `scope`, 5 `get`, 4 frames |
| Where collection can happen | only between mutation scopes (code must yield) | at any allocation safepoint |

**Build cost of each DOM stack** (library plus dependencies, `--runs 3`)

| Stack | Clean (median) | Incremental (touch DOM `lib.rs`) | rustc compilations (clean) | Max rustc RSS |
|---|---|---|---|---|
| A: `s1-a-dom` → `s1-a-gc` → derive → `syn`… | 2.41 s | 0.13 s | 11 | 251 MiB (`syn`) |
| B: `s1-b-dom` → `s1-b-gc` | 0.58 s | 0.13 s | 4 | 170 MiB |

- **Incremental rebuilds are identical.** The clean-build gap is the one-time proc-macro toolchain.
- **Sampler limitation.** The 200 ms aggregate-RSS sampler misses builds shorter than about 0.2 s, so the incremental aggregate peaks read 0. The per-process `rustc` peaks still capture them.

**Findings**
1. **Both patterns pass S1-Q1, with a structural difference.**
   - **Pattern A** confines *substantial* `unsafe` (24 blocks plus 2 macro templates) behind a sound, well-tested boundary.
   - **Pattern B** needs none at all, at the price of bounds and epoch checks plus hand-written tracing.
2. **The ceremony is of different kinds.**
   - **Pattern A's cost is architectural:** code that runs long must become a resumable state machine, because collection waits for the stack to unwind.
   - **Pattern B's cost is lexical:** `bind`/`unbind`/`scope` noise, the "soup" the Nova author describes. Local code stays structured normally.
   - For a JS engine (ADR-0001 R2), Pattern A would force a stackless interpreter. Pattern B allows a conventional recursive interpreter with rooting.
3. **Pattern B's bound is tighter (overshoot ≤ 1 node), and its pauses were shorter in this run** (71 µs vs 145 µs). Peaks are not comparable in node counts, because the node sizes differ; CP7 compares bytes, throughput, and pauses against live-set size.

### CP7 — completed 2026-10-17

**7a delivered**
- **`s1-bench`** (`#![forbid(unsafe_code)]`):
  - `plan`: benchmark inputs built from the CP2 tree generator. Every op is validated by the reference interpreter before it is recorded. Each plan carries its expected tree digests and dispatch log.
  - `dom`: a `Dom` trait with `PatternA` and `PatternB` adapters that apply ops exactly as each pattern's driver does.
  - `harness`: time-based warm-up, ABBA-alternating samples, medians, TSV output, aggregation across runs.
  - `suites`: `build`, `traverse`, `mutate`, `dispatch`, `collect`, `scenario`. Every case verifies both patterns before it times anything.
  - `script_io`: a text format for scripts, so memory measurements can run a script without generating it.
  - `cli`: the `cargo bench` target, and the `s1-probe` binary (`bench`, `aggregate`, `gen`, `run`, `profile`, `sizes`).
- **Measurement scripts:** `tools/s1-bench.sh`, `tools/s1-memory.sh`, `tools/s1-profile.sh`, `tools/samply-summary.py`, `tools/s1-ceremony.sh` (§7).
- **`samply` 0.13.1 installed** with `cargo +1.94.1 install --locked samply`, under the rustc RSS wrapper. That build is a data point for a large dependency graph on this machine:
  - 281 crates in 83 s wall time (411 s of CPU);
  - largest single `rustc`: 911 MiB (the `samply` binary crate, in release);
  - aggregate compiler peak: 2.0 GiB.
- **Workspace:** 89 tests pass. clippy (pedantic) and the policy check are green; `s1-bench` has zero `unsafe`.

**Methodology corrections found in the shakedown run** (before any reported number)
1. **An invalid case.** "Traversal after random mutation" walked only 3,458 of 129,839 nodes, because the mutation mix detaches subtrees. `plan::shuffle` now moves nodes only under attached elements, so every node stays attached. The suite asserts the full count.
2. **Allocation order.** The CP2 generator grows trees breadth-first; a parser creates nodes in document order.
   - Plans now default to document order: same ids, same tree, same digest (tested).
   - Breadth-first and shuffled orders are kept as traversal variants.
3. **Clock ramp-up.** The first, short case ran at a low clock: `traverse/tree-10k` measured 11.9 ns/node for A, against 6.6 ns once warm. Warm-up is now measured in time (300 ms per case), not iterations.
4. **Line attribution.** samply's presymbolicated sidecar holds inline frames per function, not per address. `tools/samply-summary.py` resolves lines with `dsymutil` and `atos -i` instead.
5. **Profile scope.** Profiling loops run through `profiled_iteration`, which is never inlined and never tail-called. Summaries exclude input generation.

**Change to both DOM crates (dispatch allocations), with before and after measurements**
- **What the profiles showed:**
  - both dispatches regrew a fresh event-path `Vec` on every dispatch;
  - Pattern B also allocated a new rooted snapshot `Vec<Scoped>` on every visit. A's state machine already reused its snapshot buffer.
- **The symmetric fix:**
  - both crates now size the path first;
  - B uses one snapshot frame and one buffer per dispatch. Its snapshot roots are released when the dispatch returns, which scripts cannot observe.
- **Verification:** both oracle suites are green again (edge cases, seeds at both pacings, pressure bounds, A ≡ B). Miri on `s1-a-dom` (Stacked Borrows, strict provenance) found no undefined behaviour (98.7 s).

| Case (3 runs, median of medians) | A before | A after | B before | B after | B/A before → after |
|---|---|---|---|---|---|
| `dispatch` (per dispatch) | 491 ns | 482 ns | 1,271 ns | 1,022 ns | 2.59 → 2.12 |
| `scenario/listeners` | 158.2 ms | 157.1 ms | 178.3 ms | 163.3 ms | 1.13 → 1.04 |
| `scenario/churn` | 158.4 ms | 161.8 ms | 181.3 ms | 181.2 ms | 1.14 → 1.12 |

Line-level profiles attribute B's remaining dispatch gap to root-stack traffic: `RootFrame::push` and `Scoped::get` on a `Rc<RefCell<Vec<RootSlot>>>`. Without it, B's dispatch costs about what A's does. 7c analyses this.

**7b: formal runs** (commit `d5d8272`, clean tree, AC power, no thermal warnings)
- **Runtime suites:** `tools/s1-bench.sh`, 3 process runs with 60 s cool-downs (`target/s1-results/bench-20261017-070142`). Background load reached about 8 during run 2, and A's scenario timings in that run were up to about 50% slower (B's up to 13%); the median of the run medians absorbs one disturbed run.
- **Memory and instructions:** `tools/s1-memory.sh`, 3 runs (`memory-20261017-070549`). Run-to-run spread was at most 1.3%.
- **Profiles:** `tools/s1-profile.sh`, 7 cases × 2 patterns, 5 s each (`profiles-20261017-070732`).
- **Builds:** `tools/measure-build.sh --runs 3` on the A and B stacks (dev and release), on `s1-bench` (release), and on the workspace (dev, all targets).
- **Rooting unit costs:** `s1-probe roots` (added in 7c), 3 runs. Frame open and close 4.08 ns, root 2.05 ns, read 1.58 ns, identical across runs.
- **Ceremony:** `tools/s1-ceremony.sh`.

**7c: results note.** [S1 results](s1-results.md) holds every table, the attribution of each difference to rooting discipline, representation or collector (§4.4), the threats to validity, and the recommendation.

| Question | Result |
|---|---|
| S1-Q1 | Yes for both. A: 24 blocks in one module plus 2 macro templates. B: no `unsafe` anywhere |
| S1-Q2 | Similar volume, different kind: architectural (A) against lexical (B). Owner judgment at CP9 |
| S1-Q3 | Mixed. B: collections 1.3–2.8× faster from 64k elements, allocation-heavy work takes 30% less time, lower maximum pauses, 13% less memory per live node. A: cache-friendly traversal 18–22% faster, dispatch 2.06× faster (B's root-stack traffic) |
| S1-Q4 | Small for both: the S1 workspace builds clean in 4.2 s, every `rustc` under 255 MiB. B's stack builds clean 4× faster (no `syn`) |
| Recommendation | Pattern B for ADR-0003, conditionally (C1–C5), medium confidence. New open questions OQ-13 to OQ-15 |

**Verification:** the four standard checks are green (§6). `s1-bench` gained the `roots` module and a test for it.

### CP8 — in progress (8a completed 2026-10-17)

**8a delivered: `s2-window`**
- **Allowlist.** Registered in `tools/unsafe-policy.txt`: `src/main.rs` is `#![deny(unsafe_code)]`, and `unsafe` is allowed only in `src/platform/`, under invariants P1–P6 (`platform/mod.rs`).
- **AppKit.** `NSApplication` and an `NSWindow`, created through objc2 0.6.5 and its framework crates 0.3.2.
  - The content view is layer-hosting, with a `CAMetalLayer` (BGRA8, `framebufferOnly = false` so it can be read back).
  - The event loop is pumped by hand (`nextEventMatchingMask:…`) and blocks when there is nothing to draw.
- **Metal.** The device, the queue, and a render pipeline whose MSL source is compiled at run time (`newLibraryWithSource:…`). The quad's corners come from `vertex_id`; blending is premultiplied alpha.
- **CoreText.** A `CTLine` is rasterized by CoreGraphics into an 8-bit coverage mask in a Rust-owned buffer, uploaded as an `R8Unorm` texture, and drawn 1:1 in pixels.
- **`--smoke`.** It draws 3 frames, blits the last presented drawable into a shared buffer, and checks:
  - the background, exactly;
  - no ink outside the text box;
  - the number of ink pixels inside the box against the count predicted from the coverage mask by the blending formula.

  The first run matched exactly: 19,052 of 19,052 pixels.
- **Tests:** 6, under `cargo test`, with no window needed:
  - argument parsing and centring;
  - the oracle accepting a correct frame;
  - the oracle rejecting stray ink, missing text, and misplaced text;
  - CoreText rasterization;
  - device creation, MSL compilation, pipeline creation and texture upload.
- **Dependencies.**
  - objc2's framework crates compile every header by default: AppKit alone enables 307 features and pulls in CloudKit, CoreData, CoreImage, CoreVideo and IOSurface bindings.
  - The workspace turns defaults off; `s2-window` enables only what it uses.
  - A `full-bindings` feature restores the defaults, so both configurations can be measured.
- **`tools/s2-measure.sh`:** binary size; smoke runs with the app's timings plus `/usr/bin/time -l`; idle runs sampled with `top` (CPU, idle wakeups, energy impact, footprint) and `ps`.

**`unsafe` inventory: 19 blocks, every one with a `// SAFETY:` comment**

| Kind | Blocks | Invariant |
|---|---|---|
| Raw pointers into Rust memory (texture upload, vertex bytes, bitmap context, out-parameters, null font matrix) | 6 | P2 |
| Reading a shared GPU buffer | 1 | P3 |
| GPU bounds (indexed subscripts, binding resources, draw, blit, texture descriptor) | 6 | P4, P5 |
| Window ownership (`initWithContentRect…`, `setReleasedWhenClosed`) | 2 | P1 |
| Extern statics (`NSDefaultRunLoopMode`, CoreText attribute keys) | 2 | P6 |
| CoreFoundation and CoreText functions the bindings mark `unsafe` without a pointer argument | 2 | Documented per call |

**Findings so far**
1. **There is no offline Metal compiler without Xcode.** `xcrun -f metal` fails with only the Command Line Tools installed. Shaders must be compiled from MSL source at run time, and the OS caches the result.
   - The first compile after a build took about 0.5 s.
   - Later runs had the device and shaders ready about 75 ms after `main` starts.
2. **CoreText draws black unless told otherwise.** The context's fill colour is used only when `kCTForegroundColorFromContextAttributeName` is set. The smoke oracle caught the empty mask on the first run.
3. **A layer-hosted Metal view needs no subclass and no Objective-C class definitions** (`define_class!`). Without one, live resizing stretches the last frame until the mouse is released; that is acceptable for a spike, and it is noted.
