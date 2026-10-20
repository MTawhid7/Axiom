# ADR-0019: Performance Methodology and Budgets

| | |
|---|---|
| **Status** | **Proposed** (2026-10-20), for the owner at Step 6's Gate B, with an amendment to the process model's budgets B1–B7 |
| **Date** | 2026-10-20 |
| **Deciders** | Project owner (only the owner can move an ADR to Accepted) |
| **Related** | [Performance budgets](../performance.md) (PF1–PF24); [the process model](../security/process-model.md) §7 and its change record; [ADR-0004](0004-process-model-and-sandbox.md) (D6); [ADR-0002](0002-implementation-language.md) (T1–T4); [S6 results](../research/s6-results.md); [S8 results](../research/s8-results.md); [the corpus](../corpus.md); [ADR-0018](0018-corpus-standards-and-testing.md); decision-map item X3; [OQ-13](../open-questions.md), [OQ-23](../open-questions.md) |

## Decision history

| Date | Event |
|---|---|
| 2026-10-20 | Proposed at Step 6, CP7 |

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

**Proposed:** adopt [the performance budgets](../performance.md) (PF1–PF24) as binding, and **amend the process model's §7** with the final B1–B7:

| # | Budget | Final value (proposed) | Initial value (2026-10-17) |
|---|---|---|---|
| B1 | Browser process, idle, one window | **≤ 60 MB for the core, plus each visible window's tiles at ≤ 6.2 bytes per window pixel** (25.4 MB at 2,560 × 1,600; 32.1 MB at 2,880 × 1,800), ≤ 7 bytes during updates, the spares freed within 1 s | ≤ 60 MB in all, revised by the owner on 2026-10-20 to exclude tiles |
| B2 | Renderer, sandboxed and ready, empty document | ≤ 10 MB | Unchanged |
| B3 | Renderer, typical page | **≤ 50 MB, as the median over the documents and content tiers' corpus pages; and no corpus page above its ceiling:** half of Chrome Canary 158's measured cost, capped at 100 MB (documents), 150 MB (content), 300 MB (media and maps) and 300 MB (applications). So Wikipedia 100 MB, MDN 70, Maps 300, Three.js 235, VS Code 225; the tier's cap for the other pages until they are measured | ≤ 50 MB |
| B4 | Network process, idle | ≤ 25 MB | Unchanged |
| B5 | Renderer soft limit | 16 | Unchanged |
| B6 | Whole browser, typical session | **≤ 1 GB for 10 tabs of document and content pages on about 6 sites;** sessions with media or application pages bounded by each page's ceiling, B5 and discarding under memory pressure | ≤ 1 GB (10 tabs, about 6 sites) |
| B7 | Per-renderer cap | 2 GiB | Unchanged |

**A page's cost** includes the GPU and compositor memory allocated for it, charged to its renderer (PF4). The process model's change record gains this amendment when the owner accepts it.

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

- **The process model's §7 changes** on acceptance, with a change-record entry.
- **Every milestone's gate** runs its budgets as benchmarks (TS13), under PF1's conditions.
- **The graphics, images and media milestones** charge GPU memory to pages.
- **Milestone 1's budgets** (CP9) come from these: B2, B3's documents ceilings, PF13's frames and PF16's idle rule.
