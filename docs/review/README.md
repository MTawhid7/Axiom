# The Architecture Review

| | |
|---|---|
| **Status** | **Living.** Round 1 delivered on 2026-10-21, and **disposed by the owner the same day: all 22 findings accepted** (§5). The accepted changes are being made in checkpoints (§5.3); spikes S10, S12 and S15 are authorized (§5.2) |
| **Question** | How is the Phase Zero baseline reviewed before Milestone 1 starts: in what format, with which severities, how do findings become changes, and when does the review end? |
| **Builds on** | The owner's decision 8 at Step 6's Gate C (2026-10-20): Milestone 1 waits "until the architecture and roadmap undergo independent, multi-round adversarial review"; the recommendation in [status](../status.md) for the review's brief; the findings format the [handover](../handover-phase-0.md) §3 suggests |
| **Last updated** | 2026-10-21 |

## 1. Purpose and scope

**The owner's words** (Gate C, 2026-10-20): "We are deferring implementation (Milestone 1 kickoff) until the architecture and roadmap undergo independent, multi-round adversarial review." The [handover](../handover-phase-0.md) states the bar: implementation begins only when "no significant flaws or gaps remain, and further review yields no meaningful improvements".

**The scope** is the baseline approved at Gate C (`main` at `4fb9ca6`): every ADR, the threat and process models, the designs under `architecture/`, the binding documents of Step 6, and spikes S1–S9. The handover's §11 is the standing agenda.

**Round 1's commission** (the owner, 2026-10-21) added:
- five focus areas: the staging from L0 to L3; text and fonts; the content tier's 150 MB cap; the JavaScript engine and its bindings; ThreadSanitizer;
- four deliverables: a risk matrix, detailed findings with counter-proposals, challenged assumptions, and actions for the next round.

## 2. Rounds

| Round | Date | Documents | Findings | State |
|---|---|---|---|---|
| 1 | 2026-10-21 | [Risk matrix](round-1/01-risk-matrix.md) · [Findings](round-1/02-findings.md) · [Challenged assumptions](round-1/03-challenged-assumptions.md) · [Actions before Round 2](round-1/04-round-2-actions.md) | 22: 2 Critical, 6 High, 10 Medium, 4 Low | All accepted (2026-10-21); changes in progress (§5.3) |

## 3. Findings and severities

**IDs** are `RV<round>-<nn>`, for example RV1-01. They belong to this folder. Spikes the review proposes continue the project's numbering from S9.

**Each finding gives:**
- its severity;
- the milestone it blocks;
- the claim it challenges, quoted with its rule ID;
- the evidence, labelled as the [research conventions](../research/README.md) require and dated;
- the failure mode;
- a counter-proposal;
- the cost of making the change later instead of now;
- the rules it would amend.

**Severities:**

| Severity | Meaning | When it must be settled |
|---|---|---|
| **Critical** | Built as written, a gate can't pass, or a security boundary is weaker than its level claims | Before the milestone it blocks starts |
| **High** | A decision whose later change would rework several crates, or certify a security level again | Before the milestone it blocks starts |
| **Medium** | A gap or an inconsistency whose fix is local, but needed by its milestone | By the milestone it blocks |
| **Low** | A stale fact, a missing decision with an easy default, or a risk to record | By its milestone, or recorded |

**Blocks** names the milestone whose plan shouldn't start until the owner has decided the finding.

## 4. How a round runs

1. **The reviewer reads the baseline** at a fixed commit, and writes findings in §3's format.
2. **The owner decides each finding,** in the disposition log (§5): **accepted**, **rejected** or **deferred**, with a reason; a deferred finding names when it returns.
3. **Accepted changes are made in checkpoints** on `architecture-review`, as Round 1's [actions](round-1/04-round-2-actions.md#5-how-accepted-findings-become-changes) describe:
   - an ADR's change is an amendment, recorded in its decision history with the finding's ID;
   - a spike needs its own authorization.
4. **The next round** verifies the changes, then reviews what the previous round didn't cover in depth.
5. **`architecture-review` merges into `main`** only when the owner says so.

## 5. The disposition log

The owner's decisions, in the owner's words. "Changed" names the checkpoint (§5.3) and the documents that carry an accepted change; `git log --grep 'review-r1-cpN'` finds a checkpoint's commit.

**Round 1's dispositions** (the owner, 2026-10-21) begin: "I have reviewed the Round 1 risk matrix, findings, and recommendations in docs/review/round-1/. Here are my formal dispositions to be recorded in docs/review/README.md §5 and status documents."

**The review's process** (§3–§6): "Approved as written in docs/review/README.md (§3–§6), including finding formats, severities, the disposition log, and the stopping rule."

### 5.1 Findings

| ID | Severity | Blocks | Disposition | The owner's words | Date | Changed |
|---|---|---|---|---|---|---|
| RV1-01 | Critical | M1 | **Accepted** | "Adopt an interim CSS2 conformance target over the 5,652 script-free tests (shipping browser median on that subset)." "Implement a script-free screenshot harness for Milestone 1 (modeled on wptrunner's executorservo.py); move full 97% CSS2 target and WebDriver runner to Milestone 3 when JavaScript lands." "Run the test harness end-to-end at M1 Checkpoint 2 rather than Checkpoint 10." | 2026-10-21 | CP1: [Milestone 1](../milestone-1.md) MA1, MA2, MA4, MA7, MA10, MA11; [testing](../testing.md) TS2, TS3, TS12; [standards](../standards.md) SB10, SB11; [EM9](../architecture/embedder-api.md); the [roadmap](../roadmap.md) §4.1; ADRs 0009, 0018, 0020, 0021; `--list-free` and `--tests` in the WPT tools. The interim target is ≥ 97% of the 5,652 tests, their shipping median (97.8%) rounded down |
| RV1-02 | Critical | M1 | **Accepted** | "Formally design the sandboxed GPU/compositor process topology now (Renderer → GPU Process → Browser CALayer tree)." "Build the separate GPU/compositor process at Milestone 2; split Budget B1 accordingly." "Ensure Level 3 (L3) security certification at Milestone 6 is performed on the final multi-process topology." | 2026-10-21 | CP2: the [process model](../security/process-model.md) §1, §3 note 1, §4, §7 (B1a, B1b), §9, §10 and change record; ADR-0004 D1; [graphics](../architecture/graphics.md) GR1, GR2, GR10–GR14, §7–§9; the [pipeline](../architecture/rendering-pipeline.md) PL7–PL9, PL15, PL16; IN1; EL §4.2; the [overview](../architecture/overview.md) §1, §2, §6; the [roadmap](../roadmap.md) MS3–MS5, §4.1, §7, §10; MA1, MA2, MA6, MA7, MA10–MA12; PF4, PF6, PF13; the threat model's L1, V1 and §5; RQ12, RQ35, RQ53; ADRs 0005–0007, 0012, 0019–0021. Scrolling and animation stay in the browser, on layers; WebGL's and WebGPU's process is M9's decision |
| RV1-03 | High | M1 | **Accepted** | "Adopt the single binding rule: cross-site frames are separate agents with independent event loops and heaps behind remote proxies from Milestone 1." "Correct conflicting staging text in overview.md §6, frame-tree-and-navigation.md §5, and roadmap.md M5." | 2026-10-21 | CP3: [FT10](../architecture/frame-tree-and-navigation.md) and the frame tree's §5; [SH1](../architecture/script-and-heap.md); WB20 and the [bindings](../architecture/bindings.md)' §8; the overview §6; the roadmap's MS5 and §4.1 (M1, M3, M5); TS14's V9 from M3; MA1; ADRs 0007, 0008, 0017. A search for "move to separate agents" and "become their own agents" finds them only in this review |
| RV1-04 | High | M3 | **Accepted** | "Fix the GC end-state before Milestone 3 via an amendment to ADR-0008: generational nursery, allocation-driven incremental marking, parallel marking in pauses, and non-moving tenured arenas with selective evacuation." "Evaluate pause times on a 1M-object heap in Spike S11." | 2026-10-21 | CP6: [SH10](../architecture/script-and-heap.md)'s end-state and ADR-0008's decision 8; EL11's second exception (ADR-0006); JE29; PF15; ADRs 0017, 0019. Concurrent marking's possibility and the barrier's semantics are decided with S11 before M3 |
| RV1-05 | High | M4 | **Accepted** | "Replace the flat static 150 MB cap with a normalized workload (standardized 50-item scroll depth)." "Gate on memory growth slope per 100 items scrolled plus post-GC idle plateau recovery; retain 150 MB as a stretch target. Enforce zero-copy resource sharing." | 2026-10-21 | CP6: the [performance budgets](../performance.md) §4.1 (the content tier on a normalized workload to the 50th item, by a slope and a plateau, 150 MB a stretch target, zero-copy a rule); PF11's B6; the process model's B3 and change record; [OQ-25](../open-questions.md) answered; ADR-0019. Chrome's slope comes from the census addendum |
| RV1-06 | High | M3 | **Accepted** | "Evaluate compact 32-bit heap slots vs. 64-bit NaN-boxed slots in Spike S11 before Milestone 3 freezes the object model." | 2026-10-21 | CP6: [JE9](../architecture/javascript-engine.md), JE10, SH13 and the performance §4.1: the slot width open until S11, before M3; ADRs 0008, 0017, 0019 |
| RV1-07 | High | M1 | **Accepted** (the recommended option) | "Run real, sandboxed renderer and network processes from Milestone 1 using the profiles validated in Spikes S4, S5, and S7." "Retain single-process mode strictly for rapid automated testing, fuzzing, and Miri." | 2026-10-21 | CP2: MS3, MS4, the roadmap §4.1 and §10; the process model §9; IP2; EL §4.11; MA1, MA2, MA6, MA7, MA10–MA12; the overview §6; ADRs 0004, 0005, 0020, 0021 |
| RV1-08 | High | M1, M3 | **Accepted** | "Reclassify WHATWG text encodings, Temporal calendar arithmetic, and BigInt as replaceable details behind clean interfaces (DP2)." "Audit and grade modern encoding_rs (SIMD off) before Milestone 1, and temporal_rs before Milestone 3." | 2026-10-21 | CP4: DP1 (g) and DP2 ([policy](../technology/dependency-policy.md)); the [register](../technology/dependency-register.md); [TX3](../architecture/text-and-fonts.md) and §4; the [engine](../architecture/javascript-engine.md)'s §4, JE13, JE22; ADRs 0010, 0011, 0017; the [audit of `encoding_rs`](../research/encoding-rs-audit.md) 0.8.42, **`ub-risk-3`**, so the encodings stay Axiom's own; [`tools/unsafe-census.py`](../../tools/unsafe-census.py). `temporal_rs` is graded before M3, BigInt's candidate before M4 |
| RV1-09 | Medium | M1 | **Accepted** | "Renderers write pixels into shared memory entries; only the compositor owns IOSurfaces; renderer profiles explicitly deny iokit-open." | 2026-10-21 | CP2: the process model §4.1, §10; IM1; GR10; PL7, PL8; the threat model's V1 and TS14; MA7; ADRs 0004, 0007, 0012 |
| RV1-10 | Medium | M1 | **Accepted** | "Establish explicit rules for CoreText emoji advance metrics and -webkit-font-smoothing. Retain the hybrid CoreText + HarfRust/skrifa architecture; reject the single-rasterizer alternative." | 2026-10-21 | CP5: [TX22–TX24](../architecture/text-and-fonts.md), TX13's calibration metric, TX21's atlases, §9's alternative G (rejected by the owner); SB5; TS16; ADR-0011 |
| RV1-11 | Medium | M1 | **Accepted** | "Structure concurrency verification in order: Miri race detection → loom primitives → deterministic schedule exploration on EL24 → TSan via updated nightly or -Z external-clangrt." | 2026-10-21 | CP5: [TS11](../testing.md)'s ordered checks; MA7, MA10, MA12 and the risk rows; the requirements' §8; ADRs 0018, 0021. S12 tests the routes |
| RV1-12 | Medium | M4 | **Accepted** | "RV1-12 to RV1-14 (Corpus Capabilities, WebAssembly/JIT Timing, Site Compatibility): Accepted. Schedule for Milestones 3–5 as outlined in 04-round-2-actions.md." | 2026-10-21 | CP5: WebSockets to M4, dedicated workers if measured, and targets with their exclusions ([roadmap](../roadmap.md) MS5, §4.1, §5); the census addendum in the [corpus](../corpus.md) §10; ADR-0020 |
| RV1-13 | Medium | M3 | **Accepted** | As RV1-12 | 2026-10-21 | CP5: WebAssembly's architecture before M3 with S14, the JIT at M5's gate (roadmap MS5, §7; [OQ-3](../open-questions.md); I6; JE30, JE31); ADRs 0017, 0020 |
| RV1-14 | Medium | M4 | **Accepted** | As RV1-12 | 2026-10-21 | CP5: the [standards](../standards.md)' §5.4, SB13 (a Safari-compatible user agent with an Axiom token) and SB14 (interventions as data); MA5, MA12; the roadmap §7; ADR-0018 |
| RV1-15 | Medium | M1 | **Accepted** | "Require headless embedder text dumps at M1, crash reporting at M2, and console logging at M3." | 2026-10-21 | CP1: EM9; [JE27](../architecture/javascript-engine.md); MA1, MA11; the roadmap §4.1; ADRs 0009, 0017, 0020, 0021 |
| RV1-16 | Medium | M1 | **Accepted** | "Enforce panic = "abort" per binary; enforce checked arithmetic / overflow checks in parsers and IPC validators." | 2026-10-21 | CP1: [ADR-0002](../adr/0002-implementation-language.md)'s failure semantics; IR10 ([process model](../security/process-model.md) change record); [IP11](../architecture/ipc.md) and ADR-0005; MA7, MA10, MA11; the [overview](../architecture/overview.md) §2 |
| RV1-17 | Medium | M1 | **Accepted** | "Validate local self-signed codesigning persistence via Spike S15." | 2026-10-21 | CP5: ADR-0004 D2 and the process model §10, a stable local identity if S15 passes; MA10, MA12. S15 validates it |
| RV1-18 | Medium | M1, M3 | **Accepted** | "Split generated WebIDL crates by interface group; measure compile times in Spike S13." | 2026-10-21 | CP5: the [overview](../architecture/overview.md) §2's bindings split and build topology; WB22; PF18; MA6, MA11; ADRs 0017, 0019, 0021. S13 is authorized before M3 |
| RV1-19 | Low | M5 | **Accepted** | "RV1-19 to RV1-22 (Timing Side-Channels, Rustls ECH, Passkeys, Product Gaps): Accepted. Correct stale facts and schedule resolutions for Milestones 5–6." | 2026-10-21 | CP5: the [threat model](../security/threat-model.md)'s NG4 and §9; PL9 from M5; TS14's V4; ADR-0007 |
| RV1-20 | Low | M5 | **Accepted** | As RV1-19 | 2026-10-21 | CP5: [networking](../architecture/networking.md)'s open question 4 and its source; the handover's three mentions; the roadmap's M5; ADR-0013 |
| RV1-21 | Low | M6 | **Accepted** | As RV1-19 | 2026-10-21 | CP5: [OQ-16](../open-questions.md) and the threat model §9 add the hybrid transport; the roadmap's M6 and §7 |
| RV1-22 | Low | M6 | **Accepted** | As RV1-19 | 2026-10-21 | CP5: downloads, find in page and zoom at M5, session restore at M6 (roadmap); PDFs in Preview and WebRTC on the [not-planned list](../standards.md#53-not-planned); JPEG's decoder at M1's plan (IM2, the register, MA12); ADRs 0012, 0020 |

**Where the owner's words differ from a counter-proposal, the owner's words govern.** Where a counter-proposal offered a choice the owner's words don't make, the checkpoint records the choice it made and why, for the owner to overrule.

**Corrections found while making the changes:**
- **RV1-01's precedent** (CP1): the process-per-test executor that the finding, and so the owner's words, call `executorservo.py` is `ServoLegacyRefTestExecutor` in `executorservolegacy.py`; at the pinned revision `executorservo.py` holds Servo's WebDriver executors (observed 2026-10-21). The documents name the right file, and the finding carries a note.

### 5.2 Spikes

The owner's authorizations (2026-10-21). Each covers only the questions it names (CLAUDE.md), and the questions are those of the [actions](round-1/04-round-2-actions.md#4-proposed-spikes) §4.

| Spike | When | The owner's words | State |
|---|---|---|---|
| S10 | "Authorized Immediately (Pre-Milestone 1)" | "GPU process IPC hop latency, display list validation, and zero-copy texture sharing." | Authorized |
| S12 | As S10 | "ThreadSanitizer workaround routes on macOS 27 (-Z external-clangrt)." | **Done** (2026-10-21): both routes work ([results](../research/s12-results.md)) |
| S15 | As S10 | "Stable self-signed codesigning persistence across rebuilds." | Authorized |
| S11 | "Authorized Before Milestone 3" | "Spike S11 (GC pauses & 32-bit slot width)" | Authorized, not started |
| S13 | As S11 | "Spike S13 (WebIDL build times)" | Authorized, not started |
| S14 | As S11 | "Spike S14 (WebAssembly & WasmGC)" | Authorized, not started |
| Census addendum, by hand | "Authorized Before Milestone 4" | "Manual census addendum (Chrome memory-infra traces & UseCounters on normalized feeds)." | Authorized, not started; the owner's hand (CO6) |

### 5.3 Checkpoints

**The owner's cadence:** "1. Update docs/review/README.md §5 with these formal dispositions. 2. Apply the accepted documentation changes as clean, atomic checkpoints on architecture-review" (CP1–CP5 below). "3. Execute the authorized Pre-M1 Spikes (S10, S12, S15). 4. Run all five validation checks after each checkpoint. Keep architecture-review pushed to origin."

| Checkpoint | The owner's name | Findings | State |
|---|---|---|---|
| CP1 | "M1 Conformance & Test Harness" | RV1-01, RV1-15, RV1-16 | Done, `docs(review-r1-cp1)` |
| CP2 | "Multi-Process Topology & GPU Process Design" | RV1-02, RV1-07, RV1-09 | Done, `docs(review-r1-cp2)` |
| CP3 | "Frame Tree & Cross-Site Agents" | RV1-03 | Done, `docs(review-r1-cp3)` |
| CP4 | "Developer Scope & Dependencies" | RV1-08, with `encoding_rs` graded | Done, `docs(review-r1-cp4)` |
| CP5 | "Text, Concurrency & Roadmap Updates" | RV1-10, RV1-11, RV1-12–RV1-14, RV1-17–RV1-22 (RV1-15 and RV1-16 are CP1's) | Done, `docs(review-r1-cp5)` |
| CP6 | Added: the collector, slot width and the memory gate | RV1-04, RV1-05, RV1-06 | Done, `docs(review-r1-cp6)` |

**CP6 is added** because the five named checkpoints leave out three accepted findings, and their changes are documents too. Their evidence follows from S11 and the census addendum, which are authorized for later; CP6 records the decisions and the questions those spikes answer.

## 6. The stopping rule

**The cycle ends** when a round raises no Critical or High finding, after the previous round's accepted changes have been made and verified. The owner then decides whether to authorize Milestone 1 (MA10). The owner may end or extend the cycle at any time.

## 7. Round 1's coverage and limits

**Read in full:**
- the handover, the charter, the overview and the status log's current state;
- the threat model and the process model;
- ADRs 0001, 0004 and 0008;
- the designs for script and heap, the JavaScript engine, and text and fonts;
- the performance budgets, Milestone 1 and the roadmap's milestones.

**Read by rule, section or targeted search:**
- the other ADRs, through their decisions and their risk and revisit sections;
- the designs for the event loop, the frame tree, IPC, the rendering pipeline, HTML and the DOM, bindings, graphics, images, and networking;
- the standards, testing and dependency documents, and the corpus;
- the results of S5, S8 and S9;
- the open questions and the decision map.

**Not reviewed in depth,** and given to Round 2 ([actions](round-1/04-round-2-actions.md#6-round-2s-scope) §6): accessibility, storage and workers, style's and layout's algorithms, the embedder API, IPC's details, the requirements' trace, and S1–S4's methods.

**Evidence:**
- **No spike was re-run.** One count is new (RV1-01), made with the [tool](../../tools/wpt-script-deps.py) this round adds, over S9's local WPT checkout.
- **S8's raw records** under `target/` were consulted for RV1-10.
- **External sources** were read on 2026-10-21, through an automated fetch; quotations were checked against the fetched text.

**Independence is limited.** Round 1 was written by Claude (Opus 5.5) at the owner's request, and the baseline was also written with Claude's help (see the ADRs' "recorded by Claude"). A reviewer from another model family, or a person, would add independence that this round can't.
