# The Architecture Review

| | |
|---|---|
| **Status** | **Living.** Round 1 delivered on 2026-10-21, and **disposed by the owner the same day: all 22 findings accepted** (§5). The accepted changes are made in checkpoints (§5.3), and the three spikes authorized before Milestone 1, S10, S12 and S15, are done (§5.2). **The owner approved the results** and **commissioned Round 2** (2026-10-21, §5.4). **Round 2 delivered** (2026-10-21): Round 1's changes verified; 15 findings, no Critical or High, so **the stopping rule is met** for the scope commissioned (§6, §8). **Disposed by the owner the same day: all 15 accepted** (§5.5–§5.7), the changes made and verified in checkpoints CP1–CP6, and `architecture-review` pushed; **an independent external review follows,** which the owner commissions before the merge into `main` |
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
| 1 | 2026-10-21 | [Risk matrix](round-1/01-risk-matrix.md) · [Findings](round-1/02-findings.md) · [Challenged assumptions](round-1/03-challenged-assumptions.md) · [Actions before Round 2](round-1/04-round-2-actions.md) | 22: 2 Critical, 6 High, 10 Medium, 4 Low | All accepted (2026-10-21); changes made (§5.3); the spikes' results and the choices approved (§5.4) |
| 2 | 2026-10-21 | [Risk matrix](round-2/01-risk-matrix.md) · [Verification of Round 1](round-2/02-verification.md) · [Findings](round-2/03-findings.md) · [Actions](round-2/04-actions.md) | 15: 0 Critical, 0 High, 10 Medium, 5 Low | All accepted (2026-10-21, §5.5); changes made and verified (§5.7). The stopping rule is met for the scope commissioned |

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
   - a spike needs its own authorization;
   - **the entry documents change with the rules** (added in Round 2, RV2-01): a checkpoint that changes a rule also updates the [handover](../handover-phase-0.md) and the [decision map](../decision-map.md) wherever they summarize it, and the round's last checkpoint searches them, and every living document, for each finding's old wording, as [Round 2's verification](round-2/02-verification.md#2-method) §2 did, with the phrases RV2-02 found added.
4. **The next round** verifies the changes, then reviews what the previous round didn't cover in depth.
5. **`architecture-review` merges into `main`** only when the owner says so.

## 5. The disposition log

The owner's decisions, in the owner's words. "Changed" names the checkpoint (§5.3 for Round 1, §5.7 for Round 2) and the documents that carry an accepted change; `git log --grep 'review-rR-cpN'` finds a checkpoint's commit, for round R.

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
| S10 | "Authorized Immediately (Pre-Milestone 1)" | "GPU process IPC hop latency, display list validation, and zero-copy texture sharing." | **Done** (2026-10-21): the commit reaches the browser's layer tree in 48.8 µs at the median, within PF13's 1 ms; zero-copy sampling from memory entries works under RV1-09's renderer profile; validation costs about 9.4 ns an item ([results](../research/s10-results.md)). The owner approved the GPU process's rules and its call before its profile (§5.4) |
| S12 | As S10 | "ThreadSanitizer workaround routes on macOS 27 (-Z external-clangrt)." | **Done** (2026-10-21): both routes work ([results](../research/s12-results.md)) |
| S15 | As S10 | "Stable self-signed codesigning persistence across rebuilds." | **Done** (2026-10-21): S4's probes and the JIT entitlements behave as ad hoc, and Keychain access survives a rebuild; privacy permissions by hand at M1 ([results](../research/s15-results.md)) |
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

### 5.4 Decisions on Round 1's results

The owner, 2026-10-21: "The Round 1 remediation and spike results (S10, S12, S15) on architecture-review are approved. Here are my decisions on the open questions:"

| # | The owner's heading | The owner's words | Recorded in |
|---|---|---|---|
| 1 | GPU Process Cache Warm-Up | "Approved. The GPU process may resolve its cache directory via confstr(_CS_DARWIN_USER_CACHE_DIR) immediately before applying its sandbox profile. This avoids widening the sandbox profile with opendirectoryd/dirhelper and avoids the 450 ms shader recompilation penalty on launch/restart. Update ADR-0004 D2 to document this single exception." | ADR-0004 D2 and its history; the [process model](../security/process-model.md) §10 and change record |
| 2 | GPU Process Sandbox Rules | "Confirmed. The 10 rules identified in docs/research/s10-results.md §6.1 are adopted into docs/security/process-model.md §4.1. Incorporate the removal tests into Milestone 2's validation suite to catch silent framework failures." | The process model §4.1 and change record; [testing](../testing.md) TS14's V1; the [roadmap](../roadmap.md)'s M2; ADR-0004's history |
| 3 | Budget B1b (15 MB) | "Approved. Keep the 15 MB baseline ceiling. Milestone 2 will measure display-list memory density per item and introduce per-window scaling if complex DOM trees exceed the margin." | The process model's B1; [performance](../performance.md) PF6; the roadmap's M2; ADR-0019's history |
| 4 | Strategic Choices (WPT fonts, single ordered stream, browser layer animation, B1 split) | "All approved as implemented." | None needed: the documents already carry them (CP1, CP2) |
| 5 | Branch Merge | "Wait on merging architecture-review into main until Round 2 is complete, preserving main as the verified baseline." | This log and the status |
| 6 | Commission Round 2 | "Authorized. Please proceed immediately with executing Round 2 of the architecture review on branch architecture-review": verify "that the CP1–CP6 amendments completely resolve findings RV1-01 through RV1-22 without introducing regressions or contradictions"; "Conduct an in-depth audit of the deferred subsystems: Accessibility (ADR-0014), Storage & Workers (ADR-0013), Style Cascade & Layout Algorithms (LY1–LY32, ST1–ST36), Embedder API (EM1–EM12), and granular IPC protocols (IP13–IP22)"; "Evaluate whether the stopping rule (zero Critical or High findings) is met." | §2; Round 2's documents |

### 5.5 Round 2's dispositions

The owner, 2026-10-21: "I have reviewed the Round 2 findings and actions in docs/review/round-2/." "Before concluding the review phase and merging into main, I will commission one final, independent external review with Gemini to ensure complete cross-model validation." "Here are my formal dispositions and instructions:"

| # | The owner's heading | The owner's words | Recorded in |
|---|---|---|---|
| 1 | Dispose Findings | "All 15 findings (RV2-01 through RV2-15) are Accepted as proposed." | §5.6 |
| 2 | Test Configuration (RV2-03) | "Approved as recommended." "Run WPT conformance suites (MA4's gate) in real, sandboxed processes on host time"; "Run time-dependent and scheduler tests in single-process mode on EL24's virtual clock"; "Run visual reference tests under both configurations at milestone review gates." | CP2 (§5.7) |
| 3 | Web Locks Placement (RV2-04) | "Approved. Arbitrate Web Locks in the browser process (matching Chromium's LockManager pattern) so that helper process crashes/restarts cannot violate mutual exclusion." | CP2 (§5.7) |
| 4 | Checkpoints Execution | "Approved. Please proceed with applying all six checkpoints (CP1–CP6) as outlined in docs/review/round-2/04-actions.md §5" | §5.7 |
| 5 | Post-Checkpoint State | "Run all five validation checks after each commit." "Once CP1–CP6 are verified and committed, push architecture-review to origin." "DO NOT merge into main yet. Notify me when the branch is pushed so I can initiate Gemini's review." | The [status](../status.md) |

### 5.6 Round 2's findings

Every finding: **Accepted** as proposed, 2026-10-21 (§5.5, item 1). Where a counter-proposal offered a choice, the owner's items 2 and 3 make it, or the checkpoint records the choice it made, for the owner to overrule.

| ID | Severity | Blocks | Changed |
|---|---|---|---|
| RV2-01 | Medium | M1 | CP1: the [handover](../handover-phase-0.md), brought up to Round 1 throughout (its status, summary, diagram, ADR table, budgets, roadmap, spikes, trade-offs, deferred decisions, assumptions, risks, open questions and §11's flagged areas); the [decision map](../decision-map.md)'s F2, F3, P7, X1–X4 and Review rows; §4's new step |
| RV2-02 | Low | M1 | CP1: graphics §3 and §9; the pipeline's header; layout §7; OQ-3, OQ-13, OQ-16, OQ-24; performance §9; the status's ADR-0019 row; RQ24, RQ29; the roadmap's summary; script and heap's summary and §4; EL24; EM10, EM12; the engine's §3; images §1; the overview §5.4; the threat model §10; ADR-0009's decision 7 and history. **Found by the sweep and added:** OQ-20's reason, which predates RV1-07, and script and heap's summary of C4, which predates RV1-04 |
| RV2-03 | Medium | M1 | CP2, with item 2's configuration: [testing](../testing.md) TS2, TS8 and the summary; [Milestone 1](../milestone-1.md) MA4, MA8; [EM5](../architecture/embedder-api.md)'s defaults, a column per request, and EM9; EL24; ADRs 0006, 0009, 0018, 0021 |
| RV2-04 | Medium | M1 (M2, M3, M8) | CP2, with item 3's placement: [IR10](../security/process-model.md) by process type; GR13; EM4; PF19's watchdog over every helper; SW3's `sessionStorage` copy, **SW10's Web Locks in the browser**, SW13's tests; IPC's new families; MA7's helper-recovery test; the roadmap's M2; ADRs 0004, 0009, 0013, 0019, 0020, 0021 |
| RV2-05 | Medium | M2 | CP3: [PF4](../performance.md)'s accounting by page in the GPU process; [GR12](../architecture/graphics.md)'s protocol for reusing tiles across processes, and GR8; PL8's note; IPC's question 4 answered; the roadmap's M2; ADRs 0005, 0007, 0012, 0019, 0020 |
| RV2-06 | Medium | M4 | CP5: [AX2](../architecture/accessibility.md)'s bounds in the space of the nearest layer-backed scroll or animation node, and advances per grapheme cluster; AX4's hit-test policy; PL16's note; ADRs 0007, 0014 |
| RV2-07 | Medium | M4 | CP5: AX7's tiers (full by KVO on VoiceOver and Switch Control, basic for other clients, off again after idle, 60 s a hypothesis); AX12; ADR-0014 |
| RV2-08 | Medium | M4 | CP5: AX11's placement of each suite; [TS2](../testing.md)'s Get Computed Role and Label from M4; EM9's note; ADRs 0009, 0014, 0018 |
| RV2-09 | Low | M4 | CP5: AX4's eight web notifications and `ariaNotify`; AX10's `inverted-colors`; live regions in full by M7 (the design's §9; the roadmap's M7); ADRs 0014, 0020 |
| RV2-10 | Medium | M3 | CP4: [SW1](../architecture/storage-and-workers.md)'s key with an ancestor chain bit; SW9, SW10, SW11 and SW13 by the whole key; SI3 (a) and the process model's change record; NW8's note on why the network key has no bit; NW11's check at M2; ADRs 0004, 0013 |
| RV2-11 | Medium | M3 | CP4: the storage design's new §5.5, SW14–SW17 (blob data; **the `blob:` URL store in the browser,** a choice the finding left open, made because the frame tree already relies on the browser's records, a relaunch of the network process mustn't lose them, and Chromium's registry is in its browser process; the user's files read by the browser; tests); SG1, EM5, IP15's one exception, SW11; the frame tree §4; the [standards](../standards.md)' §5.2 row; the core thread's duties; ADRs 0004–0007, 0009, 0013, 0018 |
| RV2-12 | Low | M3 | CP4: the storage design's open question 2 answered; SW3's 5 MiB; SG10; PF24's quota per site over its keys; ADRs 0004, 0019. **Also fixed:** the storage design's §6 still sent locks to the network process, which CP2 moved, and its §7 said the browser holds no web storage, now with the copies it keeps |
| RV2-13 | Medium | M1 (M2) | CP2: IR5; [IP16](../architecture/ipc.md); PF21; SG11; MA7's containment test; the roadmap's M2; ADRs 0004, 0005, 0019, 0021 |
| RV2-14 | Low | M1 | CP2: IP7, IP13, §4.12, §8's question 3; IR6; ADRs 0004, 0005. Question 4 is closed with RV2-05 (CP3). **Also fixed:** a blank line Round 1's CP1 left inside the process model's change record, which split the table so that its Round 1 rows didn't render as one |
| RV2-15 | Low | M4 | CP6: [style](../architecture/style.md)'s complete model (§8) names the four hooks; ST4 (effective zoom), ST12 (the position fallback origin), ST19 (`attr()`'s dependencies), ST25 (anchor positioning's fallbacks as a nested lifecycle under IV1); `zoom` and `attr()` staged at M4; layout §8; three CSS drafts cited; ADR-0016 |

### 5.7 Round 2's checkpoints

| Checkpoint | The owner's name | Findings | State |
|---|---|---|---|
| CP1 | "Update the master Handover report (docs/handover-phase-0.md), decision map, and clean up stale statements" | RV2-01, RV2-02 | Done, `docs(review-r2-cp1)` |
| CP2 | "Process model, helper crashes, IPC never-blocking, and test configs" | RV2-03, RV2-04, RV2-13, RV2-14 | Done, `docs(review-r2-cp2)` |
| CP3 | "GPU process memory accounting and tile reuse protocols" | RV2-05 | Done, `docs(review-r2-cp3)` |
| CP4 | "Storage key ancestor bit, File API design, and storage quotas" | RV2-10, RV2-11, RV2-12 | Done, `docs(review-r2-cp4)` |
| CP5 | "Accessibility geometry, tiered VoiceOver enablement, and test harnesses" | RV2-06–RV2-09 | Done, `docs(review-r2-cp5)` |
| CP6 | "Style & layout hooks for CSS Interop 2026" | RV2-15 | Done, `docs(review-r2-cp6)` |

**Verified before the push** (2026-10-21), as the owner asked: "Once CP1–CP6 are verified and committed, push architecture-review to origin."
- **Each finding's change** was found in the documents its row above names, against the "done when" of [Round 2's actions](round-2/04-actions.md).
- **The round's final sweep** (§4's new step) searched every living document, the handover included, for each finding's old wording, Round 1's and Round 2's. It found and corrected what Round 2's own changes had left: the summaries of PF4's charge and of virtual time, IN9's enablement, and three ADR records that read as current (ADR-0004's option O1, ADR-0005's evidence on signing, ADR-0012's constraints). The remaining hits are records that are meant to stay: the owner's quoted words, session logs, research notes.
- **The five checks** passed before every commit.

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

## 8. Round 2's coverage and limits

**The baseline:** `architecture-review` at `ec7c6b3` (2026-10-21).

**Read in full:** the designs for accessibility, storage and workers, style, layout and paint, the embedder API and IPC; the process model's IPC rules (IR1–IR10); the disposition log, and each document its "Changed" column names.

**Read by rule, section or targeted search:**
- the handover, the decision map and the status, for the verification;
- the event loop (EL1, EL8, EL24), the pipeline (PL15, PL16), graphics (GR8, GR12, GR13), the frame tree's special documents, networking (NW8, NW10, NW11), the threat model (SG1, SG10, SG11), testing (TS2, TS3, TS8), performance (PF4, PF19, PF21, PF24) and Milestone 1 (MA1, MA4, MA8, MA10);
- S10's results;
- every living document, for each Round 1 finding's old wording ([verification](round-2/02-verification.md#2-method) §2).

**Not reviewed,** because the owner's commission didn't name them: the requirements' trace (RQ1–RQ66), the methods of S1–S4, the decision map's couplings beyond staleness, dated facts older than six months, invalidation (IV), HTML and the DOM, the JavaScript engine's details, and networking beyond its keys. A finding there could still be Critical or High; the stopping rule is met for the scope reviewed ([Round 2's actions](round-2/04-actions.md#6-after-the-stopping-rule) §6).

**Evidence:**
- **No spike was run.** The findings rest on the documents, S10's results and primary sources.
- **External sources** were read on 2026-10-21. Every quotation was checked against a copy downloaded with `curl` (Chromium's sources and commit messages, WPT's files at the pinned revision, the CSS drafts, Fetch, Chrome's blog), except one search summary, marked secondary in the findings. Automated fetches return processed text, so no quotation rests on one alone.
- **SDK headers** are those of the Command Line Tools' `MacOSX27.0.sdk`, on this Mac.

**Independence is limited,** as in Round 1: Round 2 was also written by Claude (Opus 5.5), which helped write the baseline and Round 1.
