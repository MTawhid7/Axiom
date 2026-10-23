# Milestone 1: The Implementation Plan

| | |
|---|---|
| **Status** | **Adopted** (2026-10-23) as Milestone 1's first checkpoint, M1-CP1. The owner: "Milestone 1 Plan: Formally adopted as Milestone 1 Checkpoint 1 (M1-CP1). The 15-checkpoint sequence prioritizing the Walking Skeleton at M1-CP4 is approved. Interface notes will be submitted for approval at CP3, CP4, CP5, and CP11 prior to code implementation." With it, ADRs [0022](adr/0022-code-architecture-and-engineering-standards.md) and [0023](adr/0023-observability-and-diagnostics.md) were accepted, MA10 authorized ("Formally authorized. Proceed with establishing the crate skeleton, workspace configurations, pinned test fixtures, and tooling.") and §2's decisions approved ("Approved as written"). Then: "Please proceed immediately with Checkpoint M1-CP2".<br>• **First drafted** after the review cycle ended, as the owner directed: "Produce the implementation plan (docs/milestone-1-plan.md)" ([the review](review/README.md) §5.9, item 12). It was merged into `milestone-1` as "adopt Milestone 1 implementation plan".<br>• **Then revised** at the owner's request for the discipline of "a mature software engineering organization": "Feel free to refine or rewrite it as needed. Once the plan is finalized, we can proceed with implementation in stages." The revision added an engineering foundation (ADR-0022), observability (ADR-0023) and the order the owner chose: "Walking skeleton first (Recommended)" |
| **Question** | How is Milestone 1 built? On what engineering foundation, and in which checkpoints, each with what scope, tests, efficiency checks, observability, gates and exit criterion? Which decisions does its start need, and where do the owner's gates fall? |
| **Builds on** | [Milestone 1](milestone-1.md) (MA1–MA12), which this plan refines and doesn't change; the [roadmap](roadmap.md)'s MS7 and MS10; the [crate architecture](architecture/crates.md) (CR1–CR16); the [engineering standards](technology/engineering-standards.md) (ES1–ES29); [observability](architecture/observability.md) (OB1–OB27); the [performance budgets](performance.md) (PF1–PF30, PF5's three tiers); the [testing strategy](testing.md); the [dependency policy](technology/dependency-policy.md) and [register](technology/dependency-register.md); the architecture review's [Round 4 actions](review/round-4/07-actions.md) §4 |
| **Last updated** | 2026-10-23: adopted as M1-CP1, with §2's decisions approved and two settled by the owner's directions (items 12 and 19). Earlier: 2026-10-22, revised at the owner's request |

**How to read it.**
- §1 says what the owner authorizes to start, and §2 settles the decisions MA12 and the new documents leave to the plan, each with a recommendation.
- §3 says how every checkpoint is built.
- §4 is the checkpoints, M1-CP2 to M1-CP15.
- §5 is the efficiency section MS10 asks of every plan; §6 the risks, each with the checkpoint that retires it; §7 the owner's gates.
- §8 maps the checkpoints to MA11's outline, and §9 is the tracker.

Rule IDs are their documents'. Where this plan says more than they do, it is **inference,** a proposal for the owner. Checkpoint numbers here are Milestone 1's own, written M1-CPn, so that they don't collide with the review's.

---

## 1. What starting Milestone 1 authorizes (MA10, with this plan's additions)

The owner's authorization of MA10's six items, as amended through Round 4. **This revision adds** the items marked, which follow from ADR-0022 and ADR-0023. **Authorized by the owner** (2026-10-23), as put with these additions: "MA10 Authorization: Formally authorized. Proceed with establishing the crate skeleton, workspace configurations, pinned test fixtures, and tooling."

| Item | What it covers |
|---|---|
| **The branch and this plan** | `milestone-1` from `main`, already on `origin`; this document adopted as M1-CP1, with ADRs 0022 and 0023 |
| **The skeleton** | M1's crates as the [crate architecture](architecture/crates.md) §4 lists them, under `crates/`, each created when a checkpoint first needs it. The executables `axiom`, `axiom-renderer`, `axiom-network` and `axiom-headless`, signed by the stable local identity the owner creates (S15); each helper launched under its profile from S4, S5 and S7. RV1-16's release profiles (`panic = "abort"`) and lints, with the [standards](technology/engineering-standards.md)' §15. **Added:** the spikes' own workspace in `spikes/`, their files unmoved and their lock moved with them (CR15, ADR-0022 D12); `tools/crate-map.toml` and `tools/file-size-allowlist.txt` |
| **The repository's tools** (added) | `tools/check.sh` (ES25), `tools/check-dependencies.py` (CR16, DP22's tool), `tools/check-file-size.py` (ES1), `tools/fetch-test-data.sh` (pinned revisions with checksums), `tools/trace-summary.py` and `tools/health-report.py` (OB20, OB21), each printing its usage with `--help`; the `codegen` crate (ES19) and the test crates (crate architecture §4.6); `deny.toml` and `supply-chain/` for `cargo-deny` and `cargo-vet` |
| **The dependencies,** each in the register before use (DP6, DP8) | **Foundational:** `objc2` and its framework crates with `block2`; ICU4X's segmentation, properties and normalization; rustls.<br>**Replaceable:** HarfRust, `read-fonts`, `skrifa`, `unicode-bidi`, `vello_cpu` (graded first), `png`, `gif`, `image-webp`, **JPEG's decoder** (§2), `moxcms`, `aws-lc-rs` (C, under DP10's exception), `miniz_oxide`, `brotli-decompressor`, `ruzstd`, **`smallvec`** (graded; admissible at `ub-risk-3` under DP8 as amended) and **`encoding_rs`** (`sandboxed`, under DP8's conditions).<br>**Test-only:** html5ever and xml5ever (differential references), `loom`, `libfuzzer-sys`, wptrunner and WPT's tools in a virtual environment under `target/`.<br>**No other third-party dependency:** the trace recorder, the errors, the test servers and the generators are Axiom's own |
| **The test data,** pinned (MA3) | WPT at one revision (§2); html5lib-tests at `c777c408`; Unicode 17's conformance files; the corpus copies, dated, never committed (CO12). All fetched into `target/` by `tools/fetch-test-data.sh` |
| **The tools** | `cargo-deny` and `cargo-vet` (DP22); cargo-fuzz 0.13.2 under `target/`; `nightly-2026-10-21` for Miri and ThreadSanitizer (S12) |
| **Live loads** | Wikipedia and MDN, within CO3 and CO4 |

## 2. The decisions this plan settles

Each with a recommendation, for the owner to confirm at M1-CP1. "Moved" marks a decision this revision settles earlier than the draft did. **Approved as written by the owner** (2026-10-23): "Plan Startup Decisions (§2): Approved as written", naming no Xcode for M1, WPT at `0f26972422`, Unicode 17.0 through ICU4X 2.3, `nightly-2026-10-21`, tests on this Mac only, and the three audits at M1-CP2. Items 12 and 19 carry the owner's further directions, quoted in them.

| # | Decision | Recommendation | Settled at |
|---|---|---|---|
| 1 | **ADR-0022,** code architecture and engineering standards | **Accept,** decision by decision (D1–D15) | M1-CP1 |
| 2 | **ADR-0023,** observability and diagnostics | **Accept** (D1–D10) | M1-CP1 |
| 3 | **Xcode** (OQ-7) | **Not for M1.** No design needs it (GR14); `samply` and command buffers' timestamps measure. Revisit if M2's GPU process needs Metal's debugger | M1-CP1 |
| 4 | **WPT's revision** (MA3) | **`0f26972422`** (2026-10-18), which wpt.fyi aligned and the standards calibrated, unless a newer aligned revision is wanted; MA4's subset and floors are recomputed there either way | M1-CP1 |
| 5 | **Unicode's version** | **17.0, through ICU4X 2.3** (SB1, SB2): one version throughout | M1-CP1 |
| 6 | **The nightly** | **`nightly-2026-10-21`,** which S12 validated for ThreadSanitizer with its own runtime | M1-CP1 |
| 7 | **Hosted runners** (TS17) | **This Mac only:** the checks already run here before every commit | M1-CP1 |
| 8 | **The signing identity** (RV1-17) | **Created by the owner** in the login keychain at M1's start; TCC's persistence checked by hand at M1-CP12 | M1-CP1 |
| 9 | **Vendoring** (the dependency policy's alternative E and open question 3, "the first engine milestone") | **Not vendored.** `Cargo.lock`'s checksums already pin each crate's contents, and vendoring would bloat a public repository. Revisit if builds must work offline | M1-CP2 |
| 10 | **The commit tier's time budget** (ES25) | **About 3 minutes for an incremental change,** a hypothesis measured on the new workspace; past it, suites move to the checkpoint tier. **Measured at M1-CP2:** 20–24 s with CP2's three crates, the spikes' tests 10–13 s of it (§9) | M1-CP2 |
| 11 | **A pre-commit hook** (the standards' §16, G) | **Offered, the owner's choice:** it sets `core.hooksPath` in this clone only | M1-CP2 |
| 12 | **`smallvec`'s grade** (moved from M1-CP5) | **Graded** at the version HarfRust, ICU4X and `vello_cpu` lock (1.16.3 compiles 43 `unsafe` blocks, 11 commented). At `ub-risk-3` it is admitted in renderers under DP8's conditions: Axiom's audit names each block's invariant, and a fuzz target reaches them. **The grade also decides IDNA in the browser** (the crate architecture's open question 1): at `ub-risk-3`, NW2's address bar asks the network process for a typed host's ASCII form, an amendment for the owner. **The owner's conditional approval** (2026-10-23): "If smallvec is graded ub-risk-3 during CP2, approve moving the IDNA domain lookup into the network process as an amendment to NW2." **Done** (2026-10-23; [audit](research/smallvec-audit.md)): `ub-risk-3`, so the approval took effect and NW2 is amended | M1-CP2 |
| 13 | **`vello_cpu`'s grade** (moved from M1-CP5) | **Graded:** its root forbids `unsafe`, so the grade rests on its dependencies, `smallvec` among them. **Done** (2026-10-23; [audit](research/vello-cpu-audit.md)): all 18 crates it compiles graded, every one `ub-risk-2` or better but `smallvec`; no unsoundness found | M1-CP2 |
| 14 | **`encoding_rs`'s DP8 conditions** | **The audit at M1-CP2:** each `unsafe` block across its tree, `simdutf8` included (143 compiled blocks in 2026-10-21's census). **The fuzz target at M1-CP5,** through TX3's interface under AddressSanitizer, before any legacy encoding is decoded. The skeleton decodes UTF-8 through the same interface with the standard library. **The audit is done** (2026-10-23; [audit](research/encoding-rs-audit.md) §6): every block holds; `simdutf8` `ub-risk-2` | M1-CP2, M1-CP5 |
| 15 | **QoS overrides on one-way messages** (OQ-20) | **Measured** with the first Mach channels: a one-way message with an override, to a thread waiting in `kevent64` | M1-CP3 |
| 16 | **The objc2 classes** (EM11; OQ-11; moved from the shell's checkpoint) | **Validated with the first window,** before anything depends on the shell, risk first: `NSTextInputClient` and `NSAccessibility` defined through objc2. The Swift fallback stays behind the same API | M1-CP3 |
| 17 | **The IDL's syntax** (IP7: "its syntax is chosen at implementation") | **In its interface note,** for the owner | M1-CP3 |
| 18 | **The diagnostics region's format** (OB10) and **the browser's recorder** (OB16) | **In their interface note,** for the owner | M1-CP3 |
| 19 | **PF19's pings while idle** (observability's open question 5) | **Settled by the owner** (2026-10-23): "Confirm that watchdog pings run only during active window sessions to guarantee zero idle wakeups under PF16." PF19 is amended: the pings run only during an active session, and stop when every watched thread is idle. M1-CP3's note on task ages designs the restart, and PF16 is measured there | M1-CP1; checked at M1-CP3 |
| 20 | **The skeleton's starter set** | **A list of `css/CSS2` reference tests** the thin slice can pass, with its expected pass count, fixed in M1-CP4's interface notes | M1-CP4 |
| 21 | **Surfaces allocated ahead** (GR12) | **A pool sized from S6's slowest frames,** a hypothesis measured at the paint checkpoint | M1-CP9 |
| 22 | **JPEG's decoder** (IM2) | **`zune-jpeg` without its `x86` and `neon` features,** where its root forbids `unsafe`, if it matches `jpeg-decoder`'s pixels and isn't slower on the corpus's images; `jpeg-decoder` with `platform_independent` otherwise. Neither has `unsafe` in that configuration; `zune-jpeg` is maintained, `jpeg-decoder` dormant since 2025-06-21 | M1-CP10 |
| 23 | **AWS-LC's jitter entropy** | **Keep AWS-LC's default** unless its 29–43 ms first-use cost misses PF25's helper start-up, measured in the network process | M1-CP11 |
| 24 | **The user-agent string** (SB13) | **Safari-compatible with an Axiom token,** from the Safari shipping at M1's start | M1-CP11, before live loads |

## 3. How every checkpoint is built

- **The frame.** ADR-0022's documents govern the code:
  - every crate in the crate map, with its allowed dependencies and processes, checked on every commit (CR2, CR4, CR16);
  - the standards for size, types, errors, `unsafe`, determinism, documentation and tests (ES1–ES24).

  ADR-0023's governs what the code records (OB1–OB27).
- **Interface notes gate their code** (CR8, ES27; the owner's choice, "Interfaces only"). Each checkpoint that defines an architectural interface opens with its notes, in `docs/architecture/interfaces/`, put to the owner as one gate:
  - **M1-CP3:** the IDL's syntax and the generated API; `ipc`'s channels and `Untrusted<T>`; `event-loop`'s tasks and task ages; `trace`'s API, and the diagnostics region's format and protocol;
  - **M1-CP4:** navigation, commit and the loader's stream; the display-list format; the XML sink; what `style` and `layout` read from the DOM; computed styles; the lifecycle's tokens; `text`'s results; fragments; the embedder API's first version; the starter set;
  - **M1-CP5:** the HTML tree sink;
  - **M1-CP11:** the loader's protocol over HTTP;
  - **any later change** to an approved note.

  A crate's internal design is its charter (ES15), committed before its code and reviewed by the owner asynchronously.
- **Commits.** A checkpoint is a series of commits, each passing the commit tier of `tools/check.sh`, named `<scope>(m1-cpK): …` (ES25, ES26). Its last commit updates the tracker (§9), after the checkpoint tier has passed.
- **The walking skeleton** (the owner's choice). After the infrastructure (M1-CP2, M1-CP3), **M1-CP4 builds the thinnest slice through every stage and process**, so the interfaces between crates are exercised together first, then each later checkpoint widens one stage to M1's scope. Thin code carries `TODO(M1-CPn, …)` naming the checkpoint that replaces it (ES17), so none survives unnoticed.
- **From M1-CP4, every checkpoint** runs PF5's ratchet against `perf/baseline.toml`. It also runs the `css/CSS2` selection through wptrunner, recording the pass count and each directory's in the tracker, so MA4's progress shows throughout and not only at M1-CP13.
- **A checkpoint is done** when ES29 says: its exit criterion met, both tiers green, its notes approved, its dependencies registered, its TODOs listed and its measurements in the tracker.

## 4. The checkpoints

Each leaves the workspace clean and working (MS10). **Gate** lists what the owner approves before the checkpoint's code; **done when** is its exit criterion.

### M1-CP2. The engineering foundation (MA11, item 1, first half)

- **Scope:**
  - **the workspaces:** the engine at the root, with `crates/*`; the spikes in `spikes/` with their lock; `fuzz/` when its first target lands (CR15);
  - **the checks and their configuration:** the crate map, with every M1 crate's entry; `tools/check.sh`, `check-dependencies.py`, `check-file-size.py` and the allowlist; the workspace's lints and `clippy.toml` (the standards' §15); release profiles with `panic = "abort"`; `deny.toml` and `supply-chain/`, with `cargo-vet` importing the organizations DP15 names; `tools/fetch-test-data.sh`;
  - **the first crates:** `trace`'s recorder in process (OB5–OB9); `codegen`'s framework with `--check`; `test-support`'s golden files and counting allocator;
  - **the audits:** `smallvec`'s and `vello_cpu`'s grades and `encoding_rs`'s per-block audit, each in the register (§2, items 12–14);
  - **CLAUDE.md** updated: the layout, the code rules, the checks.
- **Tests:**
  - each checker's self-test, as `tools/check-unsafe-policy.sh --self-test` does;
  - the dependency checker refusing a disallowed edge, a cycle through a development dependency, and a test crate in a shipping closure;
  - `trace`'s first-tier tests: no allocation when recording; its instructions per record (OB25).
- **Efficiency:** the commit tier's time (§2, item 10); `trace`'s first tier.
- **Done when:** `tools/check.sh` runs every check green on the new workspace; each checker's self-test passes; the three audits are in the register with their grades.

### M1-CP3. The platform, the processes and IPC (MA11, item 1, second half)

- **Gate:** the interface notes for the IDL, `ipc`, `event-loop` and `trace` with the diagnostics region (§3).
- **Scope:**
  - **crates:** the platform crates M1's processes need: `platform-mach`, `-event`, `-process`, `-metal`, `-layers` and `-appkit`, each with `unsafe` only in its `src/sys/` (CR6); `ipc`, `protocol` and `event-loop`;
  - **processes:** the browser launching `axiom-renderer` and `axiom-network` under browser-compiled profiles (ADR-0004 D2), each with a Mach channel and a memory entry;
  - **IPC from the start:**
    - handles owned or destroyed (IP9, IP11; RV4-05);
    - returned messages kept whole or destroyed (IP16);
    - in-memory channels that model Mach's limits (IP2; RV4-11);
    - the rendezvous pinning the PID and its version, one hello per child (IP4; RV4-12);
    - PF19's watchdog with task ages (RV4-03);
  - **the window:** a Metal clear and a CoreAnimation layer, with **EM11's objc2 classes validated in it** (§2, item 16);
  - **observability:** the diagnostics regions, the `diagnostics` collector, the flight recorder with the browser's file-backed one, and crash, hang and violation reports (OB10–OB17).
- **Tests:**
  - S4's denial probes in each helper; each renderer profile denies `iokit-open` (MA7);
  - the IDL's generated decoders under their first fuzz target; the region decoder's fuzz target and the lying producer (OB24);
  - Miri on the platform crates' safe logic, and loom on the channels and rings (TS11);
  - a helper's deliberate panic, stall and malformed message, each giving its report (OB27);
  - OQ-20's measurement.
- **Efficiency:** tier 1: the sizes of IPC headers, handle lists and records; the regions' memory charged as S16 measured (OB25).
- **Done when:** a window shows a Metal-cleared layer; both helpers run under their profiles, answer pings with their task ages and pass the probes; a helper's panic leaves a report with its message; EM11's classes work, or the fallback is recorded for the owner.

### M1-CP4. The walking skeleton, the harness and the efficiency harness (MA11, item 2, with items 3–6 thin)

- **Gate:** the skeleton's interface notes and its starter set (§3, §2 item 20).
- **Scope:**
  - **the harness:** the headless embedder's offscreen texture and PNG output (EM9); wptrunner's `axiom` product with the script-free executor (TS2), in `tests/wpt/`; plain HTTP/1.1 to wptrunner's server on the loopback (NW3), and `file:` URLs;
  - **the thin slice, in real sandboxed processes:**
    1. the XML parser's subset for well-formed XHTML, since 5,423 of the 5,652 tests need XML (MA4);
    2. the DOM's arena and mutation path;
    3. a UA stylesheet subset with type, class, ID and descendant selectors, and a dozen properties through the generated property table (ST3);
    4. Ahem through the Rust path (`skrifa`, `vello_cpu`), shaped by HarfRust, broken by ICU4X;
    5. block and inline layout;
    6. backgrounds, solid borders and text, painted;
    7. display list v0 with PL9's validator;
    8. the compositor's tiles;
    9. the offscreen texture, and the PNG;
  - **the orchestration:** `document` running the lifecycle with its tokens (IV1);
  - **the dumps:** the text dumps of the DOM, computed styles, fragments and the display list (MA1, RV1-15);
  - **the efficiency harness** (PF5; RV4-06): `perf-harness`, `rusage_info_v6` at each process's exit, `perf/baseline.toml` and the ratchet;
  - **observability:** spans for tasks, lifecycle phases, IPC flows, navigations and loads (OB4, OB7); metrics into PF5's harness; the network log as the `net` category (MA1).
- **Tests:**
  - the starter set through wptrunner, in real processes and again in single-process mode (TS2);
  - golden dumps (ES21);
  - fuzz targets for the XML subset (against xml5ever) and URLs;
  - HD33's chunking test on the XML subset.
- **Efficiency:**
  - tier 1: the node's 36 bytes (HD10), and the computed style's, fragment's and display item's sizes;
  - tier 2: the first baseline, over the starter set and parsing the corpus copies (instructions, peak footprint);
  - T1–T4 with every M1 crate present (RV1-18; ADR-0022's assumption).
- **Done when:** the starter set runs end to end in real sandboxed processes at its expected pass count; the ratchet runs in the checkpoint tier; T1–T4 are recorded; the code matches its approved notes.

### M1-CP5. The parsers and the DOM (MA11, item 2)

- **Gate:** the HTML tree sink's note.
- **Scope:**
  - **the HTML tokenizer and tree builder** (HD1–HD8) with their runners (TS4);
  - **the XML parser completed for documents** (HD9; RV4-01): namespaces; XHTML's public identifiers resolved to HTML's named character references, compiled in; no external entity fetched; expansion bounded; well-formedness errors shown as an error document;
  - **the DOM at M1's scope** (HD10–HD16);
  - **sniffing** (HD3) through `encoding`, with `encoding_rs` adopted under DP8's conditions (§2, item 14).
- **Tests:**
  - html5lib's tokenizer tests at 100%;
  - the tree-construction runner over the 1,973 `.dat` tests, reporting against MA4's ≥ 96%;
  - fuzz targets for the tokenizer and tree builder (against html5ever), the XML parser (against xml5ever) and the encodings under AddressSanitizer;
  - HD33's 100 random chunkings.
- **Efficiency:** tier 2: parsing the corpus copies.
- **Done when:** the tokenizer suite passes; tree construction is reported with every failure attributed; `encoding_rs`'s conditions are met; the skeleton's parsing TODOs are gone.

### M1-CP6. Style (MA11, item 3)

- **Scope:** CSS syntax, selectors, the cascade and computed values (style's first-stage row); style sharing and invalidation sets (ST19); the computed-style dump; style's generated property code, with T1–T4 measured (RV1-18).
- **Tests:** fuzz targets for CSS syntax, selectors, values and media queries; the computed-style dumps of the corpus copies, reviewed; IV14's self-oracle on a hover; ST22's cache-free oracle.
- **Efficiency:** tier 1: the computed style's size and the sharing cache's hits on fixtures; tier 2: styling the corpus copies; T1–T4 (PF18).
- **Done when:** both corpus copies style without error, and T1–T4 hold with the generated code.

### M1-CP7. Text (MA11, item 4)

- **Scope:** the font database through CoreText (TX6) and system fonts rasterized by CoreText (TX12), in `platform-text`; fallback; bidi; line breaking; the atlas; the Rust path for WPT's test fonts completed (TX21).
- **Tests:** Unicode's line-breaking and bidi files at 100% (MA4); S5's shaping comparison against CoreText as a regression test.
- **Efficiency:** tier 2: shaping and breaking the corpus copies' text.
- **Done when:** the conformance files pass, and the corpus copies' text shapes and breaks.

### M1-CP8. Layout (MA11, item 5)

- **Scope:** floats and positioning, then tables (approximate, LY10), lists and generated content; the box and fragment dumps.
- **Tests:** the `css/CSS2` directories counted with expectation files (SB9); layout's limits for hostile documents under fuzzing (layout §7).
- **Efficiency:** tier 1: the fragment's size; tier 2: laying out the corpus copies.
- **Done when:** both corpus copies lay out with ≥ 99% of their visible text placed (MA5's measure).

### M1-CP9. Paint and the compositor (MA11, item 6)

- **Scope:**
  - every CSS 2.1 border style, and outlines;
  - the compositor's tiles, damage and scrolling layers (GR2–GR8);
  - **release at a renderer's death** (PL8, IR10; RV4-04);
  - a tile committed once its commands are scheduled (GR12; RV4-10);
  - the pool of surfaces allocated ahead (§2, item 21).
- **Tests:** MA7's renderer-death test (nothing mapped 1 s later); the display-list decoder's fuzz target, completed; S6's frame measurements as benchmarks (PF13).
- **Efficiency:** tier 2: frames for the corpus copies' first paint and a scroll; the window's tiles against B1's 6.2 bytes a pixel.
- **Observability:** the compositor's frame spans (PF13).
- **Done when:** the corpus copies paint and scroll within PF13 at the median, and a renderer's death leaves nothing mapped.

### M1-CP10. Images (MA11, item 7)

- **Scope:** PNG, JPEG (§2, item 22), GIF and WebP behind IM9's interface, decoded incrementally and at display size, with colour management through `moxcms`.
- **Tests:** each decoder's wrapper under its fuzz target; S5-Q5's pixel comparisons as regression tests.
- **Efficiency:** tier 2: decoding the corpus copies' images (instructions, peak footprint).
- **Done when:** the corpus copies' images decode, and the JPEG decision is recorded in the register.

### M1-CP11. Networking (MA11, item 8)

- **Gate:** the loader's protocol over HTTP.
- **Scope:**
  - HTTP/1.1 over TLS: rustls with `aws-lc-rs`, and the system verifying certificates in the worker pool (NW6);
  - DNS-SD (NW7) and the in-memory cache;
  - mixed content, HSTS with its preload list compiled in, and Referrer-Policy (NW12);
  - content decoding with NW13's limits;
  - the URL parser completed, run against WPT's URL test data in a Rust runner (inference: the data is JSON, as TS4's tokenizer tests are);
  - the network log.
  HTTP/2 waits for M2 (RV4-08).
- **Tests:**
  - MA7's network-process tests: killed, stalled, and a storage thread stalled while the control thread answers;
  - the containment test (a stalled renderer);
  - fuzz targets for HTTP/1.1 framing and content decoding;
  - **S7-Q2's comparison per byte** of rustls and Network framework, measured again on a direct network path, since the re-run on 26B118 couldn't reproduce it ([S7 results](research/s7-results.md) §12).
- **Efficiency:** PF26's CPU per page load begins: the corpus copies over the loopback, then live (§2, item 24 first).
- **Observability:** the network process's categories.
- **Done when:** both pages load live within CO3 and CO4, and the network process recovers from each induced failure.

### M1-CP12. The shell (MA11, item 9)

- **Scope:**
  - the address field, back and forward, and history, on the window and classes M1-CP3 validated;
  - no text entry into pages before M4 (RV4-08): forms submit as navigations, and MA7's password rule holds;
  - **observability:** the command that saves a trace (OB21); the health monitor and its breach reports (OB19); the daily health log (OB20).
- **Tests:**
  - MA7's test that a form with a non-empty password field isn't submitted below L2;
  - TCC's persistence across a rebuild, by hand (§2, item 8);
  - the health log writing nothing while idle (PF28).
- **Efficiency:** tier 2: start-up (PF25), from a warm launch to the first window drawn.
- **Done when:** the owner can navigate both pages from the address field, with back and forward, and save a trace.

### M1-CP13. Reference tests at scale (MA11, item 10)

- **Scope:** the `css/CSS2` selection through wptrunner, with expectations (SB8, SB9), against MA4's target. It has been tracked since M1-CP4; this checkpoint closes the gap.
- **Tests:** MA4's subset and floors recomputed at the pinned revision by `tools/wpt-script-deps.py --needs`, and checked by `tools/wpt-recount.sh` (SB11). The run reports every directory against its floor.
- **Done when:** the subset's two counts agree, and the pass rate is reported against **≥ 90.0% with the floors** (RV4-09), every failure attributed to a capability or a defect.

### M1-CP14. Fuzzing, determinism, stress and budgets (MA11, item 11)

- **Scope:**
  - every fuzz target's campaign, at least an hour under AddressSanitizer (MA7);
  - MA8's determinism tests;
  - **the stress configuration** in real processes, through the stress builds of CR13 (TS8, MA8; RV4-11);
  - every budget of MA6, PF25–PF30 included, measured under PF1;
  - **observability's own costs** (OB25).
- **Done when:** no fuzzing finding is open; the determinism and stress runs equal the plain runs; MA6's report has a value for every row; OB25's values are measured.

### M1-CP15. The corpus checks and the gate (MA11, item 12)

- **Scope:** MA5's checks on the dated copies and live; MA9's gate report, with every criterion's measured value beside its target, observability's costs and the ratchet's accepted regressions, and the raw records under `target/`.
- **Done when:** the owner holds the gate report (§7).

## 5. Efficiency (MS10, as amended in Round 4)

| Checkpoint | What it adds to the cost | The tier that catches a regression | The budget it draws on |
|---|---|---|---|
| M1-CP2 | The checks themselves; `trace` | The commit tier's time; tier 1: recording's cost | — ; OB25 |
| M1-CP3 | Three processes, their channels and regions | Tier 1: IPC headers, handle lists and records. From M1-CP4, tier 2: each helper's start-up and footprint | B2, B4, PF25, OB25 |
| M1-CP4 | Every stage, thin; the first baseline | Tier 1: the node's, computed style's, fragment's and item's sizes. Tier 2: the starter set, and parsing the copies. T1–T4 | B3's pages, PF18 |
| M1-CP5 | The parsers and the DOM | Tier 2: parsing the copies | B3 |
| M1-CP6 | Style and its caches | Tier 1: computed styles' size; tier 2: styling; T1–T4 | B3; PF23's caches; PF18 |
| M1-CP7 | Fonts, shaping, the atlas | Tier 2: shaping and breaking | B3 |
| M1-CP8 | Fragments | Tier 1: the fragment's size; tier 2: layout | B3 |
| M1-CP9 | Tiles and display lists | Tier 2: frames, the window's tiles | B1, PF13 |
| M1-CP10 | Decoded images | Tier 2: decoding | B3, IM10 |
| M1-CP11 | TLS and the cache | Tier 2: CPU per page load | B4, PF26 |
| M1-CP12 | The shell; the health log | Tier 2: start-up; the log's writes | B1, PF25, PF28 |
| M1-CP14, CP15 | — | Tier 3: PF1's formal run of every budget, and observability's costs | MA6, PF25–PF30, OB25 |

## 6. Risks, and where each is retired

| Risk | Retired at |
|---|---|
| The milestone is large: the whole static pipeline | Every checkpoint leaves the workspace working; the skeleton makes progress measurable from M1-CP4; the gate counts only M1's scope |
| A helper's profile silently denies something M1's code needs | M1-CP3's probes; each later checkpoint's tests run under the real profiles |
| `smallvec` or `encoding_rs` fails Axiom's audit | M1-CP2, before anything depends on them: an upstream fix, or the owner's decision. For `smallvec` in the browser, the crate architecture's open question 1 |
| EM11's classes can't be defined through objc2 | M1-CP3, before the shell's checkpoint: the Swift fallback behind the same API |
| The skeleton's thin code stays thin | Its TODOs name the checkpoint that replaces each part; each widening checkpoint ends with them gone (ES17, ES29) |
| Interfaces change while stages widen | Expected, and cheap early; each change to an approved note returns to the owner (CR8) |
| Too many crates for T1–T4 | M1-CP4 measures with every crate present; crates merge where a split buys nothing (ES3) |
| The commit tier grows slow | ES25's budget: suites move tiers, never skipped |
| Observability costs more than its budget | First-tier tests from M1-CP2; OB25 measured at M1-CP14 |
| The XML parser's scope grows beyond documents | M1-CP4 thin, M1-CP5 documents only; `DOMParser` and XHR at M3, SVG documents at M4 |
| The CSS target is missed | Tracked from M1-CP4; M1-CP13 attributes every failure; a miss with its reason goes to the owner (SB11) |
| Efficiency drifts within the budgets | The ratchet from M1-CP4 |
| Wikipedia's chrome needs flexbox and grid | MA5 judges the article; the chrome is M4's |

## 7. The owner's gates

1. **M1-CP1:** ADRs 0022 and 0023; this plan, with §2's decisions; MA10's authorization with §1's additions. **Passed** (2026-10-23).
2. **The interface notes** at M1-CP3, M1-CP4, M1-CP5 and M1-CP11, and any change to an approved note (CR8).
3. **Each dependency's adoption,** with its register entry, as the checkpoints reach it. MA10's authorization lists them; their adoptions weren't approved together at M1-CP1, so each comes to the owner with the checkpoint that first uses it.
4. **Any amendment** to an ADR or a binding rule that M1 finds necessary (MS12). That includes the one `smallvec`'s grade may raise for NW2 (§2, item 12).
5. **The gate:** MA9's report at M1-CP15; then the merge of `milestone-1` into `main`, `--no-ff`, when the owner says so.

## 8. The checkpoints against MA11's outline

The owner's choice of a walking skeleton refines MA11's order. Its twelve items and MA4–MA9's criteria are unchanged.

| MA11's item | Checkpoints |
|---|---|
| 1. The skeleton, the checks, profiles and lints; the window; the helpers under their profiles | M1-CP2 (the engineering foundation), M1-CP3 (platform, processes, IPC, the window) |
| 2. The harness end to end; URLs and `file:`; the HTML parser; the XML parser; the DOM; the efficiency harness | M1-CP4 (the harness, a thin XML parser and DOM, the efficiency harness), M1-CP5 (the parsers and the DOM in full), M1-CP11 (URLs completed) |
| 3. Style | M1-CP4 (thin), M1-CP6 |
| 4. Text | M1-CP4 (Ahem on the Rust path), M1-CP7 |
| 5. Layout | M1-CP4 (block and inline), M1-CP8 |
| 6. Paint, display lists, the compositor | M1-CP4 (thin), M1-CP9 |
| 7. Images | M1-CP10 |
| 8. Networking | M1-CP4 (plain HTTP/1.1 on the loopback), M1-CP11 |
| 9. The shell | M1-CP3 (the window, EM11), M1-CP12 |
| 10. Reference tests at scale | Tracked from M1-CP4; M1-CP13 |
| 11. Fuzzing, determinism, budgets | Fuzz targets with each parser's checkpoint; M1-CP14 |
| 12. The corpus checks and the gate | M1-CP15 |

## 9. Tracker

| Checkpoint | State | Record |
|---|---|---|
| M1-CP1 | **Done** (2026-10-23): the plan adopted, ADRs 0022 and 0023 accepted, MA10 authorized, §2 approved; the acceptance sweep made | `docs(m1-cp1)` commits |
| M1-CP2 | **Done** (2026-10-23): the workspaces split; the checks in two tiers through `tools/check.sh`, with the crate map, the dependency, file-size and census tools and their self-tests; the lints, profiles, `deny.toml` and `supply-chain/`; the pinned test data; `trace`, `codegen` and `test-support`; the three audits in the register (`smallvec` `ub-risk-3`; `encoding_rs` every block holding, `simdutf8` `ub-risk-2`; `vello_cpu`'s tree `ub-risk-2` or better but `smallvec`); CLAUDE.md updated | `(m1-cp2)` commits. **Measured** (observed, 2026-10-23, AC power, Low Power Mode off, load average 2.3): the commit tier 22 s unchanged, 20 s after touching `trace` and 24 s after touching `test-support`, against its 180 s budget, the spikes' tests 10–13 s of each; the checkpoint tier 22 s, Miri 5 s over `test-support`'s 7 tests. **`trace`'s first tier** (OB25), per record net of the loop, optimized: a disabled check 1.0 instructions and no measurable cycles; a span record 130.5 instructions and 28 cycles; a counter 103.0 instructions and 19–20 cycles, so an enabled span costs about 9 ns at the performance cores' 3.2 GHz (inference), under OB25's 50 ns. The TODO list is empty. **Deviations:** a commit went in with the unsafe-policy check failing, its status hidden by a pipe; it was fixed and amended before any push, and CLAUDE.md now says to read each check's own status. The first census missed `peniko`'s and `color`'s impls behind their own `allow`; `--force-warn` and `tools/unsafe-sites.py` fixed the method, and the earlier counts held |
| M1-CP3 | **In progress** (2026-10-23): its four interface notes proposed, for the owner's gate before its code ([interface notes](architecture/interfaces/README.md)): the IDL and the generated API; `ipc`'s channels and `Untrusted<T>`; `event-loop`'s tasks and task ages; `trace`'s API and the diagnostics region | `docs(m1-cp3)` commits |
| M1-CP4 to M1-CP15 | Not started | — |
