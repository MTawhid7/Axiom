# Requirements and Their Verification

| | |
|---|---|
| **Status** | **Approved** as complete for Phase Zero (2026-10-20, Step 6's Gate C), with the [overview](architecture/overview.md) v2: "The Architecture Overview v2 and Requirements Traceability matrix (docs/requirements.md, RQ1–RQ66) are approved as complete for Phase Zero." Written at Step 6, CP10; under the independent architecture review. **Amended by the review's Round 1** (the owner's dispositions, 2026-10-21; [the review](review/README.md) §5): RQ45 and RQ46 for RV1-01 (CP1) |
| **Question** | Does every requirement trace from its source to the milestone that verifies it, and to the check that does? |
| **Answers** | Part of Step 6 question Q7 ([tracker](research/step-6-standards-budgets-roadmap.md) §3); the tracker's handed-over question 30 (§2.2). Planned since Step 1 |
| **Builds on** | The [charter](charter.md); the accepted ADRs; the [threat model](security/threat-model.md) and [process model](security/process-model.md); the [corpus](corpus.md), [standards](standards.md), [testing](testing.md) and [performance](performance.md) documents; the [roadmap](roadmap.md) and [Milestone 1](milestone-1.md) |
| **Last updated** | 2026-10-21: Round 1's accepted findings |

Each entry RQ1–RQ66 names a requirement, where it comes from, the milestone that first verifies it, and how. Later milestones keep every check green (MS2, MS7). **A requirement with no milestone is a gap;** §8 lists the gaps, and there is none. The milestones are the [roadmap](roadmap.md)'s M1–M9; "every gate" means every milestone's gate.

## 1. Goals and scope

| # | Requirement | Source | Verified at | How |
|---|---|---|---|---|
| RQ1 | Axiom implements its core mechanisms itself: parsing and the DOM, CSS, layout, painting, incremental updates, navigation and loading, security policy, the JavaScript engine | Charter G1; [ADR-0001](adr/0001-scope-and-target.md) R2; DP1 | M1 (parsing to painting, incremental updates); M2 (navigation, policy); M3 (the engine) | The [register](technology/dependency-register.md) holds no third-party engine for them (DP1), reviewed at each gate |
| RQ2 | Useful on a defined and growing set of real sites | Charter G2; [corpus](corpus.md) CO1 | M1 onward, page by page (MS6) | The corpus checks at every gate (TS12; [Milestone 1](milestone-1.md) MA5) |
| RQ3 | The architecture can reach the full modern app web | G2; ADR-0001 R1 | M8, M9 (the applications) | The complete-model checks (AC8, SC9); the applications' corpus checks |
| RQ4 | A security posture adequate for sensitive logins, in verified stages | G3; ADR-0001 R3 | M6 (L3) | V1–V9 pass, with their evidence (threat model §8; TS14) |
| RQ5 | Efficiency with measurable budgets on the target machine | G4 | Every gate | The budgets as benchmarks (PF5, TS13) |
| RQ6 | Decisions from evidence, recorded in ADRs | G5; charter §10 | Every gate | Each decision's ADR, with its decision history |
| RQ7 | The target: a MacBook Air M1 with 8 GB on macOS 27 | Charter §4; ADR-0001 | Every measurement | PF1's conditions; this Mac authoritative (TS17) |
| RQ8 | DRM-protected media is a non-goal | Charter §6; ADR-0001 | Every gate | No EME API exposed (standards §5.3, SB4) |
| RQ9 | Standards first; standards-compliant and approximate behaviour kept apart | Charter principle 5; SC5 | Every gate | Support classes (SB3), the approximations listed (SB5), the claims rule (SB12) |
| RQ10 | No architectural corners cut: stage the implementation, never the architecture | Charter principle 7; RC6 | Every gate | Amendments to the owner (MS12); the overview's interfaces unchanged across levels (AC1) |

## 2. Security

| # | Requirement | Source | Verified at | How |
|---|---|---|---|---|
| RQ11 | L0 never handles credentials | Threat model §8 | M1 | No credential store, no script engine, password forms refused below L2 (MA7) |
| RQ12 | L1: web content parsed and run only in sandboxed renderers; a sandboxed network process; the browser parses no web content; IPC by IR1–IR10; SG1, SG4–SG7, SG9, SG11 | Threat model §8; [process model](security/process-model.md) §4 | M2 | V1's escape suite in each process type; every IPC decoder fuzzed (V2's L1 part) |
| RQ13 | L2: renderers locked to one site; credentials and storage mediated with lock checks; ORB; third-party state partitioned; SG2, SG3, SG10 | Threat model §8 | M5 | V3's lock-enforcement tests; V4's cross-site leak suites |
| RQ14 | L3: out-of-process iframes and V1–V9 | Threat model §8; OQ-18 | M6 | V1–V9 pass, evidence recorded |
| RQ15 | No sensitive logins before isolation is verified | ADR-0001 R3; Gate A's account policy | M5 (test accounts), M6 (the owner's) | The login track (MS8); MA7's refusal below L2 |
| RQ16 | Untrusted input is parsed only where the Rule of 2 allows | Threat model; SC2; DP10 | Every gate | Review; the register's groups (DP9); `unsafe` policy |
| RQ17 | `unsafe` only in the listed modules, each with its invariants | ADR-0002; AC9 | Every commit | `tools/check-unsafe-policy.sh`; Miri on crates with `unsafe` (TS11); V5 at L3 |
| RQ18 | The JIT policy: none before L2, never for sensitive sites, a global switch | [ADR-0004](adr/0004-process-model-and-sandbox.md) J1–J7 | M6 (V7); M8's gate (whether to build one) | V7's tests; MS9 |
| RQ19 | Every dependency justified, classed, graded, audited, and its advisories watched | [ADR-0010](adr/0010-dependency-and-supply-chain-policy.md) DP1–DP23 | Every adoption, from M1 | The register (DP6); `cargo-vet` and `cargo-deny` (DP15, DP16); weekly advisory checks (TS15) |
| RQ20 | Fixes arrive only through rebuilds, on a routine | Threat model §9; DP20 | M1 onward | TS15: weekly checks, rebuilds before next use |
| RQ21 | The checks that depend on macOS run again after every macOS update | TS16 | M1 onward | The OS-dependent suites |

## 3. Architecture

| # | Requirement | Source | Verified at | How |
|---|---|---|---|---|
| RQ22 | No rewrite from L0 to L3 (AC1) | [ADR-0007](adr/0007-frame-tree-pipeline-and-input.md); overview §6; MS4 | M2, M5, M6 | Each level changes process selection only; the interfaces' tests pass unchanged |
| RQ23 | The browser is the authority (AC2) | FT1–FT10 | M2 | Records and routing in the browser; V3 at M5 |
| RQ24 | Idle costs nothing (AC3) | EL1–EL3; PM8 | M1 (one process), M2 (each process) | PF16: no idle wakeups over 30 s |
| RQ25 | The browser never decodes images or parses web fonts (AC4) | PL7–PL9; TX7 | M2 | Display lists validated at the process boundary; decoding only in renderers |
| RQ26 | One single-threaded heap per agent (AC5) | [ADR-0008](adr/0008-script-agents-and-heap.md) SH1 | M3 | The engine's tests; Miri where `unsafe` |
| RQ27 | A deterministic, testable core (AC6) | EL24; EM9; IV14 | M1 | MA8: chunking and repetition independence; the virtual clock |
| RQ28 | Memory is designed and budgeted (AC7) | Each design's memory section; ADR-0019 | Every gate | The budgets (MS7) |
| RQ29 | IPC: Mach, endpoints as capabilities, generated safe decoders, `Untrusted<T>` | [ADR-0005](adr/0005-ipc-transport-and-interfaces.md) | M1 (in-process channels, generated code), M2 (across processes) | Decoders fuzzed (TS9); V2 |
| RQ30 | One wait per thread; a browser core thread; one thread and heap per event loop | [ADR-0006](adr/0006-event-loop-and-threading.md) | M1 (threads), M3 (the HTML event loop) | Idle checks (PF16); ThreadSanitizer when its runtime works (S9-Q2) |
| RQ31 | Five owned stages; a compiler-checked lifecycle; reuse at each stage | ADR-0007 | M1 | IV14's self-oracle (MA8) |
| RQ32 | Pattern B's heap with `SliceStack`; the node arena never compacted | ADR-0003; ADR-0008 (with SH10's amendment) | M1 (the node arena), M3 (the JavaScript heap) | The collector's pause budget (PF15); stable identities (HD33) |
| RQ33 | The shell as an embedder over a message API; a headless embedder; a pure-Rust shell | [ADR-0009](adr/0009-embedder-api-and-shell.md) | M1 | The objc2 classes validated (EM11); the headless embedder runs WPT (TS2) |

## 4. Subsystems

| # | Requirement | Source | Verified at | How |
|---|---|---|---|---|
| RQ34 | Text: ICU4X, HarfRust, CoreText for system fonts, a Rust path for web fonts | [ADR-0011](adr/0011-text-fonts-and-unicode.md) | M1 (system fonts), M4 (web fonts) | Unicode's conformance files (TS6); fuzzed font paths at M4 |
| RQ35 | Graphics: Metal, CoreAnimation layers, damage-limited tiles, Display P3 | [ADR-0012](adr/0012-graphics-images-and-media.md) GR | M1; M9 (the GPU process) | S6's frame budgets (PF13) |
| RQ36 | Images and media: safe decoders in renderers; AVIF and media in the media utility process | ADR-0012 IM | M1 (PNG, JPEG, GIF, WebP), M7 (media, AVIF) | Decoders fuzzed (TS9); the media corpus checks at M7 |
| RQ37 | Networking: own URL parser and framing, rustls with the system verifying, partitioned caches and cookies | [ADR-0013](adr/0013-networking-storage-and-workers.md) NW | M1 (HTTP/1.1, HTTP/2, TLS), M2 (cache, cookies, CORS), M4 (HTTP/3), M5 (ORB) | WPT's `fetch/`, `cookies/`, `cors/` at their milestones |
| RQ38 | Storage and workers behind endpoints | ADR-0013 SW | M3 (Web Storage), M5 (IndexedDB, Cache API), M8 (workers) | WPT's `webstorage/`, `IndexedDB/`, `workers/`, `service-workers/` |
| RQ39 | Accessibility: trees pushed by renderers, the browser's cache, text markers | [ADR-0014](adr/0014-accessibility.md) | M1 (the objc2 classes), M4 (the tree and cache) | The shell's accessibility tests |
| RQ40 | HTML and the DOM: our own parsers, one mutation path, the rest of the DOM to the specifications | [ADR-0015](adr/0015-html-parsing-and-the-dom.md) | M1 (parser, DOM), M4 (Shadow DOM, custom elements, XML) | Tokenizer and tree-construction tests (MA4); WPT `dom/` from M3 |
| RQ41 | Style, layout and paint: our own CSS engine and layout, with `:visited` by partitioned history | [ADR-0016](adr/0016-style-layout-and-paint.md) | M1 (CSS 2.1's model), M4 (flexbox, grid, the rest) | `css/CSS2`'s reference tests (MA4); the CSS directories at M4 |
| RQ42 | The JavaScript engine and bindings, our own | [ADR-0017](adr/0017-javascript-engine-and-bindings.md) | M3; M4 (`Intl`, BigInt); M8 (Temporal) | test262 (TS5) |

## 5. Standards and testing

| # | Requirement | Source | Verified at | How |
|---|---|---|---|---|
| RQ43 | Specifications and suites pinned at each milestone's start | [ADR-0018](adr/0018-corpus-standards-and-testing.md) SB1, SB2 | Every milestone's plan | MA3 for M1 |
| RQ44 | A staged API isn't exposed until it works | SB4 | Every gate | Interface exposure reviewed against the support matrix |
| RQ45 | Completed areas at the shipping median, with no regressions; interim targets over subsets gated by a capability, with the whole directory's target at the named milestone | SB10–SB12 | Every gate | Expectation files (SB8); MA4 for M1, whose `css/CSS2` target is interim until M3 (RV1-01) |
| RQ46 | WPT through wptrunner's `axiom` product: a script-free reference-test executor without script, a test-only WebDriver server with it; reference tests through the offscreen texture | TS2, TS3 | M1 (script-free), M3 (WebDriver) | wptrunner's `axiom` product (MA4), run end to end at M1's second checkpoint (RV1-01) |
| RQ47 | Axiom's own runners for html5lib-tests, test262 and Unicode | TS4–TS6 | M1 (html5lib, Unicode), M3 (test262) | MA4 |
| RQ48 | Differential oracles and self-oracles, under virtual time | TS7, TS8 | M1 | MA8; html5ever's differential fuzzing (MA7) |
| RQ49 | Every parser and IPC decoder fuzzed; sanitizers; Miri | TS9–TS11 | Every milestone, for what it adds | MA7 for M1; S9's results for the tools |
| RQ50 | V1–V9 built as tests | TS14 | M2 (V1, V2 at L1), M5 (V3, V4), M6 (all) | The gate's evidence |
| RQ51 | Corpus pages measured within each site's terms: automated only where permitted, by hand otherwise | CO3–CO6; TS12 | Every gate with corpus targets | Terms and robots read again before each census (CO3) |

## 6. Performance

| # | Requirement | Source | Verified at | How |
|---|---|---|---|---|
| RQ52 | One method for every measurement | [ADR-0019](adr/0019-performance-methodology-and-budgets.md) PF1–PF5 | Every measurement | PF1's conditions, recorded with each figure (MA9) |
| RQ53 | B1: the core browser at most 60 MB, plus tiles at 6.2 bytes per window pixel | PF6; process model §7 | M1 (with B2, one process: MA6), M2 (the browser process alone) | `footprint` after 30 s idle |
| RQ54 | B2: a ready renderer with an empty document at most 10 MB | PF7 | M2 | `footprint` |
| RQ55 | B3: a 50 MB median over the documents and content pages, and every corpus page within its ceiling | PF8; performance §4.1 | M1 (Wikipedia, MDN), M4 (the median, with the content pages), M7–M9 (their pages) | `footprint` after 30 s without input, GPU memory charged (PF4) |
| RQ56 | B4: the network process idle at most 25 MB | PF9 | M2 | `footprint` |
| RQ57 | B5, B6 and B7: 16 renderers, a 1 GB session, a 2 GiB cap per renderer | PF10–PF12 | M2 (B5, B7), M4 (B6) | Process-manager tests; a 10-tab session benchmark |
| RQ58 | Frames, long tasks and the collector's pauses | PF13–PF15 | M1 | MA6 |
| RQ59 | No idle wakeups; hidden pages throttled; energy no higher than Safari's on animating pages | PF16, PF17 | M1 (idle), M2 (hidden pages), M7 (energy against Safari) | `top`'s counters (PF2) |
| RQ60 | The build within T1–T4 | PF18; [ADR-0002](adr/0002-implementation-language.md) | Every gate | `/usr/bin/time -l` |
| RQ61 | Limits and deadlines: hangs, scrolling, IPC, navigation, the parser, storage, images | PF19–PF24 | M1 (scrolling, the parser, images), M2 (hangs, IPC, navigation), M3 (storage quotas) | Each value's test, at its milestone |

## 7. How the work runs

| # | Requirement | Source | Verified at | How |
|---|---|---|---|---|
| RQ62 | Each milestone starts only with the owner's authorization, and stops at its gates | Charter §10; CLAUDE.md; MS10 | Every milestone | Its plan's CP1 and gates |
| RQ63 | Milestones ordered and gated, not dated | OQ-1; MS1 | Every milestone | The roadmap |
| RQ64 | Every commit passes the five checks | CLAUDE.md | Every commit | The checks |
| RQ65 | Status recorded every substantial session; trackers per step and milestone | Charter §10 | Every session | `status.md`; the trackers |
| RQ66 | The corpus changes only with the owner | CO1; OQ-2 | Every gate | The corpus's record of changes |

## 8. Gaps

**None:** every requirement above has a milestone and a check. Two items are deliberately open and placed rather than gaps:
- **WebAssembly** (I6, OQ-3), decided at M8's gate for M9 (roadmap MS9);
- **ThreadSanitizer** for RQ30 and RQ49, until a nightly whose runtime works on macOS 27 is pinned (S9-Q2; MA10).
