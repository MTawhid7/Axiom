# ADR-0019: Performance Methodology and Budgets

| | |
|---|---|
| **Status** | **Accepted** (2026-10-20, Step 6's Gate B), with the amendment to the process model's budgets B1–B7: "Accepted as proposed, including the B1-B7 process model amendment". **Amended by the architecture review's Round 1** (2026-10-21): B1 split into B1a and B1b; PF4, PF6, PF13 (RV1-02, RV1-09); PF18 (RV1-18); the content tier's gate, PF11 and PF15 (RV1-04–RV1-06). **Amended by Round 4** (2026-10-22): PF19 and PF21 (RV4-03); PF2, PF5 and PF25–PF30 (RV4-06). **Amended at Milestone 1's first checkpoint** (2026-10-23): PF19's pings only during an active session (decision history) |
| **Date** | 2026-10-20 |
| **Deciders** | Project owner (only the owner can move an ADR to Accepted) |
| **Related** | [Performance budgets](../performance.md) (PF1–PF24); [the process model](../security/process-model.md) §7 and its change record; [ADR-0004](0004-process-model-and-sandbox.md) (D6); [ADR-0002](0002-implementation-language.md) (T1–T4); [S6 results](../research/s6-results.md); [S8 results](../research/s8-results.md); [the corpus](../corpus.md); [ADR-0018](0018-corpus-standards-and-testing.md); decision-map item X3; [OQ-13](../open-questions.md), [OQ-23](../open-questions.md) |

## Decision history

| Date | Event |
|---|---|
| 2026-10-20 | Proposed at Step 6, CP7 |
| 2026-10-20 | **Accepted** at Step 6's Gate B. The owner: "ADR-0019 (Performance Methodology and Budgets): Accepted as proposed, including the B1-B7 process model amendment (B1 tile scaling formula and B3 per-page memory ceilings)." The [process model](../security/process-model.md)'s §7 and change record carry the final B1–B7 |
| 2026-10-20 | **The eight pages measured by hand get their ceilings,** by the accepted rule (half of Chrome's cost, capped by tier), from S8's session by hand, signed in ([S8 results](../research/s8-results.md) §7): Google Search, Instagram and Reddit 150 MB (the content cap); YouTube 300 MB (the media cap); Twitch 200 MB; Discord 210 MB; Figma 300 MB (the applications cap); Prism 245 MB. All thirteen are in the [performance budgets](../performance.md) §4.1. **A condition for revisiting is met:** the content pages' Chrome costs (578–1,260 MB) put their cap at 12–26% of Chrome's cost, so the cap goes to the owner at Gate C |
| 2026-10-20 | **At Step 6's Gate C, the content tier's cap kept.** The owner: "Content Tier Memory Cap (150 MB): Keep the 150 MB ceiling as the baseline hypothesis for review." The ceilings stand as the accepted rule gives them; the architecture review examines the cap ([OQ-25](../open-questions.md)). **Safari's census by hand,** logged out, was recorded at the same gate ([S8 results](../research/s8-results.md) §7.4). It changes no ceiling, since the rule takes Chrome's figures; the ceilings come to 16–54% of Safari's costs |
| 2026-10-21 | **Amendment accepted** in the owner's dispositions of the architecture review's Round 1 ([the review](../review/README.md) §5.1). RV1-02: "Formally design the sandboxed GPU/compositor process topology now (Renderer → GPU Process → Browser CALayer tree)." "Build the separate GPU/compositor process at Milestone 2; split Budget B1 accordingly." "Ensure Level 3 (L3) security certification at Milestone 6 is performed on the final multi-process topology." RV1-09: "Renderers write pixels into shared memory entries; only the compositor owns IOSurfaces; renderer profiles explicitly deny iokit-open." **B1 is split:** B1a, the browser's core at ≤ 60 MB; B1b, the GPU/compositor process at ≤ 15 MB (a hypothesis S10 measures) plus the tiles' 6.2 bytes per window pixel. PF4 counts a renderer's image once; PF13 gains the hop's budget, a hypothesis. The [performance budgets](../performance.md) and the [process model](../security/process-model.md) §7 carry it |
| 2026-10-21 | **Amendment accepted** in the owner's dispositions of the architecture review's Round 1 ([the review](../review/README.md) §5.1), RV1-18: "Split generated WebIDL crates by interface group; measure compile times in Spike S13." PF18: spike S13 measures generated bindings before M3, and style's generated code is measured at M1's third checkpoint |
| 2026-10-21 | **Amendment accepted** in the owner's dispositions of the architecture review's Round 1 ([the review](../review/README.md) §5.1), RV1-05: "Replace the flat static 150 MB cap with a normalized workload (standardized 50-item scroll depth)." "Gate on memory growth slope per 100 items scrolled plus post-GC idle plateau recovery; retain 150 MB as a stretch target. Enforce zero-copy resource sharing." RV1-04: "Fix the GC end-state before Milestone 3 via an amendment to ADR-0008: generational nursery, allocation-driven incremental marking, parallel marking in pauses, and non-moving tenured arenas with selective evacuation." "Evaluate pause times on a 1M-object heap in Spike S11." And RV1-06: "Evaluate compact 32-bit heap slots vs. 64-bit NaN-boxed slots in Spike S11 before Milestone 3 freezes the object model." The [performance budgets](../performance.md) §4.1 gate the content pages on a normalized workload (to the 50th item), by a slope at most half of Chrome's per 100 items and a plateau after a collection, with 150 MB a stretch target and zero-copy resources a rule; PF11's B6 is derived again at M6's plan with out-of-process frames counted; PF15's trigger gives way to the collector's end-state fixed before M3, with S11's pauses; the slot width is measured by S11, not at M8. OQ-25 is answered |
| 2026-10-21 | **Spike S10's results** ([S10 results](../research/s10-results.md)): PF13's hop, from the GPU process's layer commit to the browser's CoreAnimation commit, measured 48.8 µs at the median and 118.7 µs at the 99th percentile, against ≤ 1 ms (§3); B1b's ≤ 15 MB measured 13.4 MB for the GPU process without its tiles, with S6's page kept, a margin smaller than the gap between S10's compact display list and S6's (§7). Both stay as budgets; M2 measures them again with the real process |
| 2026-10-21 | **B1b kept,** the owner: "Budget B1b (15 MB): Approved. Keep the 15 MB baseline ceiling. Milestone 2 will measure display-list memory density per item and introduce per-window scaling if complex DOM trees exceed the margin." The [process model](../security/process-model.md)'s B1, PF6 and the [roadmap](../roadmap.md)'s M2 record it |
| 2026-10-21 | **Amendments accepted** in the owner's dispositions of the architecture review's Round 2 ([the review](../review/README.md) §5.5): "All 15 findings (RV2-01 through RV2-15) are Accepted as proposed." RV2-04: PF19's watchdog covers every helper, a helper silent for 5 s ended and recovered as its death would be. RV2-13: PF21's backlog bound applies per renderer in the network and GPU processes, and to a child's own backlog ([performance](../performance.md)) |
| 2026-10-21 | **Amendment accepted** in the owner's dispositions of the architecture review's Round 2 ([the review](../review/README.md) §5.5): "All 15 findings (RV2-01 through RV2-15) are Accepted as proposed." RV2-05: PF4's GPU-side memory is accounted by page in the GPU process, since S10 found macOS charges tiles to the process that allocates them; `IOSurfaceSetOwnershipIdentity` tested at M2 as the alternative ([performance](../performance.md)) |
| 2026-10-21 | **Amendment accepted** in the owner's dispositions of the architecture review's Round 2 ([the review](../review/README.md) §5.5): "All 15 findings (RV2-01 through RV2-15) are Accepted as proposed." RV2-12: PF24's quota per site covers all of a site's storage keys ([performance](../performance.md)) |
| 2026-10-22 | **Amendment accepted** in the owner's dispositions of the architecture review's Round 4 ([the review](../review/README.md) §5.9): "All 17 findings (RV4-01 through RV4-17) are Accepted as proposed." RV4-03: PF19's deadlines apply to task age, for renderers and for every helper's watched threads; PF21's bound on synchronous requests is the answering thread's deadline ([performance](../performance.md)) |
| 2026-10-22 | **Amendment accepted** in the owner's dispositions of the architecture review's Round 4 ([the review](../review/README.md) §5.9): "All 17 findings (RV4-01 through RV4-17) are Accepted as proposed." RV4-06, the owner: "Adopt the 3-tier validation strategy (commit unit tests, checkpoint instruction/memory ratchet, gate wall/energy budgets) and formalize budgets PF25–PF30 in docs/performance.md." PF2's counters, PF5's three tiers and PF25–PF30 are added ([performance](../performance.md)), and the decision below |
| 2026-10-23 | **Amendment accepted** at Milestone 1's first checkpoint, with [ADR-0023](0023-observability-and-diagnostics.md): "Watchdog vs Idle: Confirm that watchdog pings run only during active window sessions to guarantee zero idle wakeups under PF16." **PF19 is amended** ([performance](../performance.md)): the pings run only during an active session, while some watched process has work, and stop when every watched thread is idle, so that PF16 holds; a helper that leaves idle for work the browser didn't send tells the browser, so no task runs unwatched. M1-CP3 designs and measures it |

## Question

How is performance measured on this Mac, and what must Axiom meet: the final memory budgets B1–B7, which ADR-0004's D6 leaves to Step 6, and the time, energy, build, limit and deadline budgets the designs left open?

## Requirements and constraints

- **Charter G4:** efficiency is a primary goal, "with measurable budgets on the target machine". **§7:** 8 GB of memory; a fanless machine that throttles.
- **ADR-0004, D6:** B1–B7 are initial targets, set finally by Step 6.
- **The owner's B1 revision** (2026-10-20): "budget visible window tile memory separately based on surface pixel dimensions (e.g. 25.2 MB for this Retina window), and maintain a 60 MB budget for the core browser process state."
- **ADR-0002:** the build thresholds T1–T4, accepted 2026-10-17.
- **OQ-13 and SH10:** a stop-the-world collector until corpus measurements demand incremental marking. Step 6 sets the pause budget.
- **The designs' open questions:** hangs, starvation, scrolling, IPC queues, navigation, the parser, caches, strings, the bindings, quotas and images ([performance](../performance.md) §1).
- **RC1 and RC2:** every budget is measurable on this Mac, and traces to evidence or is a labelled hypothesis.

## Alternatives considered

- **A. Absolute budgets from first principles only.** Independent of other engines, but most of Axiom's costs can't be estimated before code exists.
- **B. Budgets equal to a reference engine's measured costs.** Measured, but they copy another design's costs, and the charter makes efficiency a goal of its own.
- **C. Fractions of a reference engine's measured cost, frozen at a date, capped by tier.** Measured and ambitious: half of Chrome Canary 158's cost for each corpus page, capped by tier.
- **D. Leave every budget to its milestone.** No guesses, but no target to design against, and RC1 unmet.

## Historical context and modern developments

- **Chromium's 2018 median renderer** held 42 MB of private memory; the process model's B3 came from it (sourced, process model §7).
- **S8 measured today's corpus in Chrome Canary 158** (measured): each page's steady-state cost above about:blank ran from 145 MB (MDN) to 627 MB (Maps). The GPU process carried much of it on GPU pages, and Wikipedia's article had a document for each of its SVG images.
- **The engines' deadlines differ widely** (sourced): a renderer is unresponsive after 3 s in WebKit, a slow script is stopped after 10 s in Firefox, and Chromium's desktop hang dialog comes after 15 s. Chrome throttles hidden pages' timers to once a second, and to once a minute after five minutes. Firefox gives scroll-blocking listeners 400 ms, and prepaints 3.5 viewport heights.
- **S6 measured Axiom's compositing on this Mac** (measured): tiles at 6.14 bytes per window pixel; a newly exposed row of tiles in 1.3–2.2 ms at the median.

## Analysis

- **C for memory,** because it is measured where B is, and ambitious where B isn't:
  - The caps keep B3's 50 MB median meaningful for typical pages, and B6's 1 GB session intact.
  - The risk is the JavaScript heap, whose layout may cost more than V8's compressed pointers (hypothesis). The ceilings are hypotheses that return to the owner if a milestone misses them.
- **A, with sourced precedent, for limits and deadlines.** Axiom's values sit within the engines' range, or are tied to its own measurements. One example: one prepainted tile row instead of Firefox's 3.5 viewport heights, because S6 measured that drawing a row is cheap.
- **Every budget becomes a test** (PF5), so a budget can't drift unnoticed.

## Evidence and sources

[Performance budgets](../performance.md) §2 and its sources:
- S1, S2, S4, S6 and S8;
- Chromium's, WebKit's and Firefox's constants;
- V8's string thresholds;
- the Long Tasks draft and the Storage Standard;
- MDN on quotas (secondary);
- Mach's limits in the SDK headers.

## Decision

**Accepted:** adopt [the performance budgets](../performance.md) (PF1–PF24) as binding (**as amended in Round 4,** RV4-06: with PF25–PF30, and PF5's validation in three tiers), and **amend the process model's §7** with the final B1–B7:

| # | Budget | Final value | Initial value (2026-10-17) |
|---|---|---|---|
| B1 | Browser process, idle, one window | **≤ 60 MB for the core, plus each visible window's tiles at ≤ 6.2 bytes per window pixel** (25.4 MB at 2,560 × 1,600; 32.1 MB at 2,880 × 1,800), ≤ 7 bytes during updates, the spares freed within 1 s | ≤ 60 MB in all, revised by the owner on 2026-10-20 to exclude tiles |
| B2 | Renderer, sandboxed and ready, empty document | ≤ 10 MB | Unchanged |
| B3 | Renderer, typical page | **≤ 50 MB, as the median over the documents and content tiers' corpus pages; and no corpus page above its ceiling:** half of Chrome Canary 158's measured cost, capped at 100 MB (documents), 150 MB (content), 300 MB (media and maps) and 300 MB (applications). So Wikipedia 100 MB, MDN 70, Maps 300, Three.js 235, VS Code 225; the tier's cap for the other pages until they are measured (measured on 2026-10-20: decision history). **As amended in Round 1** (RV1-05): the content tier's pages are gated on a normalized workload by a slope and a plateau, with 150 MB a stretch target | ≤ 50 MB |
| B4 | Network process, idle | ≤ 25 MB | Unchanged |
| B5 | Renderer soft limit | 16 | Unchanged |
| B6 | Whole browser, typical session | **≤ 1 GB for 10 tabs of document and content pages on about 6 sites;** sessions with media or application pages bounded by each page's ceiling, B5 and discarding under memory pressure | ≤ 1 GB (10 tabs, about 6 sites) |
| B7 | Per-renderer cap | 2 GiB | Unchanged |

**A page's cost** includes the GPU and compositor memory allocated for it, charged to its renderer (PF4). The process model's change record has carried this amendment since the owner accepted it (2026-10-20). **As amended in Round 2** (RV2-05): the GPU process accounts for that memory by page, since macOS charges it to the process that allocates it.

## Rationale

- **G4** asks for budgets on this Mac; PF1–PF5 make every one measurable, and PF5 enforceable.
- **ADR-0004's D6** asks Step 6 for final values. B1's split follows the owner's revision, with S6's measured rate per pixel.
- **B3's ceilings** follow the corpus, which the owner made the standard evaluation set.
- **The deadlines** are bounded by the engines' range and by S6's measurements.

## Assumptions

- S8's figures for Chrome are representative of the pages as they were on 2026-10-20.
- The ceilings are frozen: Chrome's later versions don't move them.
- Pages measured by hand get ceilings from their Chrome figures, half of each and capped by tier, when S8's session by hand measures them.

## Risks and unresolved issues

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Axiom's JavaScript heap costs more than V8's on application pages | Medium | VS Code's ceiling (225 MB) is missed | Measured at the applications milestone; a revision goes to the owner (PF5) |
| The corpus pages grow heavier over time | High | Live ceilings are missed | Ceilings are checked on the milestone's dated measurements, beside a fresh Chrome baseline |
| One tile row of prepaint shows blank rows in fast scrolling | Medium | Visible checkerboarding | Measured at the scrolling milestone; a skate margin while scrolling fast is the fallback (PF20) |
| Values set as hypotheses (deadlines, slices, caches) prove wrong | High | Revisions | Each names its milestone; revisions return to the owner |

## Conditions for revisiting

- A milestone misses a budget for a reason other than an engineering defect.
- The corpus changes.
- This Mac is replaced.
- S8's session by hand shows a corpus page's Chrome cost to differ from its assumed tier cap.

## Consequences

- **The process model's §7 changed** on acceptance, with a change-record entry (2026-10-20).
- **Every milestone's gate** runs its budgets as benchmarks (TS13), under PF1's conditions.
- **The graphics, images and media milestones** charge GPU memory to pages.
- **Milestone 1's budgets** (CP9) come from these: B2, B3's documents ceilings, PF13's frames and PF16's idle rule.
