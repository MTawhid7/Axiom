# Status and Handoff Log

> **Read this first when resuming work.** Then check the documents it links to against the actual state of the repository (`git log`, `git status`, and the checkpoint tracker). Don't assume earlier work is still accurate.

## Current state (2026-10-17)

| Item | State |
|---|---|
| Phase | **Phase Zero, Step 2b: validation spikes** (authorized 2026-10-16). Production engine code is **not** authorized |
| Repository | git. `main` = reviewed baseline (`a223fe2`); work in progress on branch **`step-2b`** |
| Current checkpoint | CP1–CP8 ✅ done: **S1 and S2 are complete**; see the [S1 results](research/s1-results.md) and [S2 results](research/s2-results.md). **Next: CP9**, the synthesis: owner decisions, ADR-0002 final, ADR-0003 proposed, merge to `main`; see the [Step 2b tracker](research/step-2b-validation-spikes.md#6-checkpoints) |
| Code | Spike code only, under `spikes/` (throwaway). Tooling under `tools/` |
| Verified capabilities | None at engine level. Spike verification results are in the tracker's §8. S1's findings and recommendation (Pattern B for ADR-0003, conditionally) are in the [S1 results](research/s1-results.md) |
| Accepted decisions | [ADR-0001](adr/0001-scope-and-target.md): scope, target, JavaScript strategy, security posture |
| Provisionally accepted | [ADR-0002](adr/0002-implementation-language.md): Rust (stable, 2024 edition), pending S1 and S2 |

## Next steps

1. Finish the current checkpoint in the [Step 2b tracker](research/step-2b-validation-spikes.md) and run its verification commands.
2. Continue CP2 → CP9 in order. Each checkpoint ends with a green verification run and a commit on `step-2b`.
3. At CP9, the owner reviews the S1 and S2 results, decides on final acceptance of ADR-0002, and merges `step-2b` into `main`.

## Session log

### 2026-10-17 — Session 3 (CP7)

**Owner input:** "continue" with CP7, after the CP3–CP6 progress report.

**CP7a completed** (benchmark harness):
- `s1-bench`, with reference-validated inputs, six verified suites, and the `s1-probe` binary.
- Measurement scripts in `tools/`: `s1-bench.sh`, `s1-memory.sh`, `s1-profile.sh`, `samply-summary.py`, `s1-ceremony.sh`.
- `samply` 0.13.1 installed, as planned. Building it was itself measured: 281 crates in 83 s, with a 2.0 GiB aggregate compiler peak.
- The shakedown run found five methodology problems, all fixed before any reported number:
  - an invalid traversal case;
  - unrealistic allocation order;
  - clock ramp-up;
  - samply's line attribution;
  - profile scoping.
- Profiles also showed incidental allocations in both patterns' dispatch. A symmetric fix went into both DOM crates; the oracle tests and Miri (`s1-a-dom`) were re-run green; before and after numbers are in the tracker's §8.

**CP7b and CP7c completed** (measurements and results):
- Formal runs, at commit `d5d8272` with a clean tree:
  - runtime suites, 3 runs;
  - memory and instructions, 3 runs;
  - profiles;
  - builds (dev and release).
- Rooting unit costs measured with the new `s1-probe roots`.
- [S1 results](research/s1-results.md) written. Its recommendation for ADR-0003 is Pattern B (Nova style), conditional on C1–C5, with medium confidence.
- New open questions OQ-13 to OQ-15.
- Design facts checked against current sources: gc-arena (intrusive list, incremental, barriers), piccolo (stackless because of gc-arena) and Nova (`stack_refs` is a `RefCell<Vec>`).

**CP8a completed** (S2 window):
- `s2-window` draws CoreText through a Metal pipeline whose shaders are compiled from source at run time, in an AppKit window, with no Xcode.
- 19 `unsafe` blocks, all in `src/platform/`, under P1–P6.
- The smoke oracle matches the predicted ink pixel for pixel.
- Six tests run under `cargo test` without a window.
- The Command Line Tools have no offline Metal compiler.

**CP8b and CP8c completed** (S2 measurements and results):
- Startup: first frame at 174 ms; idle: 0.0% CPU and no wakeups.
- Build: 6.0 s clean with trimmed bindings, against 29 s with objc2's defaults; the whole workspace builds clean in 8.1 s.
- [S2 results](research/s2-results.md) written, with the proposed build thresholds T1–T4 for ADR-0002 revisit condition 3.

**Decisions waiting for the owner** (at CP9):
- S1-Q2: is the ceremony acceptable?
- ADR-0003: the GC pattern.
- OQ-15: should S3 become required?
- OQ-12: accept thresholds T1–T4?
- Visual check of the S2 window (`cargo run -p s2-window`).

**Verified**
- `samply` works on this machine without Xcode, including line-level attribution through `dsymutil` and `atos` (Command Line Tools).
- The system allocator is Apple's "xzone" malloc. Some of its paths call `mach_absolute_time`, so allocation in hot loops is relatively costly here.


### 2026-10-16 — Session 2 (Step 2b kickoff)

**Owner inputs**
- Step 2b (S1 and S2) authorized.
- Rust provisionally accepted.
- Owner is fluent in Rust (OQ-6).
- No deadline or fixed budget (OQ-1).
- git requested (OQ-8).
- Efficiency (memory, CPU, power) is a primary goal (charter G4).
- CLI-only measurement tools.

**What changed**
- `git init`; baseline docs committed on `main` (`a223fe2`).
- Branch `step-2b` created.
- Plan and checkpoint tracker written: [research/step-2b-validation-spikes.md](research/step-2b-validation-spikes.md).
- ADR-0002 marked provisionally accepted; evaluation §15.3 gained rule 7 (macro-generated `unsafe`).
- Charter, open questions and decision map updated.

**CP1 completed** (foundation):
- workspace with resolver 3 and the 2024 edition;
- toolchain pinned to 1.94.1;
- workspace lints (pedantic, plus unsafe hygiene);
- `tools/check-unsafe-policy.sh`, with a self-test;
- `tools/measure-build.sh`, with the `rustc` and linker RSS wrappers;
- the `s1-scenarios` model types, RNG, digests and counters, with 17 tests.

Verification results and the baseline build numbers are in the tracker's §8.

**Owner decision (2026-10-16):** keep exactly the two canonical patterns.
- Pattern A is gc-arena: raw pointers and mark–sweep.
- Pattern B is Nova: `u32` indices and mark–compact.
- There is no third variant. The storage confound is handled in the analysis (§4.4).

**CP2 completed** (workload model and oracle):
- the `reference` interpreter;
- four seeded workload generators at three sizes, each producing its expected outcome;
- 10 golden tests;
- a gate of 100 seeds per workload;
- a coverage gate.

Results, workload volumes and the pressure bound for CP4 and CP6 are in the tracker's §8.

**CP3 completed** (Pattern A heap):
- `s1-a-gc`: a canonical gc-arena design. All `unsafe` is under `src/raw/`, governed by invariants I1–I6.
- `s1-a-gc-derive`.
- The first external dependencies: `syn`, `quote` and `proc-macro2`, justified in `Cargo.toml`.
- Verification:
  - 12 tests plus compile-time assertions;
  - 6 compile-fail doctests, each with a verified reason and a pinned error code;
  - Miri clean under both Stacked Borrows and Tree Borrows with strict provenance, with leak checking on.
- Clean build in 2.6 s, with a peak rustc RSS of 252 MiB (`syn`).
- One-time setup: the Miri component was added to the nightly toolchain.

**CP4 completed** (Pattern A DOM):
- `s1-a-dom` is `forbid(unsafe_code)`. Dispatch is a resumable state machine that lives in the arena root.
- It matches the reference exactly on:
  - the 10 edge cases;
  - 100 Small seeds × 4 workloads;
  - Medium seeds;

  both at default pacing and when collecting at every boundary.
- Pressure peak: 2,211 live nodes, against a bound of 2,314 and about 131k without mid-dispatch collection.
- Miri is clean (Stacked Borrows and Tree Borrows).
- `s1_scenarios::edge_cases` was added.
- **S1-Q1 for Pattern A: yes.** The only `unsafe` in the expanded code comes from the two audited macro templates (rule 7).

**CP5 completed** (Pattern B heap):
- `s1-b-gc`: a canonical Nova design with **zero `unsafe`** (`forbid`), including its `heap!` macro.
- It provides `u32` handles bound to `GcScope`/`NoGcScope`, RAII root frames, typed arenas, and a mark–compact collector with rank-based forwarding.
- Verification:
  - 9 tests, including the reachability oracle under compaction;
  - debug detection of stale handles and of missed marks, both proven by tests;
  - 2 compile-fail doctests (E0502, E0597);
  - Miri clean.
- Clean build in 1.1 s.

**CP6 completed** (Pattern B DOM):
- `s1-b-dom` has zero `unsafe` across its whole stack. Dispatch is an ordinary function with rooted frames.
- It matches the reference everywhere, at both pacings.
- A ≡ B on every workload.
- The debug run with epoch checks found no stale handles.
- Pressure peak: 2,940 live nodes, against a tight bound of 2,940 (vs about 131k without mid-dispatch collection).
- Miri is clean.
- **S1-Q1 for Pattern B: yes, unconditionally.**
- A preliminary ceremony comparison and the build cost of each stack are in the tracker's §8.

**Verified**
- Experiment: `#![forbid(unsafe_code)]` doesn't catch `unsafe impl` generated by external derive or `macro_rules!` macros, but does catch hand-written ones (rustc 1.94.1).
- Toolchains present:
  - 1.94.1 with clippy, rustfmt and rust-src;
  - nightly 2026-04-08, without Miri (to be added at CP3).
- `samply` was not installed (installed in Session 3, CP7a).

### 2026-10-16 — Session 1 (Phase Zero, Steps 0–2a)

**What changed**
- Created the `docs/` foundation:
  - README, charter, decision map, open questions, this status log;
  - research conventions;
  - the ADR template, ADR-0001 and ADR-0002;
  - the language evaluation.

**Verified**
- Target machine facts, read from the system:
  - MacBook Air M1 (`MacBookAir10,1`): 4 performance + 4 efficiency cores, 8 GB RAM;
  - macOS 27.2;
  - Rust 1.94.1, Swift 6.4, Apple clang 21;
  - Command Line Tools only.

**Decisions made**
- ADR-0001 Accepted.
- ADR-0002 Proposed. It was provisionally accepted in Session 2.

**Verification gaps carried forward**
- Boa's 2026 test262 numbers come from a secondary source.
- The Miri limitations are stated as background; re-check the Miri README before relying on them.
- The Fuzzilli integration path for a Rust engine is a hypothesis.
