# ADR-0020: The Roadmap and Its Security Levels

| | |
|---|---|
| **Status** | **Accepted** (2026-10-20), by the owner at Step 6's Gate C, as the proposed baseline: "The nine-milestone progression (M1–M9) and security levels L0–L3 are approved as the baseline under review." The independent architecture review comes before any milestone starts (decision history). **Amended by the review's Round 1** (2026-10-21): the roadmap's §4.1 (RV1-01, RV1-15); MS3–MS5, §4.1, §7 and §10 (RV1-02, RV1-07); MS5, §4.1, §5 and §7 (RV1-12–RV1-14, RV1-19–RV1-22) |
| **Date** | 2026-10-20 |
| **Deciders** | Project owner (only the owner can move an ADR to Accepted) |
| **Related** | [The roadmap](../roadmap.md) (MS1–MS12); [Milestone 1](../milestone-1.md) and [ADR-0021](0021-milestone-1.md); the [threat model](../security/threat-model.md) §8; the [overview](../architecture/overview.md) §6; [ADR-0001](0001-scope-and-target.md); [ADR-0004](0004-process-model-and-sandbox.md) (J1); [ADR-0018](0018-corpus-standards-and-testing.md); [ADR-0019](0019-performance-methodology-and-budgets.md); [OQ-1](../open-questions.md), [OQ-3](../open-questions.md), [OQ-9](../open-questions.md), [OQ-16](../open-questions.md), [OQ-18](../open-questions.md) |

## Decision history

| Date | Event |
|---|---|
| 2026-10-20 | Proposed at Step 6, CP10 |
| 2026-10-20 | **Accepted** at Step 6's Gate C. The owner: "ADR-0020 (The Roadmap and Its Security Levels): Accepted as proposed baseline. The nine-milestone progression (M1–M9) and security levels L0–L3 are approved as the baseline under review." With it: "We are deferring implementation (Milestone 1 kickoff) until the architecture and roadmap undergo independent, multi-round adversarial review." The [roadmap](../roadmap.md) is binding as that baseline; a change the review brings returns to the owner under MS12 |
| 2026-10-21 | **Amendments accepted** in the owner's dispositions of the review's Round 1 ([the review](../review/README.md) §5.1). For the roadmap's §4.1: RV1-01, "move full 97% CSS2 target and WebDriver runner to Milestone 3 when JavaScript lands"; RV1-15, "Require headless embedder text dumps at M1, crash reporting at M2, and console logging at M3." M1 runs reference tests through a script-free executor with an interim `css/CSS2` target; M2 adds local crash reports; M3 adds the WebDriver server, the full target and the console |
| 2026-10-21 | **Amendments accepted** in the same dispositions: RV1-02, "Formally design the sandboxed GPU/compositor process topology now (Renderer → GPU Process → Browser CALayer tree)." "Build the separate GPU/compositor process at Milestone 2; split Budget B1 accordingly." "Ensure Level 3 (L3) security certification at Milestone 6 is performed on the final multi-process topology." And RV1-07, "Accepted (Recommended Option)": "Run real, sandboxed renderer and network processes from Milestone 1 using the profiles validated in Spikes S4, S5, and S7." "Retain single-process mode strictly for rapid automated testing, fuzzing, and Miri." The roadmap's MS3 and MS4 are amended, M1 runs real sandboxed renderer and network processes, M2 builds the GPU/compositor process, and M9 keeps WebGL and WebGPU, whose process M9's plan places. L3's checks at M6 then run on the final topology |
| 2026-10-21 | **Amendments accepted** in the same dispositions: "RV1-12 to RV1-14 (Corpus Capabilities, WebAssembly/JIT Timing, Site Compatibility): Accepted. Schedule for Milestones 3–5 as outlined in 04-round-2-actions.md." And "RV1-19 to RV1-22 (Timing Side-Channels, Rustls ECH, Passkeys, Product Gaps): Accepted. Correct stale facts and schedule resolutions for Milestones 5–6." WebSockets move to M4, and the content targets name what they exclude (RV1-12); WebAssembly's architecture is decided before M3 and the JIT at M5's gate (RV1-13); the user-agent string at M1's plan (RV1-14); downloads, find in page and zoom at M5, session restore at M6, ECH with DNS over HTTPS at M5's plan, passkeys' hybrid transport before M6, and JPEG's decoder at M1's plan (RV1-20–RV1-22). Decision 4 records the timing |
| 2026-10-21 | **Amendment accepted** in the owner's dispositions of the architecture review's Round 2 ([the review](../review/README.md) §5.5): "All 15 findings (RV2-01 through RV2-15) are Accepted as proposed." RV2-04 and RV2-13: M2 tests the GPU process's death and hang, and a stalled renderer ([roadmap](../roadmap.md) §4.1) |
| 2026-10-21 | **Amendment accepted** in the owner's dispositions of the architecture review's Round 2 ([the review](../review/README.md) §5.5): "All 15 findings (RV2-01 through RV2-15) are Accepted as proposed." RV2-05: M2 builds the GPU process's accounting by page and tests tile reuse across processes with a window attached ([roadmap](../roadmap.md) §4.1) |
| 2026-10-21 | **Amendment accepted** in the owner's dispositions of the architecture review's Round 2 ([the review](../review/README.md) §5.5): "All 15 findings (RV2-01 through RV2-15) are Accepted as proposed." RV2-09: M7 brings live regions in full and `ariaNotify`, for Twitch's chat ([roadmap](../roadmap.md) §4.1) |

## Question

Which milestones lead from a static-document engine at L0 to the sensitive-use gate at L3 and the app web? In which order, each at which security level, against which corpus pages? Where does each decision point fall, and how does an implementation milestone run?

## Requirements and constraints

- **ADR-0001:** milestones begin with static documents; the full modern app web stays reachable; sensitive logins wait for the isolation milestone.
- **The threat model §8:** the levels L0–L3, the checks V1–V9; "Step 6 assigns each level to a milestone".
- **OQ-18** (the owner, 2026-10-17): out-of-process iframes are mandatory before L3.
- **OQ-1:** no deadline and no hourly budget.
- **The account policy** (Gate A, 2026-10-20): the owner's sensitive accounts at L3, test accounts from L2.
- **J1:** no JIT before L2, and only on evidence.
- **The corpus** (CO2): a page is a target at the first milestone with everything it needs, at a level that permits its use.
- **Charter principle 7:** stage the implementation, never the architecture.

## Alternatives considered

- **A. Features first, by corpus tier:** documents, content, media, applications, then the security levels. The corpus's pages would arrive sooner, but the widest attack surfaces (media codecs, GPU, WebAssembly) would exist before the levels whose checks they must meet.
- **B. Security first, with the sensitive-use gate as early as its prerequisites allow,** and the rest after it. Proposed (below).
- **C. The sensitive-use gate last,** after media and applications. Simple to describe, but the owner's accounts would wait longest, and three large surfaces would be retrofitted to L3's checks.
- **D. Script before the sandbox:** script at L0, processes after. Interactive pages sooner, but web script would run in an unsandboxed process.
- **E. Dated milestones.** Excluded by OQ-1.

## Historical context and modern developments

- **Chromium enabled Site Isolation by default in Chrome 67** (2018) and calls it "a major architecture change for Chrome"; **Firefox's Project Fission "has been a massive multi-year project"** (sourced, [roadmap](../roadmap.md) §2). Both retrofitted processes into engines designed for fewer (inference).
- **Ladybird, a new engine, added a compositor process and sandboxes on by default** in 2026, before its first alpha (sourced, roadmap §2).
- **Axiom's interfaces carry processes and frames from L0** (AC1), so its levels change process selection, not interfaces (the overview §6).

## Analysis

- **B keeps security ahead of surface.** Sandboxing at M2, site locks at M5, out-of-process iframes and V1–V9 at M6, then media, applications, the GPU and WebAssembly at L3. Each later milestone re-runs L3's checks for what it adds (MS2), the same work C would do once and late. The owner's own accounts reach the content sites at M6.
- **A delays the levels** behind features, and builds media, GPU and WebAssembly code before the checks that should shape it.
- **D is cheaper only in calendar terms,** which OQ-1 sets aside, and it puts a new script engine's bugs in an unsandboxed process (MS3).
- **Every milestone's criteria take one shape** (MS7), from the standards, testing and performance decisions already accepted, so a milestone's gate is measurable when its plan is written.

## Evidence and sources

The [roadmap](../roadmap.md)'s sources: Chromium's Site Isolation page, Mozilla's Fission post, Ladybird's site and newsletters, and the terms of Google, Instagram and Reddit on accounts, all read on 2026-10-20. Every design's staging section; S8's census of the corpus; S9's results for the test tools.

## Decision

**Accepted** (2026-10-20, as the baseline under review): adopt [the roadmap](../roadmap.md) (MS1–MS12) as binding:
1. **Nine milestones, ordered and gated, never dated:** M1 static documents (L0); M2 the sandboxed engine, M3 script, M4 the modern document (L1); M5 site isolation and logins (L2); M6 the sensitive-use gate, M7 media, M8 applications, M9 the GPU and WebAssembly (L3).
2. **Each milestone has one level,** and keeps it after its gate (MS2). **No script from the web before L1** (MS3). **Every interface from M1** (MS4). **As amended in Round 1** (2026-10-21): M1 runs real, sandboxed renderer and network processes, with single-process mode kept for automated tests, fuzzing and Miri (RV1-07); the GPU/compositor process arrives at M2, so L3 is certified on the final topology (RV1-02).
3. **Corpus targets** by milestone (MS6), **the login track** (MS8): test accounts at M5, made by hand in the owner's name, the owner's own accounts at M6.
4. **Decision points placed** (MS9), each taken at its milestone by the owner: WebAssembly before M9; a JIT not before L2, at M8's gate; passkeys before M6; and Step 5's deferred decisions. **As amended in Round 1** (RV1-13, 2026-10-21): WebAssembly's architecture before M3, with S14; the JIT at M5's gate.
5. **Each milestone runs as a Phase Zero step did** (MS10): the owner's authorization, its own branch, a plan with gates, checkpoints with the checks green, dependencies adopted after grading with the owner's approval, the advisory routine, and a merge on the owner's word.

## Rationale

- **ADR-0001 and the threat model** order the work: documents first, sensitive use last among the security steps, every level before the use it permits.
- **OQ-18** puts out-of-process iframes at M6, the first moment L3 can be claimed.
- **Charter G3 and principle 7:** security in verified stages, without retrofits.
- **G2 and the corpus:** each tier's pages become targets as soon as their capabilities and level exist.

## Assumptions

- The owner is the only human developer (OQ-9).
- The designs' interfaces hold across levels, as AC1 and the designs' staging sections say.
- The sites' terms on accounts and automation stay as read; each census and milestone reads them again (CO3).

## Risks and unresolved issues

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Milestone 1 is large: the whole static pipeline | High | A long first milestone | Checkpoints that each leave the workspace working ([Milestone 1](../milestone-1.md) MA11) |
| Keeping L3 from M6 on costs every later milestone a re-run of V1–V9 for what it adds | Certain | Work at M7–M9 | The checks are tests (TS14), so the re-run is mostly automatic |
| Test accounts meet bot checks or limits | Medium | M5's corpus checks need the owner's hands | Bot checks are passed by hand, never worked around (CO4); the owner chooses the accounts |
| A deferred decision turns out to need an architectural change | Low | An amendment | MS12: raised to the owner, recorded in the ADR it changes |
| The corpus pages need capabilities later than planned (Maps and VS Code with WebAssembly) | Medium | A target moves later | Roadmap §11; moved at a gate, with the owner (MS12) |

## Conditions for revisiting

- The owner changes the corpus, or a page's needs change its milestone.
- A milestone shows an interface can't hold across levels.
- WebAssembly, the JIT or passkeys are decided in a way that moves a milestone.
- The threat model's levels change.

## Consequences

- **Milestone 1** is defined by [ADR-0021](0021-milestone-1.md); its implementation needs the owner's separate authorization.
- **The open questions** gain their milestones: OQ-3 (M8's gate), OQ-7 (M1's plan), OQ-16 (before M6), OQ-18 (M6), OQ-20 (M2), OQ-9 (recorded). **As amended in Round 1:** OQ-3's architecture before M3 (RV1-13), OQ-20 at M1 (RV1-07).
- **Corpus checks for pages measured by hand** need the owner at each gate from M4 on.
- **The overview v2** maps its levels onto these milestones.
