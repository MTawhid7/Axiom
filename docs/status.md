# Status and Handoff Log

> **Read this first when resuming work.** Then check the documents it links to against the actual state of the repository (`git log`, `git status`, and the checkpoint tracker). Don't assume earlier work is still accurate.

## Current state (2026-10-17)

| Item | State |
|---|---|
| Phase | **Phase Zero. Step 3 is in review**: the threat model, the process model and the macOS confinement note are written, and ADR-0004 is Proposed. Production engine code is **not** authorized |
| Repository | git, hosted at [github.com/MTawhid7/Axiom](https://github.com/MTawhid7/Axiom) (public; remote `origin`). Local folder: `~/Documents/Axiom`. `main` includes Step 2b. **Step 3 is on branch `step-3`**, not yet merged or pushed |
| Current checkpoint | Step 3 CP1–CP5 ✅ ([tracker](research/step-3-security-and-process-model.md)); waiting for the owner's review |
| Code | Spike code only, under `spikes/`; it stays as the baseline for spike S3. Tooling under `tools/`. Step 3 added no code |
| Verified capabilities | None at engine level. The spikes' verified results are in the [S1 results](research/s1-results.md) and [S2 results](research/s2-results.md). Step 3's observations of this Mac are in the [macOS note](research/macos-process-confinement.md) §9 |
| Accepted decisions | [ADR-0001](adr/0001-scope-and-target.md): scope, target, JavaScript strategy, security posture |
| | [ADR-0002](adr/0002-implementation-language.md): Rust (stable, 2024 edition), **final** (2026-10-17), with build thresholds T1–T4 |
| | [ADR-0003](adr/0003-gc-handle-and-rooting-pattern.md): GC handle and rooting pattern, Pattern B (Nova style), with obligations C1–C5 (2026-10-17) |
| Proposed decisions | [ADR-0004](adr/0004-process-model-and-sandbox.md): process model, sandbox and JIT policy (2026-10-17). Acceptance should depend on spike S4 |

## Next steps: the owner's review of Step 3

**What to read:**
1. [ADR-0004](adr/0004-process-model-and-sandbox.md), the decision in one place.
2. [Threat model](security/threat-model.md) and [process model](security/process-model.md), the design behind it.
3. [macOS confinement note](research/macos-process-confinement.md), the evidence, including spike S4's design (§7).

**Decisions only the owner can make:**
1. **Threat model.** Accept the assets, adversaries, goals and non-goals. Notable choices: no Safe Browsing-style reputation service (NG5); no strong anti-fingerprinting (NG3); the compromised renderer treated as an expected case.
2. **Security levels L0–L3, and the gate's checks V1–V8** for sensitive use (threat model §8). Optionally, set a higher bar for L3.
3. **Authorize spike S4.** It is throwaway code under `spikes/s4/` plus a signing script in `tools/`, in three checkpoints (S4a probes, S4b measurements, S4c results). It confirms the sandbox mechanism on this Mac and measures what a helper costs.
4. **ADR-0004.** Recommended: accept it once S4-Q1 and S4-Q2 pass. Alternatively, accept it now with S4 as a revisit condition.
5. **JIT policy J1–J7** (OQ-4), in particular: no JIT before L2; a JIT in one executable only; sensitive sites always without a JIT.
6. **Cross-site iframes.** Accept the interim rule SI3, which keeps cross-site iframes in the embedder's process under compensating rules, with out-of-process iframes deferred (OQ-18). The alternative is to plan out-of-process iframes earlier.
7. **Memory targets B1–B7** as initial targets, to be finalized in Step 6.
8. **Merge `step-3` into `main`** after the review. Push it to GitHub now or only with the merge?

**After the review:**
- S4, if authorized;
- then ADR-0004's acceptance, and the merge;
- then **Step 4**: event loop, threading, pipeline data model, invalidation, embedder API. Spike S3 is required there (ADR-0003, C1).

Step 3 adds these constraints for Step 4: message-based process boundaries; a browser-owned frame tree; one agent cluster per process; a display list free of image and font decoding ([decision map](decision-map.md) §4).

## Session log

### 2026-10-17 — Session 5 (Step 3)

**Owner input:** "Continue implementation". Read with status.md, this was taken as authorization for Step 3's design work. Spikes need their own authorization, so no code was written.

**Done** (branch `step-3`, CP1–CP5; [tracker](research/step-3-security-and-process-model.md)):
- **CP1:** the plan and checkpoint tracker.
- **CP2:** the [threat model](security/threat-model.md): assumptions, assets, adversaries, attack surfaces, trust boundaries, goals SG1–SG12, non-goals, security levels L0–L3 and the gate's checks V1–V8.
- **CP3:** the [macOS confinement note](research/macos-process-confinement.md), with mechanisms, browser precedent, observations of this Mac, and the design of spike S4.
- **CP4:** the [process model](security/process-model.md): five process types, a capability matrix, site isolation SI1–SI7, IPC trust rules IR1–IR10, memory targets B1–B7, staging.
- **CP5:** [ADR-0004](adr/0004-process-model-and-sandbox.md) (Proposed), including the JIT policy J1–J7 for OQ-4. Decision map, open questions (OQ-16 to OQ-18 added), indexes and this status updated.

**Verified on this Mac** (read-only; nothing compiled or installed):
- The macOS 27 SDK **kills processes that use the named sandbox profiles** (`sandbox_init(3)`).
- Chrome Canary 158, built against SDK 27.0, still applies custom Seatbelt profiles through `sandbox_init_with_parameters` and `sandbox_apply` (`nm -u`). Firefox uses `sandbox_init_with_parameters`.
- WebKit ships deny-by-default profiles for its WebContent, GPU and Networking processes. Its content profile denies the window server and font server, and its two no-JIT content variants deny `dynamic-code-generation`.
- App Sandbox's base profile allows 170 global Mach services.
- Multiple WebKit XPC instances rely on the private `_MultipleInstances` key.
- The SDK contains BrowserEngineKit with macOS 14.3 availability, but Apple documents it for iOS, iPadOS and Mac Catalyst only, with entitlements by request.
- A minimal process has a 1.03 MB footprint.

**Corrected during review:** WebKit's XPC services carry no hardened-runtime flag. They are platform binaries with library validation only.

**Verification:** the four standard checks pass, unchanged from Step 2b. Every relative link in `docs/` resolves.

### 2026-10-17 — Session 4 (renamed to Axiom)

**Owner input:** the project is renamed from Arbor to **Axiom**, including its folder on disk. The owner created the GitHub repository <https://github.com/MTawhid7/Axiom>.

**Done**
- Every reference in the repository now uses the new name: the docs, the workspace manifest, the S2 window's title, drawn text and tests, and the build-measurement scripts, whose log variables are now `AXIOM_RUSTC_LOG` and `AXIOM_LINK_LOG`.
- Earlier commits are unchanged, so their files still use the old name.
- The project folder moved from `~/Documents/Arbor` to `~/Documents/Axiom`.
- The GitHub repository is the remote `origin`. `main` and `step-2b` are pushed to it.

**Verified:** the four standard checks pass, and the S2 smoke test passes with the new text.

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

**CP9 synthesis written:**
- ADR-0002 recommends final acceptance, with a Step 2b evidence table.
- ADR-0003 is proposed: Pattern B, with obligations C1–C5.
- The language evaluation has dated Step 2b notes.
- The decision map, open questions and indexes are updated.

**The owner's decisions (2026-10-17)** close Step 2b:
1. S1-Q2: Pattern B's ceremony is acceptable.
2. ADR-0002 is **accepted as final**.
3. ADR-0003 is **accepted as proposed**: Pattern B, with obligations C1–C5.
4. Thresholds T1–T4 are accepted (OQ-12 resolved).
5. S3 is a **required** benchmark, run in Step 4 (OQ-15 resolved).
6. The S2 smoke test and the owner's visual check are satisfactory.

**Recorded and merged.** The decisions are recorded in ADR-0002, ADR-0003, the open questions, the decision map, the indexes and the tracker. `step-2b` was merged into `main` with a merge commit (`--no-ff`), which keeps Step 2b's history together.

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
