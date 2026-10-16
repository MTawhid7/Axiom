# Step 2b — Validation Spikes S1 and S2: Plan and Checkpoint Tracker

| | |
|---|---|
| **Status** | In progress |
| **Authorized** | 2026-10-16, by the owner |
| **Branch** | `step-2b`. It is merged into `main` only after owner review at CP9 |
| **Feeds** | [ADR-0002](../adr/0002-implementation-language.md) final acceptance; a future ADR-0003 on the GC handle/rooting pattern |
| **Last updated** | 2026-10-16 |

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
│   │   └── bench/          s1-bench         harness=false benches, differential tests (CP7)
│   └── s2/window/          s2-window        AppKit + Metal + CoreText (CP8)
└── tools/
    ├── check-unsafe-policy.sh   + unsafe-policy.txt (allowlist)
    ├── measure-build.sh         clean / no-op / incremental builds
    ├── rustc-rss-wrapper.sh     RUSTC_WRAPPER: per-crate wall time and max RSS
    └── linker-rss-wrapper.sh    -C linker=…: per-link wall time and max RSS
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
| CP3 | Pattern A heap and derive; Miri | ⬜ |
| CP4 | Pattern A DOM and driver; oracle equality; Miri on small scenarios | ⬜ |
| CP5 | Pattern B heap; Miri | ⬜ |
| CP6 | Pattern B DOM and driver; oracle equality; A ≡ B differential | ⬜ |
| CP7 | S1 measurements (runtime, ceremony, build) and S1 results note | ⬜ |
| CP8 | S2 platform spike, measurements, and S2 results note | ⬜ |
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
| Runtime throughput and latency | `cargo bench` with `harness = false` benchmarks: warm-up, then K iterations, reporting median, min and max |
| GC pauses, heap bytes, object counts | Each GC crate's metrics API |
| Peak RSS at run time | `/usr/bin/time -l` on each bench and scenario binary |
| CPU profiles | `samply record` (Firefox Profiler UI; works without Xcode) on the `profiling` profile (release plus line tables) |
| Undefined behaviour | `cargo +nightly miri test` on GC crates and on small DOM scenarios |

**Caveats**
- The MacBook Air is fanless, so sustained load throttles. Runs are repeated, with cool-downs, and the thermal state is noted.
- Background load on the machine is not controlled. Medians of repeated runs are reported, not single runs.

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
