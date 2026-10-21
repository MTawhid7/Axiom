# ADR-0021: Milestone 1, Static Documents

| | |
|---|---|
| **Status** | **Accepted** (2026-10-20), by the owner at Step 6's Gate C, as the specification baseline for M1: "The scope, pinned baselines, and measurable criteria (MA1–MA12) are approved as written." **Starting it (MA10) is on hold** until the independent architecture review is done (decision history). **Amended by the architecture review's Round 1** (2026-10-21): MA1, MA2, MA4, MA7, MA10 and MA11 for RV1-01, RV1-15 and RV1-16; MA1, MA2, MA6, MA7, MA10–MA12 for RV1-02, RV1-07 and RV1-09; MA6, MA7, MA10–MA12 for RV1-11, RV1-14, RV1-17, RV1-18 and RV1-22 |
| **Date** | 2026-10-20 |
| **Deciders** | Project owner (only the owner can move an ADR to Accepted) |
| **Related** | [Milestone 1](../milestone-1.md) (MA1–MA12); [the roadmap](../roadmap.md) and [ADR-0020](0020-roadmap-and-security-levels.md); [ADR-0018](0018-corpus-standards-and-testing.md); [ADR-0019](0019-performance-methodology-and-budgets.md); [S9 results](../research/s9-results.md); the [charter](../charter.md) §8 |

## Decision history

| Date | Event |
|---|---|
| 2026-10-20 | Proposed at Step 6, CP10 |
| 2026-10-20 | **Accepted** at Step 6's Gate C. The owner: "ADR-0021 (Milestone 1, Static Documents): Accepted as the specification baseline for M1. The scope, pinned baselines, and measurable criteria (MA1–MA12) are approved as written." And: "Milestone 1 Implementation (MA10): ON HOLD. Do not create crate skeletons or write production code. The next phase will be conducted on branch architecture-review in a dedicated review cycle." |
| 2026-10-21 | **Amendments accepted** in the owner's dispositions of the architecture review's Round 1 ([the review](../review/README.md) §5.1):<br>• **RV1-01:** "Adopt an interim CSS2 conformance target over the 5,652 script-free tests (shipping browser median on that subset)." "Implement a script-free screenshot harness for Milestone 1 (modeled on wptrunner's executorservo.py); move full 97% CSS2 target and WebDriver runner to Milestone 3 when JavaScript lands." "Run the test harness end-to-end at M1 Checkpoint 2 rather than Checkpoint 10." The shipping median on the 5,652 tests is 97.8% (Chrome 98.2%, Firefox 97.8%, Safari 96.4%), so MA4's target is ≥ 97% of them, recomputed with the tools and checked by an independent count. The executor the finding meant is `ServoLegacyRefTestExecutor` in `executorservolegacy.py`; at the pinned revision `executorservo.py` holds Servo's WebDriver executors (observed 2026-10-21).<br>• **RV1-15:** "Require headless embedder text dumps at M1, crash reporting at M2, and console logging at M3."<br>• **RV1-16:** "Enforce panic = "abort" per binary; enforce checked arithmetic / overflow checks in parsers and IPC validators."<br>MA1, MA2, MA4, MA7, MA10 and MA11 are amended in [Milestone 1](../milestone-1.md), and decision 3 records it. **One choice the dispositions left open** is made: RV1-01's counter-proposal offered the test fonts either on the Rust path or on CoreText with reruns after macOS updates; MA4 takes the Rust path, which TX21 and TS3 already require |
| 2026-10-21 | **Amendments accepted** in the same dispositions:<br>• **RV1-07** ("Accepted (Recommended Option)"): "Run real, sandboxed renderer and network processes from Milestone 1 using the profiles validated in Spikes S4, S5, and S7." "Retain single-process mode strictly for rapid automated testing, fuzzing, and Miri."<br>• **RV1-02:** "Formally design the sandboxed GPU/compositor process topology now (Renderer → GPU Process → Browser CALayer tree)." "Build the separate GPU/compositor process at Milestone 2; split Budget B1 accordingly." "Ensure Level 3 (L3) security certification at Milestone 6 is performed on the final multi-process topology."<br>• **RV1-09:** "Renderers write pixels into shared memory entries; only the compositor owns IOSurfaces; renderer profiles explicitly deny iokit-open."<br>MA1, MA2, MA6, MA7, MA10, MA11 and MA12 are amended in [Milestone 1](../milestone-1.md): real sandboxed helpers from M1, with S4's probes in each and an explicit `iokit-open` denial in renderers; budgets per process; the compositor a browser thread until M2. Decision 1 records it |
| 2026-10-21 | **Amendments accepted** in the same dispositions: RV1-11, "Structure concurrency verification in order: Miri race detection → loom primitives → deterministic schedule exploration on EL24 → TSan via updated nightly or -Z external-clangrt." RV1-17, "Validate local self-signed codesigning persistence via Spike S15." RV1-18, "Split generated WebIDL crates by interface group; measure compile times in Spike S13." And "RV1-12 to RV1-14 (Corpus Capabilities, WebAssembly/JIT Timing, Site Compatibility): Accepted. Schedule for Milestones 3–5 as outlined in 04-round-2-actions.md." "RV1-19 to RV1-22 (Timing Side-Channels, Rustls ECH, Passkeys, Product Gaps): Accepted. Correct stale facts and schedule resolutions for Milestones 5–6." MA7 requires TS11's concurrency checks in order, with ThreadSanitizer by S12's route; MA10 adds `loom`, the nightly chosen with S12, and the stable signing identity if S15 passes; MA6 and MA11 measure style's generated code at the third checkpoint; MA12 adds the signing identity, JPEG's decoder and the user-agent string to the decisions at M1's plan |
| 2026-10-21 | **Spike S12's results** ([S12 results](../research/s12-results.md)): MA7's ThreadSanitizer has a route, a nightly of 2026-10-21 or later with its own runtime, with `-Z external-clangrt` as the fallback; MA10 records it for the nightly M1 pins |
| 2026-10-21 | **Spike S15's results** ([S15 results](../research/s15-results.md)): the stable local identity behaves as ad-hoc signing for S4's probes and the JIT entitlements, and keeps Keychain access across rebuilds; MA10 signs with it, created by the owner at M1's start |

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
- **No new engine meets the shipping median on `css/CSS2/`'s reference tests yet:** on the 5,848 in M1's scope, Servo passes 93.7% and Ladybird 91.2%, against a median of 97.8% (measured from wpt.fyi's runs, with the corrected counting of the standards baseline §6). On the 5,652 that need no script, Servo passes 93.7% and Ladybird 91.4%, against the same median (measured, 2026-10-21, RV1-01).

## Analysis

- **Conformance without script** rests on three suites M1 can run: html5lib's tokenizer tests, the tree-construction `.dat` files, and `css/CSS2`'s reference tests, with Unicode's conformance files for text. Each has a number.
- **The corpus checks** combine a measured check, of visible text laid out in order without overlaps, with the owner's judgement on a fixed checklist. They follow the corpus's rules on copies and live loads.
- **The budgets** are B1–B3 and the ceilings, read for one process at L0: the idle process at 95 MB, and the two pages' costs above it. **As amended in Round 1** (RV1-07): per process, the browser at 85.4 MB with its tiles, a ready renderer at 10 MB, the network process at 25 MB.
- **The security criteria** make L0's "never credentials" hold by construction: no script engine linked, no credential store, no password-form submission below L2.

## Evidence and sources

[Milestone 1](../milestone-1.md)'s sources: WPT at `0f26972422`, with its tree-construction data and `css/CSS2`; wpt.fyi's runs of 2026-10-19; html5lib-tests at `c777c408`; S9's results.

## Decision

**Accepted** (2026-10-20, as the specification baseline; starting it on hold): adopt [Milestone 1](../milestone-1.md) (MA1–MA12) as the first milestone's definition:
1. **Scope** (MA1, MA2): the static-document engine at L0, with CSS 2.1's visual formatting model without paged media, in one process with every interface of the later levels; no script, cookies, storage, web fonts, SVG, flexbox, grid or separate processes. **As amended in Round 1** (RV1-02, RV1-07, RV1-09, 2026-10-21): the browser with real, sandboxed renderer and network processes under S4's, S5's and S7's profiles, single-process mode kept for automated tests, fuzzing and Miri; the compositor as a browser thread behind the GPU/compositor process's interface, which M2 builds; renderers without IOKit.
2. **The baseline** pinned at its start (MA3).
3. **Acceptance criteria** (MA4–MA8), each a number with its method:
   - **conformance:** tokenizer tests 100%; tree construction at least 96%; `css/CSS2`'s 5,848 reference tests in scope at least 97%, their shipping median rounded down; Unicode's conformance files 100%; no regressions. **As amended in Round 1** (RV1-01, 2026-10-21): `css/CSS2`'s target at M1 is interim, at least 97% of the 5,652 tests in scope that need no script (their shipping median, 97.8%, rounded down), run through a script-free executor and checked end to end at the second checkpoint; the full target over all 5,848 applies at M3, with WebDriver;
   - **the corpus:** Wikipedia and MDN render readably, links navigate, and 10 loads never crash or hang;
   - **budgets:** 95 MB idle; Wikipedia ≤ 100 MB and MDN ≤ 70 MB above it; S6's frame budgets; no task over 50 ms; no idle wakeups; pauses within 4 ms; T1–T4;
   - **security and robustness:** no script engine; no credentials by construction; every parser and decoder fuzzed at least an hour with no open finding; `unsafe` only where allowed; every dependency registered and graded. **As amended in Round 1** (RV1-16): every executable aborts on a panic, and the browser's validators, the parsers and the decoders are panic-free with checked arithmetic;
   - **diagnostics,** added in Round 1 (RV1-15): the headless embedder's text dumps of the DOM, computed styles, boxes and fragments, the display list and the network log;
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
- A Rust nightly with a working ThreadSanitizer exists by M1's start; otherwise MA7 records it as blocked. **As amended in Round 1** (RV1-11): TS11's first three concurrency checks don't depend on it, and spike S12 tries an updated nightly and `-Z external-clangrt`.

## Risks and unresolved issues

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| The milestone is large | High | A long milestone | MA11's checkpoints |
| `css/CSS2`'s median is high, and no new engine meets it yet | High | The conformance gate is missed | M1's target is interim, over the tests that need no script (RV1-01); a further interim target needs a reason, for the owner (SB11) |
| ThreadSanitizer stays broken | Medium | `platform`'s threads lose one check | TS11's order: Miri, loom and schedule exploration first (RV1-11); S12's routes; MA7 records it; rechecked with each nightly |
| Wikipedia's chrome needs flexbox and grid | High | The page's header looks wrong | MA5 judges the article's content |

## Conditions for revisiting

- The roadmap moves M1's scope (ADR-0020).
- A criterion proves unmeasurable, or a pinned suite changes its format.
- The owner changes the corpus's documents tier.

## Consequences

- **Phase Zero's last criterion is met** once this ADR is accepted: Milestone 1's measurable acceptance criteria are written.
- **Starting Milestone 1** needs the owner's authorization of MA10's list: the branch, the skeleton, the dependencies, the data, the tools and the live loads.
- **`tools/wpt-pass-rates.py`** gains `--manifest` and `--types`, to compute medians over reference tests (MA4), and, in Round 1, `--tests`, to compute them over a list such as `tools/wpt-script-deps.py --list-free` writes (RV1-01).
