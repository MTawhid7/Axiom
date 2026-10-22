# Status and Handoff Log

> **Read this first when resuming work.** Then check the documents it links to against the actual state of the repository (`git log`, `git status`, and the checkpoint tracker). Don't assume earlier work is still accurate.

## Current state (2026-10-22)

| Item | State |
|---|---|
| Phase | **Phase Zero's baseline is complete:** Step 6's **Gate C passed** (2026-10-20), the final design review. ADRs 0020 and 0021 are accepted, and the architecture proposal (the overview v2, with the requirements) is approved as the Phase Zero baseline. **The independent, multi-round adversarial review is under way,** on branch `architecture-review`: **Round 1 is written and disposed** (2026-10-21): the owner accepted all 22 findings; the accepted changes are made, and the three spikes authorized before Milestone 1 are done ([the review](review/README.md) §5). **The owner approved the results and commissioned Round 2** (2026-10-21). **Round 2 is delivered and disposed** (2026-10-21): Round 1's changes verified; 15 findings, none Critical or High, so the stopping rule is met for the scope commissioned; **the owner accepted all 15,** and their changes are made and verified in checkpoints CP1–CP6, with `architecture-review` pushed. **Round 3, the independent external review** by Gemini, was delivered on 2026-10-21 ([its record](review/round-3/01-gemini-review.md)): "Blocked by Findings". **The owner commissioned Round 4** (2026-10-22; [the review](review/README.md) §5.8): test Round 3's verdict rather than take it, apply three themes (scope by value, reuse of mature software, efficiency first), and review everything no round had, with spike S16. **Round 4 is delivered** (2026-10-22): 17 findings, 1 Critical, 1 High, 7 Medium and 8 Low, so the stopping rule isn't met. **The owner accepted all 17 the same day** ([the review](review/README.md) §5.9), with checkpoints CP1–CP7, the re-runs of S5–S7, the end of the review after CP7, the merge into `main` and Milestone 1's plan. **CP1–CP7 are done and verified, and the review cycle is complete** (the review's §6). **Now: Milestone 1's first checkpoint** (2026-10-22), on branch `milestone-1`. The owner asked for its plan to be reviewed with "the discipline and rigor of a mature software engineering organization". The plan is revised, and ADRs 0022 (code architecture and engineering standards) and 0023 (observability) are proposed. **No engine code** until the owner adopts the plan and authorizes MA10 |
| Repository | git, hosted at [github.com/MTawhid7/Axiom](https://github.com/MTawhid7/Axiom) (public; remote `origin`). Local folder: `~/Documents/Axiom`. **`main` includes the architecture review,** merged from `architecture-review` with `--no-ff` on 2026-10-22 (`bca6b8b`) and pushed, as the owner directed; before it, Step 6 (`4fb9ca6`). **`milestone-1`**, from `main` and pushed, holds Milestone 1's plan: merged from `milestone-1-plan` as "adopt Milestone 1 implementation plan" (`9887179`), then revised at M1-CP1. The branches `milestone-1-plan`, `architecture-review` and `step-2b` to `step-6` are kept |
| Current checkpoint | **Step 6 is done:** CP1–CP11 ✅, Gates A–C passed ([tracker](research/step-6-standards-budgets-roadmap.md)). **Review Round 1 is delivered and disposed** (2026-10-21): 22 findings, all accepted. **Its changes are done,** on `architecture-review` and pushed: checkpoints CP1–CP6, a sweep for stale statements, and spikes S10, S12 and S15 ([the review](review/README.md) §5.2, §5.3). **The owner approved them** (2026-10-21, [the review](review/README.md) §5.4). **Review Round 2 is delivered and disposed** (2026-10-21; [the review](review/README.md) §2, §5.5–§5.7, §8): 15 findings, 10 Medium and 5 Low, all accepted; the stopping rule met for the scope commissioned. **Round 2's checkpoints CP1–CP6 are done and verified,** and `architecture-review` is pushed. **Round 3** (Gemini, 2026-10-21) and **Round 4** (2026-10-22; [the review](review/README.md) §2, §5.8, §9, §10) are delivered, Round 4 with spike S16. **Round 4 disposed** (2026-10-22, §5.9–§5.11), its checkpoints CP1–CP7 done and verified; **the review cycle is complete.** **Merged into `main`** (2026-10-22, `bca6b8b`, pushed). **[Milestone 1's implementation plan](milestone-1-plan.md) drafted,** then **revised at M1-CP1** (2026-10-22): the [crate architecture](architecture/crates.md), the [engineering standards](technology/engineering-standards.md) and [observability](architecture/observability.md) proposed with ADRs 0022 and 0023; the plan rewritten around an engineering foundation and a walking skeleton. **Now: the owner's gate** ("Next steps") |
| Code | Spike code only, under `spikes/`: S1–S7, all complete. **S8, the corpus census** (`spikes/s8/`, Python), is complete: in Chrome, automated and by the owner's hand; in Safari, by the owner's hand, logged out, with the corrected tool ([results](research/s8-results.md)). **S9, the test tools** (`spikes/s9/`), is complete ([results](research/s9-results.md)): fuzzing and wptrunner work here; the installed nightly's ThreadSanitizer doesn't. **The review's spikes,** authorized by the owner on 2026-10-21 ([the review](review/README.md) §5.2): **S10, the GPU/compositor process** (`spikes/s10/gpu`), **S12, ThreadSanitizer's routes** (S9's `race` crate) and **S15, a stable signing identity** (`spikes/s15/probe`) are complete; S11, S13 and S14 are authorized before Milestone 3. **S16, lifetimes of shared memory and port rights, and efficiency counters** (`spikes/s16/lifetime`), authorized for Round 4 on 2026-10-22 ([the review](review/README.md) §5.8), is complete ([results](research/s16-results.md)). Tooling under `tools/`, now with `s6-tails.py`, `png-region-stats.py` and, from the review's Round 1, `wpt-script-deps.py`, `unsafe-census.py` and the scripts of S10, S12 and S15; from Round 4, `s16-measure.sh`, `crate-facts.py` and `wpt-file-facts.py`, with `wpt-script-deps.py` following references; working files in `scratch/`, which git ignores. The repository is licensed "MIT OR Apache-2.0" (`LICENSE-MIT`, `LICENSE-APACHE`) |
| Verified capabilities | None at engine level. Spike results: [S1](research/s1-results.md), [S2](research/s2-results.md), [S3](research/s3-results.md), [S4](research/s4-results.md), [S5](research/s5-results.md), [S6](research/s6-results.md), [S7](research/s7-results.md), [S8](research/s8-results.md), [S9](research/s9-results.md), and for the review [S10](research/s10-results.md), [S12](research/s12-results.md), [S15](research/s15-results.md) and [S16](research/s16-results.md). The [handover](handover-phase-0.md) summarizes them all. Observations of this Mac: [macOS note](research/macos-process-confinement.md) §9 |
| Accepted decisions | [ADR-0001](adr/0001-scope-and-target.md): scope, target, JavaScript strategy, security posture |
| | [ADR-0002](adr/0002-implementation-language.md): Rust (stable, 2024 edition), **final**, with build thresholds T1–T4 |
| | [ADR-0003](adr/0003-gc-handle-and-rooting-pattern.md): GC handle and rooting pattern, Pattern B (Nova style), with obligations C1–C5 |
| | [ADR-0004](adr/0004-process-model-and-sandbox.md): process model, sandbox and JIT policy (2026-10-17), validated by spike S4. The owner confirmed S4's refinements. Amended in Round 1 (D1, D2's signing identity) and after S10 (2026-10-21): the GPU process resolves its cache directory before its profile, D2's one exception |
| | [Threat model](security/threat-model.md) (2026-10-17), amended: out-of-process iframes are required for L3 |
| | [Process model](security/process-model.md), binding with ADR-0004 (2026-10-17); refined at Step 5's Gate C (2026-10-20) in its change record: S5's and S7's metadata rules, ImageIO's place, and B1's scope (the core browser at 60 MB, tiles budgeted by their pixels). **The final B1–B7** at Step 6's Gate B (2026-10-20), with ADR-0019. Amended in Round 1, and the GPU/compositor process's rules from S10 adopted (2026-10-21) |
| | [ADR-0005](adr/0005-ipc-transport-and-interfaces.md): IPC, Mach transport, capabilities, IDL, `Untrusted<T>` (2026-10-17, Gate B) |
| | [ADR-0006](adr/0006-event-loop-and-threading.md): event loop and threading, with EL11 amended for read-only lending of DOM arenas (2026-10-17, Gate B) |
| | [ADR-0007](adr/0007-frame-tree-pipeline-and-input.md): frame tree, rendering pipeline, invalidation and input across frames (2026-10-17, Gate B); IV1 and IV2 amended for size container queries (2026-10-20, Step 5's Gate C) |
| | [ADR-0008](adr/0008-script-agents-and-heap.md): script, agents and heap. C1 is met by `SliceStack`, a safe root stack, so the `gc` crate has no `unsafe` (2026-10-18, Gate C); SH10 amended: the DOM's node arena is never compacted (2026-10-20, Step 5's Gate C) |
| | [ADR-0009](adr/0009-embedder-api-and-shell.md): engine/embedder API and the pure-Rust shell (2026-10-17, Gate B) |
| | [ADR-0010](adr/0010-dependency-and-supply-chain-policy.md): the dependency and supply-chain policy, with DP10's refinement (2026-10-18, Step 5's Gate B) |
| | [ADR-0011](adr/0011-text-fonts-and-unicode.md): text, fonts and Unicode; CoreText rasterizes system fonts (2026-10-20, Step 5's Gate C); `vello_cpu` adopted as the renderer's Rust rasterizer, subject to grading |
| | [ADR-0012](adr/0012-graphics-images-and-media.md): graphics, images and media; AVIF in the media utility process (2026-10-20, Step 5's Gate C). Colour tags confirmed by S6's second run; GR5's amendment deferred to the graphics milestone |
| | [ADR-0013](adr/0013-networking-storage-and-workers.md): networking, storage and workers (2026-10-19), the owner's Gate B condition met by S7; verification in the worker pool confirmed at Gate C |
| | [ADR-0014](adr/0014-accessibility.md): accessibility, with the settings list and the reserved key equivalents (2026-10-18, Step 5's Gate B) |
| | [ADR-0015](adr/0015-html-parsing-and-the-dom.md): HTML parsing and the DOM (2026-10-20, Step 5's Gate C); XSLT deferred (OQ-22) |
| | [ADR-0016](adr/0016-style-layout-and-paint.md): style, layout and paint, with `:visited` by partitioned history (2026-10-20, Step 5's Gate C) |
| | [ADR-0017](adr/0017-javascript-engine-and-bindings.md): the JavaScript engine and bindings (2026-10-20, Step 5's Gate C) |
| | [ADR-0018](adr/0018-corpus-standards-and-testing.md): the corpus, the standards baseline and testing, X1 and X2 (2026-10-20, Step 6's Gate B); the automation boundary confirmed, the not-planned list approved |
| | [ADR-0019](adr/0019-performance-methodology-and-budgets.md): performance methodology and budgets, X3, with the final B1–B7 (2026-10-20, Step 6's Gate B); the content tier's 150 MB cap kept at Gate C as the baseline hypothesis for review ([OQ-25](open-questions.md)), and replaced in Round 1 by a slope and a plateau on a normalized workload, 150 MB a stretch target (RV1-05) |
| | [ADR-0020](adr/0020-roadmap-and-security-levels.md): the roadmap and its security levels, M1–M9 and L0–L3, accepted as the baseline under review (2026-10-20, Step 6's Gate C) |
| | [ADR-0021](adr/0021-milestone-1.md): Milestone 1, static documents, accepted as M1's specification baseline (2026-10-20, Step 6's Gate C); **starting it is on hold** |
| | The [overview](architecture/overview.md) v2 and the [requirements](requirements.md) (RQ1–RQ66), approved as the Phase Zero baseline (2026-10-20, Step 6's Gate C) |
| The corpus | 13 sites, set by the owner on 2026-10-20 ([OQ-2](open-questions.md)), and [defined](corpus.md) at Step 6's CP2, binding since Gate B. Eight sites' terms forbid automated access, even logged out, so they are measured by hand; five may be loaded by automation, with care. Every page has a memory ceiling ([performance](performance.md) §4.1) |

## Next steps: the owner's gate for Milestone 1

**Done** (2026-10-22): Milestone 1's first checkpoint, at the owner's request: the plan reviewed and rewritten, and the documents it needs proposed (Session 23 below).

Decisions only the owner can make, each with a recommendation:
1. **[ADR-0022](adr/0022-code-architecture-and-engineering-standards.md), code architecture and engineering standards.** *Recommended:* accept, decision by decision. The ones with most consequence:
   - D2: the platform layer split by facility, so the crate graph proves where Apple's frameworks run;
   - D3: foundation crates for strings, geometry, URLs, encodings and tracing, fixing placements in the overview's proposal that would have linked the TLS stack into renderers;
   - D5: generated code committed and checked fresh, which amends IP8's and PL11's wording;
   - D12: the spikes in their own workspace and lock;
   - D13: `core` renamed `browser`.
2. **[ADR-0023](adr/0023-observability-and-diagnostics.md), observability and diagnostics.** *Recommended:* accept. Axiom's own recorder, helpers' regions read as untrusted input, a flight recorder, reports, a daily health log, no content recorded; the window at M2, as you chose.
3. **[The revised plan](milestone-1-plan.md).** *Recommended:* adopt it as M1-CP1. Its order is the walking skeleton you chose, and it gates the interface notes at M1-CP3, CP4, CP5 and CP11.
4. **MA10's authorization,** with the plan's §1 additions: the repository's tools, the spikes' workspace, `codegen` and the test crates; no new third-party dependency. *Recommended:* authorize.
5. **The plan's §2 decisions,** 24 in all. *Recommended:* as the plan recommends. Notably:
   - no Xcode; WPT at `0f26972422`; no vendoring;
   - the audits of `smallvec`, `vello_cpu` and `encoding_rs` moved to M1-CP2, before anything depends on them;
   - EM11's objc2 classes checked with the first window at M1-CP3.
6. **The size limits** (ES1: 600 lines warn, 1,000 fail; functions 100). *Recommended:* keep them, and revisit at M1's gate with the files nearest them. Tighter limits would split cohesive code.

## Session log

### 2026-10-22 — Session 23 (Milestone 1's first checkpoint: code architecture, engineering standards, observability; the plan revised)

**The owner's request** (2026-10-22), on branch `milestone-1` after merging the plan's draft into it: "we need to approach it with the discipline and rigor of a mature software engineering organization." Their aims:
- "modular, maintainable, and extensible";
- no "monolithic files that accumulate thousands of lines of code";
- "establish the architectural principles, project structure, coding standards, testing strategy, dependency management approach, and key technical decisions before implementation begins";
- on the plan: "Feel free to refine or rewrite it as needed. Once the plan is finalized, we can proceed with implementation in stages."

**In plan mode** the owner chose:
- "Walking skeleton first (Recommended)";
- "Interfaces only (Recommended)", for design notes that gate code;
- on the first draft: "I think we should implement a robust observability mechanism that will help us monitor the system effectively in the long run";
- the diagnostics window at "M2, with crash reports (Recommended)".

Then the owner approved the plan.

**Done:**
- **[The crate architecture](architecture/crates.md)** (CR1–CR16, proposed):
  - seven layers, with an allowlist of internal dependencies per crate in a planned `tools/crate-map.toml`;
  - placement checked per executable;
  - the platform layer split by facility;
  - process assemblies as libraries;
  - test-only code never as Cargo features;
  - change scenarios.

  **It corrects the overview's proposal,** which calls itself provisional:
  - URLs sat in `net` and encodings in `text`, which would have linked rustls and `aws-lc-rs` into renderers and the browser, and CoreText into the HTML parser and the network process;
  - strings, atoms and the lifecycle's tokens had no crate;
  - one `platform` crate made DP21's placement uncheckable;
  - a crate named `core` hides Rust's `core` from its dependents.
- **[The engineering standards](technology/engineering-standards.md)** (ES1–ES29, proposed):
  - size limits; typed interfaces with exhaustive matching;
  - hand-written errors; RV1-16's panic classes, checked;
  - the `unsafe` policy finalized;
  - determinism and placement by Clippy's disallowed methods and types;
  - each crate's documentation as its charter;
  - committed generated code;
  - tests beside the code;
  - two tiers of checks behind `tools/check.sh`;
  - design notes, the review checklist and the definition of done.
- **[Observability and diagnostics](architecture/observability.md)** (OB1–OB27, proposed):
  - four signals in one record model, and Axiom's own recorder in every process;
  - a region per helper, read by the browser as untrusted input, on Perfetto's model;
  - a flight recorder, with reports on crashes, hangs, violations and budget breaches;
  - retention within PF27, with no idle writes, and a daily health log for 90 days;
  - exports, privacy rules, budgets and staging.
- **[ADR-0022](adr/0022-code-architecture-and-engineering-standards.md)** (D1–D15) **and [ADR-0023](adr/0023-observability-and-diagnostics.md)** (D1–D10), proposed, each with its decision history in the owner's words.
- **[The plan](milestone-1-plan.md) rewritten:**
  - §1 MA10 with the tools, the workspace split and `codegen`;
  - §2 with 24 decisions;
  - §3 how every checkpoint is built;
  - M1-CP2, the engineering foundation; M1-CP3, the platform, processes and IPC; M1-CP4, the walking skeleton with the harnesses; M1-CP5–CP12, each stage widened; M1-CP13–CP15, conformance at scale, fuzzing and budgets, the gate;
  - observability in CP2, CP3, CP4, CP9, CP11, CP12 and CP14;
  - the risks and the gates;
  - a map to MA11 and a tracker.
- **Records:**
  - ADR-0021's decision history and a note on MA11: the walking skeleton, a refinement that leaves the items and criteria unchanged;
  - the decision map: X5 and X6; its Review row, stale since 2026-10-22, now "Done"; a Milestone 1 row;
  - the index's rows and header.

**Evidence:**
- **Observed on this Mac:**
  - Clippy's lints and settings, with `cargo clippy --explain` (clippy 0.1.94);
  - in a scratch workspace in the session's scratchpad, with cargo and rustc 1.94.1: `cargo new core` only warns, but in a dependent crate, `core::mem` and a foreign macro expanding to `::core::mem` fail with E0433;
  - `cargo tree -i smallvec`: HarfRust, `icu_segmenter` and `vello_cpu`'s tree;
  - the spikes' sizes (`wc -l`: 219 files, 16 over 600 lines, 2 over 1,000) and their suppressions (86 `#[expect(` against 11 outer `#[allow(`).
- **Read on 2026-10-22:**
  - the Cargo Book's resolver chapter (features unify across the packages of one build) and its workspaces chapter;
  - Chromium's `checkdeps` README;
  - Miri's README;
  - crates.io's API (`icu_normalizer` 2.3.0 depends on `smallvec`);
  - the Rust 1.81 announcement;
  - rust-analyzer's architecture document;
  - "Design Docs at Google" (secondary);
  - Perfetto's security-model, buffers, formats and UI pages;
  - Chromium's `persistent_histogram_allocator.h`;
  - the Servo Book's profiling chapter.
- **Not confirmed, so not claimed:**
  - that the Firefox Profiler opens Chrome's trace format; the design names Perfetto's UI only;
  - that Perfetto's web UI keeps an opened file on the machine; OB23 leaves the viewer to the owner.

**For the owner's attention:**
- **`smallvec` and the browser's IDNA.** ICU4X's normalizer, which NW2's address bar uses for full IDNA, depends on `smallvec`. A `ub-risk-3` grade at M1-CP2 would bar it from the browser and move that lookup to the network process, an amendment to NW2 (the crate architecture's open question 1).
- **PF19 against PF16.** PF19 pings each helper once a second, and PF16 allows no idle wakeups. Observability relies on the cycle only while a window is active, and M1-CP3 measures it (observability's open question 5).

**Changed from the approved plan:**
- **Two commits instead of five:** the documents link to one another, and every commit must pass the link checker;
- **the URL parser completed at M1-CP11,** with WPT's URL test data in a Rust runner;
- **vendoring settled at M1-CP2,** since the dependency policy left it to "the first engine milestone".

**Not changed, until the owner accepts the ADRs:** the overview §2, MA1 and MA10, IP8, PL11, the language evaluation's §15.3 and §15.4, DP22, the requirements, the threat model, the roadmap, the handover and CLAUDE.md. ADR-0022's and ADR-0023's consequences list them.

**Side effects:**
- the scratch workspace in the session's scratchpad (`corename/`, with its build);
- `cargo info` refreshed the local crates.io index cache;
- read-only requests to crates.io and the documentation sites above;
- no crash reports.

### 2026-10-22 — Session 22 (Round 4's dispositions; its checkpoints; the merge; Milestone 1's plan)

**The owner's dispositions** (2026-10-22), in full in [the review](review/README.md) §5.9: "All 17 findings (RV4-01 through RV4-17) are Accepted as proposed." With them:
- **RV4-01:** "Implement Axiom's own non-validating XML 1.0 document parser in Milestone 1. External XML crates are rejected due to recent DoS advisories and dormant maintenance."
- **RV4-02:** "Amend DP8 to admit ub-risk-3 Rust crates in sandboxed helpers under DP10's conditions (Axiom audit of unsafe blocks + libFuzzer target under AddressSanitizer). Amend DP4 to prefer mature supporting infrastructure by default, reserving custom implementations for core differentiators."
- **RV4-08:** HTTP/2 to M2; "in-page text editing and IME mirrors to Milestone 4 (form submission and shell validation stay in M1)"; the preload scanner to M3; "Adopt encoding_rs at Milestone 1 under the amended DP8."
- **RV4-09:** "≥ 90.0% pass rate over the 5,652 static tests, with per-directory floors (no chapter of 100+ tests falling more than 10 points below the median). The full shipping median (≥ 97.8%) moves to Milestone 3 when script and WebDriver land."
- **RV4-06:** the three tiers, and "formalize budgets PF25–PF30 in docs/performance.md".
- **RV4-13:** "Re-run S5, S6, and S7 once on the current macOS build (26B118) prior to Checkpoint 7." **RV4-15:** `rustls-webpki` updated in CP2.
- **Round 3's verdicts:** "Approved as documented". **Checkpoints CP1–CP7:** "Approved". **After CP7:** "the review cycle is officially complete (no Round 5)"; then "Merge architecture-review into main with --no-ff and push main to origin" and "Produce the implementation plan (docs/milestone-1-plan.md)".

**Done:**
- **The dispositions recorded:** [the review](review/README.md) §5.9 (the owner's words), §5.10 (the findings), §5.11 (the checkpoints, with the owner's names); the round's status rows; the [index](README.md); this status.
- **CP1, M1's conformance and scope (RV4-01, RV4-09).** The XML parser for documents moves to M1 (MA1, MA2, MA7's fuzz target, MA11's second checkpoint; HD9 and §8; the roadmap's M1 and M4; RQ40), as Axiom's own. MA4's `css/CSS2` target is ≥ 90.0% of the 5,652, at least 5,087 tests, with floors for the 16 directories of 100 tests or more, from 78.3% (`floats`) to 90.0%; the full median (≥ 97.8%) at M3. **The recount** with `--needs`: 5,423 of the 5,652 need XML; 33 still need what M1 lacks (web fonts 9, SVG 19, form controls 3, print 2, overlapping) and count as failures. SB11's independent count now covers every missing capability. ADRs 0015, 0018 and 0021; the handover and the decision map.
- **CP2, the dependency policy and crate audits (RV4-02, RV4-08's `encoding_rs`, RV4-15).** DP8 admits `ub-risk-3` crates in sandboxed helpers on DP10's conditions, with Axiom's audit of each `unsafe` block and a libFuzzer target under AddressSanitizer; DP4 makes a mature implementation the default for supporting infrastructure; DP9 and DP10 follow. **`encoding_rs` adopted** for M1 behind TX3's interface in the renderer and the network process (the browser decodes UTF-8 only); its per-block audit and fuzz target are M1's (MA7, MA12). `smallvec` is admissible at `ub-risk-3` on the same conditions; JPEG's choice at M1's plan is `zune-jpeg` without SIMD or `jpeg-decoder`, neither with `unsafe` there. **Corrected:** Round 4's reuse table had `jpeg-decoder` at 2 `unsafe` blocks; with `platform_independent` it has none. `rustls-webpki` updated to 0.103.16 in the lock (RUSTSEC-2026-0382). ADRs 0010–0012; the register, the overview, the handover and the decision map.
- **CP3, the event loop's progress and the watchdogs (RV4-03).** PF19's deadlines apply to task age as well as to unacknowledged input: each watched thread (every event loop; the network process's I/O and storage threads; from M2 the GPU process's) records when its current task began, and the control thread reports the oldest age with each ping. A renderer's offer to stop comes at 5 s, at once if another agent waits on its Web Lock; a helper's thread past 5 s ends and recovers the helper. PF21 and IP13 now bound synchronous requests by the answering thread's deadline; EL §4.7 and EL17; SW10 notes that hung holders are ended, never revoked; MA5's task age and MA7's stalled storage thread. ADRs 0005, 0006, 0013 and 0019.
- **CP4, process robustness and IPC (RV4-04, RV4-05, RV4-07, RV4-10, RV4-11, RV4-12).** A renderer's death releases its resources at once, and the compositor never reads renderer memory with its CPU (PL8, IR10, with the process model's change record); PF4 counts what is mapped. Every handle in a received message is owned or destroyed with `mach_msg_destroy`, and a failed send's returned message is kept whole or destroyed (IP9, IP11, IP16). The browser takes one right per surface lifetime, validates each new surface, strips its values and sets its colour from the commit (GR14; CoreAnimation contexts hosted by ID recorded as rejected in graphics §9); a tile is committed once its commands are scheduled (GR12; IM11's video frames). In-memory channels model Mach's limits (IP2); the rendezvous pins the PID's version (IP4). Tests: MA7's renderer death and lying peer, MA8's stress configuration, TS8, TS14's V2. ADRs 0004, 0005, 0007 and 0012.
- **CP5, the efficiency strategy and its ratchet (RV4-06).** PF5 runs validation in three tiers: deterministic counts as unit tests on every commit; instructions retired, peak footprints and bytes written on fixed workloads against a committed baseline that only ratchets down at every checkpoint (tolerances 1%, 2% and no unexpected writes, hypotheses); PF1's formal measurements at every gate. PF2 reads `rusage_info_v6`. **PF25–PF30** in the performance document's new §9, each a hypothesis first measured at M1: start-up (warm ≤ 300 ms, a helper ≤ 10 ms), CPU per page load (no more than Safari's), disk (the bundle ≤ 100 MB, the cache 256 MiB), writes (none while idle beyond session state), background activity (≤ 0.1% of a core, no request of Axiom's own) and memory pressure (rebuildable caches released within 1 s). The baseline is planned as `perf/baseline.toml`, created by M1's second checkpoint. TS1, TS13, RQ5, MA6, MA11, MS7, MS10, the charter's G4; ADRs 0018–0021.
- **CP6, staging and scope (RV4-08, RV4-16, RV4-17).** HTTP/2 with HPACK moves to M2 (MA1, MA2, MA7, MA11; NW4; networking §8, whose stale L1 row is corrected); text entry into pages and IN7's IME mirror for them move to M4 with form controls, form submission and the shell's validation staying at M1 (HD28, HD §8, input §5); the preload scanner moves to M3 (HD6). The storage engine (SW2) is chosen at M2's plan by SW13's crash tests over the cache and the cookie jar (the roadmap, storage §8 and SW13, NW10, the register). From M2 the GPU process forwards child frames' rectangles with the layer commit (IV13, FT12). ADRs 0007, 0013, 0015, 0020 and 0021; the handover and the decision map.
- **CP7, the re-runs, the sweep and the verification (RV4-13, RV4-14).** **S5, S6 and S7 re-run once on 26B118,** each compared in a new section of its results: S5's and S6's verdicts hold throughout; S7-Q1, S7-Q3 and S7-Q4 pass, and S7-Q2's handshakes hold, but its per-byte comparison didn't reproduce on a network path several times slower that day (a VPN client was running), so M1's networking checkpoint measures it again. Every results document records its build. **RV4-14:** RQ30, RQ54, RQ56, RQ61; invalidation's and HTML's open questions; the decision map's stale items and its four new couplings; 26 answered or moved open questions in 12 designs; the handover brought up to Rounds 3 and 4, with S16. **The sweep** corrected images §4's JPEG row, the audit's summary, the process model's R6 and its note on surfaces, and the index. **The verification** added RV4-07's M2 test and MA10's missing dependencies. **The independent recount** (`tools/wpt-recount.sh`, added): 196 with script and 229 needing neither, as before; 6 more script-free files, which the manifest types as manual or visual. **The review cycle is complete** (the review's §6).
- **The merge:** `architecture-review` into `main` with `--no-ff` (`bca6b8b`), after the five checks on `main`'s tree, and `main` pushed, as the owner directed.
- **[Milestone 1's implementation plan](milestone-1-plan.md),** drafted on a new branch, `milestone-1-plan`, from `main`: what MA10 authorizes; fifteen decisions with recommendations; checkpoints M1-CP2 to M1-CP13, each with its scope, tests, efficiency checks and exit criterion; the efficiency section MS10 asks for; risks; the owner's gates. The branch isn't pushed.

**Side effects:**
- **Under `target/` (git-ignored):** the re-runs' records, `target/s5-results/20261022-182712/`, `target/s6-results/20261022-183751/` and `target/s7-results/20261022-185224/`, with their builds under `target/s5/`, `target/s6/` and `target/s7/`.
- **Network requests:** S7's read-only requests to its public test hosts, about 2 × 50 MB per run, as authorized; `cargo update` reading crates.io's index.
- **The window:** S6 opened its test window at the top left of the screen during its runs.
- **Crash reports:** none, checked after the re-runs.

### 2026-10-22 — Session 21 (Round 4: Round 3 tested, the owner's themes, everything unreviewed; spike S16)

**The owner's commission** (2026-10-22), in full in [the review](review/README.md) §5.8: "Perform one final round of review, as we should not take Gemini's verdict for granted without testing." Three themes: scope by value ("prioritizing the core functionality, the most common use cases, and the features that deliver the greatest value"); reuse ("take full advantage of mature, battle-tested software wherever appropriate, provided it meets our security, compatibility, and architectural requirements"); and efficiency ("should we make efficiency a first-class architectural requirement from the very first phase"). After it: "incorporate the findings, update the documentation, resolve any remaining architectural questions, and establish a clear implementation plan." **In plan mode** the owner chose "S16, all three questions (Recommended)" and "Everything unreviewed", and approved the plan.

**Done:**
- **Round 3 recorded:** Gemini's file, kept verbatim as `docs/review/round-3/01-gemini-review.md`.
- **Spike S16** (`spikes/s16/lifetime`; `unsafe` only in `src/platform/`, invariants L1–L10; `tools/s16-measure.sh`), with a formal run of three runs under PF1's power and load conditions (cool-downs of 10 s, noted as a threat) and a replication ([results](research/s16-results.md)):
  - **Q1:** a dead renderer's images aren't re-attributed to the compositor, but stay resident, charged to no process, until it unmaps them; a CPU read charges them;
  - **Q2:** rights in dropped messages leak one entry each; a failed send returns its rights with an extra reference to the destination; the kernel cleans messages queued at a port's destruction;
  - **Q3:** instructions retired vary 0.03–0.15% run to run on fixed work, against 1.1–37% for wall time.
- **`tools/wpt-script-deps.py` fixed** to follow references and their chains, with `--needs`, `--list-without` and `--tests-only`; it reproduces Round 1's counts and finds the XML dependency. **`tools/wpt-file-facts.py`** (encodings, Ahem, pseudo-classes) and **`tools/crate-facts.py`** (crates.io, GitHub, RustSec, `cargo-vet` audits) added.
- **Round 4's seven documents** in `docs/review/round-4/`: the [risk matrix](review/round-4/01-risk-matrix.md), the [verification of Round 3](review/round-4/02-verification.md) (RV3-01 to RV3-18), [reuse and scope](review/round-4/03-reuse-and-scope.md), [efficiency](review/round-4/04-efficiency.md), the [unreviewed areas](review/round-4/05-unreviewed-areas.md), the [findings](review/round-4/06-findings.md) (RV4-01 to RV4-17) and the [actions](review/round-4/07-actions.md), which classify every open question and outline Milestone 1's plan.
- **The indexes:** the review's README (status, Rounds 3 and 4, §5.8, §9, §10), [docs/README.md](README.md), the [research index](research/README.md) and the [spikes' README](../spikes/README.md).
- **No ADR, design or binding document changed:** they wait for the owner's dispositions.

**Side effects:**
- **Under `target/` (git-ignored):** S9's WPT checkout gained `css/reference/` by `git sparse-checkout add`, at the same revision (25 files); `target/review-r4/` holds RustSec's database (`4b587f0099`), six `cargo-vet` audit files, wpt.fyi's run summaries and five crates' sources downloaded from crates.io, each checked against the index's checksum; `target/s16-results/` and `target/s16-smoke/` hold S16's records.
- **Under `scratch/review-r4/` (git-ignored):** the round's notes, the downloaded sources as `.txt`, the crate facts, the test lists and a few counting scripts.
- **Network requests:** read-only downloads from GitHub, crates.io and its index, wpt.fyi's storage, RustSec and W3Techs, identified where the API asks for it.
- **Crash reports:** none, checked before and after S16's runs. Metal printed non-fatal `sandbox_extension_issue_file` messages under the GPU profile.

### 2026-10-21 — Session 20 (Round 2's dispositions; its checkpoints)

**The owner's dispositions** (2026-10-21), in full in [the review](review/README.md) §5.5:
- "I have reviewed the Round 2 findings and actions in docs/review/round-2/." "Before concluding the review phase and merging into main, I will commission one final, independent external review with Gemini to ensure complete cross-model validation."
- **Findings:** "All 15 findings (RV2-01 through RV2-15) are Accepted as proposed."
- **Test configuration (RV2-03):** "Approved as recommended": "Run WPT conformance suites (MA4's gate) in real, sandboxed processes on host time"; "Run time-dependent and scheduler tests in single-process mode on EL24's virtual clock"; "Run visual reference tests under both configurations at milestone review gates."
- **Web Locks (RV2-04):** "Approved. Arbitrate Web Locks in the browser process (matching Chromium's LockManager pattern) so that helper process crashes/restarts cannot violate mutual exclusion."
- **Checkpoints:** "Approved. Please proceed with applying all six checkpoints (CP1–CP6) as outlined in docs/review/round-2/04-actions.md §5".
- **After them:** "Run all five validation checks after each commit." "Once CP1–CP6 are verified and committed, push architecture-review to origin." "DO NOT merge into main yet. Notify me when the branch is pushed so I can initiate Gemini's review."

**Done:**
- **The dispositions recorded:** [the review](review/README.md) §5.5 (the owner's words), §5.6 (the findings), §5.7 (the checkpoints); the round's status rows; the [index](README.md); this status.
- **CP1, the entry documents and stale statements (RV2-01, RV2-02).**
  - **The [handover](handover-phase-0.md) rewritten for Round 1,** as the external review's starting point: its status, summary and "what the review changed"; the redrawn diagram; each ADR's amendments; the GPU/compositor process, real processes from M1 and the signing identity in §4.3; B1a and B1b; the collector's end-state, the slot width, the reuse line, text's rules, the two executors, the content tier's gate, the amended roadmap and Milestone 1; spikes S10, S12 and S15; trade-offs, deferred decisions, assumptions, risks and open questions; and a note on each of §11's thirty flagged areas that Round 1 or 2 answered.
  - **The [decision map](decision-map.md):** Round 1's notes on F2, F3, P7 and X1–X4; the Review row "Under way".
  - **The review's process** (§4): the entry documents change with the rules, and the last checkpoint of a round searches for each finding's old wording.
  - **The stale statements** RV2-02 listed, corrected, and two more the sweep found: OQ-20's reason (it predates RV1-07) and script and heap's summary of C4 (it predates RV1-04).
- **CP2, processes, IPC and test configuration (RV2-03, RV2-04, RV2-13, RV2-14).**
  - **RV2-03, as the owner chose:** conformance in real, sandboxed processes on the host clock; time-dependent and scheduler tests in single-process mode on EL24's virtual clock; visual reference tests under both at each milestone's gate (TS2, TS8, MA4, MA8, EM9, EL24). EM5's table gains each request's default.
  - **RV2-04:** IR10 recovers by process type (the network process relaunched with new endpoints; the GPU process as GR13 says); PF19's watchdog covers every helper; EM4 reports helpers' crashes and hangs; **Web Locks are arbitrated in the browser's core,** as the owner chose (SW10); `sessionStorage`'s browser copy is kept current (SW3); SW13 checks the clients' view; MA7 tests the network process killed and stalled; M2 tests the GPU process's.
  - **RV2-13:** IR5 and IP16 cover every process that serves several peers; a child's sends never block a thread (EL1), with a bounded local backlog drained on Mach's send-possible notification; PF21's bound per peer; SG11's note; a stalled-renderer test at M1 and M2.
  - **RV2-14:** IP13 takes PF21's rule, with one exception for a service worker's answer; IR6 names the network process; IP7 bars `iosurface` from renderers' interfaces; IPC's families gain network control, locks and video frames; IPC's question 3 answered.
  - **Also fixed:** a blank line Round 1's CP1 had left inside the process model's change record, which kept its Round 1 rows from rendering as a table.
- **CP3, the GPU process's memory and tiles (RV2-05).** PF4: the GPU process accounts its allocations by page and reports them, since macOS charges tiles to it (S10), with `IOSurfaceSetOwnershipIdentity` tested at M2 as the alternative. GR12: a spare tile is drawn into only when `IOSurfaceIsInUse` says no process reads it, the browser deleting each port right after lookup, with a bounded pool allocated ahead, tested at M2 with a window. IPC's question 4 answered; M2's content; PL8's and GR8's notes.
- **CP4, storage and the File API (RV2-10, RV2-11, RV2-12).**
  - **RV2-10:** the storage key gains an ancestor chain bit (SW1, SI3 (a)), and shared workers, Web Locks, `BroadcastChannel` and the origin private file system use the whole key; NW8 keeps its key, now with the reason; NW11's partitioned cookies are checked at M2.
  - **RV2-11:** the File API designed in the storage design's §5.5 (SW14–SW17): small blobs in their renderer, shared or large ones in the network process; **the `blob:` URL store in the browser,** a choice the finding left open (the frame tree already relies on the browser's records, a network relaunch mustn't lose them, and Chromium's registry is browser-side); the user's files read by the browser and passed as data, so SG1, EM5 and IP15 agree, with IP15's one exception for the origin private file system; a row in the standards' §5.2.
  - **RV2-12:** the storage design's open question 2 answered; SW3's 5 MiB; SG10 lists all a network process holds; PF24's quota per site over its keys.
  - **Also fixed:** the storage design's §6 still sent Web Locks to the network process, which CP2 moved, and its §7's "the browser holds no web storage" now names the opaque copies the browser keeps.
- **CP5, accessibility (RV2-06 to RV2-09).** AX2's bounds in the space of the nearest layer-backed scroll or animation node, with advances per grapheme cluster in the full tier; AX4 answers hit tests from the cache and refines them asynchronously; AX7's tiers: full while VoiceOver or Switch Control runs (KVO), basic for other clients, off after 60 s without queries (a hypothesis); AX11 places each suite: roles and names through WebDriver's Get Computed Role and Label (TS2, from M4), the platform mapping against the shell's build with an `axiom:processID` capability and the owner granting the runner Accessibility; AX4's eight web notifications (their sources inferred from the names) and `ariaNotify`; AX10's `inverted-colors`; live regions in full by M7.
- **CP6, style's hooks from Interop 2026 (RV2-15).** Style's complete model names four hooks: the position fallback origin as a value of ST12's origin field; anchor positioning's fallbacks as ST25's nested lifecycle for one element, under IV1's amendment; effective zoom in ST4's computed stage; `attr()`'s dependencies in ST19's invalidation. `zoom` and `attr()` staged with style's second stage (M4), anchor positioning when needed; layout's §8; the CSS drafts cited.
- **Verified** before the push, as the owner asked: each finding's change against the actions' "done when"; the round's final sweep over every living document for each finding's old wording, which corrected what Round 2's own changes had left (PF4's summaries, the virtual-time summaries, IN9, and three ADR records); the five checks green after every commit.
- **Pushed:** `architecture-review`, with the dispositions, CP1–CP6 and this verification. **Not merged** into `main`, as instructed.

### 2026-10-21 — Session 19 (the owner's decisions on Round 1's results; Round 2)

**The owner's decisions** (2026-10-21): "The Round 1 remediation and spike results (S10, S12, S15) on architecture-review are approved." In short (the words in full in [the review](review/README.md) §5.4):
1. **The GPU process's warm-up:** "Approved. The GPU process may resolve its cache directory via confstr(_CS_DARWIN_USER_CACHE_DIR) immediately before applying its sandbox profile." "Update ADR-0004 D2 to document this single exception."
2. **Its rules:** "Confirmed. The 10 rules identified in docs/research/s10-results.md §6.1 are adopted into docs/security/process-model.md §4.1. Incorporate the removal tests into Milestone 2's validation suite to catch silent framework failures."
3. **B1b:** "Approved. Keep the 15 MB baseline ceiling. Milestone 2 will measure display-list memory density per item and introduce per-window scaling if complex DOM trees exceed the margin."
4. **The choices made within the dispositions** (WPT's fonts, one ordered stream, layers animated in the browser, B1's split): "All approved as implemented."
5. **The merge** waits until Round 2 is complete, "preserving main as the verified baseline."
6. **Round 2:** "Authorized. Please proceed immediately with executing Round 2 of the architecture review on branch architecture-review."

**Done:**
- **The decisions recorded and applied:** ADR-0004 D2's one exception and its history; the process model's §4.1 GPU column, §10 and change record; TS14's V1 with the removal tests from M2; the roadmap's M2; B1b in the process model, PF6 and ADR-0019; S10's results; the review's log (§5.4).
- **Round 2 written** in `docs/review/round-2/` ([the review](review/README.md) §2, §8), at the baseline `ec7c6b3`:
  - **the verification:** all 22 of Round 1's findings resolved as the owner worded them; ten leave stale statements elsewhere (RV2-01, RV2-02); one regression, accessibility's geometry after PL15's amendment (RV2-06); two contradictions the changes introduced (RV2-03, RV2-05);
  - **the audit:** accessibility, storage and workers, style and layout, the embedder API and IPC are sound in their architecture; their gaps are RV2-04 to RV2-15;
  - **15 findings: no Critical, no High, 10 Medium, 5 Low,** so the stopping rule is met for the scope commissioned. Not reviewed, as the commission didn't name them: the requirements' trace, S1–S4's methods, the decision map's couplings, dated facts, invalidation, HTML and the DOM, the engine's details, networking beyond its keys.
- **Evidence:** no spike ran. Primary sources were downloaded with `curl` into `scratch/review-r2/` (git-ignored), and every quotation was checked against them: Chromium's `StorageKey`, `NetworkAnonymizationKey`, `LockManager`, `BlobUrlRegistry` and network service README; Chromium's commit and code reviews on accessibility; Chrome's blog; WPT's files at `0f26972422`; the CSS drafts; Fetch. One source is secondary, a search summary of a blink-dev intent, and is marked so. SDK headers from `MacOSX27.0.sdk`. Nothing was left outside the repository.
- **Corrections made while writing:** the A→B→A finding (RV2-10) first covered the network partition key too, but Chromium's network anonymization key treats a→b→a as same-site, so NW8 is at parity and the finding covers the storage key only. Chromium's 2024 change on accessibility enablement keeps a basic mode for clients other than VoiceOver, which RV2-07's counter-proposal now follows. Two passages found while sweeping (performance §9's first question; the threat model §10's OQ-16) joined RV2-02.
- **Updated:** the review's README (status, §2, §8), the [index](README.md), and this status (current state, next steps).

### 2026-10-21 — Session 18 (Round 1's dispositions; the accepted changes; spikes S10, S12 and S15)

**The owner's dispositions** (2026-10-21), "to be recorded in docs/review/README.md §5 and status documents". The words for each finding are in [the review](review/README.md)'s log (§5.1); in short:
- **The review's process:** "Approved as written in docs/review/README.md (§3–§6), including finding formats, severities, the disposition log, and the stopping rule."
- **All 22 findings accepted,** with RV1-07's recommended option: "Run real, sandboxed renderer and network processes from Milestone 1 using the profiles validated in Spikes S4, S5, and S7."
- **RV1-10:** "Retain the hybrid CoreText + HarfRust/skrifa architecture; reject the single-rasterizer alternative."
- **Spikes:** "Authorized Immediately (Pre-Milestone 1)": S10 ("GPU process IPC hop latency, display list validation, and zero-copy texture sharing"), S12 ("ThreadSanitizer workaround routes on macOS 27 (-Z external-clangrt)") and S15 ("Stable self-signed codesigning persistence across rebuilds"). "Authorized Before Milestone 3": S11, S13 and S14. "Authorized Before Milestone 4": the "Manual census addendum (Chrome memory-infra traces & UseCounters on normalized feeds)".
- **The cadence:** record the dispositions; "Apply the accepted documentation changes as clean, atomic checkpoints on architecture-review" (CP1–CP5); "Execute the authorized Pre-M1 Spikes (S10, S12, S15)"; "Run all five validation checks after each checkpoint. Keep architecture-review pushed to origin."

**Done:**
- **The dispositions recorded:** [the review](review/README.md) §5 (findings, spikes, checkpoints), the round's status rows, the [index](README.md) and this status. CP6 is added for RV1-04–RV1-06, which the five named checkpoints leave out.
- **CP1, Milestone 1's conformance and harness (RV1-01), diagnostics (RV1-15) and failure semantics (RV1-16).**
  - **The interim `css/CSS2` target** is ≥ 97% of the 5,652 tests in scope that need no script: their shipping median is 97.8% (Chrome 5,550, Firefox 5,527, Safari 5,450 passing), computed with the tools' new `--list-free` and `--tests`, and checked by an independent count (grep found the same 196 tests with script). The full target over all 5,848 is M3's.
  - **A correction:** the script-free precedent is `ServoLegacyRefTestExecutor` in `executorservolegacy.py`; at the pinned revision `executorservo.py`, the file RV1-01 and the owner's words name, holds Servo's WebDriver executors. Recorded in the review's log, ADR-0018 and ADR-0021.
  - **A choice the dispositions left open:** WPT's test fonts take the Rust path from M1, as TX21 and TS3 already said (RV1-01's first option).
  - Amended: Milestone 1, testing, the standards, EM9, JE27, the roadmap §4.1, IR10, IP11, the overview §2, and ADRs 0002, 0005, 0009, 0017, 0018, 0020 and 0021.
- **CP2, the multi-process topology and the GPU/compositor process (RV1-02, RV1-07, RV1-09).**
  - **The topology:** renderer → GPU/compositor process → browser, from M2. The GPU process validates display lists, holds the property trees, rasterizes tiles into IOSurfaces it owns and sends the browser layer commits; the browser holds the CoreAnimation layer tree, **scrolls and animates layers itself** (RV1-02's open choice, decided so that input never crosses a process), routes input and keeps the display links. At M1 the compositor runs as a browser thread behind the same interface.
  - **Choices made within the dispositions, for the owner to overrule:** one ordered stream through the GPU process rather than WebKit's two streams (revisited if S10 shows the second hop costs a frame); WebGL and WebGPU placed at M9's plan, with a recommendation that they get their own process, since the compositor's holds every site's pixels; B1 split as B1a (the browser, 60 MB) and B1b (the GPU process, 15 MB as a hypothesis, plus the tiles).
  - **Real processes from M1:** sandboxed renderer and network processes under S4's, S5's and S7's profiles, with S4's probes in each helper; single-process mode only for automated tests, fuzzing and Miri. **Renderers have no IOKit:** pixels in Mach memory entries, an explicit `iokit-open` denial, sampled in place by the compositor (S10 measures it).
  - Amended: the process model, ADR-0004 D1, graphics, the pipeline, input, the event loop, IPC, images, the overview (with its diagram redrawn and aligned), the roadmap, Milestone 1, performance, the threat model, testing, the requirements, the standards, the decision map, and ADRs 0005, 0006, 0007, 0012, 0019, 0020 and 0021.
- **CP3, cross-site frames as separate agents from M1 (RV1-03):** FT10 states the one rule; the frame tree's §5, the overview's §6 and the roadmap's M5 now say cross-site frames *remain* separate agents, and L2 adds site locks; SH1; WB20 moves into the bindings' first script stage; V9's agent half runs from M3. ADRs 0007, 0008 and 0017 record it.
- **CP4, scope and dependencies (RV1-08).** The arithmetic behind Temporal and BigInt becomes a replaceable detail behind the engine's interfaces (DP1 (g), DP2), with `temporal_rs` graded before M3 and BigInt's candidate before M4; the builtins and `RegExp` stay core.
  - **`encoding_rs` 0.8.42 graded `ub-risk-3`** by Axiom's audit ([audit](research/encoding-rs-audit.md)), with SIMD off on this Mac: safety comments on 22 of its 143 compiled `unsafe` blocks; uninitialized pages treated as initialized behind an empty `asm!` barrier; and `simdutf8`'s NEON validator, ungraded, an unconditional dependency on aarch64, so "SIMD off" can't be had here by feature. DP8 still bars it, and the encodings stay Axiom's own behind Axiom's interface (TX3). The owner approves the audit with the dependency (DP15).
  - **A new tool,** [`tools/unsafe-census.py`](../tools/unsafe-census.py): the `unsafe` a crate compiles in one configuration, for the DP8 audits still due (`vello_cpu`, `smallvec`, `temporal_rs`).
- **CP5, text, concurrency and the roadmap (RV1-10–RV1-14, RV1-17–RV1-22).**
  - **Text:** TX22, metrics follow the rasterizer (CoreText's advances and bounds for `sbix` faces such as Apple Color Emoji); TX23, `-webkit-font-smoothing` on both paths; TX24, optical sizing with one instance for metrics and raster; TX13's calibration metric; the single-rasterizer alternative recorded as the owner rejected it.
  - **Concurrency:** TS11 in the owner's order: Miri, loom, schedule exploration on EL24, then ThreadSanitizer by S12's route.
  - **The roadmap:** WebSockets to M4, with targets naming their exclusions; WebAssembly's architecture before M3 and the JIT at M5's gate; downloads, find in page and zoom at M5, session restore at M6; ECH with DNS over HTTPS at M5's plan; passkeys' hybrid transport before M6; JPEG's decoder at M1's plan.
  - **Site compatibility:** SB13, a Safari-compatible user agent with an Axiom token, frozen per milestone (Safari 27.2 on this Mac today); SB14, interventions as data outside the engine.
  - **Signing and builds:** ADR-0004 D2's stable local identity if S15 passes; the bindings split by interface group, with S13 before M3.
  - **The Low findings:** pixel stealing in the threat model and PL9 from M5; the stale ECH fact corrected (in the handover too); PDFs in Preview and WebRTC on the not-planned list.
- **CP6, added: the collector, the slot width and the memory gate (RV1-04–RV1-06).**
  - **The collector's end-state** (ADR-0008's decision 8, SH10): a generational nursery; incremental marking driven by allocation as well as idle time; parallel marking in pauses, on arenas lent read-only (EL11's second exception); non-moving tenured arenas with selective evacuation. Concurrent marking's possibility and the barrier's semantics are decided with S11 before M3.
  - **The slot width** (JE9, JE10): open until S11, between 64-bit NaN-boxed slots and 32-bit slots; values in registers stay NaN-boxed.
  - **The content tier's gate** (the performance §4.1): a normalized workload to the 50th item; a slope per 100 items at most half of Chrome's; a plateau after a collection; 150 MB a stretch target; zero-copy resources a rule. OQ-25 is answered. B6 is derived again at M6's plan, with out-of-process frames' renderers counted.
- **A sweep for stale statements** after CP1–CP6 fixed seven more (RQ18's JIT timing, ADR-0020's consequences, IPC's L0 staging, the content tier's table, ADR-0019's B3, and B1's and B3's summaries).
- **Spike S12, ThreadSanitizer routes** ([results](research/s12-results.md)): **both work.** The nightly of 2026-10-21 (rustc 1.101.0, 19833af72) runs ThreadSanitizer with its own runtime on macOS 27.2; the April nightly S9 used runs it with `-Z external-clangrt` and Apple clang 21's runtime. Each passes the clean test and reports S9's planted race (2 reports, exit 66 with `abort_on_error=0`); the April nightly's own runtime still crashes before `main`. **Recommended for M1:** a nightly of 2026-10-21 or later with its own runtime, `-Z external-clangrt` as the fallback (TS11, MA7, MA10).
  - **Side effects:** the toolchain `nightly-2026-10-21` installed beside `nightly` (removable with `rustup toolchain uninstall`); rustup updated itself from 1.29.0 to 1.29.1 during that install, which I didn't intend; two crash reports moved from `~/Library/Logs/DiagnosticReports` into the results under `target/`.
- **Spike S15, a stable local signing identity** ([results](research/s15-results.md)): **both questions pass.**
  - **S15-Q1:** under a self-signed identity (untrusted, `CSSMERR_TP_NOT_TRUSTED`, yet usable by `codesign`), S4's 18 probes, the JIT matrix with `allow-jit` and `single-jit`, CoreText in its profile, start-up and IPC give the same output as ad hoc, but for the helpers' directory.
  - **S15-Q2:** a rebuild under the identity reads the first build's Keychain item with no dialog, where an ad-hoc rebuild is refused (`errSecAuthFailed`); the item trusts the designated requirement, `identifier … and certificate root = H"…"`, which the rebuild satisfies. Privacy permissions (TCC) need the shell as an app bundle and the owner's clicks: left to M1's shell checkpoint.
  - **The probe** (`spikes/s15/probe`) has `unsafe` only in `src/platform/`; `tools/s4-sign.sh` gains `--identity` and `--keychain`.
  - **No side effects:** the throwaway keychain under `target/` was created and deleted with its key; the search list, the login keychain and trust settings were untouched.
- **Spike S10, the GPU/compositor process** ([results](research/s10-results.md)): **all three questions answered,** from a formal run of three runs (`target/s10-results/20261021-193709/`).
  - **S10-Q1:** the browser applied each layer commit 48.8 µs at the median after the GPU process sent it (p99 118.7 µs; 1 of 1,740 frames over 1 ms). PF13's 1 ms holds about twentyfold, so GR14's one ordered stream stands and graphics' alternative G stays rejected. The GPU process used 0.64–0.82 ms of CPU a frame, the compositor's work moved out of the browser, which kept 67–85 µs.
  - **S10-Q2:** sampling renderer pixels from Mach memory entries without a copy works under RV1-09's renderer profile: 18–62 µs to register a 4 MiB image, nothing added to the GPU process, the pages charged to the renderer. A blit holds each image twice; linear textures cost 13–65% more GPU time when minified.
  - **S10-Q3:** validation costs 9.3–9.6 ns an item; 144,876 corrupted lists were refused or decoded, with no panic.
  - **The GPU process's profile:** ten rules, each checked by removal; five fail silently, two as crashes inside Apple's frameworks and three as slower starts. B1b's 15 MB holds as measured, 13.4 MB with S6's page kept, narrowly; the tiles are charged to the GPU process that creates them.
  - **For the owner:** a warm-up before the GPU process's profile, which ADR-0004 D2's "first act" doesn't allow, and the process model's §4.1 rules ("Next steps" 1 and 2).
  - **Measured on a busy Mac:** media analysis and Spotlight's indexer ran throughout. The medians agree across the runs and with the quick pass; the tails are this session's.
  - **Corrections to my own trials:** the first "cold compile" (948 ms) wasn't one. S4's launcher gives helpers an empty environment, so the nonce never arrived, and the starts were slow because that early profile denied Metal's cache paths. And the helpers at first compiled their own profiles, which cost the GPU process 2.2 MiB; they now apply the browser's bytecode, as D2 has it.
  - **Side effects:** nine crash reports, four from scripted removal tests and five from trials, all moved into results under `target/`; Metal's per-user shader cache holds S10's shaders (3,060 KiB before, 3,172 KiB after), left to Metal, which manages its size.

### 2026-10-21 — Session 17 (the review's Round 1)

**The owner's commission** (2026-10-21): an "uncompromising, rigorous, adversarial review" of the Phase Zero baseline, with "no polite praise, generic approval, or surface-level summaries".
- **Five focus areas:** the staging from L0 to L3; CoreText beside HarfRust and `skrifa`; the content tier's 150 MB cap; the JavaScript engine and its binding layer; ThreadSanitizer.
- **Four deliverables:** a risk matrix, detailed findings with counter-proposals, challenged assumptions, and actions for Round 2.
- **The plan:** the session began in plan mode, and the owner approved its plan before anything was written: the documents, one read-only tool, a commit on `architecture-review` and its push.

**Done:**
- **[The review](review/README.md)'s process:** the finding format, severities, the disposition log (every finding awaiting) and the stopping rule.
- **Round 1:** the [risk matrix](review/round-1/01-risk-matrix.md), the [findings](review/round-1/02-findings.md), the [challenged assumptions](review/round-1/03-challenged-assumptions.md) and the [actions before Round 2](review/round-1/04-round-2-actions.md).
  - **RV1-01 (Critical):** Milestone 1's `css/CSS2` target of 97% can't be met without script. 196 of the 5,848 in-scope tests use script, so the ceiling is 96.65%. And wptrunner's WebDriver reference-test executor runs script in the page (observed in S9's checkout, `executorwebdriver.py` lines 1410 and 1451).
  - **RV1-02 (Critical):** the unsandboxed browser composites through L3, while Safari (since 2023), Chrome and Ladybird (2026) composite in a separate process; GR14's later move is undesigned.
  - **Six High:** the documents disagree on when cross-site frames become separate agents (RV1-03); the collector's end-state against PF14 and PF15 (RV1-04); the content cap as a gate (RV1-05); the width of heap slots (RV1-06); Milestone 1 in one process (RV1-07); scope against one developer (RV1-08).
  - **Ten Medium and four Low:** among them, renderers' IOKit access (RV1-09), the text model's gaps (RV1-10), concurrency checks without ThreadSanitizer (RV1-11), panics and overflow (RV1-16), and the signing identity (RV1-17).
- **[`tools/wpt-script-deps.py`](../tools/wpt-script-deps.py):** standard library only, with `--help`. It counts the script-dependent tests of a WPT directory from a manifest.
- **Updated:** the [index](README.md) and this status.

**Evidence:**
- **Nothing was re-measured.** The one new count (RV1-01) ran over S9's local WPT checkout at `0f26972422`, and S8's raw records were consulted for `-webkit-font-smoothing` (RV1-10).
- **External sources were read on 2026-10-21,** each listed with its dates in the findings:
  - WebKit's bugs and WebContent profile;
  - Chromium's renderer profile, Viz README, `harfbuzz_face.cc` and FIDO code;
  - V8's concurrent-marking post, Oilpan's README and JavaScriptCore's Riptide;
  - Apple's TN3127;
  - Servo's blog and Ladybird's newsletter;
  - the Rustls ECH announcement and the GPU.zip paper;
  - the Miri, loom and Shuttle READMEs, and winit's AppKit view.

**Not changed:** no ADR, design or binding document. The baseline stands until the owner's dispositions.

**Clean-up:** nothing to clean. The session ran reads, a counting script in its scratchpad (superseded by the new tool) and the five checks, and produced no crash reports.

### 2026-10-20 to 2026-10-21 — Session 16 (Gate C; Safari's census; the handover; the merge)

**Gate C, the owner's decisions** (2026-10-20), with the preamble: "Step 6 Gate C is approved, completing the Phase Zero architectural baseline. We are deferring implementation (Milestone 1 kickoff) until the architecture and roadmap undergo independent, multi-round adversarial review."
1. "ADR-0020 (The Roadmap and Its Security Levels): Accepted as proposed baseline. The nine-milestone progression (M1–M9) and security levels L0–L3 are approved as the baseline under review."
2. "ADR-0021 (Milestone 1, Static Documents): Accepted as the specification baseline for M1. The scope, pinned baselines, and measurable criteria (MA1–MA12) are approved as written."
3. "Architecture Proposal as a Whole: Approved as the Phase Zero baseline. The Architecture Overview v2 and Requirements Traceability matrix (docs/requirements.md, RQ1–RQ66) are approved as complete for Phase Zero."
4. "Content Tier Memory Cap (150 MB): Keep the 150 MB ceiling as the baseline hypothesis for review."
5. "New Safari S8 Census Data: Fold the clean Safari dataset from target/s8-results/20261020-222359-hand-safari-logged-out into docs/research/s8-results.md (§7.4 and status)."
6. "Phase Zero Handover Report: Please generate a comprehensive handover report (docs/handover-phase-0.md)", with the contents listed in the [tracker](research/step-6-standards-budgets-roadmap.md) §2.
7. "Merge Step 6 & Review Branch Creation: Once the Safari data and Handover Report are committed and all checks pass, merge step-6 into main with --no-ff, and push main to origin. Branch architecture-review from main and push to origin."
8. "Milestone 1 Implementation (MA10): ON HOLD. Do not create crate skeletons or write production code. The next phase will be conducted on branch architecture-review in a dedicated review cycle."

**Recorded:**
- ADRs 0020 and 0021 accepted, with their decision histories;
- the roadmap, Milestone 1, the requirements and the overview v2, with its §9 and RC10;
- ADR-0019's decision history and the performance budgets for the content cap, and a new OQ-25;
- the decision map, with Step 6 done and the review phase next; the open questions; the charter's §8; the tracker; the indexes.

**Safari's census by hand, second session** ([S8 results](research/s8-results.md) §7.4): ten pages logged out, three runs each, 22:24–23:21 on 2026-10-20, with the corrected tool. The run met the power conditions (AC, Low Power Mode off), and began at a load average of 4.04, above PF1's 3.0. Each page's cost above the census's list page:
- Maps 1,880 MB, YouTube 1,624, VS Code 1,076;
- Twitch 795, Reddit 736, Google Search 568, Three.js 433, Wikipedia 385, Instagram 278, MDN 181.

Two notes from the owner, later in the session, are recorded with them:
- "In the reddit case, I forgot to open the private window." Reddit's third run is the one whose record differs: Safari's own process held 45–61 MB, against 203–230 MB elsewhere, which the run's own baseline cancels.
- "for all sites, I ran some advanced browsing tasks such as running live code demo in mdn, street view in maps, etc", so Safari's figures aren't like for like with Chrome's automated runs.

Other findings:
- One impossible reading of `top`'s energy counter (29.9 million, Reddit's third run) is left out.
- Safari's network process grew to 233–241 MB with VS Code for the Web, a datum for B4 (inference).
- The ceilings don't change; they come to 16–54% of Safari's costs.
- The S8 results' §10 had a code block left open by a sentence on its closing fence; it is closed.

**The [Phase Zero handover](handover-phase-0.md)** written: the blueprint across the 21 ADRs, spikes S1–S9, trade-offs, deferred decisions, assumptions, known risks, open questions, and 30 areas flagged for critical review.

**CP11:** `step-6` merged into `main` with `--no-ff` once the above were committed with every check green; `main` pushed; `architecture-review` branched from `main` and pushed. This entry is committed on `step-6` just before the merge, so the merge commit itself isn't named here; `git log main --merges -1` shows it.

**Clean-up:** none needed. Since the evening's census sessions, `~/Library/Logs/DiagnosticReports` gained only a routine macOS analytics file (`SFA-swtransparency`, 23:17), like those of 07:48 and 14:14, and no crash report; it was left alone.

### 2026-10-20 — Session 15 (Gate B; S8 by hand; S9; Step 6 part 2)

**Gate B, the owner's decisions** (2026-10-20):
1. "ADR-0018 (Corpus, Standards Baseline, and Testing Strategy): Accepted as proposed."
2. "ADR-0019 (Performance Methodology and Budgets): Accepted as proposed, including the B1-B7 process model amendment (B1 tile scaling formula and B3 per-page memory ceilings)."
3. "Automation Boundary: Confirmed. Automating only the 5 permitted pages and reserving the other 8 for manual testing was the correct decision to respect site terms."
4. "Spike S8 Manual Session: Completed. I have executed the manual testing sessions across both Chrome Canary and Safari. The formal results are recorded on disk: Chrome Canary (Signed-in, 8 sites): target/s8-results/20261020-181706-hand-chrome-signed-in/ Safari (Logged-out, 5 sites): target/s8-results/20261020-190125-hand-safari-logged-out/ Please integrate these baselines into docs/corpus.md, docs/performance.md, and s8-results.md to calibrate the final B3 ceilings for all 13 sites." With it: "I have used my secondary accounts for these tests," and Prism's URL for that account.
5. "Spike S9 (Testing and Fuzzing Tooling): Authorized. Proceed with validating cargo-fuzz, AddressSanitizer/ThreadSanitizer, and wptrunner on this Mac."
6. "Not-Planned List: Approved as written in docs/standards.md §5.3 (DRM, plugins, Web SQL, AppCache, WebXR, Web MIDI, etc.)."
7. "Cadence: Approved. Proceed through CP8–CP10 (Roadmap, Milestone 1 specification, and Phase Zero final synthesis) without pausing up to Gate C."

**Recorded:**
- ADRs 0018 and 0019 accepted, with their decision histories;
- the process model's §7 holds the final B1–B7, with a change-record entry;
- the four documents binding;
- the decision map (X1–X3 accepted), OQ-13 and OQ-23, the indexes and the tracker.

**S8's session by hand, folded in** ([results](research/s8-results.md) §7):
- **The baseline:** the session recorded none, so `s8.py blank` (new) measured a fresh Canary on about:blank: 207 MB without its extension renderers.
- **Chrome's costs, signed in:** Reddit 1,260 MB, Figma 1,111, YouTube 908, Google Search 582, Instagram 578, Prism 494, Discord 427, Twitch 401.
- **Every page's ceiling** by the accepted rule ([performance](performance.md) §4.1). The content cap binds at 12–26% of Chrome's cost, which meets one of ADR-0019's conditions for revisiting, so it goes to the owner at Gate C.
- **Safari's runs need repeating.** The tool had the owner go back to the list in the same tab, so Safari kept earlier pages alive and later runs counted them; only the first run measures one page. The tool is corrected.
- **Prism's new URL** is in `scratch/corpus-urls.txt` only; its `u=` stays out of the repository.

**Spike S9** ([results](research/s9-results.md)):
- **S9-Q1 passes:** cargo-fuzz with AddressSanitizer found a planted bug in about a second, fuzzed the checked decoder 10 minutes with no finding, twice, and reported 93.25% region coverage.
- **S9-Q2 fails with the installed nightly:** its ThreadSanitizer runtime crashes before `main` on macOS 27.2, while Apple clang's works. Milestone 1 pins a nightly whose runtime works.
- **S9-Q3 passes:** wptrunner ran `dom/nodes` against Chrome Canary in 140 s at 99.6%, wpt.fyi's Chrome being at 99.5%.
- The first fuzzing run found the Mac on battery with Low Power Mode on; the step was repeated on AC power.

**A correction to CP4's calibration.** `tools/wpt-pass-rates.py` had left reference and crash tests out, because wpt.fyi's summaries give them no subtest counts. Corrected, and [the standards baseline](standards.md) §6 recomputed: 42 of 48 rows changed, and `css/CSS2/`'s median is 98.1%, not 98.9%. **Gate B's report said Ladybird meets the median on HTML parsing, the DOM and CSS 2; after the correction it meets it on HTML parsing only,** is 0.1 point short on the DOM, and 4.3 points short on CSS 2. Recorded in ADR-0018's decision history; SB10 and SB11 stand.

**CP8, [the roadmap](roadmap.md)** (MS1–MS12):
- nine milestones, from static documents at L0 to the GPU and WebAssembly at L3;
- the sandbox at M2, site locks at M5, out-of-process iframes and V1–V9 at M6, before media, applications, the GPU and WebAssembly;
- no script from the web before L1;
- corpus targets, the login track and the decision points placed.

**CP9, [Milestone 1](milestone-1.md)** (MA1–MA12): static documents at L0, with CSS 2.1's visual model; conformance without script (tokenizer 100%, tree construction ≥ 96%, `css/CSS2`'s reference tests ≥ 97%, Unicode 100%); the corpus checks; budgets (95 MB idle, the pages' ceilings, frames, idle, pauses, T1–T4); security by construction; what starting it needs the owner to authorize.

**CP10, the final synthesis:**
- [requirements](requirements.md) (RQ1–RQ66, no gaps);
- the [overview](architecture/overview.md) v2, with RC1–RC10 and Phase Zero's criteria checked;
- ADRs [0020](adr/0020-roadmap-and-security-levels.md) and [0021](adr/0021-milestone-1.md) proposed;
- the open questions, decision map, charter's §8 and index updated.

**A deviation, stopped.** A first attempt at the baseline opened the dedicated profile by script: two runs on about:blank, 19:46–19:51, with no page loaded and no debugging connection. CO14 says that profile is never opened by automation. So the attempt was stopped and its records aren't used. The profile closed cleanly (`SessionEnded`), and `s8.py blank` now opens only a fresh profile.

**Clean-up:**
- Chrome Canary left two crash reports at 17:32–17:33, during the owner's `s8.py signin` (non-fatal guard faults). Both were removed from `~/Library/Logs/DiagnosticReports`.
- S9's ThreadSanitizer runs left four crash reports there, removed; the script now moves such reports into its results.
- S9 installed cargo-fuzz under `target/s9/tools` and the nightly's `llvm-tools-preview` component, as authorized; `rustup component remove llvm-tools-preview --toolchain nightly` takes the component away.
- One report, at 17:10, came from macOS's Software Update settings; it was left alone.
- The fresh profiles were deleted after each run.

**Gate C,** the final design review: the decisions are in "Next steps".

### 2026-10-20 — Session 14 (Gate A; Step 6 from CP2)

**Gate A, the owner's decisions** (2026-10-20), each as recommended:
1. "Step 6 Plan Approval: Approved as written. The scope, readiness criteria RC1–RC10, 11-checkpoint sequence, and review gates are approved. Proceed through CP2–CP7 without pausing until Gate B."
2. "Spike S8 (Corpus Census): Authorized. Proceed with measuring the 13 corpus sites across Chrome Canary and Safari. The two-mode design (automated logged-out mode; passive, non-automated logged-in mode in a dedicated scratch profile) is approved."
3. "Account Policy: Approved as recommended: For Spike S8 measurements in shipping browsers, use my regular accounts via the dedicated local profile. For Axiom engine testing, sensitive personal accounts will wait for Level 3 (L3) verification, while non-sensitive test accounts may be used starting at Level 2 (L2)."
4. "URL Parameter Scrubbing: Approved. Please amend the local CP1 commit to remove the identifying parameters (Prism's u= and Figma's t=) from the committed repository documents, while preserving the full URLs in scratch/ for Spike S8."
5. "Push Branch: Authorized. Push branch step-6 to origin after amending the commit, and keep it pushed as checkpoints land."

**CP1 amended, then pushed:**
- `e86ea98` became `a73ccd9`. The two URLs lost `u=` and `t=`, and CP1's log gained a line saying so.
- Neither value appears in any commit on `step-6` that isn't on `main`, in diffs or messages (checked with `git log -p`). The original commit stays only in this Mac's reflog, never pushed.
- `step-6` was pushed with upstream tracking and stays pushed.

**CP2, [the corpus](corpus.md)** (rules CO1–CO14):
- **Every site's terms of use read,** with its robots file, matched as RFC 9309 specifies. **Eight sites forbid automated access even logged out:** Google Search, through robots.txt; YouTube; Instagram; Reddit; Twitch; Discord; Figma; and Prism, under OpenAI's terms (a secondary source). The plan had cited only Discord's.
- **So S8 automates only five pages:** Wikipedia, Google Maps, MDN, Three.js and VS Code for the Web. Automation identifies itself, checks robots.txt, loads each site at most five times, and never signs in. The other eight are measured by hand, in the passive mode the owner approved, which needs the owner at Gate B.
- **The accounts,** as decided; the measurement protocol, window and browsers; live baselines, with copies only where terms and licences allow (Wikipedia, MDN, Three.js); and privacy rules for the dedicated profile.

**Also updated:** the tracker (§2, §6, §7, §9, §10.1 refined, §11), the charter's §8, OQ-2's entry, the decision map, and both indexes.

**CP3, [spike S8](research/s8-results.md),** the corpus census, built and run:
- **The tools:** a DevTools Protocol client over Chrome's pipe; features from Chrome's own use counters, with web-features' identifiers; sizes; processes sampled with `footprint` and `top`.
- **The census:** the five pages automation may load, three valid runs each, in Chrome Canary 158.
  - Two hidden runs were replaced, which used Wikipedia's and Three.js's fifth and last loads.
  - The pages touch 116 of web-features' features: 72 widely available, 19 newly, 25 in only some engines.
  - Steady-state cost above about:blank: Maps 627 MB, Three.js 470, VS Code 457, Wikipedia 319, MDN 145.
- **Pending:** the session by hand (eight pages; Safari, whose remote automation is off), for the owner at Gate B.

**CP4, [the standards baseline](standards.md)** (SB1–SB12):
- WHATWG standards pinned at commit snapshots per milestone; five support classes; four approximate behaviours.
- A matrix of about 50 areas, with each area's suites and the corpus's needs.
- Targets: the median of Chrome, Firefox and Safari on each WPT directory, calibrated on wpt.fyi's runs of 2026-10-19 with the new `tools/wpt-pass-rates.py`.

**CP5, [the testing strategy](testing.md)** (TS1–TS18):
- WPT through a test-only WebDriver server in the headless embedder, as Ladybird runs it; Axiom's own runners for html5lib-tests, test262 and Unicode.
- Fuzzing with cargo-fuzz and AddressSanitizer, ThreadSanitizer, Miri; V1–V9 as tests; weekly advisory checks, and the OS-dependent checks again after macOS updates.
- Spike S9 designed for Gate B.

**CP6, [the performance budgets](performance.md)** (PF1–PF24):
- The method as rules; a page's cost includes the GPU memory spent for it.
- The final B1–B7 for the owner: B1 at 60 MB plus 6.2 bytes per window pixel for tiles. B3's 50 MB median stands, with a ceiling for every corpus page: half of Chrome's cost, capped by tier.
- Frame, pause and long-task budgets; energy; the build; the limits and deadlines, each labelled sourced, measured or hypothesis.

**CP7:** [ADR-0018](adr/0018-corpus-standards-and-testing.md) (the corpus, the standards baseline and testing) and [ADR-0019](adr/0019-performance-methodology-and-budgets.md) (performance, with the final B1–B7 as an amendment to the process model) proposed. The decision map, OQ-13, the index and status updated. **Gate B.**

**Clean-up:**
- The terms pages and robots files were downloaded into the session's scratchpad, read, and left there.
- S8's Safari check made `safaridriver` launch Safari. A system agent it started left a report of a non-fatal guard fault in `~/Library/Logs/DiagnosticReports`, which was removed. Safari and its WebKit GPU helper, both launched by the check, were ended.
- S8's profiles were deleted after each run. Nothing else outside the repository changed.

### 2026-10-20 — Session 13 (Gate C follow-ups; Step 6 kickoff)

**The owner's decisions after Gate C** (2026-10-20):
1. "Display P3 Color Pattern (S6-Q5): I've captured a screenshot of the three squares at the end of the run. Please analyze the image critically.." The second formal run: `target/s6-results/20261020-074751`.
2. "Real-World Site Corpus (OQ-2): Use the following 13 sites as Axiom's standard evaluation corpus for performance budgets, WPT suites, and memory baselines in Step 6", with the 13 URLs ([Step 6 tracker](research/step-6-standards-budgets-roadmap.md) §2.4).
3. "Memory Target B1 (OQ-23): Approved. In Step 6, budget visible window tile memory separately based on surface pixel dimensions (e.g. 25.2 MB for this Retina window), and maintain a 60 MB budget for the core browser process state."
4. "GPU Path Rasterization (OQ-24): Approved. Keep path rasterization in the renderer for now to preserve analytic edge quality. Defer the GR5 display list amendment to the graphics milestone."
5. "Renderer Rasterizer: Approved. Adopt vello_cpu as the primary Rust rasterizer for web fonts and paths, subject to crate dependency grading at the text milestone."
6. "Step 6 Authorization: Authorized. Proceed with creating branch step-6 from main and drafting the Phase Zero Step 6 plan (standards baseline, testing strategy, performance budgets, and Milestone 1 roadmap) for Gate A review."

During the session the owner added: "You can store important scratcpad codes and files in this project folder so that you can use them later even if the system wipes out the scratchpad." And: "Many of the websites that I have mentioned, requires log in. We need to create a mechanism to log in to get the full experience."

**S6's follow-up** ([S6 results](research/s6-results.md)):
- **The second formal run** reproduced the first: S6-Q1 and S6-Q3 pass at the median again. The tails were again in the session's first runs (225 and 127 of 600 typical updates over 2 ms in runs 1 and 2, 1 in run 3); one 10 s idle window recorded a package idle wakeup; S6-Q4's pixels were identical, and S6-Q5's footprints within 0.1 MB.
- **The Display P3 check,** measured in the owner's screenshot with the new `tools/png-region-stats.py`: the P3-tagged red is (255, 0, 0) in Display P3, and the sRGB-tagged red (234, 51, 35), exactly sRGB's red converted, computed independently (a plain 2.2 gamma would give (233, 54, 40)). The untagged surface is (255, 0, 0), unconverted. **S6-Q5's colour condition is met,** and every surface is tagged (GR6).
- **The display runs scaled,** 1,440 × 900 points in a 2,880 × 1,800 framebuffer on the 2,560 × 1,600 panel, so S6's window was not full screen; the results are corrected.

**Recorded:** ADRs 0011, 0012 and 0016; graphics GR2, GR5, GR6 and GR12; text TX12 and TX15; layout and paint; the process model's B1 and change record; the register; the open questions (OQ-2 and OQ-23 resolved, OQ-24 deferred); the decision map; the overview §2, §3, §5 and §8; the charter's §8.

**Tooling and working files,** as the owner allowed: `tools/s6-tails.py` and `tools/png-region-stats.py`, with `--help`; `scratch/` in `.gitignore`; CLAUDE.md's environment and layout sections. The screenshot is kept with its run, in `target/`.

**Step 6, CP1:** branch `step-6` from `main` at `b4948ce`, and [its plan](research/step-6-standards-budgets-roadmap.md): 7 questions; the 30 questions earlier steps handed to Step 6; readiness criteria RC1–RC10; 11 checkpoints; gates A–C; spike S8, the corpus census, designed for Gate A, with S9 only if needed.
- **Logging in,** as the owner asked: S8 gains a logged-in mode, in a dedicated profile the owner signs into by hand, where nothing is automated inside the page; the roadmap gains a login track (CP8).
- **Discord's terms** were read: they forbid scraping "by using any robot, spider, crawler, scraper" and "unauthorized software designed to modify the services", the reason the logged-in mode is passive.

**Not pushed:** `step-6` stays local until Gate A.

**Clean-up:** nothing to clean. No code ran but the analysis scripts and the five checks; no crash reports.

### 2026-10-20 — Session 12 (Gate C; spike S6 reported; Step 5 merged)

**Gate C, the owner's decisions** (2026-10-20), each as recommended:
1. "ADR-0015 (HTML Parsing and the DOM): Accepted as proposed."
2. "ADR-0008 Amendment (HD11): Accepted. Node slots in the arena never move; node identity remains stable."
3. "ADR-0016 (Style, Layout, and Paint): Accepted as proposed."
4. "ADR-0007 Amendment (ST25, ST31): Accepted. Nested lifecycle phases for size container queries are adopted."
5. ":visited Policy (ST10): Accepted. Partitioned history by top-level site is approved."
6. "ADR-0017 (JavaScript Engine and Bindings): Accepted as proposed."
7. "XSLT (OQ-22): Confirmed. Defer planning XSLT until a concrete use case arises."
8. "ADR-0011 (Text, Fonts, and Unicode): Accepted. I have visually inspected the S5 sheets and verified the Hebrew niqqud and Apple Color Emoji differences. CoreText is approved for system font rasterization under TX12."
9. "ADR-0012 (Graphics, Images, and Media): Accepted. The formal S6 benchmark run is complete (target/s6-results/20261019-201443/summary.txt) and easily meets all frame time thresholds. IM3's fallback (AVIF in the media utility process) is approved."
10. "NW6 TLS Verification: Confirmed. Certificate evaluation must run on worker pool threads to keep the network I/O thread non-blocking."
11. "Process Model: Approved. Record the S5 and S7 sandbox metadata rules in the process model change record."
12. "CP19 (Merge to main): Authorized. Please record S6's formal results, update the documents and indexes, and merge step-5 into main with --no-ff, followed by pushing main and step-5 to origin."

**CP11, spike S6, reported** ([results](research/s6-results.md)), from the owner's formal run on 2026-10-19 (`45bff95`, a clean tree, AC power with Low Power Mode off, the window visible in every run):
- **S6-Q1 and S6-Q3 pass at the median.** A typical update took 0.17–0.28 ms on the GPU and 0.67–1.84 ms on the compositor thread; moving layers took 0.19–0.40 ms a frame. S6-Q2: damage-limited tiles cost a spinner 47–49× less GPU time and 7–9× less energy impact than full redraws.
- **Checked against the criteria, not only the summary.** The thresholds hold at the median in every run, as the owner's words say, and the tails are recorded. Run 1 began on a busy Mac (load averages 103.80, 59.25 and 24.79, with no build to explain them): 192 of its 600 typical updates exceeded 2 ms on the compositor thread, against 0 and 2 in runs 2 and 3. In each run, 2–7 of 1,085 scrolling frames that only moved layers exceeded 1 ms.
- **S6-Q5's memory condition is not met as measured:** one window's compositor state, trimmed, leaves about 4 MB of B1 (OQ-23). The owner's words don't describe the Display P3 comparison, which S6-Q5's colour condition rests on. ADR-0012's acceptance is the owner's explicit decision, and is recorded with these results beside it.
- **S6-Q4:** GPU path filling pays in time (3.9–4.8× sooner) and memory (31–62× less), not in quality, so GR5's amendment goes to the owner (OQ-24). `vello_cpu` filled paths 2.1–6.8× faster than `zeno`.

**Recorded** (the CP19 documentation commit):
- the ADRs' statuses and decision histories: 0007, 0008, 0011, 0012, 0013 and 0015–0017;
- the designs, now binding, with IV1, IV2 and SH10 amended, and TX12, IM3, NW6, GR2, GR5, GR6, GR8 and GR12 updated;
- the process model's new change record: S5's and S7's metadata rules, and ImageIO in the media utility process;
- the register (CP3's to CP5's entries decided); the open questions (OQ-22 resolved; OQ-23 and OQ-24 raised); the decision map; the overview (§2, §3, §5, §8); S5's results' status; the tracker; the indexes.

**CP19:** `step-5` merged into `main` with `--no-ff`, and `main` and `step-5` pushed to `origin`.

**Clean-up:** nothing to clean. This session ran only analyses of the run's files and the five checks, and the formal run left no crash reports.

### 2026-10-19 — Session 11 (part 2: CP13–CP18; Gate C reached)

**Part 2 of Step 5, run without pausing as planned** (the owner's Gate B instruction: "Proceed with executing Spikes S5, S6, and S7 as planned"; the plan runs CP13–CP18 until Gate C):
- **CP13, [HTML and the DOM](architecture/html-and-dom.md)** (HD1–HD33, `4b81d95`): our own HTML and XML parsers; decoding straight into the DOM's 8- and 16-bit strings; a node arena of `u32` columns whose slots never move, which needs an amendment to ADR-0008 (HD11); one mutation path with fixed hooks.
- **CP14, [style](architecture/style.md)** (ST1–ST36, `3581d47`): our own CSS parser over 8- and 16-bit text; shared, grouped computed styles with a rule tree, a sharing cache and evictable caches; invalidation sets; container queries in nested lifecycles during layout, which needs an amendment to ADR-0007 (ST25, ST31); `:visited` by partitioned history.
- **CP15, [layout and paint](architecture/layout-and-paint.md)** (LY1–LY32, `8a5ede7`): a 1/64-pixel fixed-point layout unit; LayoutNG's contract; tables marked approximate; scroll nodes with snap areas, sticky constraints and compositor-moved scrollbar thumbs; paint into PL7's primitives.
- **CP16, [the JavaScript engine](architecture/javascript-engine.md)** (JE1–JE32, `48bc182`): heap-free compiled scripts; a register machine dispatched by a `match` loop; NaN-boxed values carrying the scope lifetime; `RegExp` interpreted with a linear-time fallback; `Intl` on ICU4X; Temporal of our own; evidence for OQ-3.
- **CP17, [WebIDL bindings](architecture/bindings.md)** (WB1–WB23, `04527fc`): our own generator over webref's curated IDL, emitting safe Rust; platform objects as heap entries, not wrappers; rooting only where a later conversion may collect; remote window proxies.
- **CP18:** [the overview, v1](architecture/overview.md); [ADR-0015](adr/0015-html-parsing-and-the-dom.md), [ADR-0016](adr/0016-style-layout-and-paint.md) and [ADR-0017](adr/0017-javascript-engine-and-bindings.md), Proposed; the decision map (P5–P7), the open questions (OQ-3's evidence; OQ-22, XSLT), the register (part 2 adds no runtime dependency) and the indexes.
- **Precedent** was read from primary sources at their latest commits on 2026-10-19. Notable current facts: Ladybird moved style (2026-08-13) and layout (2026-07-28) to Rust, put style and layout and paint on their own threads (2026-10-03), switched its DOM to wrappers built as host objects (2026-10-05), and gained an optimizing JIT; SVG 2 published a new Candidate Recommendation Snapshot (2026-10-06); Temporal is finished for ECMAScript 2027, and V8 builds it on `temporal_rs`.
- **Gate C reached:** the decisions are listed in the tracker's Gate C entry, each with a recommendation. No code was written, nothing was installed, and no crash reports or other side effects outside the repository were produced.

### 2026-10-19 — Session 10 (spike S7; ADR-0013 accepted)

**CP12, spike S7** ([results](research/s7-results.md); tracker §11), under the owner's Gate B authorization: "Spike S7 (Network Process & TLS): Authorized for questions S7-Q1 through S7-Q4 under spikes/s7/, including read-only outbound network requests to public test hosts."
- **Code:** `spikes/s7/net` (`s7-net`): a broker that compiles the network profile, and a sandboxed helper with modes for each question; its `unsafe` in `src/platform/` (Security, Network framework, DNS-SD, kqueue, Mach, sockets, libproc) under invariants N1–N9; 8 offline tests; `tools/s7-measure.sh`. Commits `4840d7d` and `13daa44`.
- **Network access:** read-only GETs to public test hosts and DNS queries through the system's resolver. No certificate was added to a keychain, and nothing on the Mac was reconfigured.
- **Formal run:** `4840d7d`, from a clean tree, on AC power with Low Power Mode off (`target/s7-results/20261019-071313/`); three runs, the Mac in normal use. Supplementary `startup` runs at `13daa44`.
- **S7-Q1: passes.** In the broker-compiled profile, rustls verified through `SecTrustEvaluateWithError` gave the expected verdict for every host it reached, the same as unsandboxed and as Network framework. The profile has eight rules; two metadata rules were found by failures the kernel didn't log (`/var`, for DNS-SD; the executable's directory, without which `SecPolicyCreateSSL` returns NULL). Every denial is explained, using reporting runs that log every attempted operation.
- **S7-Q2: reported; NW6 settled as designed.** rustls retired 2.4–3.3× fewer instructions per handshake than Network framework, 1.6× fewer per byte. rustls needs its `prefer-post-quantum` default feature (without it, Cloudflare answered every full handshake with a HelloRetryRequest). A process's first TLS use costs 29–43 ms of CPU, for AWS-LC's jitter entropy source.
- **S7-Q3: passes.** DNS-SD resolved nine queries on one shared connection, from kqueue, with one thread.
- **S7-Q4: passes.** One I/O thread drove rustls, `quinn-proto` (an HTTP/3 GET), DNS-SD, Mach messages received in the wait, a timer and a user event from one `kevent64` per turn, and woke only for real deadlines when idle.
- **ADR-0013 accepted** (2026-10-19), under the owner's Gate B words: "Pre-approved; will move to Accepted upon S7-Q1, S7-Q3, and S7-Q4 passing." The networking and storage designs are now binding; the register's CP6 and CP7 entries are decided.
- **For Gate C:** verification in the worker pool, off the I/O thread (NW6, because `SecTrustEvaluateWithError` blocks its caller and EL1 allows one place to block); noting the two metadata rules in the process model's network column; AWS-LC's jitter entropy, at the network milestone.
- **Clean-up:** no crash reports. One empty helper data directory in `target/release/`, left by an informal run cut short by `head`, was removed.

### 2026-10-18 — Session 9 (Gate B; spikes S5–S7)

**Gate B, the owner's decisions** (2026-10-18), each as recommended:
1. "ADR-0010 (Dependency and Supply-Chain Policy): Accepted as proposed, including the DP10 refinement."
2. "ADR-0011 (Text, Fonts, and Unicode): Pre-approved; will move to Accepted upon S5-Q1 and S5-Q3 passing, with TX12 settled by S5-Q2."
3. "ADR-0012 (Graphics, Images, and Media): Pre-approved; will move to Accepted upon S6-Q1, S6-Q3, and S6-Q5 passing, and S5-Q5 validating ImageIO inside the renderer sandbox."
4. "ADR-0013 (Networking, Storage, and Workers): Pre-approved; will move to Accepted upon S7-Q1, S7-Q3, and S7-Q4 passing."
5. "ADR-0014 (Accessibility): Accepted as proposed."
6. "Spike S5 (Text Stack & ImageIO Sandbox): Authorized for questions S5-Q1 through S5-Q5 under spikes/s5/, including the listed candidate crates and OFL/WPT test web fonts."
7. "Spike S6 (Metal Compositing): Authorized for questions S6-Q1 through S6-Q5 under spikes/s6/. The performance thresholds are confirmed: typical update ≤ 4 ms GPU / ≤ 2 ms compositor thread; scrolling/transforms ≤ 1 ms."
8. "Spike S7 (Network Process & TLS): Authorized for questions S7-Q1 through S7-Q4 under spikes/s7/, including read-only outbound network requests to public test hosts."
9. "Settings List: Approved as written in docs/architecture/accessibility.md §6.1."
10. "Reserved Key Equivalents: Approved as written in docs/architecture/accessibility.md §6.2 (Chromium reserved set, plus ⌘L, ⌘,, and Esc un-overridable even in fullscreen)."
11. "OQ-21 (Axiom License): Approved as 'MIT OR Apache-2.0'. Please add the standard LICENSE-MIT and LICENSE-APACHE files to the repository root."
12. "Proceed with executing Spikes S5, S6, and S7 as planned."

**Recorded** in ADRs 0010–0014's decision histories, the six design documents (the policy and the accessibility design are now binding), the register, the decision map, the open questions (OQ-21 resolved), the indexes, the tracker (§2, §6, §7, §10, §11) and this status.

**The license:**
- `LICENSE-APACHE` is the Apache Software Foundation's text, unchanged: fetched from apache.org on 2026-10-18, SHA-256 `cfc7749b…bc523d30`.
- `LICENSE-MIT` is the standard MIT text, word for word SPDX's, with "Copyright (c) 2026 Md. Tawhidul Islam".
- The workspace manifest declares `license = "MIT OR Apache-2.0"`, which every crate inherits. The root README states the dual license and the usual contribution terms.

**CP10, spike S5** ([results](research/s5-results.md); tracker §11):
- **Code:** three crates under `spikes/s5/` (`s5-rust-text`, with no `unsafe` and no Apple framework in its web-font executable; `s5-text`; `s5-image`), 40 tests, and `tools/s5-fetch.sh`, `tools/s5-images.sh`, `tools/s5-measure.sh`. Commits `f20e7f4` and `bad07a8`.
- **Formal runs:** `bad07a8` (primary, `target/s5-results/20261018-204642/`) and `f20e7f4` (reproduction), on AC power with Low Power Mode off, from clean trees; every deterministic output identical between them.
- **S5-Q1:** HarfRust chose CoreText's glyphs in all 432 runs; the Hebrew niqqud differ in position, in a font with no mark-positioning data. Awaits the owner's visual check.
- **S5-Q2:** the Rust rasterizers match CoreText above 20 px and at 2×; CoreText recommended for system fonts (TX12), pending the owner's comparison.
- **S5-Q3: passes.** 322 WPT fonts, 4,000 mutants, no crash, hang or sandbox denial.
- **S5-Q4:** B2 achievable; HarfRust shapes in 566 ns a glyph.
- **S5-Q5: missed its condition.** AVIF needs VideoToolbox's decoder service, which renderers may not reach; IM3's fallback (the media utility process) recommended for Gate C. JPEG keeps the safe decoder.
- **Findings recorded:** the PingFang stub and TX6's file-backed check; ImageIO's NULL-bundle crash (R1); CoreText's normal track against HarfRust's `trak` handling (TX10); `flate2` in `png`'s tree; ImageIO's non-deterministic JPEG pixels.
- **The owner's visual check:** published as a private page in this session, and in the results directory.
- **Clean-up** (outside the repository): seven crash reports left by informal S5 runs were deleted from `~/Library/Logs/DiagnosticReports/Retired`, after copies were made in the session's scratchpad. The formal runs' crash reports (six per run) were moved into their results directories by the script. Nothing else was left outside the repository.

### 2026-10-18 — Session 8 (Step 5 kickoff)

**The owner's decisions:**
1. "Step 4 is formally accepted."
2. "Step 5 Authorization: Authorized."
3. "Proceed with planning Phase Zero Step 5 (Subsystems) on a new branch `step-5`."
4. "Submit the Step 5 plan and checkpoint tracker first for Gate A review, defining the subsystem sequence, review gates, and identifying which areas may require targeted validation spikes."

**Done:**
- Branch `step-5` created from `main` at `c642ab3`. It is not pushed.
- **CP1:** the [Step 5 plan and tracker](research/step-5-subsystems.md):
  - 14 questions; the 21 questions earlier steps handed to Step 5, each assigned to a checkpoint;
  - the subsystem criteria SC1–SC10;
  - 19 checkpoints: the dependency policy and six platform-facing subsystems (text, graphics, images, networking, storage, accessibility); then the spikes; then the document and script engine (HTML and the DOM, style, layout and paint, the JavaScript engine, bindings), the overview and the merge;
  - three gates: A, the plan; B, part 1's ADRs and the spikes; C, part 2's ADRs and the merge;
  - three candidate spikes: S5 (text stack), S6 (compositing on Metal), S7 (the network process and TLS).
- The plan proposes a new decision-map item, X4 (dependency and supply-chain policy), for the owner to approve. Until then the map is unchanged except for Step 5's row.

**Verification:** the four standard checks and the link check pass, with the code unchanged.

**Gate A, the owner's decisions** (2026-10-18), each as recommended:
1. "Plan Approval: Approved as written." "The 19-checkpoint sequence, two-part structure, and subsystem criteria SC1–SC10 are approved." "Step 4 interfaces remain binding under SC1."
2. "Decision Map Addition: Approved. Add X4 (Dependency and Supply-Chain Policy) to docs/decision-map.md, to be decided at CP2 in ADR-0010."
3. "Candidate Spikes: Confirmed." S5, S6 and S7 "are approved as candidate spikes, with code authorization to be decided individually at Gate B."
4. "Cadence: Approved. Proceed through CP2–CP9 without pausing until Gate B."
5. "Push Branch: Authorized." `step-5` pushed to `origin`.

**Recorded** in the tracker (§2, §7, §11), the decision map (X4 added) and this status.

**CP2 (completed 2026-10-18):** the [dependency policy](technology/dependency-policy.md) (**Proposed**; DP1–DP23) and a living [register](technology/dependency-register.md):
- The core of charter G1 is Axiom's own. The infrastructure layer may be reused behind interfaces Axiom owns. Axiom never writes its own cryptography.
- Dependencies get soundness grades (Google's `ub-risk` scale) and groups after Chromium's (`trusted`, `sandboxed`, `build`, `test`). A reused parser of untrusted input must be pure Rust at `ub-risk-1` or better, or it counts as memory-unsafe code under the Rule of 2.
- The supply chain: `cargo-vet` audits with Firefox's imports, `cargo-deny`, a license allow-list, minimal features, T1–T4.
- **Found:** the repository has no license file, so Axiom's license is unchosen. It is raised for Gate B.

**CP3 (completed 2026-10-18):** [text, fonts and Unicode](architecture/text-and-fonts.md) (**Proposed**; TX1–TX21), and spike S5's design ([tracker](research/step-5-subsystems.md) §10.1):
- ICU4X is the one Unicode source. Encodings are Axiom's own, because Google grades `encoding_rs` at `ub-risk-3`.
- CoreText's database and fallback serve system fonts. HarfRust shapes every font. Web fonts take a Rust-only path, so no C parser sees them. Glyphs are rasterized in the renderer, with CoreText or a Rust rasterizer for system fonts, as S5 decides.
- The dependency policy's DP10 is refined: `ub-risk-2` across the tree in sandboxed helpers, `ub-risk-1` in the browser. Applying CP2's stricter rule would have excluded every Rust font stack while admitting CoreText.

**CP4 (completed 2026-10-18):** [graphics](architecture/graphics.md) (**Proposed**; GR1–GR14), and spike S6's design ([tracker](research/step-5-subsystems.md) §10.2):
- Metal directly, through `platform`, with no portability layer in the browser.
- The window server composites retained CoreAnimation layers, and Axiom draws damaged tiles into IOSurfaces, as Firefox does on macOS, where it measured large power savings.
- Our own pipelines draw the display items. Paths are rasterized in the renderer until S6 reports; if GPU filling pays, an amendment adding path items to PL7 goes to the owner.
- Display P3 at 8 bits by default. Compositor animations: the transform family and opacity, then `filter`, `backdrop-filter` and scroll-driven timelines.

**CP5 (completed 2026-10-18):** [images and media](architecture/images-and-media.md) (**Proposed**; IM1–IM13):
- Images decode in the renderer: safe Rust for PNG, JPEG, GIF, WebP and JPEG XL, and our own BMP and ICO decoders. `zune-jpeg` is graded `ub-risk-3`, so JPEG uses a safe decoder.
- AVIF goes to ImageIO in the renderer, under DP10's exception, accepted by ADR-0012; S5 gains a question (S5-Q5) to test ImageIO in the sandbox.
- Pixels decode at display size and are discarded when not visible. `moxcms` manages color.
- Media runs in a sandboxed utility process, with VideoToolbox, IOSurface frames and audio output.

**CP6 (completed 2026-10-18):** [networking](architecture/networking.md) (**Proposed**; NW1–NW18), and spike S7's design ([tracker](research/step-5-subsystems.md) §10.3):
- Protocols are state machines driven from one wait on the I/O thread: no tokio.
- One URL parser of our own in every process; in the browser, only canonical ASCII URLs from other processes, which settles CP3's IDNA question.
- HTTP/1.1, HTTP/2 and HTTP/3 framing are our own (`httparse` is graded `ub-risk-4`). QUIC is `quinn-proto`. TLS is rustls with `aws-lc-rs`, under DP10's exception, and Security verifies certificates.
- DNS through DNS-SD. Caches, connections, sessions and cookies are partitioned.
- Fetch's policy is enforced from bound endpoints: CORS, ORB, CORP and COEP, mixed content, HSTS, and Local Network Access.

**CP7 (completed 2026-10-18):** [storage and workers](architecture/storage-and-workers.md) (**Proposed**; SW1–SW13):
- All web storage lives in the network process, keyed by origin and top-level site.
- One engine: `redb` if its audit agrees, with SQLite under DP10's exception as the fallback; crash tests decide. The cache and the cookie jar share the engine.
- `localStorage` is cached in the renderer. IndexedDB values arrive serialized, so the network process never parses script values.
- Service workers are intercepted inside Fetch. Shared and service workers run in a renderer locked to their site, launched when none exists.

**CP8 (completed 2026-10-18):** [accessibility](architecture/accessibility.md) (**Proposed**; AX1–AX12):
- Renderers push trees built under HTML-AAM and Core-AAM, once an assistive technology first asks. The browser answers from its cache.
- Text markers, public in SDK 27, carry text navigation, with boundaries computed in the renderer. The bridge is our own.
- Two questions for the owner at Gate B: the settings list, and reserved key equivalents.

**CP9 (completed 2026-10-18):** ADRs [0010](adr/0010-dependency-and-supply-chain-policy.md)–[0014](adr/0014-accessibility.md) Proposed:
- the dependency policy;
- text, fonts and Unicode, pending S5;
- graphics, images and media, pending S6 and S5-Q5;
- networking, storage and workers, pending S7;
- accessibility.

The decision map has I1–I5 Proposed. The open questions gain OQ-21 (Axiom's license), and OQ-7 notes that the graphics design needs no Xcode. **Gate B** is listed under "Next steps".

### 2026-10-17 to 2026-10-18 — Session 7 (Gate B; spike S3; Gate C; S3 addendum; merge)

**Gate B, the owner's decisions** (each as recommended):
1. ADR-0005 (IPC), ADR-0006 (event loop and threading), ADR-0007 (frame tree, pipeline, input) and ADR-0009 (embedder API and pure-Rust shell): **accepted**. OQ-11 and OQ-17 are resolved.
2. ADR-0008 (script, agents, heap): "Pre-approved; will move to Accepted upon S3-Q1 and S3-Q2 passing."
3. **Spike S3** is "Authorized for the full scope (questions S3-Q1 through S3-Q5 under spikes/s3/)". S3-Q2's threshold is "Confirmed at ≤10% of runtime spent rooting".
4. **OQ-19:** "Amend criterion AC5 and rule EL11 to allow read-only scoped lending of DOM arenas to helper threads during borrow-checker-frozen rendering stages. Keep the actual parallel style/layout implementation deferred until profiling benchmarks justify it."
5. The S4 test fix from Session 6 is approved.

**Recorded** in the ADRs' decision histories, the design documents (now binding where their ADR is accepted), AC5 in the tracker, EL11, PL10, the decision map, the open questions, the indexes and this status.

**CP13 (spike S3a, completed 2026-10-18)** ([tracker](research/step-4-architecture-proposal.md) §11):
- **Crates:** `s3-gc` (S1's heap with NaN-boxed values, three root stacks and a barrier hook; its only `unsafe` is 3 blocks in `src/raw/`), `s3-vm` (a register-machine interpreter with callback builtins, four dispatch techniques and a reference evaluator) and `s3-bench` (probes, S1's DOM ported to `s3-gc`, node layouts). Scripts `tools/s3-bench.sh`, `s3-profile.sh` and `s3-ceremony.sh`; `samply-summary.py` gains `--share`.
- **Correctness:** every workload and 34 edge-case programs match the reference evaluator on every technique, stack and pacing. Counted builds show bytecode performs zero root operations. A deliberately unrooted value fails the oracle. Miri finds no undefined behaviour under Stacked and Tree Borrows, on `s3-gc` and on the VM driving C1's stack with a collection at every allocation.
- **Refinements:** a third, safe root stack (`CellStack`), to test whether C1 needs `unsafe` at all; the stack borrowed instead of `Rc`-shared; QoS left at the command-line default, as S1's was, because raising it needs FFI.
- **Preliminary, not formal:** frames and reads meet C1's 1 ns target, but under S1's method a push costs about 2.0 ns on every stack. The method's barrier forces each push to reload the stack length the previous one stored (calibrated: 2.18 ns for any escaped counter; **corrected at CP16:** the cost depends on how the code around the push compiles). A supplementary back-to-back push probe was added. CP14 measures formally.

**CP14 (spike S3b, completed 2026-10-18):** formal runs from the clean tree at `05185cb` ([tracker](research/step-4-architecture-proposal.md) §11).
- **A first run was discarded:** on battery, Low Power Mode slowed every timing about 1.64 times. After the owner plugged in the charger, the full set ran on AC power. The repeat covered S3's suites with counts and nightly tail calls, S1's dispatch and traverse suites for Pattern A, the profiles, and the ceremony count.
- **A tool fix:** `samply-summary.py --share` skipped inlined frames without a source path (`11be3a7`).
- **Headlines:**
  - C1 reads in 0.51 ns, but pushes in 2.03 ns by S1's method (0.56 ns back to back).
  - Rooting is 0.9–1.1% of W3's time with C1's stack.
  - `match` is the fastest dispatch on stable Rust; nightly `become` is 5–22% faster than it.
  - Struct-of-arrays link columns traverse faster than both of S1's patterns.
  - Setters are free without a barrier; card marking adds about 3%.

**CP15 (spike S3c, completed 2026-10-18):** the [S3 results](research/s3-results.md), ADR-0008 updated (still Proposed), script-and-heap (SH7–SH9, SH13), open questions (OQ-14 answered), decision map and indexes. Also: `RawStack`'s lazy commitment verified (an 8 MiB stack with 1,000 roots adds 32 KiB to the footprint), and an unverified comment removed. **Gate C** is listed under "Next steps".

**Before Gate C, the owner asked** whether `RawStack`'s `unsafe` was worth its 94 ns on S1's dispatch case, and shared a second opinion that recommended `CellStack`.
- **The answer was no.** The architecture admits `gc`'s raw module "only if C1 needs it", and `CellStack` meets C1 exactly where `RawStack` does; recommending `RawStack` had given one benchmark too much weight.
- **A probe outside the repository** then found a safe contiguous stack (`SliceStack`) that compiles to `RawStack`'s push. With its length first, it halved S1's push and made dispatch 19% faster (informal).
- **It also showed that CP15's explanation of the 2 ns push was wrong.**

**Gate C, the owner's decisions** (2026-10-18):
1. "S3 Addendum Checkpoint: Authorized."
2. "C1 Implementation: Adopt `SliceStack` (Safe Contiguous Stack)." "The `gc` crate remains strictly `#![forbid(unsafe_code)]`." "Keep `RawStack` archived in spikes/s3/ as benchmark history."
3. "We will formally accept ADR-0008 and merge `step-4` into `main` (with a subsequent push to origin) immediately upon completion of the formal addendum measurements."

**CP16 (the S3 addendum, completed 2026-10-18)** ([tracker](research/step-4-architecture-proposal.md) §11):
- **Code** (`17775c4`):
  - `SliceStack` and its field-order control in `s3-gc`, with no `unsafe`; `RawStack` stays, archived.
  - Every stack in the heap, root-stack and oracle tests: 197 tests pass, and Miri passes on `s3-gc` under both borrow models.
  - The new stacks in the benchmarks and profiles, and a new `s3-probe footprint` command.
- **Formal runs** at `17775c4`, on AC power. The Mac was in use, so absolute times were noisier than CP14's. The comparisons rest on ratios within each run, which reproduce CP14's.
- **Results:**
  - `SliceStack` reads in 0.53 ns and has no `unsafe`.
  - By S1's method a push costs 2.12 ns, as on every stack: **the informal 1.02 ns did not reproduce.** The push's own code is the same in both binaries; the harness's code differs. So S1's method measures the loads and stores around a push as much as the push.
  - **S3-Q1 still fails as written.** The corrected explanation is in the [S3 results](research/s3-results.md) §3.2.
  - In S1's dispatch case `SliceStack` is the fastest stack: in each run, 13–17% faster than `RawStack` and 14–20% faster than its control.
  - Rooting takes 0.8–1.0% of W3, and the counts are unchanged.
- **Documents:**
  - The S3 results, ADR-0008, and ADR-0003's decision history.
  - Script and heap, now binding; the overview, where `gc` has no `unsafe`.
  - The decision map, and the open questions: OQ-5, OQ-14 and OQ-13's design part resolved, and OQ-5's row repaired.
  - The indexes and the spikes README.
- **Disclosed:** S1's suite and the profiles ran with uncommitted Markdown changes in the tree; the code was `17775c4`.

**Gate C passed** (2026-10-18): the measurements were complete, so ADR-0008 is Accepted. **CP17:** `step-4` merged into `main` (`--no-ff`), and `main` pushed, as the owner instructed.

### 2026-10-17 — Session 6 (Step 4 kickoff)

**The owner's decisions:**
1. Push `main`: authorized.
2. ADR-0004's three S4 refinements are confirmed: browser-compiled profile bytecode, the required explicit `(deny dynamic-code-generation)`, and `single-jit` on `renderer-jit`.
3. **Step 4 is authorized** as design work on a new branch `step-4`: "Submit the checkpoint plan and tracker first, and request separate go-ahead for Spike S3's code when its design is ready."

**Done:**
- `main` pushed to `origin` at `da79e69`.
- Branch `step-4` created from `main`.
- The confirmation recorded in ADR-0004's decision history.
- **CP1:** the [Step 4 plan and tracker](research/step-4-architecture-proposal.md): 16 questions, 14 deliverables, the architecture criteria AC1–AC10, 16 checkpoints and three review gates. It proposes adding "HTML parsing and DOM" and "JavaScript engine and bindings" to Step 5, whose list doesn't name them.

**Verified** (SDK 27 headers): `CVDisplayLink` is deprecated since macOS 15, in favour of AppKit's `displayLink(target:selector:)` on `NSView`, `NSWindow` and `NSScreen`. This bears on frame timing in renderers, which have no AppKit (CP2).

**Verification:** every relative link in `docs/` resolves. The four standard checks pass, with the code unchanged.

**Gate A (owner, 2026-10-17):**
- The plan is approved as written: scope, criteria AC1–AC10, the 16 checkpoints.
- Step 5's list in the decision map gains "HTML parsing and DOM" and "JavaScript engine and bindings" (done).
- Cadence: CP2–CP12 run without pausing; Gate B, after CP12, covers ADRs 0005–0009 and S3's code together.
- `step-4` push authorized; pushed to `origin`.

**CLAUDE.md and a link checker** (the owner's request, then "proceed with your recommendation"):
- `CLAUDE.md` at the repository root holds the working instructions: session start, authorization and review gates, communication (every decision presented with a recommendation and rationale), documentation conventions, verification, code rules, git workflow, environment. It stays in the public repository.
- `tools/check-doc-links.py` (standard-library Python) checks every relative Markdown link and anchor in the repository. It is now the fifth check before every commit.

**CP12:** ADRs [0005](adr/0005-ipc-transport-and-interfaces.md)–[0009](adr/0009-embedder-api-and-shell.md) Proposed. Decision map (F4, F5, P1–P4), open questions (OQ-5, 11, 13, 14 and 17 updated; OQ-19 and OQ-20 new) and indexes updated. **Gate B** is listed under "Next steps".

**CP11:** [architecture overview v0](architecture/overview.md) (Proposed): the system on one page, the crate structure and its `unsafe` map, five end-to-end scenarios, the criteria check (AC1–AC10 met), the complete-model table, and staging in which L3 changes only process selection.

**CP10:** [script, agents and heap](architecture/script-and-heap.md) (Proposed): one heap per agent; copies, transfers and shared buffers between agents; script only in tasks and microtasks; the root set; C1–C5 resolved by design, with C1 left to spike S3, **whose design is in the [tracker](research/step-4-architecture-proposal.md) §10** for Gate B.

**CP9:** [embedder API and shell](architecture/embedder-api.md) (Proposed): the shell as an embedder over a message API; commands, events and requests for decisions; hosted presentation, IME and accessibility objects; native browser UI only; a headless embedder for WPT; OQ-11 proposed as pure Rust with objc2, validated first at the shell milestone.

**CP8:** [input and accessibility](architecture/input-and-accessibility.md) (Proposed): input enters at the compositor and is routed by hit testing; gestures latch; scrolling chains across frames in the compositor; focus and capture are browser records; IME from a main-thread mirror (Firefox's model); activation from routed input; accessibility as a browser-side cache.

**CP7:** [invalidation](architecture/invalidation.md) (Proposed): a compiler-checked lifecycle per document; two entry points, with no synchronous flush across agents; reuse at every stage; compositor-driven scrolling and animation with display ports; a test oracle comparing incremental updates with rebuilds from scratch.

**CP6:** [rendering pipeline](architecture/rendering-pipeline.md) (Proposed): five owned stages; immutable fragments; per-frame display lists of validated primitives over property trees; glyphs rasterized in the renderer; compositing by token; one 8-bit or 16-bit string representation; style and layout on the event loop's thread within AC5, with lendable DOM arenas. Amending AC5 for parallel stages is a Gate B question.

**CP5:** [frame tree, navigation and loading](architecture/frame-tree-and-navigation.md) (Proposed): the core thread's authoritative frame tree; skeleton replicas and remote window proxies in renderers; every operation between agents routed through the browser, even within one process, so out-of-process iframes change only process selection; browser-driven navigation and session history; loader and storage endpoints bound at commit; V9 tests specified.

**CP4:** [IPC](architecture/ipc.md) (Proposed): Mach messages as the only transport between processes (OQ-17); endpoints as browser-issued capabilities; an IDL with a safe-Rust generator; `Untrusted<T>` validation after WebKit; synchronous requests only toward more privilege; bounded queues, with a browser that never blocks.

**Housekeeping:** an intermittent test failure (about one run in 75) was traced to an unsound assertion in S4's `this_process_has_a_plausible_footprint`: the kernel's lifetime-maximum footprint can trail the current footprint by a page. The assertion was fixed; afterwards there were no failures in 150 runs.

**CP3:** the [event loop and threading model](architecture/event-loop-and-threading.md) (Proposed): one wait per thread; a browser **core** thread separate from AppKit's main thread; one thread and heap per event loop in renderers; input-first task selection; frames offered by the browser's compositor only to event loops that asked; throttling from Chrome's tested levels, applied per agent cluster.

**CP2:** the [macOS runtime note](research/macos-runtime-facilities.md): run loops, frame timing, QoS, surfaces, IME and accessibility on macOS 27, with browser precedent. The browser must own the frame clock, since `CADisplayLink` comes only from AppKit objects; Mach messages can carry QoS (an input to OQ-17).

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

**The owner's review** (2026-10-17), recorded in ADR-0004's decision history:
- **Guiding principle:** no calendar deadline. No architectural corners are cut and no weaker security boundaries are accepted for convenience; staged features rest on an architecture designed for the complete model from day one. Added to the charter as principle 7.
- **Accepted:**
  - the threat model;
  - levels L0–L3 and checks V1–V8, with **out-of-process iframes mandatory for L3** (new check V9);
  - the JIT policy J1–J7, with global and per-site switches;
  - SI1–SI7 as staged, with SI3 only at L1 and non-sensitive L2;
  - memory targets B1–B7 as initial baselines.
- **ADR-0004 stays Proposed** until S4-Q1 and S4-Q2 pass, then moves to Accepted.
- **Spike S4 authorized** (S4a, S4b, S4c).
- **Git:** `step-3` pushed now; it merges into `main` with `--no-ff` after S4.

**Spike S4** (CP6–CP8; [S4 results](research/s4-results.md)):
- **S4-Q1 and S4-Q2 pass.** A Rust helper built against SDK 27.0 and signed ad hoc with the hardened runtime enters a custom deny-by-default profile by three methods: `sandbox_init_with_parameters`; compile + apply; bytecode compiled by the broker. All 18 probes behave as specified, and the control run allows all 18. A *named* profile kills the helper (SIGKILL, namespace `SANDBOX`).
- **S4-Q3.** With broker-compiled bytecode, a sandboxed helper starts as fast and as small as an unsandboxed one: 2.1 ms, 1.14 MiB. Compiling in the helper costs about 6 ms and 2.8 MiB. Mach round trips are 24% faster than a socket pair (3.75 against 4.92 µs). 1 MiB transfers are comparable once socket buffers are enlarged.
- **S4-Q4.** `(deny default)` does not stop JIT; only an explicit `(deny dynamic-code-generation)` does. The one-region `MAP_JIT` limit applies only with `single-jit`. Ad-hoc signing honours `allow-jit` and `single-jit`.
- **S4-Q5.** CoreText renders pixel-identically with the window server and font server denied, given read access to system fonts and ICU's data. Without either, it degrades silently.
- **Code:** `spikes/s4/sandbox`, `spikes/s4/text`, `tools/s4-sign.sh`, `tools/s4-measure.sh`. The four standard checks pass (115 tests, 5 ignored).

**ADR-0004 Accepted** (2026-10-17), per the owner's instruction, with S4's refinements recorded:
- D2: the browser compiles profiles;
- J3: the explicit JIT deny is required;
- J5: `renderer-jit` carries `single-jit`.

**Merged** `step-3` into `main` with `--no-ff` (CP9).

**Housekeeping.** The named-profile runs left crash reports in `~/Library/Logs/DiagnosticReports`. One is archived with the formal results, and all were removed from that folder. The check is opt-in from now on.

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
