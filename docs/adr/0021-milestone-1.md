# ADR-0021: Milestone 1, Static Documents

| | |
|---|---|
| **Status** | **Proposed** (2026-10-20), for the owner at Step 6's Gate C. Accepting it defines Milestone 1; starting it is a separate authorization |
| **Date** | 2026-10-20 |
| **Deciders** | Project owner (only the owner can move an ADR to Accepted) |
| **Related** | [Milestone 1](../milestone-1.md) (MA1–MA12); [the roadmap](../roadmap.md) and [ADR-0020](0020-roadmap-and-security-levels.md); [ADR-0018](0018-corpus-standards-and-testing.md); [ADR-0019](0019-performance-methodology-and-budgets.md); [S9 results](../research/s9-results.md); the [charter](../charter.md) §8 |

## Decision history

| Date | Event |
|---|---|
| 2026-10-20 | Proposed at Step 6, CP10 |

## Question

What exactly is Milestone 1, how is it accepted, and what does starting it require the owner to authorize?

## Requirements and constraints

- **Charter §8:** Phase Zero is complete when, among other things, "the measurable acceptance criteria for Milestone 1 are written"; every milestone's criteria draw on conformance, corpus checks, budgets and security gates.
- **The roadmap's M1** (MS3–MS7): static documents at L0, no script, every interface present, one shape of criteria.
- **The threat model's L0:** "Never credentials".
- **ADR-0018:** pinned baselines (SB1, SB2), the shipping median as a completed area's target (SB11), the test layers (TS1–TS18), the corpus's rules on automation and copies (CO3, CO4, CO12).
- **ADR-0019:** the method (PF1–PF5) and the budgets, with Wikipedia's and MDN's ceilings.
- **S9:** cargo-fuzz with AddressSanitizer works here; the installed nightly's ThreadSanitizer doesn't; wptrunner reproduces wpt.fyi.

## Alternatives considered

- **A. Conformance through WPT's testharness directories,** as later milestones will. Not possible at M1: those tests are script, and M1 has none (MS3).
- **B. Readability judged by eye only.** Quick, but not measurable (RC1).
- **C. Readability measured only.** Measurable, but blind to what a reader notices first. Proposed: both, a measured text-order and overlap check, and the owner's checklist against Safari (MA5).
- **D. Fixed pass rates,** such as 100% of `css/CSS2`. Blind to tests that need what M1 leaves out, and stricter than the engines the web relies on. Proposed instead: the shipping median over the reference tests in scope (SB11).

## Historical context and modern developments

- **WPT keeps html5lib's 1,973 tree-construction tests as `.dat` files** beside the three script wrappers that run them (observed in WPT at `0f26972422`), so a runner without script can read them directly, as parsers' own test harnesses do.
- **No new engine meets the shipping median on `css/CSS2/`'s reference tests yet:** on the 5,848 in M1's scope, Servo passes 93.7% and Ladybird 91.2%, against a median of 97.8% (measured from wpt.fyi's runs, with the corrected counting of the standards baseline §6).

## Analysis

- **Conformance without script** rests on three suites M1 can run: html5lib's tokenizer tests, the tree-construction `.dat` files, and `css/CSS2`'s reference tests, with Unicode's conformance files for text. Each has a number.
- **The corpus checks** combine a measured check, of visible text laid out in order without overlaps, with the owner's judgement on a fixed checklist. They follow the corpus's rules on copies and live loads.
- **The budgets** are B1–B3 and the ceilings, read for one process at L0: the idle process at 95 MB, and the two pages' costs above it.
- **The security criteria** make L0's "never credentials" hold by construction: no script engine linked, no credential store, no password-form submission below L2.

## Evidence and sources

[Milestone 1](../milestone-1.md)'s sources: WPT at `0f26972422`, with its tree-construction data and `css/CSS2`; wpt.fyi's runs of 2026-10-19; html5lib-tests at `c777c408`; S9's results.

## Decision

**Proposed:** adopt [Milestone 1](../milestone-1.md) (MA1–MA12) as the first milestone's definition:
1. **Scope** (MA1, MA2): the static-document engine at L0, with CSS 2.1's visual formatting model without paged media, in one process with every interface of the later levels; no script, cookies, storage, web fonts, SVG, flexbox, grid or separate processes.
2. **The baseline** pinned at its start (MA3).
3. **Acceptance criteria** (MA4–MA8), each a number with its method:
   - **conformance:** tokenizer tests 100%; tree construction at least 96%; `css/CSS2`'s 5,848 reference tests in scope at least 97%, their shipping median rounded down; Unicode's conformance files 100%; no regressions;
   - **the corpus:** Wikipedia and MDN render readably, links navigate, and 10 loads never crash or hang;
   - **budgets:** 95 MB idle; Wikipedia ≤ 100 MB and MDN ≤ 70 MB above it; S6's frame budgets; no task over 50 ms; no idle wakeups; pauses within 4 ms; T1–T4;
   - **security and robustness:** no script engine; no credentials by construction; every parser and decoder fuzzed at least an hour with no open finding; `unsafe` only where allowed; every dependency registered and graded;
   - **determinism:** parsing and painting independent of chunking and repetition.
4. **The gate** measured under PF1, with every criterion's value beside its target (MA9).
5. **Starting it** needs the owner's authorization of the list in MA10.

## Rationale

- **Charter §8** asks for measurable criteria; every criterion in MA4–MA8 is a number with its method.
- **ADR-0001** puts static documents first; the corpus's documents tier is M1's target.
- **The threat model** forbids credentials at L0; MA7 makes that a property of the build.
- **ADR-0018 and ADR-0019** supply the targets and budgets; S9 shows the tools work here, except ThreadSanitizer, which MA10 handles.

## Assumptions

- The pinned WPT revision's reference tests and `.dat` files stay available at that revision.
- Wikipedia's and MDN's terms permit the copies and the few identified live loads, as read on 2026-10-20 (CO3, CO12).
- A Rust nightly with a working ThreadSanitizer exists by M1's start; otherwise MA7 records it as blocked.

## Risks and unresolved issues

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| The milestone is large | High | A long milestone | MA11's checkpoints |
| `css/CSS2`'s median is high, and no new engine meets it yet | High | The conformance gate is missed | An interim target with its reason, for the owner (SB11) |
| ThreadSanitizer stays broken | Medium | Threads checked by review and Miri | MA7 records it; rechecked with each nightly |
| Wikipedia's chrome needs flexbox and grid | High | The page's header looks wrong | MA5 judges the article's content |

## Conditions for revisiting

- The roadmap moves M1's scope (ADR-0020).
- A criterion proves unmeasurable, or a pinned suite changes its format.
- The owner changes the corpus's documents tier.

## Consequences

- **Phase Zero's last criterion is met** once this ADR is accepted: Milestone 1's measurable acceptance criteria are written.
- **Starting Milestone 1** needs the owner's authorization of MA10's list: the branch, the skeleton, the dependencies, the data, the tools and the live loads.
- **`tools/wpt-pass-rates.py`** gains `--manifest` and `--types`, to compute medians over reference tests (MA4).
