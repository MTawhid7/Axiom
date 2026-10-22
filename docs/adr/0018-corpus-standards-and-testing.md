# ADR-0018: The Corpus, the Standards Baseline and Testing

| | |
|---|---|
| **Status** | **Accepted** (2026-10-20, Step 6's Gate B): "Accepted as proposed." The automation boundary confirmed; the not-planned list approved; spike S9 authorized (decision history). **Amended by the architecture review's Round 1** (2026-10-21): TS2, TS3, SB10 and SB11 for RV1-01; TS11, TS14, TS16, SB5, SB13, SB14 and §5.3 for RV1-10, RV1-11, RV1-14, RV1-19 and RV1-22. **Amended by Round 4** (2026-10-22): SB11's independent count covers every missing capability (RV4-01) |
| **Date** | 2026-10-20 |
| **Deciders** | Project owner (only the owner can move an ADR to Accepted) |
| **Related** | [The corpus](../corpus.md) (CO1–CO14); [the standards baseline](../standards.md) (SB1–SB12); [the testing strategy](../testing.md) (TS1–TS18); [S8 results](../research/s8-results.md); decision-map items X1 and X2; [OQ-2](../open-questions.md), [OQ-22](../open-questions.md); the [charter](../charter.md) §8 and principle 5; the [threat model](../security/threat-model.md) §8–§9; [ADR-0009](0009-embedder-api-and-shell.md) (EM9); [ADR-0010](0010-dependency-and-supply-chain-policy.md) |

## Decision history

| Date | Event |
|---|---|
| 2026-10-20 | Proposed at Step 6, CP7 |
| 2026-10-20 | **Accepted** at Step 6's Gate B. The owner: "ADR-0018 (Corpus, Standards Baseline, and Testing Strategy): Accepted as proposed." |
| 2026-10-20 | **The automation boundary** ([corpus](../corpus.md) CO3) confirmed at Gate B: "Automation Boundary: Confirmed. Automating only the 5 permitted pages and reserving the other 8 for manual testing was the correct decision to respect site terms." |
| 2026-10-20 | **The not-planned list** ([standards](../standards.md) §5.3) approved at Gate B: "Not-Planned List: Approved as written in docs/standards.md §5.3 (DRM, plugins, Web SQL, AppCache, WebXR, Web MIDI, etc.)." |
| 2026-10-20 | **Spike S9** ([testing](../testing.md) §9) authorized at Gate B: "Spike S9 (Testing and Fuzzing Tooling): Authorized. Proceed with validating cargo-fuzz, AddressSanitizer/ThreadSanitizer, and wptrunner on this Mac." |
| 2026-10-21 | **Amendment accepted** in the owner's dispositions of the architecture review's Round 1 ([the review](../review/README.md) §5.1), RV1-01: "Implement a script-free screenshot harness for Milestone 1 (modeled on wptrunner's executorservo.py); move full 97% CSS2 target and WebDriver runner to Milestone 3 when JavaScript lands." TS2 now stages two executors: a script-free, process-per-test reference-test executor at M1 and M2, and the WebDriver server from M3. TS3 sends WPT's test fonts down the Rust path from M1. SB11 allows an interim target over a subset gated by a capability, and checks every tool-computed target by an independent count. **A correction to the owner's quotation:** at WPT `0f26972422`, `executorservo.py` holds Servo's WebDriver executors; the process-per-test executor the finding meant is `ServoLegacyRefTestExecutor` in `executorservolegacy.py` (observed 2026-10-21), and the documents name it. Decision 3 records the amendment |
| 2026-10-20 | **A correction to the evidence, not the decision** (Step 6, CP9): the [standards baseline](../standards.md) §6's calibration had left reference and crash tests out, because `tools/wpt-pass-rates.py` read only subtest counts, which wpt.fyi leaves at zero for them. Recomputed as SB10 defines the measure, 42 of the 48 rows changed. `css/CSS2/`'s median is 98.1%, not 98.9%. Ladybird meets the median on `html/syntax/parsing/`, not on `css/CSS2/`, where it has 93.8%. SB10 and SB11 are unchanged |
| 2026-10-21 | **Amendments accepted** in the same dispositions: RV1-11, "Structure concurrency verification in order: Miri race detection → loom primitives → deterministic schedule exploration on EL24 → TSan via updated nightly or -Z external-clangrt." RV1-10, "Establish explicit rules for CoreText emoji advance metrics and -webkit-font-smoothing. Retain the hybrid CoreText + HarfRust/skrifa architecture; reject the single-rasterizer alternative." And "RV1-12 to RV1-14 (Corpus Capabilities, WebAssembly/JIT Timing, Site Compatibility): Accepted. Schedule for Milestones 3–5 as outlined in 04-round-2-actions.md." "RV1-19 to RV1-22 (Timing Side-Channels, Rustls ECH, Passkeys, Product Gaps): Accepted. Correct stale facts and schedule resolutions for Milestones 5–6." **TS11** checks concurrency in that order; **TS16** reruns the smoothing calibration and emoji metrics after macOS updates; **TS14's V4** gains a pixel-stealing test from M5; **SB5** lists `-webkit-font-smoothing` and per-site interventions; the new **§5.4** sets a Safari-compatible user agent with an Axiom token, frozen per milestone (**SB13**), and interventions as data outside the engine (**SB14**); **§5.3** adds in-engine PDF viewing and WebRTC to the not-planned list. The corpus's §10 records the census addendum the owner authorized before M4 |
| 2026-10-21 | **Spike S12's results** ([S12 results](../research/s12-results.md)), evidence for TS11's fourth step: the nightly of 2026-10-21 runs ThreadSanitizer with its own runtime on macOS 27.2, and the April nightly does with `-Z external-clangrt` and the Command Line Tools' runtime; each reports S9's planted race and passes the clean test. TS11 names the route |
| 2026-10-21 | **Amendment accepted** in the owner's dispositions of the architecture review's Round 2 ([the review](../review/README.md) §5.5): "All 15 findings (RV2-01 through RV2-15) are Accepted as proposed." RV2-03, the owner: "Approved as recommended." "Run WPT conformance suites (MA4's gate) in real, sandboxed processes on host time"; "Run time-dependent and scheduler tests in single-process mode on EL24's virtual clock"; "Run visual reference tests under both configurations at milestone review gates." TS2 assigns each kind of test its process configuration, and TS8's virtual clock applies in single-process mode ([testing](../testing.md)) |
| 2026-10-21 | **Amendment accepted** in the owner's dispositions of the architecture review's Round 2 ([the review](../review/README.md) §5.5): "All 15 findings (RV2-01 through RV2-15) are Accepted as proposed." RV2-11: the [standards](../standards.md)' §5.2 gains a row for the File API, staged with script, tested by WPT's `FileAPI/` |
| 2026-10-21 | **Amendment accepted** in the owner's dispositions of the architecture review's Round 2 ([the review](../review/README.md) §5.5): "All 15 findings (RV2-01 through RV2-15) are Accepted as proposed." RV2-08: TS2's WebDriver subset gains Get Computed Role and Get Computed Label at M4 ([testing](../testing.md)) |
| 2026-10-22 | **Amendment accepted** in the owner's dispositions of the architecture review's Round 4 ([the review](../review/README.md) §5.9): "All 17 findings (RV4-01 through RV4-17) are Accepted as proposed." RV4-01: SB11's independent count checks a computed subset against every capability the milestone lacks, not script alone ([standards](../standards.md) SB11) |

## Question

How is Axiom judged? Three parts decided together, because each uses the others:
- **the corpus:** which pages, measured how, within each site's terms;
- **the standards baseline (X1):** which specifications, at which versions, how fully, against which suites and targets;
- **the testing strategy (X2):** which harnesses, oracles, fuzzers and security checks.

## Requirements and constraints

- **Charter §8:** milestone criteria draw on "pass rates on selected WPT directories, html5lib-tests and test262", on target-corpus checks, budgets and security gates. **Principle 5:** standards-compliant and approximate behaviour stay distinct.
- **ADR-0001:** the full modern app web stays reachable; DRM is a non-goal; sensitive logins wait for the isolation milestone.
- **The owner's Gate A decisions** (2026-10-20):
  - S8's two modes;
  - the S8 accounts in a dedicated profile;
  - the owner's own accounts at L3, test accounts from L2;
  - two identifying URL parameters kept out of the repository.
- **SC5:** each subsystem names dated specifications and suites, and leaves the baseline and pass rates to Step 6.
- **EM9:** a headless embedder runs WPT, reference tests, crash tests and fuzzing. **EL24:** a virtual clock for tests.
- **Threat model §8:** the checks V1–V9 gate L3. **§9 and DP20:** fixes arrive only through rebuilds, "Step 6 sets the routine and its cadence".
- **OQ-22:** XSLT deferred.

## Alternatives considered

- **A. Automate every corpus page that works logged out,** as S8's first design had it. Simple, but eight of the thirteen sites' terms forbid automated access, logged out too ([corpus](../corpus.md) §3).
- **B. Pin specifications only at reviewed snapshots** (WHATWG Review Drafts, W3C Recommendations). Stable, but up to six months behind the living standards that engines and WPT follow, and many shipped CSS modules have no Recommendation.
- **C. Fixed pass-rate targets,** such as 90% or 100% of every directory. Simple, but blind to directories whose tests cover one engine's features (`storage/`: Chrome 97.5%, the others 33%).
- **D. A WPT executor speaking the embedder API directly.** No WebDriver server to write, but no reuse of `testdriver.js`'s WebDriver mapping. The original assessment also said it had "no precedent among wptrunner's products"; that was wrong, since wptrunner keeps a process-per-test Servo executor ([testing](../testing.md) §2). **As amended in Round 1** (RV1-01), D's process-per-test form is M1's reference-test executor, until WebDriver arrives with script.
- **E. Rely on hosted services** for fuzzing and continuous integration. Less load on this Mac, but hosted runners can't stand in for macOS 27's sandbox, frameworks or performance.

## Historical context and modern developments

- **Engines test against one shared suite.** WHATWG requires tests in WPT with each normative change. wpt.fyi publishes runs of Chrome, Firefox, Safari, Servo and Ladybird at aligned revisions (sourced, [standards](../standards.md) §2).
- **New engines run WPT through WebDriver.** wptrunner's Ladybird product launches Ladybird's WebDriver server (sourced, [testing](../testing.md) §2).
- **html5lib-tests' tree-construction tests moved into WPT** on 2026-06-26 (sourced).
- **Chromium measures popular sites** with replayed recordings (system-health stories over Web Page Replay archives). For a project without agreements with those sites, their terms forbid recording or automating most of them (sourced, [corpus](../corpus.md) §3, §9).
- **web-features' Baseline** names what the web can rely on: "high" after 30 months of support across the core browsers. Chrome's WebDX use counters name the same features (sourced).

## Analysis

- **The corpus within its terms** (CO3–CO6):
  - Automation loads only the five pages whose terms and robots files permit it, identified and few; the other eight are measured by hand.
  - It costs the owner's time at each census, and measures by hand what can't be instrumented.
  - It keeps the project within every site's published rules.
- **Living standards at commit snapshots** (SB1, SB2) track what WPT tests, and moving all pins together at a milestone's start keeps changes in pass rates attributable.
- **The shipping median as the target** (SB11) is what the web relies on. A new engine can reach it on some areas: Ladybird meets it on `html/syntax/parsing/`, `css/selectors/` and `webidl/`, and comes within 0.1 point of it on `dom/` (measured from wpt.fyi's runs; corrected on 2026-10-20, decision history).
- **A WebDriver server, test-only** (TS2), follows the precedent, maps `testdriver.js` directly, and stays out of the shell's build, so the browser the owner uses has no remote-control surface.
- **This Mac as the authority** (TS17) keeps every platform-dependent result on the target, at the cost of local time for fuzzing and suites.

## Evidence and sources

The three documents' sources:
- the sites' terms and robots files;
- the WHATWG's, W3C's, Ecma's and Unicode's publications;
- web-features, Interop 2026 and wpt.fyi;
- wptrunner, testdriver and test262's documentation;
- cargo-fuzz, libFuzzer and Rust's sanitizers;
- GitHub's runners.

Plus S8's census of five pages ([S8 results](../research/s8-results.md)), and wpt.fyi's runs of 2026-10-19 computed with `tools/wpt-pass-rates.py`.

## Decision

**Accepted:** adopt [the corpus](../corpus.md) (CO1–CO14), [the standards baseline](../standards.md) (SB1–SB12) and [the testing strategy](../testing.md) (TS1–TS18) as binding. In short:
1. **The corpus** is the owner's 13 pages in four tiers. Automation loads only the pages whose sites' terms and robots files permit it, identified in its user agent, at most five times per census, never signed in. The rest are measured by hand, with only process sampling. Accounts follow the owner's Gate A decision. Copies are kept only where terms and licences permit, and never committed.
2. **The standards baseline:**
   - specifications pinned by body;
   - the baseline moved only at a milestone's start;
   - five support classes, and a staged API exposed only when it works;
   - four approximate behaviours, listed with their reasons;
   - priorities from the corpus, then Baseline, then Interop;
   - expectations kept test by test;
   - completed areas targeting the shipping median on their WPT directories, without regressions.
3. **The testing strategy:**
   - eight layers of tests;
   - WPT through a test-only WebDriver server in the headless embedder. **As amended in Round 1** (RV1-01, 2026-10-21): without script, at M1 and M2, through a script-free, process-per-test reference-test executor, with WebDriver from M3 (TS2); and a milestone may set an interim target over a directory's tests that need no script (SB11);
   - Axiom's own runners for html5lib-tests, test262 and Unicode;
   - test-only differential oracles and self-oracles, under virtual time;
   - every parser and IPC decoder fuzzed, with AddressSanitizer, ThreadSanitizer and Miri;
   - V1–V9 built as tests;
   - weekly advisory checks, and the OS-dependent checks after every macOS update;
   - this Mac authoritative.

Spike S9 ([testing](../testing.md) §9) was authorized at Gate B. The hosted runners (TS17) are the owner's choice at Milestone 1.

## Rationale

- **Charter §8** asks for measurable conformance and corpus checks; SB10–SB12 and TS12 make them measurable.
- **Principle 5** asks for approximations kept apart; SB5 lists them, in one place.
- **ADR-0001's reach** to the app web is kept by staging, not by excluding (SB3).
- **The threat model's L3 gate** gets its nine checks as tests (TS14).
- **Respecting the corpus sites' terms** keeps the project's measurements defensible, and the owner's accounts out of automation.

## Assumptions

- wpt.fyi keeps publishing aligned runs, and WPT remains the shared suite.
- The sites' terms read on 2026-10-20 stay as read until the next census, which reads them again (CO3).
- cargo-fuzz with AddressSanitizer works on this Mac's target, as its documentation says; spike S9 would confirm it.

## Risks and unresolved issues

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| A site's terms change and forbid what a census does | Medium | A census breaches terms | Terms and robots files read again before each census (CO3); a robots file that disallows a page stops automation |
| WPT is too large for this Mac | Medium | Gates slow | WPT runs by directory, with run time and memory recorded (TS18); S9-Q3 |
| Fuzzing tools fail on this target | Low | TS9–TS11 can't run | S9 proposed; libFuzzer still gets bug fixes |
| Measurements by hand can't be repeated exactly | High | Noisier baselines for eight pages | Dated, with their spread; budgets for those pages stay hypotheses (CO12) |
| The median is skewed by a directory's engine-specific tests | Medium | A misleading target | The median of three discounts one engine's features; a milestone may set an interim target with its reason (SB11) |

## Conditions for revisiting

- A corpus site's terms or robots file change.
- wpt.fyi stops aligned runs.
- S9 shows the fuzzing tools fail on this target.
- The owner changes the corpus.
- A milestone shows the median targets are unreachable for a reason other than effort.

## Consequences

- **Milestone 1's conformance criteria** (CP9) use SB9–SB11. Its plan pins the snapshots and WPT's revision.
- **The headless embedder needs its WebDriver server** at the first milestone that runs WPT with script, M3; M1 needs only its script-free executor (RV1-01). wptrunner and WPT's Python tools are test dependencies that need the owner's authorization at that milestone.
- **Expectation files live in the repository** beside the code.
- **Every census reads the corpus's terms again,** and the session by hand needs the owner.
- **`cargo-deny`, `cargo-vet` and `cargo-fuzz`** become checks once the owner authorizes them (DP22).
