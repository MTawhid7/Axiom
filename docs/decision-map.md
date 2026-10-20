# Decision Map

| | |
|---|---|
| **Status** | Living document |
| **Last updated** | 2026-10-20 |

This map lists every major architectural decision, its layer, its status, and how the decisions constrain one another. Decisions are layered by **how expensive they are to reverse**.

```
R  Requirements     target web · JavaScript · security posture · skills / time / hardware
                    → cheap to decide, ruinous to change
F  Foundations      language ⇄ GC & DOM memory model · trust boundaries & process model · engine/embedder API
                    → changing these ≈ rewriting the core
P  Pipeline         event loop & threads · DOM → style → box tree → fragment tree → display list → compositor
                    invalidation model · string & text representation
I  Infrastructure   shaping/fonts · graphics backend · HTTP/TLS/DNS · decoders · accessibility bridge · Wasm
                    → swappable, if we own the interfaces in front of them
X  Cross-cutting    standards baseline · conformance testing · fuzzing · performance methodology
                    dependency & supply-chain policy
```

## 1. Decisions and status

| ID | Decision | Status | Record / where decided |
|---|---|---|---|
| R1 | Long-term target: full modern app web | **Accepted** | [ADR-0001](adr/0001-scope-and-target.md) |
| R2 | JavaScript engine: our own | **Accepted** | ADR-0001 |
| R3 | Security posture: sensitive logins (gated) | **Accepted** | ADR-0001 |
| R4 | Constraints: skills, time, hardware | Partly known | Charter §7; [OQ-1, OQ-6, OQ-9](open-questions.md) |
| F1 | Implementation language | **Accepted** (final, 2026-10-17): Rust | [ADR-0002](adr/0002-implementation-language.md), [evaluation](technology/language-evaluation.md), [S1](research/s1-results.md), [S2](research/s2-results.md) |
| F2 | GC and DOM memory model | **Accepted** (2026-10-17): one heap for JS and the DOM (R2), with Pattern B (Nova style) and obligations C1–C5 ([ADR-0003](adr/0003-gc-handle-and-rooting-pattern.md)). **Agents, heaps and C1–C5 decided** ([ADR-0008](adr/0008-script-agents-and-heap.md), accepted 2026-10-18 at Step 4's Gate C). C1 is `SliceStack`, a root stack with no `unsafe`, so the `gc` crate has none (spike S3 and its addendum). **SH10 amended** at Step 5's Gate C (2026-10-20): compaction skips the DOM's node arena, whose slots never move (HD11) | [S1 results](research/s1-results.md), [ADR-0003](adr/0003-gc-handle-and-rooting-pattern.md), [script and heap](architecture/script-and-heap.md), [S3 results](research/s3-results.md) |
| F3 | Trust boundaries, process model, sandbox | **Accepted** (2026-10-17): staged site isolation, with out-of-process iframes mandatory before L3; helpers that apply browser-compiled Seatbelt profiles first; IPC trust rules; security levels L0–L3; JIT policy. Validated by spike S4 | [ADR-0004](adr/0004-process-model-and-sandbox.md), [threat model](security/threat-model.md), [process model](security/process-model.md), [macOS note](research/macos-process-confinement.md), [S4 results](research/s4-results.md) |
| F4 | Engine/embedder API | **Accepted** (2026-10-17, Gate B): the shell as an embedder over a message API; native browser UI; a headless embedder; a pure-Rust shell (OQ-11), validated first at the shell milestone | [ADR-0009](adr/0009-embedder-api-and-shell.md), [embedder API](architecture/embedder-api.md) |
| F5 | IPC transport and interface definitions | **Accepted** (2026-10-17, Gate B): Mach messages (OQ-17); endpoints as capabilities; an IDL with generated safe decoders; `Untrusted<T>` | [ADR-0005](adr/0005-ipc-transport-and-interfaces.md), [IPC](architecture/ipc.md) |
| P1 | Event loop and threading model | **Accepted** (2026-10-17, Gate B): one wait per thread; a browser core thread; one thread and heap per event loop; frames driven by the browser | [ADR-0006](adr/0006-event-loop-and-threading.md), [event loop and threading](architecture/event-loop-and-threading.md) |
| P2 | Pipeline data model | **Accepted** (2026-10-17, Gate B): five owned stages; immutable fragments; per-frame display lists of validated primitives; the browser-owned frame tree (SI4). AC5 amended to allow read-only lending of DOM arenas during frozen stages (OQ-19) | [ADR-0007](adr/0007-frame-tree-pipeline-and-input.md), [rendering pipeline](architecture/rendering-pipeline.md), [frame tree](architecture/frame-tree-and-navigation.md) |
| P3 | Invalidation and incremental-update model | **Accepted** (2026-10-17, Gate B): a compiler-checked lifecycle; reuse at each stage; compositor-driven scrolling and animation. **IV1 and IV2 amended** at Step 5's Gate C (2026-10-20): nested lifecycles for size containers, and the `getComputedStyle` reads that need layout (ST25, ST31) | [ADR-0007](adr/0007-frame-tree-pipeline-and-input.md), [invalidation](architecture/invalidation.md) |
| P4 | String and text representation (UTF-8, UTF-16, Latin-1; interned atoms) | **Representation accepted** (2026-10-17, Gate B): 8-bit or 16-bit strings for JavaScript and the DOM; static and process-wide atoms. **Text and shaping accepted** (2026-10-20, Step 5's Gate C): [ADR-0011](adr/0011-text-fonts-and-unicode.md) | [ADR-0007](adr/0007-frame-tree-pipeline-and-input.md), [rendering pipeline](architecture/rendering-pipeline.md) §4.5 |
| P5 | HTML parsing and the DOM | **Accepted** (2026-10-20, Step 5's Gate C): our own HTML and XML parsers on the event loop's thread, decoding straight into 8- and 16-bit strings; one node arena of `u32` columns whose slots never move (ADR-0008's SH10 amended); one mutation path with fixed hooks; events, observers, Shadow DOM, custom elements, forms and editing. XSLT deferred until a concrete use case arises (OQ-22) | [ADR-0015](adr/0015-html-parsing-and-the-dom.md), [HTML and the DOM](architecture/html-and-dom.md) |
| P6 | Style, layout and paint | **Accepted** (2026-10-20, Step 5's Gate C): our own CSS parser, matcher and cascade, with shared computed styles and evictable caches; container queries in nested lifecycles during layout (ADR-0007's IV1 and IV2 amended); `:visited` by partitioned history, as the owner approved; a 1/64-pixel fixed-point layout unit; LayoutNG's contract; tables marked approximate; paint into PL7's primitives | [ADR-0016](adr/0016-style-layout-and-paint.md), [style](architecture/style.md), [layout and paint](architecture/layout-and-paint.md) |
| P7 | The JavaScript engine and bindings | **Accepted** (2026-10-20, Step 5's Gate C): an engine entirely our own, with heap-free compiled scripts, a register machine dispatched by a `match` loop, lifetime-carrying NaN-boxed values, `RegExp` interpreted with a linear-time fallback, `Intl` on ICU4X and Temporal of our own; a WebIDL generator of our own emitting safe Rust, with platform objects as heap entries | [ADR-0017](adr/0017-javascript-engine-and-bindings.md), [the JavaScript engine](architecture/javascript-engine.md), [bindings](architecture/bindings.md) |
| I1 | Text shaping, fonts, line breaking, bidi | **Accepted** (2026-10-20, Step 5's Gate C), after the owner's visual check of S5's sheets ([results](research/s5-results.md)): ICU4X as the one Unicode source; our own encodings; CoreText's font database for system fonts; HarfRust shaping; a Rust-only path for web fonts; glyph atlases in the renderer. **TX12: CoreText rasterizes system fonts.** **`vello_cpu` adopted** (2026-10-20) as the renderer's Rust rasterizer for web fonts and paths, on S6-Q4's evidence, subject to grading at the text milestone | [ADR-0011](adr/0011-text-fonts-and-unicode.md), [text and fonts](architecture/text-and-fonts.md) |
| I2 | Graphics backend and compositing | **Accepted** (2026-10-20, Step 5's Gate C): Metal through `platform`; retained CoreAnimation layers with damage-limited tiles; our own pipelines; Display P3. **Spike S6** ([results](research/s6-results.md), two formal runs): S6-Q1 and S6-Q3 pass at the median; CoreAnimation honours the colour tags, so every surface is tagged; S6-Q5's memory condition is not met as measured, so **B1 is revised** as the owner approved (OQ-23): a window's tiles are budgeted by their pixels in Step 6, and the core browser keeps 60 MB. **GR5's amendment,** path items in PL7, is deferred to the graphics milestone (OQ-24) | [ADR-0012](adr/0012-graphics-images-and-media.md), [graphics](architecture/graphics.md) |
| I3 | HTTP/TLS/DNS transport | **Accepted** (2026-10-19), the owner's Gate B condition met: S7-Q1, S7-Q3 and S7-Q4 pass ([S7 results](research/s7-results.md)). State-machine protocols on one I/O thread; our own URL parser and HTTP framing; rustls, with `aws-lc-rs` and the system verifying certificates, which S7-Q2 settled against Network framework; DNS-SD; partitioned caches and cookies; Fetch's policy from bound endpoints; storage in the network process. The network process's profile is S7's eight rules, whose two metadata rules the process model's change record notes. **Verification in the worker pool,** off the I/O thread (NW6), confirmed at Step 5's Gate C (2026-10-20) | [ADR-0013](adr/0013-networking-storage-and-workers.md), [networking](architecture/networking.md), [storage and workers](architecture/storage-and-workers.md) |
| I4 | Image and media decoders | **Accepted** (2026-10-20, Step 5's Gate C), with I2's, in ADR-0012: safe Rust decoders in the renderer, JPEG included (S5-Q5); ImageIO for AVIF only, under the dependency policy's exception, **in the media utility process** (IM3's fallback, approved by the owner); media in a sandboxed utility process | [ADR-0012](adr/0012-graphics-images-and-media.md), [images and media](architecture/images-and-media.md) |
| I5 | Accessibility bridge (NSAccessibility) | **Accepted** (2026-10-18, Step 5's Gate B): trees pushed by renderers; the browser's cache; text markers; our own bridge. The settings list and the reserved key equivalents approved as written | [ADR-0014](adr/0014-accessibility.md), [accessibility](architecture/accessibility.md) |
| I6 | WebAssembly: our own or embedded | Open; placed by the [roadmap](roadmap.md) at M8's gate, before M9 (Proposed, [ADR-0020](adr/0020-roadmap-and-security-levels.md)) | [OQ-3](open-questions.md) |
| X1 | Standards baseline and support matrix | **Accepted** (2026-10-20, Step 6's Gate B) in [ADR-0018](adr/0018-corpus-standards-and-testing.md): the [standards baseline](standards.md), with the [corpus](corpus.md); the not-planned list approved as written, and the corpus's automation boundary confirmed | Step 6 |
| X2 | Testing and fuzzing strategy | **Accepted** (2026-10-20, Step 6's Gate B) in [ADR-0018](adr/0018-corpus-standards-and-testing.md): the [testing strategy](testing.md); spike S9 authorized | Step 6 |
| X3 | Performance methodology and budgets | **Accepted** (2026-10-20, Step 6's Gate B) in [ADR-0019](adr/0019-performance-methodology-and-budgets.md): the [performance budgets](performance.md), with the final B1–B7 amending the [process model](security/process-model.md) | Step 6 |
| X4 | Dependency and supply-chain policy: what Axiom builds and what it may reuse; how each dependency is justified, classed and audited; how the `unsafe` and C code inside dependencies count against ADR-0002's rules and the Rule of 2 | **Accepted** (2026-10-18, Step 5's Gate B): the [dependency policy](technology/dependency-policy.md), rules DP1–DP23, with CP3's refinement of DP10 and a living [register](technology/dependency-register.md) ([ADR-0010](adr/0010-dependency-and-supply-chain-policy.md)). Axiom's own license: "MIT OR Apache-2.0" (OQ-21). **Added by the owner at Step 5's Gate A** (2026-10-18) | [Step 5 tracker](research/step-5-subsystems.md) |

## 2. How the decisions constrain each other

1. **JavaScript is the hub.** JS changes four things:
   - **Mutability.** Script can mutate the DOM at any time and demand geometry synchronously (`offsetWidth` forces style and layout), so fine-grained invalidation becomes essential.
   - **Scheduling.** The HTML event loop becomes the engine's scheduler.
   - **Bindings.** A WebIDL binding layer and GC integration appear.
   - **Threat model.** Hostile *data* becomes hostile *code with timers*. Spectre-class attacks led browsers to process isolation [S1].

   Some incremental processing is needed even without JS: streaming parsing, late-arriving images and fonts, resizing, `:hover`.
2. **The language and the GC/DOM memory model are one decision.** The DOM is a cyclic graph shared with the JS heap, and every engine solves its lifetime problem differently [S2]. ADR-0001's "our own engine" reduces this to *language × our GC design* (ADR-0002, spike S1).
3. **Trust boundaries shape every interface.** Process isolation is cheap to design and expensive to retrofit:
   - Chromium was multi-process from its 2008 launch.
   - Firefox's Electrolysis began around 2009 and reached all users around 2016–17 [S3].
   - Firefox's site isolation (Fission) arrived in late 2021 [S4].

   The plan: define message-passing boundaries with serializable data now (content ⇄ browser, ⇄ network, ⇄ compositor), and split processes later.
4. **The pipeline data model caps incremental work and parallelism.**
   - Immutable layout output (LayoutNG's fragment tree) enables reuse and possibly parallel layout [S5].
   - A parallel cascade (Stylo) depends on the language's concurrency guarantees [S6].
5. **Platform pieces are deferrable only if we own the interfaces in front of them:**
   - a loader shaped like the Fetch standard (CORS, credentials, redirects and cache modes are ours to implement);
   - a display list in front of the graphics backend;
   - a decoder interface that can move into a sandbox, since decoders are real attack surface [S7].
6. **Strings, text and accessibility are data-model properties, not late features.**
   - DOM strings are UTF-16; Rust and Swift strings are UTF-8.
   - Bidi text, complex shaping and font fallback are needed even on English pages.
   - Accessibility needs stable node identity and geometry that can be queried.
7. **The machine is a requirement.**
   - 8 GB of RAM makes memory budgets and process counts first-class.
   - Performance versus efficiency cores make thread quality-of-service matter.
   - Instruments requires Xcode.
8. **AI-assisted translation (2026) lowers the cost of changing *language*, not *invariants*.** Ladybird ported about 25,000 lines from C++ to Rust in roughly two weeks, checked against test262 [S8]. So invest in test oracles from the first milestone.

## 3. Consequences of the Step 1 decisions

These are recorded in ADR-0001.
1. One unified GC heap for JS and the DOM.
2. The language choice is weighted toward suitability for writing a GC, a VM and a JIT.
3. Multiple agents: one heap and one event loop per agent (workers, `SharedArrayBuffer`).
4. JIT policy: interpreter first; any JIT only after isolation and sandboxing; a no-JIT mode kept available.
5. App-web capabilities must not be ruled out: GPU process, media, storage, service workers, Wasm (open).
6. The dominant risk is the schedule. Mitigations: staging, test oracles, measurable acceptance criteria.
7. Per-process memory budgets are first-class.

## 4. Consequences of the Step 3 proposal

These follow from [ADR-0004](adr/0004-process-model-and-sandbox.md), accepted on 2026-10-17, and bind Step 4.
1. **Every future process boundary is a message interface from the start** (process model PM6). It also works in-process, for tests and a single-process debugging mode.
2. **The browser owns the frame tree, from the first line of code.** Each frame produces its own display list, and the browser routes input (SI4). **Out-of-process iframes are mandatory before L3** (the owner's decision), and this structure lets them arrive without an architectural rewrite.
3. **An agent cluster never spans processes** (SI7). A renderer may host several clusters of its own site. Step 4 clarified this wording, which earlier read "one agent cluster per process" ([ADR-0006](adr/0006-event-loop-and-threading.md)). This is an input to OQ-5.
4. **The browser never decodes images or parses fonts.** The display list carries only validated primitives (Rule of 2).
5. **A JIT is a property of one executable** (`renderer-jit`) and comes no earlier than security level L2 (JIT rules J1–J7).
6. **Memory targets B1–B7** (about 1 GB for a typical session; a soft limit of 16 renderers) are inputs to Step 6's budgets. Spike S6 showed that B1 needs revising, since one window's tiles take most of it; the owner approved budgeting a visible window's tiles by their pixels, and keeping 60 MB for the core browser process ([OQ-23](open-questions.md)). **The final B1–B7** were accepted at Step 6's Gate B (2026-10-20) with [ADR-0019](adr/0019-performance-methodology-and-budgets.md): tiles at 6.2 bytes per window pixel beside the core's 60 MB, and a ceiling for every corpus page beside B3's 50 MB median.

## 5. Phase Zero sequence

| Step | Topic | Status |
|---|---|---|
| 0 | Decision landscape | Done (2026-10-16) |
| 1 | Requirements: target, JavaScript, posture, defaults | Done: ADR-0001 Accepted |
| 2a | Implementation language evaluation | Done: reviewed; ADR-0002 accepted (final, 2026-10-17) |
| 2b | Validation spikes: S1 GC heap and DOM, S2 platform and build | **Done** (2026-10-17): [S1 results](research/s1-results.md), [S2 results](research/s2-results.md). ADR-0002 is final, and [ADR-0003](adr/0003-gc-handle-and-rooting-pattern.md) is accepted. Merged into `main`. S3 (the interpreter benchmark) is required, and runs in Step 4 |
| 3 | Threat model, trust boundaries, process and sandbox model on macOS | **Done** (2026-10-17): [threat model](security/threat-model.md) and [ADR-0004](adr/0004-process-model-and-sandbox.md) accepted; [process model](security/process-model.md) binding; spike S4 passed ([S4 results](research/s4-results.md)). Merged into `main` |
| 4 | Event loop, threading, pipeline data model, invalidation, embedder API | **Done** (2026-10-18): [architecture proposal v0](architecture/overview.md); ADRs 0005–0007 and 0009 accepted at Gate B, and ADR-0008 at Gate C after spike S3 and its addendum ([S3 results](research/s3-results.md)); AC5 amended for read-only lending (OQ-19). Merged into `main` ([tracker](research/step-4-architecture-proposal.md)) |
| 5 | Subsystems: text & fonts → rendering/compositing → networking/TLS → style → layout; also HTML parsing and DOM, and the JavaScript engine and bindings (added by the owner at Step 4's Gate A, 2026-10-17) | **Done** (2026-10-20; [tracker](research/step-5-subsystems.md)). Gate A passed on 2026-10-18: the plan is approved as written, and X4 is added. **Gate B passed** (2026-10-18): ADRs 0010 and 0014 accepted; ADRs 0011–0013 pre-approved, pending spikes S5–S7, which are authorized. **Spikes:** S5 (CP10, 2026-10-18), S6 (CP11, formal run 2026-10-19) and S7 (CP12, 2026-10-19), with ADR-0013 accepted. **Part 2** (CP13–CP18, 2026-10-19): five designs and ADRs 0015–0017. **Gate C passed** (2026-10-20): ADRs 0011, 0012 and 0015–0017 accepted, with the amendments to ADR-0007 and ADR-0008; the process model refined. Merged into `main`. It covers I1–I5, P5–P7, the text part of P4 and X4 as well as the subsystems listed here |
| 6 | Standards baseline, testing, performance budgets, roadmap | **In progress** (authorized 2026-10-20; [tracker](research/step-6-standards-budgets-roadmap.md)): CP1–CP10 ✅, **at Gate C**: the [roadmap](roadmap.md) (ADR-0020), [Milestone 1](milestone-1.md) (ADR-0021), the [requirements](requirements.md) and the overview v2 put to the owner. **Gate B passed** (2026-10-20): ADRs 0018 and 0019 accepted, with the final B1–B7; spike S9 authorized and run ([results](research/s9-results.md)). **Gate A passed** (2026-10-20), with spike S8 authorized; it is complete in Chrome, automated and by hand. The corpus is set (OQ-2), 13 sites, and [defined](corpus.md) at CP2: eight sites' terms forbid automated access, so they are measured by hand. It covers X1–X3, the roadmap and Milestone 1's acceptance criteria, and ends in a design review of the whole architecture proposal |

## Sources

- [S1] Site Isolation (USENIX Security 2019) — https://research.google/pubs/site-isolation-process-separation-for-web-sites-within-the-browser/ ; Chromium design doc — https://www.chromium.org/developers/design-documents/site-isolation
- [S2] See [language evaluation §7.2](technology/language-evaluation.md#72-how-existing-engines-solve-it)
- [S3] Computerworld (secondary) — https://www.computerworld.com/article/1675025/multi-process-firefox-project-nears-completion-of-first-phase.html
- [S4] Mozilla, Project Fission — https://wiki.mozilla.org/Project_Fission
- [S5] Chrome for Developers, LayoutNG — https://developer.chrome.com/docs/chromium/layoutng
- [S6] Rust Blog, "Fearless Concurrency in Firefox Quantum" — https://blog.rust-lang.org/2017/11/14/Fearless-Concurrency-In-Firefox-Quantum/
- [S7] Isosceles, "The WebP 0day" — https://blog.isosceles.com/the-webp-0day/
- [S8] Ladybird, "Ladybird adopts Rust" — https://ladybird.org/posts/adopting-rust/
