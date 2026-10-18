# Decision Map

| | |
|---|---|
| **Status** | Living document |
| **Last updated** | 2026-10-18 |

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
```

## 1. Decisions and status

| ID | Decision | Status | Record / where decided |
|---|---|---|---|
| R1 | Long-term target: full modern app web | **Accepted** | [ADR-0001](adr/0001-scope-and-target.md) |
| R2 | JavaScript engine: our own | **Accepted** | ADR-0001 |
| R3 | Security posture: sensitive logins (gated) | **Accepted** | ADR-0001 |
| R4 | Constraints: skills, time, hardware | Partly known | Charter §7; [OQ-1, OQ-6, OQ-9](open-questions.md) |
| F1 | Implementation language | **Accepted** (final, 2026-10-17): Rust | [ADR-0002](adr/0002-implementation-language.md), [evaluation](technology/language-evaluation.md), [S1](research/s1-results.md), [S2](research/s2-results.md) |
| F2 | GC and DOM memory model | **Accepted** (2026-10-17): one heap for JS and the DOM (R2), with Pattern B (Nova style) and obligations C1–C5 ([ADR-0003](adr/0003-gc-handle-and-rooting-pattern.md)). **Agents, heaps and C1–C5 decided** ([ADR-0008](adr/0008-script-agents-and-heap.md), accepted 2026-10-18 at Step 4's Gate C). C1 is `SliceStack`, a root stack with no `unsafe`, so the `gc` crate has none (spike S3 and its addendum) | [S1 results](research/s1-results.md), [ADR-0003](adr/0003-gc-handle-and-rooting-pattern.md), [script and heap](architecture/script-and-heap.md), [S3 results](research/s3-results.md) |
| F3 | Trust boundaries, process model, sandbox | **Accepted** (2026-10-17): staged site isolation, with out-of-process iframes mandatory before L3; helpers that apply browser-compiled Seatbelt profiles first; IPC trust rules; security levels L0–L3; JIT policy. Validated by spike S4 | [ADR-0004](adr/0004-process-model-and-sandbox.md), [threat model](security/threat-model.md), [process model](security/process-model.md), [macOS note](research/macos-process-confinement.md), [S4 results](research/s4-results.md) |
| F4 | Engine/embedder API | **Accepted** (2026-10-17, Gate B): the shell as an embedder over a message API; native browser UI; a headless embedder; a pure-Rust shell (OQ-11), validated first at the shell milestone | [ADR-0009](adr/0009-embedder-api-and-shell.md), [embedder API](architecture/embedder-api.md) |
| F5 | IPC transport and interface definitions | **Accepted** (2026-10-17, Gate B): Mach messages (OQ-17); endpoints as capabilities; an IDL with generated safe decoders; `Untrusted<T>` | [ADR-0005](adr/0005-ipc-transport-and-interfaces.md), [IPC](architecture/ipc.md) |
| P1 | Event loop and threading model | **Accepted** (2026-10-17, Gate B): one wait per thread; a browser core thread; one thread and heap per event loop; frames driven by the browser | [ADR-0006](adr/0006-event-loop-and-threading.md), [event loop and threading](architecture/event-loop-and-threading.md) |
| P2 | Pipeline data model | **Accepted** (2026-10-17, Gate B): five owned stages; immutable fragments; per-frame display lists of validated primitives; the browser-owned frame tree (SI4). AC5 amended to allow read-only lending of DOM arenas during frozen stages (OQ-19) | [ADR-0007](adr/0007-frame-tree-pipeline-and-input.md), [rendering pipeline](architecture/rendering-pipeline.md), [frame tree](architecture/frame-tree-and-navigation.md) |
| P3 | Invalidation and incremental-update model | **Accepted** (2026-10-17, Gate B): a compiler-checked lifecycle; reuse at each stage; compositor-driven scrolling and animation | [ADR-0007](adr/0007-frame-tree-pipeline-and-input.md), [invalidation](architecture/invalidation.md) |
| P4 | String and text representation (UTF-8, UTF-16, Latin-1; interned atoms) | **Representation accepted** (2026-10-17, Gate B): 8-bit or 16-bit strings for JavaScript and the DOM; static and process-wide atoms. Text and shaping in Step 5 | [ADR-0007](adr/0007-frame-tree-pipeline-and-input.md), [rendering pipeline](architecture/rendering-pipeline.md) §4.5 |
| I1 | Text shaping, fonts, line breaking, bidi | Open | Step 5 |
| I2 | Graphics backend and compositing | Open | Step 5 |
| I3 | HTTP/TLS/DNS transport | Open | Step 5 |
| I4 | Image and media decoders | Open | Step 5 |
| I5 | Accessibility bridge (NSAccessibility) | Open | Step 5 |
| I6 | WebAssembly: our own or embedded | Open | [OQ-3](open-questions.md) |
| X1 | Standards baseline and support matrix | Open | Step 6 |
| X2 | Testing and fuzzing strategy | Open | Step 6 |
| X3 | Performance methodology and budgets | Open | Step 6 |

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
6. **Memory targets B1–B7** (about 1 GB for a typical session; a soft limit of 16 renderers) are inputs to Step 6's budgets.

## 5. Phase Zero sequence

| Step | Topic | Status |
|---|---|---|
| 0 | Decision landscape | Done (2026-10-16) |
| 1 | Requirements: target, JavaScript, posture, defaults | Done: ADR-0001 Accepted |
| 2a | Implementation language evaluation | Done: reviewed; ADR-0002 accepted (final, 2026-10-17) |
| 2b | Validation spikes: S1 GC heap and DOM, S2 platform and build | **Done** (2026-10-17): [S1 results](research/s1-results.md), [S2 results](research/s2-results.md). ADR-0002 is final, and [ADR-0003](adr/0003-gc-handle-and-rooting-pattern.md) is accepted. Merged into `main`. S3 (the interpreter benchmark) is required, and runs in Step 4 |
| 3 | Threat model, trust boundaries, process and sandbox model on macOS | **Done** (2026-10-17): [threat model](security/threat-model.md) and [ADR-0004](adr/0004-process-model-and-sandbox.md) accepted; [process model](security/process-model.md) binding; spike S4 passed ([S4 results](research/s4-results.md)). Merged into `main` |
| 4 | Event loop, threading, pipeline data model, invalidation, embedder API | **Done** (2026-10-18): [architecture proposal v0](architecture/overview.md); ADRs 0005–0007 and 0009 accepted at Gate B, and ADR-0008 at Gate C after spike S3 and its addendum ([S3 results](research/s3-results.md)); AC5 amended for read-only lending (OQ-19). Merged into `main` ([tracker](research/step-4-architecture-proposal.md)) |
| 5 | Subsystems: text & fonts → rendering/compositing → networking/TLS → style → layout; also HTML parsing and DOM, and the JavaScript engine and bindings (added by the owner at Step 4's Gate A, 2026-10-17) | **In progress** (authorized 2026-10-18): the plan is at Gate A ([tracker](research/step-5-subsystems.md)). It covers I1–I5 and the text part of P4 as well as the subsystems listed here |
| 6 | Standards baseline, testing, performance budgets, roadmap | Not started; ends in a design review and approval of Milestone 1 |

## Sources

- [S1] Site Isolation (USENIX Security 2019) — https://research.google/pubs/site-isolation-process-separation-for-web-sites-within-the-browser/ ; Chromium design doc — https://www.chromium.org/developers/design-documents/site-isolation
- [S2] See [language evaluation §7.2](technology/language-evaluation.md#72-how-existing-engines-solve-it)
- [S3] Computerworld (secondary) — https://www.computerworld.com/article/1675025/multi-process-firefox-project-nears-completion-of-first-phase.html
- [S4] Mozilla, Project Fission — https://wiki.mozilla.org/Project_Fission
- [S5] Chrome for Developers, LayoutNG — https://developer.chrome.com/docs/chromium/layoutng
- [S6] Rust Blog, "Fearless Concurrency in Firefox Quantum" — https://blog.rust-lang.org/2017/11/14/Fearless-Concurrency-In-Firefox-Quantum/
- [S7] Isosceles, "The WebP 0day" — https://blog.isosceles.com/the-webp-0day/
- [S8] Ladybird, "Ladybird adopts Rust" — https://ladybird.org/posts/adopting-rust/
