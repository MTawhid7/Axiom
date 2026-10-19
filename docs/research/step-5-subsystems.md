# Step 5 — Subsystem Design: Plan and Checkpoint Tracker

| | |
|---|---|
| **Status** | **Part 1 complete: CP1–CP9 ✅** (2026-10-18). **Gate B passed** (2026-10-18): ADRs 0010 and 0014 accepted; ADRs 0011–0013 pre-approved, pending their spikes; spikes S5–S7 authorized (§2, §7, §10). **Spikes:** S5 reported (CP10 ✅); S6's formal run waits for the owner (CP11); S7 reported and **ADR-0013 accepted** (CP12 ✅, 2026-10-19). **Part 2:** CP13–CP15 ✅. **Next: CP16,** the JavaScript engine |
| **Authorized** | 2026-10-18, by the owner: "Step 5 Authorization: Authorized." Planning on a new branch, `step-5`, with "the Step 5 plan and checkpoint tracker first for Gate A review" (§2). Design documents only; each spike needs its own authorization. **Spikes S5, S6 and S7 authorized at Gate B** (2026-10-18), each for the questions it names (§10) |
| **Branch** | `step-5`, from `main` at `c642ab3` (the Step 4 merge). Pushed to `origin` as the owner authorized at Gate A, and kept pushed as checkpoints land |
| **Feeds** | Decision-map items I1–I5, the text part of P4, and X4 if the owner adds it (§2.3); the questions earlier steps handed to Step 5 (§2.2); OQ-3, as evidence; ADRs from 0010 (§4); Step 6 |
| **Last updated** | 2026-10-19 |

> **Resuming in a new session?**
> 1. Read §6 to find the first checkpoint that isn't ✅.
> 2. Check the repository against §11, the results log.
> 3. Continue from that checkpoint's deliverables (§6.1).

---

## 1. Purpose

Design the subsystems behind the interfaces that Step 4 fixed: what each one computes, with which data structures and families of algorithms, which code Axiom writes and which it reuses, which process runs it, and what it costs on the 8 GB M1.

The subsystems are the ones the [decision map](../decision-map.md) assigns to Step 5:
- text and fonts (I1), with the text part of P4;
- graphics and compositing (I2);
- networking and TLS (I3);
- image and media decoders (I4);
- the accessibility bridge (I5);
- style and layout;
- HTML parsing and the DOM, and the JavaScript engine and bindings, which the owner added at Step 4's Gate A (2026-10-17).

Storage and workers are designed after networking: the [process model](../security/process-model.md) §3 places web storage in the network process, and service workers intercept its fetches.

**Step 4 fixed interfaces, ownership, threads and invariants. Step 5 designs what sits behind them, and changes none of them without the owner** (SC1). Step 6 then sets the standards baseline, the testing strategy, the budgets and the roadmap from Step 5's inputs.

**The level of design** is the decisions that are expensive to reverse: data structures, the interfaces between subsystems, families of algorithms, build or reuse, and process placement. Complete algorithms are not designed here. Each document names what it leaves to the implementation milestones.

## 2. Owner input and scope (2026-10-18)

| Input | Effect |
|---|---|
| "Step 4 is formally accepted." | Step 4 is closed: merged into `main` at `c642ab3` and pushed ([Step 4 tracker](step-4-architecture-proposal.md), CP17) |
| "Step 5 Authorization: Authorized." | This plan, as design work under the Phase Zero rule |
| "Proceed with planning Phase Zero Step 5 (Subsystems) on a new branch `step-5`." | Branch `step-5`, created from `main` at `c642ab3` |
| "Submit the Step 5 plan and checkpoint tracker first for Gate A review, defining the subsystem sequence, review gates, and identifying which areas may require targeted validation spikes." | CP1 ends at Gate A (§7). The sequence is in §6, the gates in §7, and the candidate spikes in §10. No spike is designed in detail or built before the gate that authorizes it |
| **Gate A (2026-10-18):** "Plan Approval: Approved as written." "The 19-checkpoint sequence, two-part structure, and subsystem criteria SC1–SC10 are approved." "Step 4 interfaces remain binding under SC1." | The plan stands as written (§3–§10) |
| "Decision Map Addition: Approved. Add X4 (Dependency and Supply-Chain Policy) to docs/decision-map.md, to be decided at CP2 in ADR-0010." | X4 added to the [decision map](../decision-map.md). CP2 writes the policy, and CP9 proposes ADR-0010 |
| "Candidate Spikes: Confirmed. S5 (Text), S6 (Metal Compositing), and S7 (Network/TLS Sandbox) are approved as candidate spikes, with code authorization to be decided individually at Gate B." | §10. Each candidate's design is written at its checkpoint, and its code waits for Gate B |
| "Cadence: Approved. Proceed through CP2–CP9 without pausing until Gate B." | CP2–CP9 run without pausing |
| "Push Branch: Authorized. Please push `step-5` to GitHub origin and keep it updated." | Pushed to `origin` on 2026-10-18, and kept pushed as checkpoints land |
| "Proceed with Checkpoint CP2." | CP2 starts |
| **Gate B (2026-10-18):** "ADR-0010 (Dependency and Supply-Chain Policy): Accepted as proposed, including the DP10 refinement." | [ADR-0010](../adr/0010-dependency-and-supply-chain-policy.md) Accepted; the [dependency policy](../technology/dependency-policy.md) is binding |
| "ADR-0011 (Text, Fonts, and Unicode): Pre-approved; will move to Accepted upon S5-Q1 and S5-Q3 passing, with TX12 settled by S5-Q2." | [ADR-0011](../adr/0011-text-fonts-and-unicode.md) stays Proposed until S5 reports (§10.1) |
| "ADR-0012 (Graphics, Images, and Media): Pre-approved; will move to Accepted upon S6-Q1, S6-Q3, and S6-Q5 passing, and S5-Q5 validating ImageIO inside the renderer sandbox." | [ADR-0012](../adr/0012-graphics-images-and-media.md) stays Proposed until S6 and S5-Q5 report (§10.1, §10.2) |
| "ADR-0013 (Networking, Storage, and Workers): Pre-approved; will move to Accepted upon S7-Q1, S7-Q3, and S7-Q4 passing." | [ADR-0013](../adr/0013-networking-storage-and-workers.md) **accepted** 2026-10-19: S7-Q1, S7-Q3 and S7-Q4 passed (CP12, §11) |
| "ADR-0014 (Accessibility): Accepted as proposed." | [ADR-0014](../adr/0014-accessibility.md) Accepted; the [accessibility design](../architecture/accessibility.md) is binding |
| "Spike S5 (Text Stack & ImageIO Sandbox): Authorized for questions S5-Q1 through S5-Q5 under spikes/s5/, including the listed candidate crates and OFL/WPT test web fonts." | CP10 (§10.1) |
| "Spike S6 (Metal Compositing): Authorized for questions S6-Q1 through S6-Q5 under spikes/s6/. The performance thresholds are confirmed: typical update ≤ 4 ms GPU / ≤ 2 ms compositor thread; scrolling/transforms ≤ 1 ms." | CP11 (§10.2), with the thresholds confirmed |
| "Spike S7 (Network Process & TLS): Authorized for questions S7-Q1 through S7-Q4 under spikes/s7/, including read-only outbound network requests to public test hosts." | CP12 (§10.3): ✅ 2026-10-19 ([results](s7-results.md)) |
| "Settings List: Approved as written in docs/architecture/accessibility.md §6.1." "Reserved Key Equivalents: Approved as written in docs/architecture/accessibility.md §6.2 (Chromium reserved set, plus ⌘L, ⌘,, and Esc un-overridable even in fullscreen)." | Handed-over questions 13 and 14 (§2.2) answered; [accessibility](../architecture/accessibility.md) §6 |
| "OQ-21 (Axiom License): Approved as 'MIT OR Apache-2.0'. Please add the standard LICENSE-MIT and LICENSE-APACHE files to the repository root." | Both files added; the workspace manifest declares the license; [OQ-21](../open-questions.md) resolved; DP17 updated |
| "Proceed with executing Spikes S5, S6, and S7 as planned." | CP10–CP12, then CP13–CP18 without pausing until Gate C (§7) |

### 2.1 Binding inputs

Every one of these is accepted, so Step 5 designs within them:
- **[ADR-0001](../adr/0001-scope-and-target.md):**
  - the full modern app web must stay reachable;
  - our own JavaScript engine, with JavaScript and the DOM in one heap;
  - sensitive logins, gated;
  - Apple Silicon and the current macOS only;
  - web content in all languages, in stages.
- **[ADR-0002](../adr/0002-implementation-language.md):**
  - Rust;
  - the `unsafe` policy ([evaluation](../technology/language-evaluation.md) §15.3): FFI only in `platform` modules, and no `unsafe` in parsers, network policy, IPC decoding, or image and font wrappers (rule 5);
  - build thresholds T1–T4.
- **[ADR-0003](../adr/0003-gc-handle-and-rooting-pattern.md) and [ADR-0008](../adr/0008-script-agents-and-heap.md):**
  - Pattern B, with one heap per agent;
  - `SliceStack` for C1;
  - struct-of-arrays link columns (SH8);
  - generated setters (SH9);
  - memory that reaches platform APIs lives outside the heap (SH11);
  - generated bindings (SH12);
  - room for a JIT (SH13).
- **[ADR-0004](../adr/0004-process-model-and-sandbox.md):**
  - the process types and the Rule of 2;
  - the browser never decodes images or parses fonts;
  - system parsers run only in sandboxed helpers;
  - the JIT policy J1–J7;
  - memory targets B1–B7.
- **ADRs [0005](../adr/0005-ipc-transport-and-interfaces.md)–[0007](../adr/0007-frame-tree-pipeline-and-input.md) and [0009](../adr/0009-embedder-api-and-shell.md):**
  - Mach IPC with generated safe decoders and `Untrusted<T>`;
  - one thread and heap per event loop;
  - the browser's frame tree;
  - five pipeline stages, with immutable fragments and per-frame display lists of validated primitives;
  - the invalidation lifecycle;
  - input and accessibility through the browser;
  - the embedder API, a headless embedder and the pure-Rust shell.

  The design documents behind these ADRs are binding.
- **[Charter](../charter.md):**
  - the core mechanisms of G1 are Axiom's own, and supporting infrastructure is reused only with a justification and a class (principle 4);
  - efficiency is a primary goal (G4);
  - no architectural corners are cut (principle 7).

### 2.2 Questions earlier steps handed to Step 5

| # | Question | Raised in | Checkpoint |
|---|---|---|---|
| 1 | The dependency policy, "to be finalized" | [Language evaluation](../technology/language-evaluation.md) §12, §15.4 | CP2 |
| 2 | Text and shaping (the text part of P4). Web fonts. Checking that fonts resolved, since failures are silent. Reviewed rules for user-installed fonts | [Decision map](../decision-map.md) P4; [S4 results](s4-results.md) §7, item 6 | CP3 |
| 3 | Where glyphs are rasterized: the renderer, or a GPU process | [ADR-0004](../adr/0004-process-model-and-sandbox.md), consequences; [process model](../security/process-model.md) §3, note 4; [rendering pipeline](../architecture/rendering-pipeline.md) open question 2 | CP3, CP4 |
| 4 | The rasterizer and the GPU backend behind the compositor | [Rendering pipeline](../architecture/rendering-pipeline.md) PL9, open question 3 | CP4 |
| 5 | Damage tracking and partial presentation | [Invalidation](../architecture/invalidation.md) IV7, open question 3 | CP4 |
| 6 | Which animations run on the compositor beyond `transform` and `opacity` | [Invalidation](../architecture/invalidation.md) open question 2 | CP4, CP14 |
| 7 | How the network process enforces initiator locks and ORB | [ADR-0004](../adr/0004-process-model-and-sandbox.md), consequences | CP6 |
| 8 | The TLS stack. If it is Apple's, it must stay out of the browser process | [ADR-0004](../adr/0004-process-model-and-sandbox.md), consequences; [overview](../architecture/overview.md) §2 | CP6 |
| 9 | Private Network Access-style checks | [Threat model](../security/threat-model.md) §5 | CP6 |
| 10 | Name resolution on the network process's worker pool, if a blocking resolver is chosen | [Event loop and threading](../architecture/event-loop-and-threading.md#42-the-threads-of-each-process) §4.2 | CP6 |
| 11 | Where shared and service workers run when no same-site renderer exists | [Script and heap](../architecture/script-and-heap.md) open question 4; [frame tree](../architecture/frame-tree-and-navigation.md) open question 3 | CP7 |
| 12 | The mapping from accessibility trees to `NSAccessibility` roles and actions | [Input and accessibility](../architecture/input-and-accessibility.md) open question 3 | CP8 |
| 13 | Which key equivalents a page may override, decided with the owner | [Input and accessibility](../architecture/input-and-accessibility.md) open question 2 | CP8 |
| 14 | The full list of settings, and how the sensitive-site list and the JIT switches are presented, decided with the owner | [Embedder API](../architecture/embedder-api.md) open question 2 | CP8 |
| 15 | Style invalidation's details | [Invalidation](../architecture/invalidation.md) IV3 | CP14 |
| 16 | The cascade and its caches; the layout unit | [Rendering pipeline](../architecture/rendering-pipeline.md) PL3, PL5, open question 4 | CP14, CP15 |
| 17 | The bytecode dispatch technique. S3 recommends a `match` loop | [Script and heap](../architecture/script-and-heap.md) open question 2; [S3 results](s3-results.md) §9, item 5 | CP16 |
| 18 | S3's three inputs for the engine: a value type that carries the scope lifetime; rooting many values as one heap array, as event dispatch's listener snapshot could; keeping the root stack's length where the compiler loads it on its own, with S1's dispatch case kept as a benchmark | [S3 results](s3-results.md) §9, item 6 | CP13, CP16, CP17 |
| 19 | Ropes and slices for JavaScript strings | [Rendering pipeline](../architecture/rendering-pipeline.md) PL11 | CP16 |
| 20 | The WebIDL generator | [Script and heap](../architecture/script-and-heap.md) SH12 | CP17 |
| 21 | OQ-3, WebAssembly: our own engine or an embedded one. Its evidence includes the JavaScript engine's design | [Open questions](../open-questions.md); decision map I6 | CP16, as evidence only |

### 2.3 Scope

**In scope:** the subsystems in §1, each designed to the level stated there, with its build-or-reuse decisions and the spikes the owner authorizes.

**A new decision-map item, X4, dependency and supply-chain policy,** proposed here and **added by the owner at Gate A** (2026-10-18). It is decided in Step 5, at CP2, and recorded in ADR-0010. Three reasons:
- Step 5 is the first step to choose dependencies.
- The language evaluation leaves the policy "to be finalized" (§12, §15.4).
- ADR-0002's rule 5 (no `unsafe` in parsers) needs a rule for code Axiom doesn't write, such as a font parser crate that contains `unsafe`.

**Out of scope.** Each item names where it is decided.
- **Step 6:** the standards baseline and support matrix (X1), the testing and fuzzing strategy (X2), the performance budgets (X3), and the roadmap and its milestones. Step 5 supplies their inputs: each subsystem's specifications and test suites (SC5), its costs (SC6) and its staging.
- **OQ-3, WebAssembly:** whether Axiom writes its own engine or embeds one stays open until before the first Wasm milestone. CP16 records what a Wasm engine needs from the JavaScript engine and the heap, and assesses the options as evidence.
- **Whether to build a JIT** (J1) stays evidence-driven and comes later. Step 5 keeps room for one (SH13).
- **Media codecs' internals and DRM.** DRM is a non-goal (charter §6). Step 5 places media decoding; it designs no codec.
- **Production code, a project skeleton and dependencies:** none in Phase Zero. Spikes run only as the owner authorizes each one.

**Not needed by Step 5: OQ-2, the target-site corpus.** Step 5 designs for the complete model, and the corpus orders the work in Step 6's roadmap. If the owner lists sites earlier, the staging notes will use them.

**Evidence gathering without code**, as in Steps 3 and 4:
- SDK headers, export stubs and man pages;
- `nm -u`, `otool -L` and `codesign -d` on installed browsers;
- listings of the system's fonts and ICU data;
- crate sources, metadata and documentation, read on GitHub, docs.rs and crates.io.

Nothing is downloaded with cargo or installed. Each such fact names the command or URL it comes from. Measurements come only from authorized spikes.

## 3. Questions

| ID | Question | Answered at |
|---|---|---|
| Q1 | Which code does Axiom write itself, and which may it reuse? How is each dependency justified, classed, audited and kept within the `unsafe` policy and the build thresholds, including the `unsafe` and C code inside it? | CP2 |
| Q2 | How does the engine handle Unicode: character properties, segmentation, line breaking, bidi, normalization, case mapping, IDNA, and the Encoding Standard? Where does the Unicode data come from, for text, CSS, URLs and JavaScript alike? | CP3 |
| Q3 | How are fonts found, matched and fallen back to across scripts? How are web fonts loaded safely in the renderer? How is text shaped? Where and how are glyphs rasterized, and in what form do they reach the display list? | CP3 |
| Q4 | What draws the display list: the GPU interface, path and glyph rasterization, compositing of property trees and effects, color management, damage tracking and partial presentation? Which animations run on the compositor, and how does all of it move into a GPU process later? | CP4 |
| Q5 | Which image formats are decoded, by safe Rust or by ImageIO inside the renderer's sandbox? How do decoding, animation and color profiles stay within memory budgets? Where do media decoding and audio output run? | CP5 |
| Q6 | How does the network process fetch: URLs and sites, HTTP/1.1, HTTP/2 and HTTP/3, TLS and certificate verification against the system's trust store, DNS, connections, the HTTP cache and cookies? How does it enforce Fetch's policy from the browser's records: initiator locks, CORS, ORB, CORP, mixed content, HSTS, Private Network Access? | CP6 |
| Q7 | Where and how are web storage, IndexedDB, the Cache API and quotas stored? How do service workers intercept fetches, and where do shared and service workers run when no same-site renderer exists? | CP7 |
| Q8 | How is the accessibility tree built in the renderer, and how does the browser expose it through `NSAccessibility`? With the owner: the settings list, and the key equivalents a page may override | CP8 |
| Q9 | How is HTML parsed, from bytes to tree? How is the DOM represented in Pattern B arenas: nodes, links, attributes, text, events, mutation observers, Shadow DOM, custom elements, ranges and selection, forms and editing? | CP13 |
| Q10 | How is CSS parsed and the cascade computed: selector matching, the cascade and its caches, invalidation, computed values, custom properties, queries, animations and transitions, the CSSOM? | CP14 |
| Q11 | How does layout work: the layout unit, box construction, formatting contexts, intrinsic sizing, writing modes, fragmentation, scrolling and overflow, replaced elements and child frames, fragment reuse? How is the fragment tree painted into a display list, hit-tested and queried? | CP15 |
| Q12 | How does the JavaScript engine work: parser, bytecode and interpreter, the value type, the object model and inline caches, strings, builtins and `RegExp`, `Intl`, modules, generators and async functions, the job queue? What would Wasm and a JIT need from it? | CP16 |
| Q13 | How are the WebIDL bindings generated: platform objects in the heap, argument conversion, rooting conventions, prototypes and realms, exceptions, cross-origin objects, and the hot path of event dispatch? | CP17 |
| Q14 | How do the subsystems fit together: the crate structure and `unsafe` map, the dependency register, end-to-end scenarios through the subsystems, and the check of SC1–SC10? | CP18 |

## 4. Deliverables

Planned documents are named in code font until they exist.

| # | Deliverable | Checkpoint |
|---|---|---|
| 1 | This plan and tracker | CP1 |
| 2 | `docs/technology/dependency-policy.md` (X4, if added): the rules, and the format of a register that lists every proposed dependency | CP2 |
| 3 | `docs/architecture/text-and-fonts.md` (I1, the text part of P4); spike S5's design, in this tracker | CP3 |
| 4 | `docs/architecture/graphics.md` (I2); spike S6's design | CP4 |
| 5 | `docs/architecture/images-and-media.md` (I4) | CP5 |
| 6 | `docs/architecture/networking.md` (I3); spike S7's design | CP6 |
| 7 | `docs/architecture/storage-and-workers.md` | CP7 |
| 8 | `docs/architecture/accessibility.md` (I5) | CP8 |
| 9 | ADRs for part 1, Proposed, grouped by coupling as Step 4's were. Tentatively: ADR-0010, the dependency policy (X4); ADR-0011, text, fonts and Unicode (I1); ADR-0012, graphics, images and media (I2, I4); ADR-0013, networking, storage and workers (I3); ADR-0014, accessibility (I5). CP9 settles the grouping | CP9 |
| 10 | Spikes, each only if authorized: code in `spikes/s5/`, `spikes/s6/` and `spikes/s7/`, scripts in `tools/`, results in `docs/research/s5-results.md`, `s6-results.md` and `s7-results.md` | CP10–CP12 |
| 11 | `docs/architecture/html-and-dom.md` | CP13 |
| 12 | `docs/architecture/style.md` | CP14 |
| 13 | `docs/architecture/layout-and-paint.md` | CP15 |
| 14 | `docs/architecture/javascript-engine.md` | CP16 |
| 15 | `docs/architecture/bindings.md` | CP17 |
| 16 | The [overview](../architecture/overview.md), refined; ADRs for part 2, Proposed. Tentatively: ADR-0015, HTML parsing and the DOM; ADR-0016, style, layout and paint; ADR-0017, the JavaScript engine and bindings. CP18 settles the grouping | CP18 |
| 17 | Updates to the decision map, open questions, indexes, status and spikes README | CP9, CP12, CP18 |

**Every subsystem document has the same structure:**
- status, question, what it feeds and what it builds on;
- summary;
- binding inputs: the Step 3 and Step 4 rules it must satisfy;
- specifications: what it implements, dated, and where Axiom will be approximate;
- precedent: how Chromium, Firefox, WebKit, Servo and Ladybird do it, and relevant Rust projects, sourced and dated;
- build or reuse: the candidates judged by ADR-0010's rules, with each dependency's justification and class;
- the design, as numbered rules with one prefix per document:
  - DP, the dependency policy;
  - TX, text and fonts;
  - GR, graphics;
  - IM, images and media;
  - NW, networking;
  - SW, storage and workers;
  - AX, accessibility;
  - HD, HTML and the DOM;
  - ST, style;
  - LY, layout and paint;
  - JE, the JavaScript engine;
  - WB, the WebIDL bindings;
- interfaces with neighbouring subsystems;
- security: Rule of 2 placement, `unsafe` and FFI;
- efficiency: memory owners and estimates, threads, idle behaviour;
- incremental updates, and the test oracle;
- staging across L0–L3, and the complete model: where later app-web features will live;
- open questions and sources.

Each design document becomes binding when its ADR is accepted, as Step 4's did.

## 5. Subsystem criteria

Each subsystem design is judged against these criteria. CP18 checks every one and records the evidence in the overview.

| # | Criterion | From |
|---|---|---|
| SC1 | **Step 4's interfaces hold.** Each subsystem sits behind the interfaces of ADRs 0005–0009 and changes none of them. A subsystem that needs one changed raises an amendment for the owner, recorded in the ADR that owns the interface. Nothing changes silently | AC1; ADRs 0005–0009; charter principle 7 |
| SC2 | **Untrusted input is parsed only where the Rule of 2 allows.** Every parser of web content or network bytes runs in a renderer or a sandboxed helper. Apple's C and Objective-C parsers (CoreText, ImageIO, and Apple's TLS and certificate code) run only in sandboxed helpers, never in the browser. The browser still never decodes an image or parses a font | PM2, TA5, AC4; ADR-0004 |
| SC3 | **`unsafe` stays where ADR-0002 allows it, dependencies included.** Axiom's parsers, network policy, IPC decoding, and image and font wrappers contain no `unsafe` (rule 5). New FFI (CoreText, ImageIO, Security, Metal, AppKit's accessibility) lives in `platform` modules. A dependency's own `unsafe` and C code are accounted for under ADR-0010's rules | ADR-0002, evaluation §15.3; AC9 |
| SC4 | **Build or reuse follows the charter.** The core mechanisms of charter G1 are Axiom's own. Supporting infrastructure may be reused, each dependency with a written justification and a class: required platform facility, replaceable implementation detail, or foundational dependency. Dependencies stay within build thresholds T1–T4 | Charter G1 and principle 4; ADR-0002 |
| SC5 | **Standards first, and testable.** Each subsystem names the specifications it implements, dated, and the suites that will check it: WPT directories, html5lib-tests, test262, Unicode's conformance files. It keeps standards-compliant and approximate behaviour apart. Step 6 sets the baseline and the pass rates | Charter §8 and principle 5; X1, X2 |
| SC6 | **Efficiency is designed.** Each subsystem names its long-lived structures, with owners and size estimates, its threads (EL rules), and its idle behaviour, consistent with B1–B7 on the 8 GB M1 | Charter G4; AC3, AC7; PM7, PM8 |
| SC7 | **Incremental by design.** Each pipeline subsystem says what it caches, what invalidates it, and how IV14's oracle compares its incremental output with a rebuild from scratch | P3; IV1–IV14 |
| SC8 | **Deterministic and headless.** Each subsystem runs headless and in one process, with deterministic output under the test scheduler and virtual time (EL24). Where output depends on the OS, as with system fonts and codecs, the document says how tests stay stable | AC6 |
| SC9 | **The complete model fits.** Each subsystem names where its later app-web features will live without architectural change. For example: every script and writing mode; variable and color fonts; GPU canvas and video; HTTP/3, WebSockets and service-worker interception; Wasm, `SharedArrayBuffer` and `Intl` | AC8; ADR-0001 R1; charter principle 7 |
| SC10 | **Every decision traces to evidence:** a dated source, a measurement on this Mac, or a labelled inference or hypothesis that names the step that will test it | AC10; research conventions |

## 6. Sequence and checkpoints

**The order: the platform-facing subsystems first, then the spikes, then the document and script engine.**
1. **The dependency policy first (CP2).** Every later checkpoint chooses between building and reusing, so they all need one rule. That includes how a dependency's own `unsafe` counts against ADR-0002's boundary.
2. **Part 1, the platform-facing subsystems (CP3–CP8).** Text, graphics, images, networking, storage and accessibility meet Apple's frameworks (CoreText, Metal, ImageIO, Security, `NSAccessibility`), supporting libraries and sandbox rules. So they carry most of the dependency and placement decisions that are expensive to reverse, and most of the questions that may need measurement. Designing them first lets the spikes report before the engine's designs depend on them. Within part 1, the order is the decision map's (text, then rendering, then networking), extended:
   - **Text first:** inline layout, painting (the glyph atlas), HTML (encodings), URLs (IDNA) and JavaScript (`Intl`, `RegExp`) all depend on its Unicode and font decisions.
   - **Then graphics,** which consumes text's glyph decision and Step 4's display list, and then images, whose decoded pixels, color profiles and memory meet the compositor.
   - **Then networking,** before the engine because HTML parsing consumes its streams and its URL parser. Storage and workers follow it: storage lives in the network process, and service workers intercept its fetches.
   - **Accessibility last:** it is small, and consumes the accessibility-tree contracts PL12 and IN9.
3. **The spikes, between the parts (CP10–CP12),** so that their results arrive before part 2 uses them. S5's text results, for example, feed inline layout.
4. **Part 2, the document engine, in pipeline order (CP13–CP15):** HTML and the DOM, then style, then layout and paint. This is the static-document pipeline that milestones begin with ([ADR-0001](../adr/0001-scope-and-target.md), decision 1), and each stage consumes the one before.
5. **Script last (CP16–CP17):** the JavaScript engine, then the bindings, which join the engine's object model to the DOM. DOM objects are the JavaScript objects (SH1), so CP13 fixes how nodes are stored and leaves their JavaScript facet (prototypes, expando properties) as an explicit interface, which CP17 closes.

**Every checkpoint ends with:**
- every relative link resolving, checked by `tools/check-doc-links.py`. Planned documents are named in code font, not linked, until they exist;
- the four standard checks green: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `tools/check-unsafe-policy.sh`. Only the spike checkpoints change code;
- a commit on `step-5`, and an update to this tracker.

| CP | Scope | Status |
|---|---|---|
| **CP1** | This plan; branch `step-5`; the owner's decisions recorded | ✅ 2026-10-18 (§11). **Gate A passed:** approved as written |
| | **Part 1: the platform-facing subsystems** | |
| CP2 | Dependency policy and build-or-reuse rules (X4) | ✅ 2026-10-18 (§11; [policy](../technology/dependency-policy.md), [register](../technology/dependency-register.md)) |
| CP3 | Text, fonts and Unicode (I1, the text part of P4); spike S5's design | ✅ 2026-10-18 (§11; [document](../architecture/text-and-fonts.md); S5 design §10.1) |
| CP4 | Graphics: rasterization, compositing and presentation (I2); spike S6's design | ✅ 2026-10-18 (§11; [document](../architecture/graphics.md); S6 design §10.2) |
| CP5 | Images and media (I4) | ✅ 2026-10-18 (§11; [document](../architecture/images-and-media.md); S5-Q5 added to §10.1) |
| CP6 | Networking: URLs, HTTP, TLS, DNS, cache, cookies, Fetch's policy (I3); spike S7's design | ✅ 2026-10-18 (§11; [document](../architecture/networking.md); S7 design §10.3) |
| CP7 | Storage and workers | ✅ 2026-10-18 (§11; [document](../architecture/storage-and-workers.md)) |
| CP8 | Accessibility: the tree and the bridge (I5); the shell's questions for the owner | ✅ 2026-10-18 (§11; [document](../architecture/accessibility.md)) |
| CP9 | Synthesis of part 1: ADRs (Proposed); decision map, open questions, indexes, status. **Gate B** | ✅ 2026-10-18 (§11). **Gate B passed** 2026-10-18 |
| | **Spikes**, each only if authorized at Gate B | |
| CP10 | Spike S5: the text stack | ✅ 2026-10-18 (§11; [results](s5-results.md)). S5-Q3 passes; S5-Q1 and S5-Q2 await the owner's visual check; S5-Q5's condition is not met (IM3's fallback recommended for Gate C) |
| CP11 | Spike S6: compositing on Metal | In progress: code at `c2c8ad5`, trials run; **the formal run waits for the owner** (AC power, the Mac idle, the window visible) |
| CP12 | Spike S7: the network process and TLS | ✅ 2026-10-19 (§11; [results](s7-results.md)). S7-Q1, S7-Q3 and S7-Q4 pass, so ADR-0013 is accepted under the owner's Gate B approval; S7-Q2 settles NW6; verification off the I/O thread proposed for Gate C |
| | **Part 2: the document and script engine** | |
| CP13 | HTML parsing and the DOM | ✅ 2026-10-19 (§11; [document](../architecture/html-and-dom.md)). An amendment to ADR-0008 proposed for Gate C (HD11) |
| CP14 | Style | ✅ 2026-10-19 (§11; [document](../architecture/style.md)). An amendment to ADR-0007 proposed for Gate C (ST25, ST31) |
| CP15 | Layout and paint | ✅ 2026-10-19 (§11; [document](../architecture/layout-and-paint.md)). No amendment needed |
| CP16 | The JavaScript engine | Not started |
| CP17 | WebIDL bindings | Not started |
| CP18 | The overview refined, and the criteria check; ADRs for part 2 (Proposed); decision map, open questions, indexes, status. **Gate C** | Not started |
| CP19 | Merge `step-5` into `main` (`--no-ff`), when the owner instructs | Not started |

### 6.1 What each checkpoint decides

**CP2. Dependency policy (X4; Q1):**
- **The line between the core and supporting infrastructure.** Charter G1's mechanisms are Axiom's own; supporting infrastructure may be reused (principle 4). The policy works the line through Step 5's actual candidates: Unicode data, encodings, URL parsing, shaping, font parsing, rasterization, image codecs, the GPU interface, TLS, HTTP framing, compression, storage engines and accessibility.
- **What each dependency records:**
  - its purpose and class;
  - the alternatives;
  - its license and maintenance;
  - precedent in Chromium or Firefox;
  - the `unsafe` and non-Rust code inside it, and its transitive dependencies;
  - its build cost against T1–T4;
  - the process it runs in.
- **`unsafe` in dependencies:** how a dependency's `unsafe` or C code counts against ADR-0002's rules 2 and 5 and the Rule of 2. One example: a font parser that contains `unsafe`, running in a renderer.
- **The supply chain:**
  - the pinned `Cargo.lock`;
  - `cargo-deny` (advisories, licenses, bans, sources) and `cargo-vet` (audits), specified but not installed;
  - a license allow-list and minimal features;
  - how advisories reach a browser built locally. The routine itself is Step 6's (threat model §9).
- **Apple's frameworks** as required platform facilities: which ones, from which process (SC2), always through `platform`.
- **Enforcement:** how a check like `tools/check-unsafe-policy.sh` would cover dependencies. Designed only.

**CP3. Text, fonts and Unicode (I1, the text part of P4; Q2, Q3):**
- **Unicode for the whole engine:**
  - properties and script itemization;
  - segmentation (UAX #29), line breaking (UAX #14, with CSS Text's rules) and bidi (UAX #9);
  - normalization and case mapping;
  - IDNA for URLs.

  From ICU4X, the system's ICU, or tables of our own. One source serves text, CSS, URLs and JavaScript (`Intl`, `RegExp` property escapes).
- **The Encoding Standard's decoders and encoders,** which HTML, URLs and `TextDecoder` use. The sniffing algorithm is CP13's.
- **Fonts:**
  - the system font database, through CoreText or our own index;
  - CSS Fonts' matching, generic families, and fallback per script and per character;
  - `@font-face` loading (OpenType, WOFF, WOFF2), sanitizing, and memory;
  - variable and color fonts, and emoji;
  - S4's findings: system fonts and ICU's data must be readable; failures are silent, so fonts must be checked; user-installed fonts need reviewed rules.
- **Shaping:** CoreText, a Rust shaper such as HarfRust, or our own. Also the shaped-run interface that inline layout consumes (glyphs, advances, clusters, break opportunities, bidi levels), and caches of shaped words.
- **Glyph rasterization:**
  - where it happens: the renderer now, and whether a GPU process later (PL8);
  - with CoreText or a Rust rasterizer;
  - the atlas format and its memory (PL7);
  - subpixel positioning and font smoothing.
- **The display list:** text items as PL7 defines them, or an amendment for the owner (SC1).
- **Spike S5's design,** if the comparison needs measurement (§10).

**CP4. Graphics: rasterization, compositing and presentation (I2; Q4):**
- **The GPU interface:** Metal directly, through `objc2-metal` in `platform`, or a portability layer such as wgpu. Judged on the build thresholds, where the `unsafe` sits, and the later GPU process.
- **The compositor's renderer** for display lists (PL6–PL9):
  - property trees, rounded-rect clips and effects (opacity, blend modes, filters);
  - shadows and gradients;
  - glyph quads from atlases, images, and embedded child frames;
  - batching;
  - shaders, given that the Command Line Tools have no offline Metal compiler: [S2](s2-results.md) compiled them at run time, and macOS cached them.
- **Path rasterization,** for borders, SVG and, later, canvas 2D: on the CPU or the GPU, in the renderer or the compositor, and the anti-aliasing quality.
- **Color:** sRGB and Display P3, CSS Color 4's color spaces, images' ICC profiles (with CP5), and the window's color space.
- **Presentation:**
  - `CAMetalLayer`;
  - damage tracking and partial presentation (§2.2, item 5);
  - frame pacing against the browser's display link (EL19–EL22);
  - zero cost while idle (AC3).
- **Compositor animations** beyond `transform` and `opacity` (§2.2, item 6), with CP14.
- **Where canvas 2D, WebGL and WebGPU will live** (the later GPU process), and video frames, with CP5.
- **Memory:** surfaces, atlases, and the browser's budget (B1).
- Whether anything here needs Xcode (OQ-7).
- **Spike S6's design** (§10).

**CP5. Images and media (I4; Q5):**
- **Formats and decoders:**
  - PNG, JPEG, GIF, WebP, AVIF, BMP and ICO, and whether to support JPEG XL;
  - safe Rust decoders against ImageIO inside the renderer's sandbox (SC2, SC3);
  - which formats lack a safe decoder.
- **Decoding within budgets:**
  - incremental and progressive decoding, and animation;
  - decoding on demand and at display size;
  - discarding decoded pixels under memory pressure;
  - the decoded-image cache and its budget.
- **Color management,** with CP4: ICC profiles and coding-independent code points (CICP).
- **The decoder interface:** helper-pool threads (PL10), output in shared memory (PL7, PL8), and room to move decoding into its own process later.
- **SVG as an image,** with CP15.
- **Media, placement only:**
  - decoding in a utility process with VideoToolbox and AVFoundation (process model §3);
  - video frames as display-list resources (PL7);
  - audio output on real-time threads.

**CP6. Networking (I3; Q6):**
- **URLs, origins and sites:**
  - the WHATWG URL parser, shared by every process;
  - the Public Suffix List and how it is updated;
  - IDNA, with CP3.
- **HTTP:**
  - HTTP/1.1, HTTP/2 and HTTP/3 over QUIC;
  - connection pooling and prioritization;
  - the system's proxy settings;
  - WebSockets, with room for WebTransport.
- **TLS:**
  - the stack: rustls, or Apple's, which ADR-0004 keeps out of the browser;
  - certificate verification against the system's trust store, and revocation;
  - HSTS and its preload list;
  - session resumption.
- **DNS:** the system resolver or our own; DNS over HTTPS; a blocking resolver on the worker pool, if chosen.
- **The HTTP cache:** format, partitioning by top-level site (SI3), size and eviction.
- **Cookies:**
  - the current cookie specification (RFC 6265 and its revision);
  - SameSite, Lax by default (SG8);
  - partitioning, and third-party cookies blocked by default (SI3);
  - `HttpOnly` cookies never reach a renderer (SG2).
- **Fetch's policy, in the network process, from the browser's records** (SG4, IR2):
  - initiator locks;
  - CORS and preflights;
  - ORB, CORP and COEP;
  - mixed content and Referrer-Policy;
  - Private Network Access-style checks;
  - redirects;
  - content decoding (gzip, Brotli, zstd).
- **Bodies** streamed to renderers (FT11), with backpressure (IP16, IP17).
- **Spike S7's design** (§10).

**CP7. Storage and workers (Q7):**
- **Web storage, IndexedDB, the Cache API and quotas:**
  - in the network process (process model §3, note 2), keyed by storage partition (SI3);
  - the storage engine, our own or an embedded database reused, and its file format;
  - eviction, and the budget (B4).
- **Service workers:** how registrations are stored, how fetches are intercepted within Fetch, and their lifetime.
- **Where shared and service workers run** when no same-site renderer exists (§2.2, item 11).

**CP8. Accessibility (I5; Q8):**
- **The tree, in the renderer:** roles, names and states, from HTML-AAM, Core-AAM and the accessible-name computation. Built from the DOM and the box tree (PL12) and sent as incremental updates (IN9).
- **The bridge, in the browser:**
  - the mapping to `NSAccessibility` roles, attributes and actions, using Core-AAM's mappings;
  - stitching trees across frames;
  - text ranges and notifications;
  - AccessKit or our own (ADR-0010).
- **With the owner:**
  - the settings list, and how the sensitive-site list and the JIT switches are presented (§2.2, item 14);
  - which key equivalents a page may override (§2.2, item 13).

**CP9. Synthesis of part 1:**
- ADRs for part 1, Proposed.
- Updates to the decision map (I1–I5; X4, if added), the open questions, the indexes and status.
- **Gate B** (§7).

**CP10–CP12. Spikes S5–S7,** each only if authorized at Gate B:
- built and checked;
- measured by the methods of S1–S4 (§10);
- results written in `docs/research/sN-results.md`, and their ADRs updated.

An ADR that the owner pre-approved pending its spike becomes Accepted once its conditions are met.

**CP13. HTML parsing and the DOM (Q9):**
- **Parsing:**
  - the tokenizer and tree builder (WHATWG HTML), with the encoding-sniffing algorithm (with CP3);
  - input streamed from the network (FT11), and yielding (IV10);
  - `document.write`, fragment parsing, and speculative fetching by a preload scanner;
  - XML parsing, for XHTML and SVG documents.
- **The DOM in Pattern B arenas:**
  - node kinds, and link columns as struct-of-arrays (SH8);
  - attributes and atoms (PL11), and text storage;
  - node identity and lendability: no interior mutability (PL2).
- **Events:**
  - dispatch paths and listener storage;
  - the listener snapshot rooted as one heap array (§2.2, item 18);
  - default actions, and user activation (IN8).
- **The rest of the DOM:**
  - mutation observers, ranges and selection;
  - Shadow DOM and slots, and custom elements' reactions (their JavaScript side with CP17);
  - forms and their controls;
  - editing (`contenteditable`), with the IME mirror (IN7).
- **The DOM's JavaScript facet,** left as an interface for CP17 (SH1).
- **Oracles:** html5lib-tests, and WPT's HTML-syntax and DOM directories.

**CP14. Style (Q10):**
- **Parsing:** CSS syntax (the tokenizer, error recovery), and the model of properties and values, from specified to computed and used.
- **Selector matching** over the DOM's columns: right-to-left matching, ancestor filters, and the invalidation sets of IV3.
- **The cascade:**
  - origins, layers, specificity, `!important`, Shadow DOM's scoping, and inheritance;
  - custom properties and `var()`;
  - the cascade's caches: the rule tree and style sharing (PL3).
- **Invalidation,** IV3's details: what each mutation records, and what stylesheet changes do.
- **Queries and loading:** media and container queries, `@import`, and handing `@font-face` to CP3.
- **Animations and transitions:** timelines, and which ones run on the compositor (with CP4).
- **The CSSOM,** and `getComputedStyle` through IV2's synchronous entry point.
- **Atoms across threads** (PL11), and readiness for parallel style (PL2, PL10).

**CP15. Layout and paint (Q11):**
- **The layout unit,** fixed-point or float (PL5), and the coordinate model.
- **Box construction** from the DOM and computed styles (PL4): anonymous boxes, `display: contents`, replaced elements, and child frames (FT3).
- **Formatting contexts:**
  - block, with margin collapsing;
  - inline, with CP3's shaped runs, line breaking and bidi;
  - floats and positioning;
  - flex, grid, tables and multicolumn;
  - writing modes and logical properties;
  - intrinsic sizing.
- **Fragments:**
  - immutable, each with its constraints (PL5), reused when their inputs match (IV5), with containment;
  - fragmentation, for multicolumn and printing;
  - scrolling and overflow, with scroll containers as property-tree nodes (PL15).
- **Paint:**
  - stacking contexts and painting order (CSS 2, Appendix E);
  - display items and property trees (PL6, PL7);
  - backgrounds, borders and shadows;
  - SVG rendering.
- **Queries:** fine hit testing (IN2), and geometry for script (CSSOM View).
- **Oracles:** WPT's CSS directories, and reference tests through the headless embedder (EM9).

**CP16. The JavaScript engine (Q12):**
- **The front end:** the parser (ECMA-262, with early errors), lazy parsing of functions, scopes, and bytecode generation.
- **The interpreter:**
  - a register machine, as in S3;
  - dispatch by a `match` loop (§2.2, item 17);
  - operations written as functions over the machine state.
- **The value type:** NaN-boxed (SH13), and carrying the scope lifetime (§2.2, item 18), with Nova's `bind` and `unbind` design as the precedent to examine.
- **Objects:**
  - shapes, property storage and elements;
  - dictionary mode, prototypes and inline caches;
  - exotic objects: arrays, typed arrays, `Proxy`, module namespaces.
- **Strings:** 8-bit and 16-bit (PL11), ropes and slices, atoms.
- **Execution:**
  - functions and closures;
  - generators and async functions, suspended in a register machine;
  - the job queue (SH4);
  - modules, loaded through the host (FT11).
- **The standard library:**
  - builtins;
  - `RegExp`, with an engine of our own, backtracking or automaton-based;
  - `Intl`, over CP3's Unicode data;
  - `Date`, and where time-zone data comes from;
  - number formatting;
  - `ArrayBuffer` and `SharedArrayBuffer` backing stores outside the heap (SH3, SH11), and `Atomics`.
- **Errors and control:** errors and stack traces, the interrupt flag (SH5), and where a debugger would attach.
- **What a Wasm engine needs from this engine** (OQ-3, as evidence; the decision stays open), and room for a JIT (SH13, J1–J7).
- **The oracle:** test262.

**CP17. WebIDL bindings (Q13):**
- **The WebIDL compiler:** it runs at build time on the specifications' IDL, and its generated code is safe (ADR-0002 rule 7).
- **Platform objects in the heap,** closing CP13's interface:
  - prototypes and interface objects per realm;
  - expando properties and `[SameObject]`;
  - indexed and named properties.
- **Conversions and calls:**
  - argument conversion and overload resolution;
  - exceptions (`DOMException`) and promises;
  - callbacks, such as event listeners, and how they are rooted.
- **Rooting conventions for generated code** (ADR-0003; SH6, SH7): what each method roots across calls that may allocate. The hot path of event dispatch, with S1's dispatch case as a benchmark (§2.2, item 18).
- **Cross-origin objects:** `WindowProxy` and `Location`, and remote window proxies (FT5).
- **Realms and global objects** for windows and workers.

**CP18. The overview refined, and the synthesis of part 2:**
- The [overview](../architecture/overview.md) refined:
  - the crate structure and its `unsafe` map;
  - a summary of the dependency register;
  - end-to-end scenarios traced through the subsystems' rules;
  - the check of SC1–SC10.
- ADRs for part 2, Proposed. Updates to the decision map, the open questions, the indexes and status.
- **Gate C** (§7).

**CP19. Merge** `step-5` into `main` with `--no-ff`, when the owner instructs.

## 7. Review gates

Between gates, checkpoints run without pausing, as in Steps 2b to 4: CP2–CP9, then the authorized spikes and CP13–CP18.

| Gate | After | The owner decides |
|---|---|---|
| A | CP1 | **This plan:** its scope, including adding X4 (§2.3); the criteria SC1–SC10 (§5); the sequence and checkpoints (§6); these gates and the cadence; the candidate spikes (§10). **Also:** pushing `step-5` to `origin`. **Passed 2026-10-18:** approved as written; X4 added; S5–S7 confirmed as candidates; the cadence approved; `step-5` pushed |
| B | CP9 | **The ADRs for part 1** (Proposed). Each can be accepted, or pre-approved pending its spike, as ADR-0004 waited for S4 and ADR-0008 for S3. **Each candidate spike**, with its questions and pass criteria. **The shell's questions** from CP8: the settings, and the key equivalents. **Passed 2026-10-18:** ADRs 0010 and 0014 accepted; ADRs 0011–0013 pre-approved pending their spikes; S5, S6 and S7 authorized, with S6's thresholds confirmed; both shell questions approved as written; Axiom's license chosen (OQ-21) |
| C | CP18 | **The ADRs for part 2** (Proposed); any spike result that missed its condition; the refined overview; merging `step-5` into `main` |

**A spike result that misses its condition doesn't stop part 2.** The affected ADR stays Proposed until Gate C. The exception is a result that undermines an input part 2 depends on: then work stops at that point, and the owner decides.

## 8. Method

- **Sources** follow the [research conventions](README.md): primary sources first, each dated, every claim labelled.
- **Specifications,** dated when used:
  - WHATWG: HTML (parsing, rendering, the event loop), DOM, URL, Fetch, Encoding, Infra;
  - CSS: Syntax, Cascade, Selectors, Values, Display, Box, Inline, Text, Fonts, Writing Modes, Flexbox, Grid, Tables, Positioned Layout, Overflow, Fragmentation, Color, Backgrounds and Borders, Transforms, Compositing, Filters, Animations and Transitions, CSSOM and CSSOM View;
  - Unicode: UAX #9, #14, #15 and #29, UTS #46; OpenType and WOFF2;
  - ECMA-262 and ECMA-402; WebIDL;
  - the IETF's HTTP, QUIC, TLS 1.3, cookie and HSTS RFCs; the Public Suffix List;
  - W3C ARIA, Core-AAM, HTML-AAM, and the accessible-name computation.
- **Engines,** from their documentation and source:
  - Chromium: Blink's HTML parser, style engine, LayoutNG and paint; Skia and Viz; V8's Ignition interpreter, hidden classes and inline caches; its network stack and BoringSSL;
  - Firefox: Stylo, WebRender, SpiderMonkey, Necko and NSS, `encoding_rs`;
  - WebKit: WebCore, JavaScriptCore, and its use of CoreText and Apple's network frameworks;
  - Servo: html5ever, Stylo, its layout engine, WebRender and its networking;
  - Ladybird: LibWeb, LibJS, its graphics and its networking.
- **Rust projects** to evaluate or study, each read in source: those the [language evaluation](../technology/language-evaluation.md) §12 lists (ICU4X, HarfRust, `png`, `jxl-rs`, wgpu, AccessKit, rustls, hyper, quinn; Boa, Nova, Brimstone), and others the checkpoints find.
- **Apple:** developer documentation, and the SDK 27 headers on this Mac: CoreText, ImageIO, Metal, QuartzCore, Security, Network, and AppKit's accessibility protocols.
- **Measurements:** only in authorized spikes (§10).

## 9. Risks to this plan

| Risk | Mitigation |
|---|---|
| **Breadth:** twelve design documents, each over a large specification | Design only the decisions that are expensive to reverse (§1); each document names what it leaves to implementation. Each checkpoint is committed and can be resumed on its own |
| A subsystem needs a Step 4 interface changed | SC1: an amendment for the owner, recorded in the ADR that owns the interface; never changed silently |
| Build-or-reuse choices among fast-moving crates | ADR-0010's rules. Every crate fact dated; precedent in Chromium or Firefox preferred; a revisit condition for each dependency |
| The DOM's JavaScript facet spans CP13, CP16 and CP17 | CP13 states the interface it leaves, CP17 closes it, and CP18 reconciles them |
| Spikes growing into prototypes | Each spike answers named questions against pass criteria, and its code is thrown away. The owner authorizes each scope |
| Precedent documentation is incomplete or out of date | Date every source. Prefer source code where documentation lags. Never invent history |
| A design question needs evidence that no source settles | Label it a hypothesis, and propose a spike at the next gate. No spike runs without authorization |
| Results on text, images and TLS depend on the macOS version | Record the OS build with every observation, as S4 did. Step 6 decides how such checks repeat after updates (threat model §9) |

## 10. Candidate spikes

The owner asked which areas may need targeted validation spikes. Three candidates are planned, and a fourth only if CP16 needs it. **Nothing was authorized at Gate A. At Gate B (2026-10-18) the owner authorized S5, S6 and S7,** each for the questions it names (§2). Each candidate's design (its questions, method, pass criteria, crates and `unsafe`) is written in this section at the checkpoint named, and put to the owner at Gate B. Its code is written only after it is authorized.

| Spike | Area, and the checkpoint that designs it | Draft question | Why sources can't settle it | What it decides |
|---|---|---|---|---|
| **S5** | The text stack (CP3; design in §10.1), with images added at CP5 | How do CoreText and a Rust text stack (a Rust shaper such as HarfRust, a Rust font parser and rasterizer, ICU4X's segmenters) compare on a multi-script corpus, inside the renderer's sandbox, in shaping output, speed and memory per renderer? How can an untrusted web font load there safely? | [S4-Q5](s4-results.md) showed that CoreText draws correctly in the sandbox, not how it compares with anything. The [language evaluation](../technology/language-evaluation.md) §12 records HarfRust as "not a complete port". Memory per renderer on this Mac needs measuring: B2 allows 10 MB, and drawing with CoreText took S4's helper from 1.14 to 4.05 MiB | TX: shaping, font parsing and rasterization; where glyphs are rasterized |
| **S6** | Compositing on Metal (CP4; design in §10.2) | Can the browser's compositor draw display lists of validated primitives (property trees, rounded clips, glyph quads from an atlas, images, embedded child frames) on Metal within a 60 Hz frame budget, and with what memory and energy? Does partial presentation work? How do CPU and GPU path rasterization compare? | [S2](s2-results.md) measured an idle Metal window (20–21 MB, no wakeups while idle), not a compositing workload. No source measures these costs on an M1 | GR: the GPU interface, the compositor's renderer, path rasterization |
| **S7** | The network process and TLS (CP6; design in §10.3) | Does a deny-by-default network-process profile, allowing only outbound sockets, name resolution, certificate trust evaluation and its own data directory, support a TLS client that verifies against the system's trust store? Which rules does it need? What do handshakes and requests cost with each candidate stack? | S4 validated only renderer-like profiles. Which system services certificate evaluation needs inside a profile is the kind of fact S4 found only by trying: named profiles killed their process, and CoreText degraded silently without ICU's data | NW: the TLS stack and certificate verification; the network process's sandbox rules |
| (S8) | The JavaScript engine (CP16) | Only if CP16 leaves a question that sources can't settle, such as the cost of shape-based inline caches over Pattern B arenas in safe Rust | — | Proposed at Gate C, if needed |

**No spike is planned for these areas:**
- **HTML parsing:** its algorithms are specified, and html5lib-tests is the oracle.
- **The DOM:** S1 and S3 measured its heap pattern and node layouts.
- **Style, layout and paint:** their designs rest on specifications and engine precedent. Their performance questions need real pages, so they belong to implementation milestones.
- **The bindings:** generated code; S1-Q2 and S3-Q5 counted the ceremony.
- **Storage.**
- **The accessibility bridge:** the shell milestone first validates objc2-defined accessibility classes (EM11).
- **Images:** they joined S5 at CP5 (S5-Q5), because [images and media](../architecture/images-and-media.md) IM3 relies on ImageIO inside the renderer's sandbox, which S4 didn't test.

**The methods are S1–S4's:**
- release builds;
- three process runs with cool-downs;
- AC power, with Low Power Mode off (the lesson of Step 4's CP14);
- a clean tree for formal runs;
- results under `target/`, and nothing left outside the repository. Anything a spike leaves behind is cleaned up and noted.

### 10.1 Spike S5: the text stack (designed at CP3)

**Purpose.** Settle TX12: whether system fonts are rasterized by CoreText or by a Rust rasterizer. Also check the rest of the text design on this Mac:
- HarfRust against CoreText, on system fonts;
- the web-font path (TX7), entirely in safe Rust inside the renderer's sandbox;
- the stack's memory per renderer;
- **added at CP5:** ImageIO inside the renderer's sandbox, which [images and media](../architecture/images-and-media.md) IM3 relies on for AVIF, compared with the safe Rust image decoders.

The design is in [text and fonts](../architecture/text-and-fonts.md). Until S5 runs, or if it isn't authorized, CoreText rasterizes system fonts (TX12).

| ID | Question | Passes if |
|---|---|---|
| S5-Q1 | **Shaping.** The corpus covers Latin, Greek, Cyrillic, Arabic, Hebrew, Devanagari, Bengali, Tamil, Thai, Khmer, Myanmar, Han (simplified, traditional and Japanese), Hangul and emoji sequences, each in the system font CoreText's fallback chooses. Do HarfRust's glyphs, clusters, advances and offsets match CoreText's? | No script shows a shaping error in HarfRust's output, meaning wrong glyphs or misplaced marks. Errors are judged against the font's own rules and the owner's visual check of rendered pairs. Differences that both engines' rules allow are reported, not failed |
| S5-Q2 | **Rasterization.** For the corpus's glyphs at 12–32 px, at 1× and 2×, how do a Rust rasterizer's masks compare with CoreText's, with and without macOS's font smoothing? What does a glyph cost each way? | Report: pixel differences, timings, and the owner's visual comparison. TX12's decision follows from it |
| S5-Q3 | **Web fonts.** Inside S4's renderer profile, do WOFF and WOFF2 fonts, valid and deliberately malformed, decode, validate, shape and rasterize through the Rust path alone? | Every valid font renders. Every malformed one is rejected, or renders, without a crash or a hang. The profile needs no new rule. No CoreText call receives web-font bytes |
| S5-Q4 | **Footprint and speed.** What does the stack add to a sandboxed helper's footprint after shaping and rasterizing the corpus, in three configurations: CoreText only; HarfRust with CoreText rasterization; all Rust? What do ICU4X's compiled data and the shaper add to the executable's size? How fast is shaping, per glyph? | Report. The configuration chosen must leave B2 achievable: a ready renderer in at most 10 MB |
| S5-Q5 | **Images in the renderer's sandbox** (added at CP5). Under S4's renderer profile, does ImageIO decode AVIF, and which rules does it need? How do the safe Rust decoders (`png`, `jpeg-decoder` without its `unsafe` paths, `gif`, `image-webp`) compare with ImageIO on a set of test images, in time per megapixel and peak memory? | ImageIO decodes AVIF under the profile, with any new rule reviewed by S4's method. Otherwise AVIF waits (IM3). The comparison is reported, and settles IM2's choice for JPEG |

**What would be built** (after authorization only):
- `spikes/s5/text` (`s5-text`):
  - its CoreText and CoreGraphics calls in `src/platform/`, allowlisted as S4's `s4-text` is;
  - the Rust path: `read-fonts`, `skrifa`, HarfRust, `brotli-decompressor`, and one or two rasterizers;
  - for S5-Q5, ImageIO's calls in `src/platform/`, and the safe Rust image decoders `png`, `jpeg-decoder`, `gif` and `image-webp`;
  - WOFF and WOFF2 container decoding of its own, in safe Rust.
- **Sandboxing:** S4's tooling (`s4-sandbox`, `tools/s4-sign.sh`) runs it under a renderer profile.
- **Inputs:** a text corpus in `spikes/s5/corpus`. System fonts are used where they are, never copied. Test web fonts come under licenses the owner approves at Gate B: WPT's test fonts, for example, or fonts under the OFL.
- **Dependencies:** these would be the first crates.io crates a spike uses beyond objc2. Each is justified in the manifest (DP23), and the owner's authorization covers them.
- **Method:** release builds; three process runs with cool-downs; AC power with Low Power Mode off; a clean tree for formal runs; `/usr/bin/time -l` for footprints.

**Outputs:** `docs/research/s5-results.md`, TX12 settled, and ADR-0011 updated.

### 10.2 Spike S6: compositing on Metal (designed at CP4)

**Purpose.** Measure GR2–GR6 on this Mac before any of the compositor is written:
- the cost of drawing display lists into tiled CoreAnimation layers with Metal;
- what damage-limited drawing saves;
- scrolling and animating by changing layers;
- path filling on the CPU or the GPU, which decides whether GR5's amendment to PL7 goes to the owner;
- memory against B1, and the color tagging of layers.

The design is in [graphics](../architecture/graphics.md). The time thresholds below were proposed for the owner to confirm, as S3-Q2's 10% threshold was confirmed at Step 4's Gate B. **The owner confirmed them at Gate B** (2026-10-18): "typical update ≤ 4 ms GPU / ≤ 2 ms compositor thread; scrolling/transforms ≤ 1 ms."

| ID | Question | Passes if |
|---|---|---|
| S6-Q1 | **Compositing cost.** Synthetic display lists shaped like pages are drawn with GR3's pipelines into tiled IOSurface layers of a 2,560 × 1,600 window. They hold text as glyph quads, rectangles, rounded clips, borders, shadows, gradients, images, and two nested frames. What do a full redraw and a typical update cost in GPU and CPU time per frame? | A typical update takes at most a quarter of a 60 Hz frame on the GPU (≤ 4 ms), and ≤ 2 ms on the compositor thread. With nothing changing, the process makes no wakeups and no commits |
| S6-Q2 | **Damage.** For a small animated region (Firefox's spinning-square case), and for typing into a field, what do damage-limited tile redraws save over full redraws, in GPU time and in the energy impact `top` reports? | Only the tiles that damage touches are redrawn (counted), and the saving is reported |
| S6-Q3 | **Scrolling and animation by layers.** What does scrolling a long page cost per frame, and animating transform and opacity, by changing layers (GR8)? | No tile is redrawn except newly exposed ones, and the compositor thread spends ≤ 1 ms per frame |
| S6-Q4 | **Paths.** On path-heavy content (icons, a chart, a map tile), how do two approaches compare in time, memory and quality? CPU filling in a renderer-like process (`vello_cpu` or `tiny-skia`), uploaded as images; GPU filling in the compositor, by tessellation or by sparse strips | Report. It decides whether GR5's amendment goes to the owner |
| S6-Q5 | **Memory and color.** What do the Metal device, the tiles and the pipelines cost for one window? Does CoreAnimation honour Display P3 tagging on the IOSurfaces (GR6)? | The compositor's footprint leaves B1 achievable: the browser at ≤ 60 MB, idle, with one window. A known test pattern displays correctly in P3 |

**What would be built** (after authorization only):
- `spikes/s6/compositor` (`s6-compositor`):
  - an AppKit window through objc2, as S2's was; Metal through `objc2-metal`; QuartzCore and IOSurface;
  - its `unsafe` in `src/platform/`, allowlisted;
  - GR3's pipelines, as Metal shading language source compiled at run time;
  - generated display lists, so that no renderer process is needed.
- **For S6-Q4:** CPU path filling in a renderer-like process or thread, and a GPU path filler in the compositor.
- **Measurements:**
  - command buffers' GPU timestamps, and CPU timers;
  - `top -stats power` for energy impact, which works without root (observed 2026-10-18);
  - `/usr/bin/time -l` for footprints;
  - Metal's developer HUD for spot checks.
- **Method:** release builds; three process runs with cool-downs; AC power; a clean tree; the window visible on the built-in display. During formal runs the owner is asked to leave the Mac otherwise idle, since S3's addendum ran with the Mac in use.
- **Dependencies:** objc2's framework crates, already decided, plus S6-Q4's path-rasterizer candidates, each justified in the manifest (DP23).

**Outputs:** `docs/research/s6-results.md`; GR5 settled, or its amendment proposed; ADR-0012 updated.

### 10.3 Spike S7: the network process and TLS (designed at CP6)

**Purpose.** Validate the network process's sandbox (NW17), which S4 never tested: S4 covered only renderer-like profiles. Settle NW6 by comparing rustls, verified by Security's trust evaluation, with Apple's Network framework. Check NW1 and NW7: one I/O thread, driving every protocol from one wait. The design is in [networking](../architecture/networking.md).

| ID | Question | Passes if |
|---|---|---|
| S7-Q1 | **Confinement.** Under a deny-by-default profile, compiled by a broker as S4's are, a helper's TLS client verifies servers through `SecTrustEvaluateWithError`. The profile allows outbound TCP and UDP, the system's DNS service, certificate trust evaluation and a data directory. Does verification work, and which rules exactly does it need? | Handshakes to public test hosts verify. Bad certificates (expired, self-signed, wrong host) are rejected. The profile needs no rule outside a reviewed list. Denials are logged and explained |
| S7-Q2 | **TLS stacks compared.** rustls with `aws-lc-rs`, verified through Security, against Network framework's TLS, under the same profile. What do they cost in handshake latency (full and resumed), CPU per megabyte on a large download, and footprint? | Report. It settles NW6, together with how each fits NW1 |
| S7-Q3 | **DNS-SD in the sandbox.** Do `DNSServiceGetAddrInfo` and `DNSServiceQueryRecord` (HTTPS records), driven through `DNSServiceRefSockFD` in `kqueue`, work under the profile? | Addresses and HTTPS records resolve, asynchronously, with no thread blocked |
| S7-Q4 | **One wait.** Does one I/O thread drive rustls, `quinn-proto` (one QUIC connection), DNS-SD, a Mach port and timers from a single `kevent64` call? | Everything progresses from that one wait, and the process makes no wakeups while idle |

**What would be built** (after authorization only):
- `spikes/s7/net` (`s7-net`):
  - a broker and a sandboxed helper, reusing S4's tooling (`s4-sandbox`, `tools/s4-sign.sh`);
  - its calls into Security, Network framework and DNS-SD in `src/platform/`, allowlisted;
  - rustls with the `aws-lc-rs` provider, and `quinn-proto`;
  - a minimal HTTP/1.1 client of its own, enough to fetch.
- **Network access:** to public test hosts, by read-only requests. That is outward-facing, so the owner's authorization covers it explicitly. No certificate is added to the system's keychain, and nothing on the Mac is reconfigured.
- **Measurements:** Wi-Fi timings vary, so latencies are reported as medians over many handshakes, and as indicative. CPU and footprint come from the helper's own counters and `/usr/bin/time -l`.
- **Method:** release builds; three process runs with cool-downs; AC power; a clean tree.
- **Dependencies:** rustls, `aws-lc-rs` (which compiles C and assembly at build time, so its build cost against T1–T4 is measured as part of S7), and `quinn-proto`, each justified in the manifest (DP23).

**Outputs:** `docs/research/s7-results.md`; NW6 settled; the network process's profile written down; ADR-0013 updated.

## 11. Results log

### CP1 — completed 2026-10-18

- Branch `step-5` created from `main` at `c642ab3`.
- This plan written: 14 questions; the subsystem criteria SC1–SC10; 19 checkpoints in two parts, with the spikes between them; three review gates; and three candidate spikes (S5–S7), with a fourth only if CP16 needs one.
- [Status](../status.md), the [decision map](../decision-map.md) (Step 5 in progress) and the indexes ([docs](../README.md), [research](README.md)) updated.
- The four standard checks and the link check pass, with the code unchanged.

**Gate A.** The owner reviews this plan. The decisions, with recommendations, are in [status](../status.md), "Next steps".

### Gate A — passed 2026-10-18

The owner's decisions, each as recommended, quoted in full in §2:
- **The plan:** "Approved as written." "Step 4 interfaces remain binding under SC1."
- **X4:** "Approved." Added to the [decision map](../decision-map.md), to be decided at CP2 and recorded in ADR-0010.
- **Candidate spikes:** S5, S6 and S7 "are approved as candidate spikes, with code authorization to be decided individually at Gate B."
- **Cadence:** "Proceed through CP2–CP9 without pausing until Gate B."
- **The branch:** `step-5` pushed to `origin`, and kept pushed as checkpoints land.

CP2 starts.

### CP2 — completed 2026-10-18

[Dependency and supply-chain policy](../technology/dependency-policy.md) written (**Proposed**; rules DP1–DP23). Its sources: Chromium's crate groups and audit criteria, Google's auditing standards, Mozilla's `cargo-vet` and Firefox's configuration, Servo's `cargo-deny` configuration, and Ladybird's change of policy. Main proposals:
- **The line:**
  - charter G1's mechanisms and the security policies are Axiom's own;
  - the decision map's infrastructure layer, Unicode data, encodings, URL parsing, compression and storage engines may be reused, behind interfaces Axiom owns;
  - Axiom never writes its own cryptography;
  - a dependency must save substantial, specialized work.
- **Classes and records:** every dependency has a class (DP5) and an entry in a new [register](../technology/dependency-register.md) that covers its whole transitive tree.
- **`unsafe` and C code in dependencies:**
  - each crate gets a soundness grade on Google's `ub-risk` scale, where `ub-risk-3` and `ub-risk-4` are not admitted;
  - each gets a group after Chromium's (`trusted`, `sandboxed`, `build`, `test`), inherited transitively;
  - a reused parser of untrusted input must be pure Rust at `ub-risk-1` or better. Anything weaker is memory-unsafe code under the Rule of 2: it may run only in sandboxed helpers, only with no better option, and only with an ADR.
- **The supply chain:**
  - crates.io only, with `--locked` builds;
  - `cargo-vet` audits, importing the organizations Firefox imports;
  - `cargo-deny`'s four checks;
  - a license allow-list that keeps Axiom's own license open;
  - minimal features, and costs measured against T1–T4.
- **Apple's frameworks:** a table of the processes that may use each one.
- **Enforcement:** `cargo-deny` and `cargo-vet` become pre-commit checks once the owner authorizes installing them. A `tools/check-dependencies.py` is designed.

**Found along the way:**
- **The repository has no license file,** so Axiom's own license is unchosen (observed). It is raised for Gate B; CP9 adds it to the open questions.
- **Chromium's `cargo-vet` audits file was last changed on 2025-03-22,** and its current import guide doesn't mention `cargo vet`. Its crate groups are current (observed through GitHub's API).

The register starts with what accepted ADRs already decided: three groups of platform facilities, and objc2 (ADR-0009). The spikes' crates are listed as exempt.

Every relative link resolves; the four standard checks pass.

### CP3 — completed 2026-10-18

[Text, fonts and Unicode](../architecture/text-and-fonts.md) written (**Proposed**; rules TX1–TX21). Its sources: the text code of Chromium, Firefox, WebKit and Servo; Chrome's account of moving web fonts to Skrifa; the HarfRust, ICU4X, `encoding_rs` and `unicode-bidi` projects; Google's and Chromium's published audits; this Mac's SDK headers and fonts. Main proposals:
- **ICU4X is the engine's one Unicode source,** with compiled data, for text, CSS, URLs and JavaScript. Firefox's breaking runs on it, and Chromium vendors it. The SDK has no public headers for the system ICU's segmentation, bidi or normalization (observed).
- **Encodings are Axiom's own, in safe Rust.** Google grades `encoding_rs` at `ub-risk-3`, which DP8 bars.
- **Web content sees system fonts and web fonts only.** User-installed fonts stay hidden, for fingerprinting and SG1.
- **Fonts are found through CoreText's database and fallback cascades,** as Chromium does on macOS. **HarfRust shapes every font,** including the 165 AAT-only faces of this Mac's 786 system faces. Chromium and Firefox shape with HarfBuzz on macOS; Firefox's CoreText shaping is off by default (observed).
- **Web fonts never reach CoreText:** our own WOFF and WOFF2 containers, `read-fonts`, HarfRust and a Rust rasterizer. So no C parser sees untrusted font data, where Chrome on macOS still hands web fonts to the system after OTS.
- **Glyphs are rasterized in the renderer into atlases,** so PL7 stands unchanged. **For system fonts, S5 decides** between CoreText and a Rust rasterizer. If a GPU process ever rasterizes paths, renderers send glyph outlines, so fonts still never leave the renderer.
- **Line breaking and segmentation come from ICU4X, and bidi from `unicode-bidi`;** hyphenation is staged.

**Spike S5 designed** (§10.1): four questions, on shaping against CoreText, rasterization, web fonts in the sandbox, and footprint.

**The dependency policy's DP10 refined.** CP2 required `ub-risk-1` across the whole tree for reused parsers. Google grades `bytemuck`, `zerovec` and `crc32fast` at `ub-risk-2`, and they sit under every Rust font stack, ICU4X and `png`. So the rule excluded every Rust font parser, while CoreText, which is C, stayed admissible through the exception: the reverse of its intent. DP10 now asks `ub-risk-2` across the tree in sandboxed helpers, and `ub-risk-1` in the browser, and confines the exception to C, C++ and assembly. The policy records the change.

**Found:** `smallvec`, which HarfRust and ICU4X's segmenter require, was graded `ub-risk-3` by Google at 1.11.0, and later versions carry no soundness grade. It is recorded as an open risk in the register.

The [register](../technology/dependency-register.md) gains CP3's proposed entries, and a section for crates considered and not admitted. Every relative link resolves; the four standard checks pass.

### CP4 — completed 2026-10-18

[Graphics: rasterization, compositing and presentation](../architecture/graphics.md) written (**Proposed**; rules GR1–GR14). Its sources: Firefox's Core Animation work and its native layer code; Chrome's Skia Graphite announcement and its CALayer and animation code; WebKit's remote layer tree; Vello; Apple's guidance on tile-based rendering; the SDK's QuartzCore, Metal and IOSurface headers; this Mac's GPU and display. Main proposals:
- **Metal directly, through `platform`.** No portability layer in the browser process: cross-platform support is a non-goal, and wgpu would bring a large `unsafe` tree into the most privileged process. wgpu remains a candidate for WebGPU in the GPU process.
- **The window server composites retained layers; Axiom draws tiles,** with Metal, into IOSurfaces of its own:
  - only damaged tiles are redrawn;
  - scrollers, compositor-animated subtrees, child frames and video get layers of their own.

  Firefox measured large power savings from this design on macOS (for example, an idle Google Docs at 1.6 W instead of 7.4 W, on a 2013 Intel MacBook Pro). Chromium and WebKit also present through layer trees.
- **Our own pipelines draw display items:** few of them, compiled from source at first use and kept in a binary archive, and frugal with the M1's tile memory.
- **Paths are rasterized in the renderer until S6 reports.** If GPU path filling in the compositor pays, an amendment adding path items to PL7 goes to the owner (SC1).
- **Color:** Display P3 at 8 bits per channel per window by default, matching this Mac's panel; HDR later, per layer.
- **Nothing runs while nothing changes.** Scrolling and compositor animations change layers, not pixels.
- **Compositor animations:** the transform family and opacity first; then `filter`, `backdrop-filter` and scroll-driven timelines. This answers invalidation open question 2.
- **The compositor thread draws and commits,** and commits on the main thread during live resize, as Firefox does. The headless embedder composites the same tree offscreen, without CoreAnimation, so reference tests are deterministic.

**Spike S6 designed** (§10.2): five questions, on compositing cost, damage, layers for scrolling and animation, CPU or GPU paths, and memory and color. Its time thresholds are for the owner to confirm.

Every relative link resolves; the four standard checks pass.

### CP5 — completed 2026-10-18

[Images and media](../architecture/images-and-media.md) written (**Proposed**; rules IM1–IM13). Its sources: Chromium's and Firefox's image decoders, preferences and media processes; WebKit's ImageIO decoder and GPU-process media; Ladybird; the decoder crates' `unsafe` policies; Google's published grades; the SDK's ImageIO headers. Main proposals:
- **Images decode in the renderer,** on its helper pool, into shared memory. The browser decodes nothing, not even favicons, because the decoders' trees are graded `ub-risk-2`, below what DP9 asks of the browser.
- **Safe Rust decodes every format with an admissible decoder:**
  - `png`, `gif`, `image-webp` and `jxl`;
  - a safe JPEG decoder, since `zune-jpeg` is graded `ub-risk-3`;
  - BMP and ICO decoders of our own.

  Chromium already decodes PNG and JPEG XL in Rust, and Firefox decodes JPEG XL in Rust.
- **AVIF goes to ImageIO in the renderer,** under DP10's exception, accepted by ADR-0012, because no AV1 decoder in pure Rust meets the policy. Unlike a vendored dav1d, ImageIO is patched by macOS updates.
- **Budgets:** incremental decoding; decoding at display size; discarding the pixels of images that aren't visible, as Firefox does; animations decoded frame by frame and only while visible; limits against hostile dimensions.
- **Color management** during decoding, with `moxcms` built without its SIMD `unsafe`.
- **Media** in a sandboxed utility process:
  - demuxers in safe Rust;
  - VideoToolbox and AudioToolbox for what the hardware and Apple's codecs cover;
  - video frames delivered as IOSurfaces, and audio output from the same process.

  Firefox, Chromium and WebKit all decode media outside their content processes.

**S5 gains S5-Q5** (§10.1): ImageIO under the renderer profile, compared with the safe decoders, as §10 foresaw.

Every relative link resolves; the four standard checks pass.

### CP6 — completed 2026-10-18

[Networking](../architecture/networking.md) written (**Proposed**; rules NW1–NW18). Its sources: Servo's, Chromium's and Firefox's network stacks; the rustls, `rustls-platform-verifier` and Quinn projects; the Fetch Standard; the IETF's cookie draft; the ORB proposal; Chrome's Local Network Access; Google's published grades; the SDK's Network, Security and DNS-SD headers. Main proposals:
- **Every protocol is a state machine without I/O of its own,** driven by the network process's I/O thread from one `kevent64` wait. That keeps one wait per thread and deterministic tests. rustls and `quinn-proto` already have that shape; tokio, which Servo uses, is rejected.
- **One URL parser, Axiom's own, in every process,** so that no two processes can parse a URL differently. Its IDNA goes through ICU4X in renderers, the network process and the address bar. In the browser, URLs from other processes must be canonical ASCII, which settles CP3's IDNA question.
- **HTTP/1.1, HTTP/2, HTTP/3 framing, HPACK and QPACK are Axiom's own.** Google grades `httparse` at `ub-risk-4`, and the reusable HTTP/2 and HTTP/3 crates are async-bound. QUIC is `quinn-proto`.
- **TLS is rustls, with the system verifying certificates** through Security's trust evaluation (SG7). Its `aws-lc-rs` crypto, which is C and assembly, is admitted under DP10's exception in the sandboxed network process, with ADR-0013.
- **DNS is the system's, through DNS-SD,** asynchronous and with HTTPS records, so no blocking resolver is needed.
- **Partitioning:** connections, TLS sessions, the cache and cookies are partitioned by Fetch's network partition key, and third-party cookies are blocked.
- **Fetch's policy is enforced from the endpoint's bound facts:**
  - CORS, CORP and COEP;
  - ORB before any byte reaches a renderer;
  - mixed content, Referrer-Policy, HSTS;
  - Local Network Access, the permission model that replaced Private Network Access in Chrome.

  That answers ADR-0004's questions on initiator locks, ORB and the TLS stack.

**Spike S7 designed** (§10.3): four questions, on the network process's sandbox, rustls against Network framework, DNS-SD in the sandbox, and one wait for every protocol.

Every relative link resolves; the four standard checks pass.

### CP7 — completed 2026-10-18

[Storage and workers](../architecture/storage-and-workers.md) written (**Proposed**; rules SW1–SW13). Its sources: the Storage Standard; Chromium's IndexedDB-on-SQLite PSA, its `localStorage` cache and its service-worker connector; Firefox's and WebKit's storage on SQLite; the `redb` project. Main proposals:
- **All web storage lives in the network process,** behind endpoints bound to the storage key: the origin and the top-level site.
- **One engine:** `redb`, pure Rust, if its audit grades it `ub-risk-2` or better, with large values in files and a database per storage key. SQLite, which every other engine uses or is moving to, is the fallback, under DP10's exception. Crash tests at the first storage milestone decide between them. The HTTP cache and the cookie jar use the same engine, which settles networking's open question 2.
- **`localStorage` is cached in the renderer,** loaded by one synchronous request, as Chromium does.
- **IndexedDB values arrive serialized,** so the network process never parses a script value.
- **Quotas and eviction** follow the Storage Standard's buckets.
- **Service workers:** navigations are intercepted in Fetch's "handle fetch" step, in the network process; controlled documents reach their worker over a channel the browser creates.
- **Shared and service workers run in a renderer locked to their site.** The browser launches one when none exists, and the site's documents then reuse it. That answers the handed-over question 11.

No spike: the storage engine is decided by crash tests at its milestone, as §10 planned.

Every relative link resolves; the four standard checks pass.

### CP8 — completed 2026-10-18

[Accessibility](../architecture/accessibility.md) written (**Proposed**; rules AX1–AX12, and two questions for the owner). Its sources: Core-AAM and HTML-AAM; Chromium's, WebKit's and Firefox's macOS bridges; Chromium's reserved commands; AccessKit; the SDK's HIServices headers. Main proposals:
- **The renderer builds each document's tree** under HTML-AAM, Core-AAM and the accessible-name computation, once an assistive technology first asks. It pushes the whole tree, then updates, Chromium's model.
- **The browser answers every query from its cache,** stitches frames by token only to its own frame records, and routes actions as input.
- **Text navigation uses macOS's text-marker API.** SDK 27's headers document it for "custom web browser engines", and all three engines implement it. The renderer precomputes word, sentence and line boundaries, so ICU4X never runs in the browser on a renderer's text.
- **The bridge is Axiom's own,** in the shell's `platform` module. AccessKit's macOS adapter lacks rich text and hypertext.
- **System accessibility settings reach pages as media features.**

**For the owner at Gate B:**
- the settings list, including how the sensitive-site list and the JIT switches appear;
- reserved key equivalents: Chromium's reserved set, plus ⌘L and ⌘,, reserved in fullscreen too.

Every relative link resolves; the four standard checks pass.

### CP9 — completed 2026-10-18

**ADRs written, all Proposed:**
- [ADR-0010](../adr/0010-dependency-and-supply-chain-policy.md): the dependency and supply-chain policy (X4), with CP3's refinement of DP10.
- [ADR-0011](../adr/0011-text-fonts-and-unicode.md): text, fonts and Unicode (I1), recommended for acceptance once S5-Q1 and S5-Q3 pass.
- [ADR-0012](../adr/0012-graphics-images-and-media.md): graphics, images and media (I2, I4), recommended once S6-Q1, S6-Q3 and S6-Q5 pass and S5-Q5 checks ImageIO. GR5's amendment to PL7 is a separate decision, after S6-Q4.
- [ADR-0013](../adr/0013-networking-storage-and-workers.md): networking, storage and workers (I3), recommended once S7-Q1, S7-Q3 and S7-Q4 pass. The storage engine is settled at its milestone by crash tests.
- [ADR-0014](../adr/0014-accessibility.md): accessibility (I5), with the settings list and the reserved key equivalents for the owner.

**Updates:**
- [Decision map](../decision-map.md): I1–I5 and P4's text part Proposed, with their ADRs; X4 points to ADR-0010; Step 5's row at Gate B.
- [Open questions](../open-questions.md): **new OQ-21,** Axiom's own license, with a recommendation; OQ-7 notes that the graphics design needs no Xcode.
- The indexes, the [register](../technology/dependency-register.md) (complete for part 1), and [status](../status.md).

Every relative link resolves; the four standard checks pass.

**Gate B.** The owner decides on the five ADRs, the three spikes, the two shell questions and OQ-21, as listed with recommendations in [status](../status.md), "Next steps".

### Gate B — passed 2026-10-18

The owner's decisions, each as recommended, quoted in full in §2:
- **ADR-0010** (the dependency policy) and **ADR-0014** (accessibility): accepted. The [policy](../technology/dependency-policy.md) and the [accessibility design](../architecture/accessibility.md) are binding.
- **ADRs 0011, 0012 and 0013:** pre-approved, each to move to Accepted once its spike conditions pass (S5-Q1 and S5-Q3; S6-Q1, S6-Q3, S6-Q5 and S5-Q5; S7-Q1, S7-Q3 and S7-Q4).
- **Spikes S5, S6 and S7:** authorized for their full scopes (§10.1–§10.3), including S5's crates and test fonts under the OFL or WPT's licenses, and S7's read-only requests to public test hosts. S6's thresholds are confirmed.
- **The shell's questions:** the settings list and the reserved key equivalents, approved as written.
- **OQ-21:** Axiom's license is "MIT OR Apache-2.0". `LICENSE-MIT` and `LICENSE-APACHE` are added at the repository root, and the workspace manifest declares the license.

Recorded in the ADRs' decision histories, the design documents, the register, the [decision map](../decision-map.md), the [open questions](../open-questions.md), the indexes and [status](../status.md). CP10 starts.

### CP10 — completed 2026-10-18

**Spike S5, run and reported** ([S5 results](s5-results.md)), under the owner's authorization quoted in §2: "Spike S5 (Text Stack & ImageIO Sandbox): Authorized for questions S5-Q1 through S5-Q5 under spikes/s5/, including the listed candidate crates and OFL/WPT test web fonts."
- **One deviation from §10.1:** S5 is three crates (`s5-rust-text`, `s5-text`, `s5-image`), not one, so that the web-font executable links no Apple framework and S5-Q3's last condition can be checked with `otool -L` (results §1). The authorization's scope is unchanged.
- **Formal run** at `bad07a8`, from a clean tree, on AC power with Low Power Mode off: `target/s5-results/20261018-204642/`. It reproduced an earlier formal run at `f20e7f4` byte for byte in every deterministic output. Three timed runs after 60 s cool-downs; the Mac was in normal use, not idle (results §10).

**Answers:**
- **S5-Q1:** no shaping error in 432 runs: HarfRust chose the same glyphs as CoreText in every run. The 14 runs whose positions differ are Hebrew niqqud in Lucida Grande, which has no mark-positioning data, so both engines fall back. **The criterion includes the owner's visual check of the rendered pairs, outstanding.**
- **S5-Q2:** reported. The Rust rasterizers match CoreText above 20 px and at every 2× size; at 12–16 px at 1×, and in smoothing and emoji, CoreText differs; it is also the fastest. **Recommended: CoreText for system fonts (TX12),** settled once the owner has compared the sheets.
- **S5-Q3: passes.** 322 WPT fonts as expected, sandboxed and unsandboxed alike; 4,000 mutants with no crash or hang; no sandbox denial; no Apple framework linked.
- **S5-Q4:** B2 stays achievable: a first text costs 0.75–1.31 MiB over a 1.77 MiB helper. HarfRust shapes in 566 ns a glyph, against CoreText's 1,227 on the same runs.
- **S5-Q5: the condition is not met.** ImageIO crashes in the renderer profile until R1 is added, and AVIF then needs R2–R6, of which R4 opens VideoToolbox's decoder service, which the [process model](../security/process-model.md) §4.1 denies renderers. **Recommended for Gate C:** IM3's fallback, AVIF in the media utility process. **IM2's JPEG choice:** the safe decoder stays.

**The conditional approvals** (owner's Gate B words quoted in §2):
- **ADR-0011** ("will move to Accepted upon S5-Q1 and S5-Q3 passing, with TX12 settled by S5-Q2"): S5-Q3 has passed. S5-Q1's and S5-Q2's criteria include the owner's visual checks, so the ADR stays Proposed until the owner has looked at the sheets.
- **ADR-0012** ("S5-Q5 validating ImageIO inside the renderer sandbox"): not met, so it stays Proposed until Gate C. Under §7's rule, this doesn't stop part 2: no part 2 input depends on where AVIF is decoded, since layout reaches images through IM9's interface (inference, results §9).

**Updates:** [ADR-0011](../adr/0011-text-fonts-and-unicode.md) and [ADR-0012](../adr/0012-graphics-images-and-media.md) (decision histories); [text and fonts](../architecture/text-and-fonts.md) TX6, TX10, TX12, TX13 and its open questions 1 and 6; [images and media](../architecture/images-and-media.md) IM2 and IM3; the [register](../technology/dependency-register.md) (trees, `unsafe` and grades; `flate2` in `png`'s tree); the [decision map](../decision-map.md); the indexes; [status](../status.md). **For the owner:** a refinement of the process model's renderer row (§4.1): FontServices' `Reserved` directory and the executable directory's metadata. The model is binding, so it isn't edited.

**Clean-up:** seven crash reports from informal runs, deleted from `~/Library/Logs/DiagnosticReports/Retired` after copying them to the session's scratchpad. The formal runs' crash reports are archived in their results directories.

Every relative link resolves; the four standard checks pass. CP11 (spike S6) starts.

### CP11 — in progress (2026-10-18)

**Spike S6's code is written and committed** (`c2c8ad5`): `spikes/s6/compositor` (`s6-compositor`), with its `unsafe` in `src/platform/` under invariants K1–K9, and `tools/s6-measure.sh`. Every mode runs, and the trials (informal) are close to the thresholds:
- S6-Q1: a typical update takes 0.1–0.2 ms on the GPU and 0.5–0.7 ms on the compositor thread (median); a full redraw of 96 tiles, 7–10 ms and 3 ms.
- S6-Q3: animation by layers costs 0.13–0.19 ms a frame on the compositor thread; scrolling frames without new tiles 0.3–0.4 ms (median).

**Two deviations from §10.2, for the results:** CoreAnimation's offscreen renderer (`CARenderer`) produced no pixels in this setting, so S6-Q5's colour check is the owner's visual comparison of the test pattern, with the surfaces' tags checked automatically; and frames are paced by a timer rather than a display link.

**The formal run needs the owner,** as §10.2 plans: AC power, the Mac otherwise idle, and the window visible, which it can't be over another app in full screen. During this session the Mac was in use, with a full-screen app, and later on battery. CP12 proceeds meanwhile; S6's results and ADR-0012's S6 conditions follow its formal run.

### CP12 — completed 2026-10-19

**Spike S7, run and reported** ([S7 results](s7-results.md)), under the owner's authorization quoted in §2: "Spike S7 (Network Process & TLS): Authorized for questions S7-Q1 through S7-Q4 under spikes/s7/, including read-only outbound network requests to public test hosts."
- **Code:** `spikes/s7/net` (`s7-net`), its `unsafe` in `src/platform/` under invariants N1–N9, and `tools/s7-measure.sh` (`4840d7d`); then a `startup` mode and a corrected verdict label (`13daa44`).
- **Network access:** read-only GETs to badssl.com's test hosts, example.com, www.wikipedia.org, www.apple.com, www.cloudflare.com, github.com, speed.cloudflare.com and cloudflare-quic.com, and DNS queries through the system's resolver. No certificate was added to a keychain, and nothing on the Mac was reconfigured.
- **Three deviations from §10.3,** within the authorization: the QUIC connection makes an HTTP/3 GET, with framing and QPACK of the spike's own; the Mach messages come from a second thread in the helper, not from the broker; Network framework needed no rule beyond the network profile, so there is no second profile (results §1).
- **Formal run** at `4840d7d`, from a clean tree, on AC power with Low Power Mode off: `target/s7-results/20261019-071313/`. Three runs after 30 s cool-downs; the Mac was in normal use (results §2, §10). Supplementary `startup` runs at `13daa44`: `target/s7-results/20261019-073956-startup/`.

**Answers:**
- **S7-Q1: passes.** rustls, verified through `SecTrustEvaluateWithError` in the broker-compiled profile, gave the expected verdict for every host it reached, in every run: the ten good hosts verify, and the expired, self-signed, wrong-host, untrusted-root and SHA-1-intermediate certificates are rejected with Security's codes, as unsandboxed and as with Network framework. Five of 162 attempts failed before verification, on badssl.com's servers (2 resets, 3 timeouts). **The profile has eight rules,** each opening rule S7 uses checked by removal. Two were found by failures the kernel didn't log: the metadata of `/var`, for DNS-SD's socket, and of the executable's directory, without which `SecPolicyCreateSSL` returns NULL. The system `trustd` service, in the draft, was never used and was dropped. Every denial is explained, from the denial log and from reporting runs that log every attempted operation.
- **S7-Q2: reported; NW6 settled as designed.** rustls retired 2.4–3.3× fewer instructions per handshake than Network framework and 1.6× fewer per byte, with fewer threads; Network framework never resumed a session in the configuration tried. rustls needs its `prefer-post-quantum` default feature, without which Cloudflare answered every full handshake with a HelloRetryRequest. A process's first TLS use costs 29–43 ms of CPU, for AWS-LC's jitter entropy source.
- **S7-Q3: passes.** Nine DNS-SD queries on one shared connection resolved from kqueue in 17–23 ms, with one thread throughout.
- **S7-Q4: passes.** One I/O thread drove rustls, `quinn-proto` with an HTTP/3 GET, DNS-SD, a Mach port set received in the wait, a timer with leeway and a user event, from one `kevent64` per turn. Idle for 30 s, it woke only for QUIC's idle-timeout deadline and the window's end.

**The conditional approval** (owner's Gate B words, quoted in §2): ADR-0013 "will move to Accepted upon S7-Q1, S7-Q3, and S7-Q4 passing". All three pass, so **ADR-0013 is accepted** (2026-10-19), and the networking and storage designs are binding.

**Proposed for Gate C,** each recorded where it belongs:
- **Verification in the worker pool** (NW6): `SecTrustEvaluateWithError` blocks its caller (8.4 ms median; 1.0–1.5 s in three informal cases), and EL1 gives the I/O thread one place to block.
- **The process model's network column:** note the two metadata rules, beside S5's refinement for renderers. The model is binding, so it isn't edited.
- **AWS-LC's jitter entropy source:** whether to build without it is decided at the network milestone.

**Updates:** [ADR-0013](../adr/0013-networking-storage-and-workers.md) (status, decision history, assumptions, risks, consequences); [networking](../architecture/networking.md) (status; NW1, NW5, NW6, NW7, NW17; alternatives; open question 1); [storage and workers](../architecture/storage-and-workers.md) (status); the [register](../technology/dependency-register.md) (Security, DNS-SD, rustls, `aws-lc-rs`, `quinn-proto` and Network framework; CP6's and CP7's entries decided); the [decision map](../decision-map.md); the indexes; [status](../status.md); `spikes/README.md`.

**Clean-up:** no helper crashed, so there were no crash reports. One empty helper data directory, in `target/release/`, was left by an informal run whose output was cut short by `head`: the broker panicked on the closed pipe before removing it. It was removed by hand.

Every relative link resolves; the four standard checks pass. CP13 (HTML parsing and the DOM) starts.

### CP13 — completed 2026-10-19

**[HTML and the DOM](../architecture/html-and-dom.md)** (Proposed; HD1–HD33), answering Q9 and the handed-over question 18:
- **Parsing:** the HTML standard's tokenizer (84 states, now including processing instructions) and tree builder, Axiom's own under DP1 (a), in safe Rust. It runs on the document's event-loop thread in budgeted tasks, starting from Blink's field-measured budgets, with a preload scanner on the helper pool. Off-thread tree building, Firefox's design and Servo's optional mode, is left possible behind a sink interface.
- **Input:** sniffing as HTML §13.2.3 specifies, and TX3's decoders writing the DOM's own 8- and 16-bit strings, so nothing passes through UTF-8. Fragments and the new insertion methods (`setHTML` and its family) use one tree builder, which sanitizes while parsing.
- **XML:** a non-validating parser of Axiom's own; Blink, WebKit and Ladybird use libxml2, and Firefox runs expat in an RLBox sandbox. XPath staged; XSLT an open question for the owner.
- **Nodes:** one node arena of `u32` columns (five links, owner document, kind and flags, rare data, generation: 36 bytes a node), per-kind arenas, attributes without `Attr` nodes, immutable character data.
- **One mutation path:** the DOM standard's algorithms call a fixed, ordered list of hooks, including style invalidation, box-tree dirtiness and accessibility.
- **Events:** listeners in a side table; dispatch roots the path and each listener snapshot as one heap array each, as S3 recommended; non-passive wheel and touch listeners counted for hit-test regions; trust and activation only from the browser.
- **The rest:** mutation observers, live ranges and selection, Shadow DOM, custom elements with scoped registries, forms submitted through the browser, editing through Input Events and the IME mirror, the top layer. `execCommand` is marked approximate: its own draft says it won't advance beyond draft status.
- **The DOM's JavaScript facet** is an interface for CP17.

**Precedent read** on 2026-10-19 from primary sources: the HTML and DOM standards (both 2026-10-16), the Encoding Standard, the Web Editing drafts, and Blink, Firefox, WebKit, Servo and Ladybird sources at their latest commits.

**For the owner at Gate C:**
- **An amendment to ADR-0008** (HD11): node slots never move, so SH10's compaction skips the node arena. PL2 and PL12 make node identity stable for a node's life and AX2 sends it to the browser; a compacting node arena would invalidate those identities. Recorded in ADR-0008's decision history; SH10 stands until the owner decides. **Recommended:** accept.
- **XSLT** (open question 2). **Recommended:** not planned until a use requires it.

Every relative link resolves; the four standard checks pass. CP14 (style) starts.

### CP14 — completed 2026-10-19

**[Style](../architecture/style.md)** (Proposed; ST1–ST36), answering Q10 and the handed-over questions 15 and 16, with 6 confirmed:
- **Parsing:** CSS Syntax 3's tokenizer and parser, Axiom's own under DP1 (a), generic over the DOM's 8- and 16-bit code units, so nothing passes through UTF-8; `cssparser` and Stylo parse UTF-8 text. Fetched sheets are decoded as CSS Syntax §3.2 specifies and parsed on the helper pool. **One property table** generates the parsers, the computed-style groups, the impact table and the CSSOM's accessors.
- **Matching:** right to left over the DOM's columns, lent read-only; rules bucketed by their rightmost compound; a counting Bloom filter of ancestors; `:has()` with relative-selector invalidation. Matching writes nothing into the DOM. **No selector JIT:** WebKit's needs dynamic code generation, which J3 denies to every renderer but `renderer-jit`.
- **The cascade:** Cascade 5's and 6's order, including shadow-tree context, layers and `@scope` proximity; custom properties with cycle detection and the expansion limit Custom Properties §3.3 requires.
- **Computed styles,** immutable and shared (PL3): reference-counted groups shared with the parent or the initial style; a rule tree for the identity of what matched; Stylo's style sharing cache (32 candidates per depth); a cascade cache keyed by rule node. The caches are budgeted, evictable and never needed for a correct answer, and a cache-free path is the test oracle, the two principles of Ladybird's new engine.
- **Invalidation (IV3's details):** invalidation sets from every selector, fed by HD15's hook, pending until style is needed; restyling stops where no inherited group changed; each change classified as box tree, layout, paint, composite only or nothing visible.
- **Queries:** media queries over an environment the browser sends, including AX10's preferences; **size container queries resolved during layout,** in a nested lifecycle per container, as Blink resolves them.
- **Animations:** Web Animations as the one timing model, timed by EL22's frame times; compositor animations exactly as GR9 lists them, from a description, with the renderer keeping the authoritative model.
- **The CSSOM** as a view over immutable contents, edited by copying only the path to the change.
- **`:visited`** by partitioned history (Selectors 4, Appendix C): the browser keeps it, and a renderer receives only its own partitions.

**Precedent read** on 2026-10-19 from primary sources: the W3C CSS reports listed in the document's §2, the HTML, Compatibility and Quirks Mode standards, and Blink, Stylo, WebKit and Ladybird at their latest commits. **Ladybird switched style resolution to a new engine in Rust on 2026-08-13,** built around bounded, evictable memory; its design is alternative E.

**For the owner at Gate C:**
- **An amendment to ADR-0007** (ST25, ST31): IV1's lifecycle gains nested lifecycles for size-container subtrees, and IV2's examples gain the `getComputedStyle` reads that need layout (CSSOM §9's used values). Recorded in ADR-0007's decision history; IV1 and IV2 stand until the owner decides. **Recommended:** accept. Container queries can't be implemented under IV1 as written, and the nested phases keep its compiler-checked order.
- **`:visited`** (open question 2): partitioned history once the browser's history service exists, never matching until then. **Recommended:** accept. A renderer never holds the global history, and Selectors 4 calls the older restrictions "an arms race that can't be won".

Every relative link resolves; the four standard checks pass. CP15 (layout and paint) starts.

### CP15 — completed 2026-10-19

**[Layout and paint](../architecture/layout-and-paint.md)** (Proposed; LY1–LY32), answering Q11, the layout-unit half of the handed-over question 16, and the rendering pipeline's open question 4:
- **The layout unit** is fixed-point, 1/64 of a CSS pixel in an `i32`, saturating, as in Blink, WebKit and Ladybird (Firefox counts 1/60). Integer arithmetic makes fragment cache keys exact and lets IV14 compare incremental and full layouts bit for bit. Text runs use 1/65536 internally, as Blink's do.
- **Boxes** as CSS Display generates them, from the flat tree, rebuilt only where box-affecting values changed.
- **LayoutNG's contract** for every algorithm: a box, a constraint space and a break token in, an immutable fragment out, nothing else read; fragments cached by constraint space; fragmentation through break tokens.
- **Formatting contexts** of Axiom's own: block, inline (through text's interface, reshaping line edges only where HarfRust's `UNSAFE_TO_BREAK` flag requires, which adds that flag to text's shaped runs), flex, grid, positioning, multi-column, writing modes, containment. **Tables are marked approximate:** CSS 2.1 doesn't require its automatic table layout, and CSS Table 3 says it "is not yet ready for implementation".
- **Scrolling:** scroll nodes carry snap areas, sticky constraints and overscroll behaviour; scrollbar thumbs are spatial nodes the compositor moves, so they don't lag compositor scrolling. These use PL6's spatial nodes and PL7's items, so no amendment is needed.
- **Paint:** pre-paint builds the property trees; CSS 2.1's painting order over fragments; display items keyed by box identity and reused, with whole stacking contexts reused as Blink's subsequence caching does; paths, complex borders, clip paths and masks as images in the renderer, as GR5 decides until S6 reports; form controls drawn by Axiom's own theme.
- **Queries:** hit testing finishes in the renderer; hit-test regions recorded during paint, as Blink records them; CSSOM View through IV2; ResizeObserver and IntersectionObserver where HTML's rendering update puts them, with each frame's visible rectangle sent by the browser.

**Precedent read** on 2026-10-19 from primary sources: the W3C layout and painting reports listed in the document's §2 (SVG 2 published a new Candidate Recommendation Snapshot on 2026-10-06), HTML's rendering update, and Blink, Firefox, WebKit, Servo, Ladybird, Taffy and HarfRust at their latest commits. **Ladybird moved layout to Rust on 2026-07-28,** and on 2026-10-03 put style and layout, and paint, on one thread each over copied-on-write render state; that is alternative E, rejected for now because it copies DOM-derived state that Axiom's lending avoids.

**For the owner at Gate C:** nothing new from CP15; its open questions (printing without fonts in the browser, 3D sorting, Taffy as a test oracle, parallel layout, limits for hostile documents) belong to later milestones and Step 6.

Every relative link resolves; the four standard checks pass. CP16 (the JavaScript engine) starts.
