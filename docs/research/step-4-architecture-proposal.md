# Step 4 — Architecture Proposal v0 (Event Loop, Threading, Pipeline Data Model, Invalidation, Embedder API): Plan and Checkpoint Tracker

| | |
|---|---|
| **Status** | **Done** (2026-10-18). CP1–CP17 ✅; Gates A, B and C passed. ADRs 0005–0007 and 0009 accepted at Gate B, ADR-0008 at Gate C after spike S3 and its addendum ([results](s3-results.md)); AC5 amended (OQ-19). Merged into `main` |
| **Authorized** | 2026-10-17, by the owner: "Proceed with Step 4 design work on a new branch `step-4`." Design documents only, until Gate B. **Spike S3's code was authorized at Gate B** (2026-10-17): "Authorized for the full scope (questions S3-Q1 through S3-Q5 under spikes/s3/)" (§2) |
| **Branch** | `step-4`, from `main` at `da79e69`; merged into `main` (`--no-ff`) on 2026-10-18, as the owner instructed at Gate C |
| **Feeds** | Decision-map items F4, P1, P2, P3 and part of P4; OQ-5, OQ-11, OQ-13, OQ-14, OQ-17; ADR-0003 obligations C1–C5; ADRs 0005–0009 (provisional, §4); Steps 5 and 6 |
| **Last updated** | 2026-10-18 |

> **Resuming in a new session?**
> 1. Read §6 to find the first checkpoint that isn't ✅.
> 2. Check the repository against §11, the results log.
> 3. Continue from that checkpoint's deliverables (§6.1).

---

## 1. Purpose

Produce Axiom's **architecture proposal v0**:
- how the engine runs: event loops, threads and IPC;
- what flows through it: the frame tree, and DOM → style → box tree → fragment tree → display list → compositor;
- how it updates incrementally;
- how the browser shell drives it.

The proposal must carry the complete model that Step 3 fixed from the first line of code: site isolation with out-of-process iframes, sandboxed helpers and the IPC trust rules. No move between security levels, from L0 to L3, may need an architectural rewrite (charter principle 7).

**Step 4 fixes interfaces, ownership, threads and invariants.** Step 5 designs the subsystems behind those interfaces. Step 6 sets the budgets, the testing strategy and the roadmap.

## 2. Owner input and scope (2026-10-17)

| Input | Effect |
|---|---|
| Push `main` to `origin` | Done: `main` is at `da79e69`, the Step 3 merge |
| ADR-0004's three S4 refinements are confirmed: browser-compiled profile bytecode (D2), the required explicit `(deny dynamic-code-generation)` (J3), and `com.apple.security.cs.single-jit` on `renderer-jit` (J5) | Recorded in ADR-0004's decision history |
| **Step 4 is authorized:** design work on a new branch, `step-4`, "focused on architecture design under Phase Zero rules (browser-owned frame tree, per-frame display lists, event loop, and pipeline data model)" | This plan |
| "Submit the checkpoint plan and tracker first" | CP1 ends at a review gate (Gate A, §7). CP2 starts once the owner approves the plan |
| "Request separate go-ahead for Spike S3's code when its design is ready" | S3 is designed at CP10 and put to the owner at Gate B. No S3 code is written before then |
| **Gate A (2026-10-17):** the plan is "approved as written": its scope, the criteria AC1–AC10 and the 16 checkpoints. Step 5's list gains "HTML parsing and DOM" and "JavaScript engine and bindings". The cadence is approved: "pause at Gate B (after CP12) to review proposed ADRs 0005–0009 and authorize Spike S3 code concurrently" | [Decision map](../decision-map.md) §5 updated. CP2–CP12 run without pausing |
| Push `step-4`: authorized | Pushed to `origin` on 2026-10-17, and kept pushed as checkpoints land, for off-site backup |
| **Gate B (2026-10-17):** "ADR-0005 (Mach IPC): Accepted." "ADR-0006 (Event Loop & Threading): Accepted." "ADR-0007 (Frame Tree, Pipeline, Input): Accepted." "ADR-0009 (Embedder API & Pure-Rust Shell): Accepted." | Recorded in each ADR's decision history. OQ-11 and OQ-17 are resolved, and the design documents behind the four ADRs are binding |
| ADR-0008: "Pre-approved; will move to Accepted upon S3-Q1 and S3-Q2 passing." | Recorded in ADR-0008's decision history. CP15 moves it to Accepted if both pass |
| Spike S3: "Authorized for the full scope (questions S3-Q1 through S3-Q5 under spikes/s3/)". S3-Q2's threshold: "Confirmed at ≤10% of runtime spent rooting" | §10 updated. CP13–CP15 run without pausing until Gate C |
| OQ-19 and AC5: "Approved." "Amend criterion AC5 and rule EL11 to allow read-only scoped lending of DOM arenas to helper threads during borrow-checker-frozen rendering stages. Keep the actual parallel style/layout implementation deferred until profiling benchmarks justify it." | AC5 amended (§5); EL11, PL10 and ADRs 0006 and 0007 updated; OQ-19 resolved |
| The S4 test fix (§11, CP3): "Approved" | No further change |
| "Proceed with Spike S3." | CP13 starts |
| **Gate C (2026-10-18):** "S3 Addendum Checkpoint: Authorized." "Proceed with the addendum checkpoint to integrate the length-first safe `SliceStack` into `s3-gc`, run formal 3-process benchmarks on AC power, and update Section 3 of docs/research/s3-results.md and related trackers with the store-forwarding findings." "Correct the S3-Q1 verdict in docs/adr/0008-script-agents-and-heap.md, status.md, and step-4-architecture-proposal.md." | CP16, the S3 addendum (§6, §11) |
| "C1 Implementation: Adopt `SliceStack` (Safe Contiguous Stack)." "The `gc` crate remains strictly `#![forbid(unsafe_code)]`." "A 19% dispatch improvement (628 ns vs 779 ns) with zero `unsafe` proves that hardware-aligned data layout beats manual memory micro-optimizations. Keep `RawStack` archived in spikes/s3/ as benchmark history." | ADR-0008 decision 5, SH7, the overview's `unsafe` map (AC9), ADR-0003's decision history. `RawStack` stays in `s3-gc`'s `src/raw/` |
| "We will formally accept ADR-0008 and merge `step-4` into `main` (with a subsequent push to origin) immediately upon completion of the formal addendum measurements." | A conditional approval: once CP16's measurements are complete, ADR-0008 is Accepted, and CP17 merges and pushes `main` |
| "Proceed with the S3 addendum." | CP16 starts |

**Binding inputs** (decision map §4; ADR-0003; ADR-0004):
- Every future process boundary is a message interface from the start, and it also works in-process (PM6, R6).
- **The browser owns the frame tree, each frame has its own display list, and the browser routes input, from the first line of code** (SI4). Out-of-process iframes are mandatory before L3.
- One agent cluster per process (SI7), and one single-threaded heap per agent (ADR-0003).
- The browser never decodes images or parses fonts. Display lists carry validated primitives only (Rule of 2).
- A JIT is a property of one executable, and comes no earlier than L2 (J1–J7).
- Memory targets B1–B7. Idle costs nothing (PM7, PM8).

**In scope:**
- **P1:** the event loop and threading model, in every process type.
- **IPC:** the transport (OQ-17), and how interfaces are defined, generated and checked.
- **SI4:**
  - the frame tree, navigation and session history;
  - browser-routed input, focus and hit testing.
- **P2:** the pipeline data model, per-frame display lists and compositing.
- **Part of P4:** how the DOM, style and JavaScript represent strings and atoms. Text shaping stays in Step 5 (I1).
- **P3:** invalidation and incremental updates.
- **F4:** the engine/embedder API and the shell (OQ-11).
- **Script and heap:**
  - agents and workers (OQ-5);
  - ADR-0003's obligations C1–C3, and plans for C4 and C5 (OQ-13, OQ-14);
  - the design of spike S3, and S3 itself once authorized.

**Out of scope.** Each item names the step that designs it.
- The internals of style, layout, text, graphics, networking, decoders and the accessibility bridge: Step 5 (I1–I5).
- The internals of the HTML parser, of the JavaScript engine (parser, bytecode set, object shapes, inline caches), and of the WebIDL bindings generator. Step 4 fixes only their interfaces with the event loop, the heap and the pipeline.
  - **Approved at Gate A:** Step 5's list of subsystems now names "HTML parsing and DOM" and "JavaScript engine and bindings" ([decision map](../decision-map.md) §5). It didn't name them before.
- The standards baseline, testing strategy, performance budgets and roadmap: Step 6.
- Production code, a project skeleton and dependencies: none in Phase Zero.

**Evidence gathering without code**, as in Step 3:
- SDK headers, export stubs and man pages;
- `nm -u` and `codesign -d` on installed browsers.

Nothing is compiled or installed before Gate B. Each such fact names the command or file it comes from.

## 3. Questions

| ID | Question | Answered at |
|---|---|---|
| Q1 | How does each process run the HTML event loop (task queues and sources, microtasks, timers, rendering opportunities, idle periods)? How does it wait on the OS (AppKit's run loop in the browser, IPC everywhere) with no wakeups while idle? | CP3 |
| Q2 | Which threads does each process type run, at which quality-of-service class, and what data may cross between them? Can style or layout run in parallel, given a single-threaded heap? | CP3, CP6 |
| Q3 | What drives frame production (the display's clock, frame requests, backpressure), and how are hidden and background frames throttled? | CP3, CP7 |
| Q4 | How do agents, agent clusters and workers map onto processes, threads and heaps? How do `postMessage`, transfer and `SharedArrayBuffer` work between them (OQ-5)? | CP3, CP10 |
| Q5 | Which IPC transport (OQ-17)? How are interfaces defined, generated and versioned so that decoding is safe Rust (IR1), endpoints are capabilities (IR4), and the same interfaces run in-process (R6)? | CP4 |
| Q6 | What is the browser-owned frame tree: navigables, browsing context groups, session history, site assignment? How do renderers represent frames held by other processes? How do cross-frame operations work: scripting, `postMessage`, navigating another frame? | CP5 |
| Q7 | How do navigation and resource loading run between the browser, renderers and the network process, with the site lock checked at each step? | CP5 |
| Q8 | What are the pipeline's data structures (DOM, computed style, box tree, fragment tree, display list, compositor input)? Who owns each, on which thread, and what crosses a process boundary? | CP6 |
| Q9 | What does a per-frame display list contain, so that the browser never decodes images or parses fonts? How does the compositor assemble one picture from frames in several processes? | CP6 |
| Q10 | How are strings represented across the parser, the DOM, style and JavaScript: encodings and atoms (part of P4)? | CP6 |
| Q11 | What is the rendering lifecycle? How do changes propagate from a DOM mutation to style, layout, paint and compositing, with reuse at each stage, including synchronous flushes such as reading `offsetWidth`? | CP7 |
| Q12 | Which updates bypass the renderer's main thread (scrolling, compositor-driven animation), and how does that work across processes? | CP7 |
| Q13 | How does the browser route pointer, wheel, keyboard, IME and gesture input to frames, with hit testing across processes? How are focus, scrolling and accessibility trees handled? | CP8 |
| Q14 | Where is the boundary between the engine and the browser shell? What does the API offer, and how does a headless embedder (tests, WPT) use it? Is the shell pure Rust (OQ-11)? | CP9 |
| Q15 | How does the script engine meet ADR-0003: VM state as roots, the root stack (C1), heap layout (C2), a barrier API (C3), and plans for incremental or generational collection (C4) and native interop (C5)? What must spike S3 measure? | CP10 |
| Q16 | How do the parts fit together: processes × threads × pipeline, a crate structure with its `unsafe` map, end-to-end scenarios, and the path from L0 to L3 without a rewrite? | CP11 |

## 4. Deliverables

| # | Deliverable | Checkpoint |
|---|---|---|
| 1 | This plan and tracker | CP1 |
| 2 | [`docs/research/macos-runtime-facilities.md`](macos-runtime-facilities.md), a research note on the macOS facilities the runtime needs, with observations of this Mac | CP2 |
| 3 | [`docs/architecture/event-loop-and-threading.md`](../architecture/event-loop-and-threading.md) (P1) | CP3 |
| 4 | [`docs/architecture/ipc.md`](../architecture/ipc.md) (OQ-17) | CP4 |
| 5 | [`docs/architecture/frame-tree-and-navigation.md`](../architecture/frame-tree-and-navigation.md) (SI4) | CP5 |
| 6 | [`docs/architecture/rendering-pipeline.md`](../architecture/rendering-pipeline.md) (P2, part of P4) | CP6 |
| 7 | [`docs/architecture/invalidation.md`](../architecture/invalidation.md) (P3) | CP7 |
| 8 | [`docs/architecture/input-and-accessibility.md`](../architecture/input-and-accessibility.md) (SI4) | CP8 |
| 9 | [`docs/architecture/embedder-api.md`](../architecture/embedder-api.md) (F4, OQ-11) | CP9 |
| 10 | [`docs/architecture/script-and-heap.md`](../architecture/script-and-heap.md) (OQ-5, OQ-13, OQ-14; ADR-0003 C1–C5), and spike S3's design in this tracker (§10) | CP10 |
| 11 | [`docs/architecture/overview.md`](../architecture/overview.md): the architecture proposal v0 | CP11 |
| 12 | ADRs, Proposed: [0005](../adr/0005-ipc-transport-and-interfaces.md), IPC transport and interfaces; [0006](../adr/0006-event-loop-and-threading.md), event loop and threading; [0007](../adr/0007-frame-tree-pipeline-and-input.md), frame tree, pipeline, invalidation and input (regrouped to cover SI4 whole); [0008](../adr/0008-script-agents-and-heap.md), script, agents and heap (accepted after S3); [0009](../adr/0009-embedder-api-and-shell.md), engine/embedder API and shell | CP12 |
| 13 | Updates to the decision map, open questions, indexes, status and spikes README | CP12, CP15 |
| 14 | Spike S3, if authorized: code in `spikes/s3/`, scripts in `tools/`, results in `docs/research/s3-results.md` | CP13–CP15 |

**Every architecture document has the same structure:**
- status, question, what it feeds and what it builds on;
- summary;
- binding inputs;
- precedent: how Chromium, Firefox, WebKit, Servo and Ladybird solve the problem, sourced and dated;
- alternatives and analysis;
- the design, as numbered rules with one prefix per document:
  - EL, the event loop and threads;
  - IP, IPC;
  - FT, the frame tree and navigation;
  - PL, the pipeline;
  - IV, invalidation;
  - IN, input and accessibility;
  - EM, the embedder API;
  - SH, script and heap;
- staging across L0–L3, showing that no interface changes shape;
- open questions and sources.

## 5. Architecture criteria

The proposal is judged against these criteria. CP11 checks each one and records the evidence.

| # | Criterion | From |
|---|---|---|
| AC1 | **No rewrite from L0 to L3.** Moving a component across a process boundary changes the transport and the deployment, never an interface's shape. This covers the frame tree, per-frame display lists, input routing, the loader and storage accessors, and the compositor surface | PM6, SI4, charter principle 7 |
| AC2 | **The browser is the authority.** It owns the frame tree, navigation and process assignment, and it routes input. Renderers request; the browser decides from its own records | PM3, IR2 |
| AC3 | **Idle costs nothing.** No timer, poll or frame callback runs in any process while nothing changes | PM8. S2's idle window: 0.0% CPU, no wakeups |
| AC4 | **The browser never decodes images or parses web fonts.** Display lists carry validated primitives | PM2 (Rule of 2) |
| AC5 | **One single-threaded heap per agent.** Nothing GC-managed crosses a process, or a thread with one exception, **amended at Gate B (2026-10-17, OQ-19):** an event-loop thread may lend its DOM arenas read-only to helper threads for one rendering stage, during which the borrow checker freezes the heap, so nothing mutates or collects it until the stage ends. Data moves between agents by structured clone or transfer. Parallel style and layout are built only when profiling justifies them. (As approved at Gate A, AC5 read: "Nothing GC-managed crosses a thread or a process.") | ADR-0003, SI7; the owner at Gate B |
| AC6 | **A deterministic, testable core.** Every stage can run headless and in one process, with virtual time and deterministic scheduling for tests | R6; decision map §2, item 8 |
| AC7 | **Memory is designed.** Each long-lived structure has an owner and a size estimate, consistent with B1–B7 | PM7 |
| AC8 | **The complete model fits.** For each app-web capability, the proposal names where it lives and shows that it needs no architectural change: dedicated, shared and service workers; `SharedArrayBuffer` and Wasm threads; WebGL and WebGPU; media; accessibility; IME | ADR-0001 R1, charter principle 7 |
| AC9 | **`unsafe` stays where ADR-0002 allows it.** Any new allowlisted module is named and justified | ADR-0002, evaluation §15.3 |
| AC10 | **Every decision traces to evidence:** a source, a measurement, or a labelled inference or hypothesis | Research conventions |

## 6. Checkpoints

**Every checkpoint ends with:**
- every relative link resolving, checked by `tools/check-doc-links.py` (added 2026-10-17, with the owner's approval). Planned documents are named in code font, not linked, until they exist;
- the four standard checks green: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `tools/check-unsafe-policy.sh`. CP1–CP12 change no code;
- a commit on `step-4`, and an update to this tracker.

| CP | Scope | Status |
|---|---|---|
| **CP1** | This plan; branch `step-4`; the owner's decisions recorded | ✅ 2026-10-17 (§11). **Gate A passed:** approved as written |
| CP2 | Research note: macOS facilities for the runtime | ✅ 2026-10-17 (§11; [note](macos-runtime-facilities.md)) |
| CP3 | Event loop and threading model (P1) | ✅ 2026-10-17 (§11; [document](../architecture/event-loop-and-threading.md)) |
| CP4 | IPC: transport and interface definitions (OQ-17) | ✅ 2026-10-17 (§11; [document](../architecture/ipc.md)) |
| CP5 | Frame tree, navigation and loading (SI4) | ✅ 2026-10-17 (§11; [document](../architecture/frame-tree-and-navigation.md)) |
| CP6 | Rendering pipeline: data model, per-frame display lists, compositing, strings (P2, part of P4) | ✅ 2026-10-17 (§11; [document](../architecture/rendering-pipeline.md)) |
| CP7 | Invalidation and incremental updates (P3) | ✅ 2026-10-17 (§11; [document](../architecture/invalidation.md)) |
| CP8 | Input, focus, hit testing, scrolling and accessibility across frames (SI4) | ✅ 2026-10-17 (§11; [document](../architecture/input-and-accessibility.md)) |
| CP9 | Engine/embedder API and shell (F4, OQ-11) | ✅ 2026-10-17 (§11; [document](../architecture/embedder-api.md)) |
| CP10 | Script, agents and heap (OQ-5, OQ-13, OQ-14; ADR-0003 C1–C5); spike S3's design | ✅ 2026-10-17 (§11; [document](../architecture/script-and-heap.md); S3 design §10) |
| CP11 | Architecture overview v0, and the criteria check (§5) | ✅ 2026-10-17 (§11; [overview](../architecture/overview.md)) |
| CP12 | Synthesis: ADRs (Proposed); decision map, open questions, indexes, status. **Gate B** | ✅ 2026-10-17 (§11). **Gate B passed** 2026-10-17 |
| CP13 | **S3a** (authorized at Gate B): implementation and correctness | ✅ 2026-10-18 (§11) |
| CP14 | **S3b:** measurements | ✅ 2026-10-18 (§11) |
| CP15 | **S3c:** `docs/research/s3-results.md`; ADR-0008 updated; docs synthesis. **Gate C** | ✅ 2026-10-18 (§11). **Gate C:** the owner authorized CP16 and chose `SliceStack` for C1 |
| CP16 | **S3 addendum** (authorized at Gate C): C1's safe `SliceStack` and its field-order control in `s3-gc`, the oracle and the benchmarks; formal runs; §3 of the S3 results and the S3-Q1 verdict corrected | ✅ 2026-10-18 (§11). **Gate C passed:** ADR-0008 accepted |
| CP17 | Merge `step-4` into `main` (`--no-ff`) and push `main`, as the owner instructed at Gate C | ✅ 2026-10-18 (§11) |

### 6.1 What each checkpoint decides

**CP2. macOS facilities** (research note; answers no question by itself, but supplies facts to CP3, CP6, CP8 and CP9):
- **Run loops.** CFRunLoop and NSApplication; how a Mach port or kqueue source joins a run loop; Grand Central Dispatch; waiting without AppKit, in helpers.
- **Display timing.** AppKit's `displayLink(target:selector:)` on `NSView`, `NSWindow` and `NSScreen`, which the SDK 27 header names as the replacement for `CVDisplayLink` (deprecated since macOS 15). **Inference to confirm:** these are AppKit objects and renderers have no AppKit, so a renderer's frame clock must come from the browser.
- **Threads.** Quality-of-service classes and how macOS places them on the M1's performance and efficiency cores; policies for background processes.
- **Surfaces.** IOSurfaces across processes as Mach ports; layer contents; whether the public SDK offers remote layer hosting.
- **Input and accessibility.** NSEvent and gestures; `NSTextInputClient` for IME; the `NSAccessibility` protocols, and how browsers expose web content through them.
- **Observations of this Mac:** SDK headers and availability annotations; `nm -u` on installed browsers, for the display-link and layer-hosting interfaces they import.

**CP3. Event loop and threading (P1; Q1–Q4):**
- The HTML processing model mapped onto Axiom's processes: one event loop per agent (window or worker); task sources and their priorities; microtasks; timers; rendering opportunities; idle callbacks.
- The threads of each process type and their QoS: browser (main/UI, IPC, compositor); renderer (main, IPC, workers, any helper threads); network; GPU; utility.
- How each thread waits: one wait primitive per thread for IPC, timers and OS events, so that idle means zero wakeups (AC3).
- Frame scheduling at the event-loop level: frame requests, the display's clock, backpressure, throttling (completed at CP7).
- Agents and workers on threads (OQ-5, first half).
- Deterministic scheduling and virtual time for tests (AC6).

**CP4. IPC (OQ-17; Q5):**
- The transport, chosen from S4's evidence (Mach messages or socket pairs) against requirements R1–R6.
- An interface definition language, with a generator that emits safe decoders (IR1; ADR-0002 rule 7), message bounds (R5), and versioning by build identifier (IR8).
- Endpoints as capabilities: bootstrapping, passing endpoints, binding to site locks (IR4).
- Shared memory with one writer (IR7). Handles: IOSurfaces, memory, descriptors (R2).
- Synchronous calls from children, only where the platform requires them (IR6). The browser never blocks (IR5).
- The in-process transport, for single-process mode and tests (R6).
- Where the IPC thread sits in each process (with CP3).

**CP5. Frame tree, navigation and loading (SI4; Q6, Q7):**
- The frame tree in the browser: navigables, traversable navigables, browsing context groups, documents and session history, mapped onto the HTML standard.
- Site assignment and process selection at navigation (SI1–SI7), including cross-origin isolated groups (COOP and COEP).
- In renderers: local frames, and proxies for frames held by other processes, like Chromium's frame proxies and WebKit's remote frames; what state each proxy replicates.
- Cross-frame operations, all through the browser: `window.parent`, `frames[]`, `postMessage`, `opener`, named targets, navigating another frame.
- Navigation: the commit protocol; the loader's interface in the network process, shaped like Fetch; lock checks at each step (IR2); cookie and storage accessors bound to the lock (IR4).
- L0 (one process) runs the same interfaces. Under SI3, a cross-site frame in its embedder's process still goes through the frame-tree interfaces.
- How check V9 (out-of-process iframe tests) will exercise this design.

**CP6. Rendering pipeline (P2, part of P4; Q2, Q8–Q10):**
- The data structures: the DOM in the GC heap (ADR-0003); computed style; the box tree; an immutable fragment tree; the per-frame display list; compositor input (layers or property trees).
- Ownership and the thread of each stage. Whether style or layout run in parallel, given a single-threaded heap (Q2).
- The display list: primitives; images as decoded pixels in shared memory; glyphs as masks or as runs in system fonts (Step 5 decides where glyphs are rasterized); clips, transforms, effects and scrolling; validation in the browser (AC4).
- Compositing across processes: one frame's output embedded in another's by reference; frame timing and synchronization.
- Strings and atoms across the parser, DOM, style and JavaScript (part of P4).
- Stable node identity and geometry that can be queried, for accessibility (with CP8).

**CP7. Invalidation (P3; Q3, Q11, Q12):**
- The rendering lifecycle: its phases and states, and which operations may run in each, as in Blink's document lifecycle.
- Propagation: DOM mutation → style invalidation → layout dirtiness → fragment reuse → display-list reuse → compositor damage.
- Synchronous flushes (`offsetWidth`, `getComputedStyle`) and a bound on their cost.
- Streaming parsing, late resources (images, fonts) and resizing.
- Updates off the main thread: scrolling and compositor animations, and how they cross processes.
- When a renderer produces a frame, and backpressure (completing CP3).

**CP8. Input, focus, hit testing, scrolling and accessibility (SI4; Q13):**
- Coarse hit testing in the browser, over the surfaces of the frame tree; fine hit testing in the renderer, inside one frame; event targeting across processes; pointer capture.
- Keyboard focus across frames. IME through `NSTextInputClient` in the browser, with text state from the renderer.
- Scrolling: who owns scroll offsets, and scroll chaining between frames in different processes.
- User activation, tracked in the browser (process model §3).
- Accessibility: trees from renderers, validated and merged by the browser (process model §4.2), exposed through `NSAccessibility`.

**CP9. Engine/embedder API and shell (F4, OQ-11; Q14):**
- Layers in the browser process: the shell (windows, tabs, UI); engine services (frame tree, navigation, process manager, compositor); platform.
- The API: views, navigation, lifecycle and load events, permissions, downloads, dialogs, input, presentation.
- A headless embedder, for tests and WPT, and a single-process mode (AC6).
- Pure Rust (objc2) or a thin Swift shell (OQ-11), from S2's evidence and what S2 left untested: accessibility, IME, menus, app bundles.
- Precedents: WKWebView, Chromium's content API, CEF, Servo's embedding API, Ladybird's LibWebView.

**CP10. Script, agents and heap; spike S3's design (Q4, Q15):**
- Agents, realms and heaps (OQ-5, second half): one heap per agent; `SharedArrayBuffer` memory outside the heaps; structured clone and transfer.
- Integration with the event loop: jobs, microtasks and the host hooks.
- The root set: VM registers and stacks traced as a whole; where native code roots values (C1); the root-stack design hypothesis.
- C2's layout rule; C3's barrier API; C4's plan for incremental or generational collection; C5's pinned or non-moving space.
- How the bindings connect the DOM and JavaScript in one heap, from generated code.
- **Spike S3's design**, written in §10: questions, workload, method, pass criteria and crates. Expected questions:
  - the cost of a root stack without `RefCell`, against C1's target of about 1 ns per root and per read (S1 measured 2.05 and 1.58 ns);
  - rooting traffic in an interpreter loop whose builtins call back into script. This tests ADR-0003's assumption that most rooting traffic comes from builtins, not from bytecode;
  - the dispatch techniques available on stable Rust, the language evaluation's original S3.

  Experiments for C2 (layout) and C3 (barrier ceremony) may be proposed as optional parts, which the owner can authorize separately.

**CP11. Overview v0 (Q16):**
- The whole system on one page: processes, threads, pipeline and IPC.
- A crate structure for the eventual implementation, with its `unsafe` map (AC9).
- End-to-end scenarios:
  - a first navigation;
  - typing in a text field;
  - scrolling a page that contains a cross-site iframe;
  - script forcing layout;
  - a renderer crash.
- The check of every criterion in §5, including the full-model fit (AC8).
- Staging: what exists at L0, L1, L2 and L3, with nothing rewritten (AC1).

**CP12. Synthesis:**
- ADRs 0005–0009, Proposed.
- Updates:
  - the decision map: F4 and P1–P3 Proposed; the Step 5 list, if the owner agrees (§2);
  - open questions, indexes and status.
- **Gate B** (§7).

**CP13–CP15. Spike S3**, authorized at Gate B:
- It follows S1's methods: `s1-probe roots`, the dispatch suite, three process runs, and a clean tree for formal runs.
- CP15 writes the results and updates ADR-0008.
- **Gate C** (§7).

**CP16. S3 addendum**, authorized at Gate C: C1's safe `SliceStack`, with its length first, and a control that differs only in field order; formal runs by S1's methods; the S3 results' §3 and the S3-Q1 verdict corrected in the documents. **ADR-0008 is Accepted** once its measurements are complete (the owner's condition).

**CP17. Merge** `step-4` into `main` with `--no-ff`, and push `main`, as the owner instructed at Gate C.

## 7. Review gates

Between gates, checkpoints run without pausing, as in Steps 2b and 3.

| Gate | After | The owner decides |
|---|---|---|
| A | CP1 | This plan, including its scope (§2). **Passed 2026-10-17:** approved as written |
| B | CP12 | ADRs 0005–0009 (Proposed). ADR-0008's acceptance may wait for S3, as ADR-0004's waited for S4. Also: authorizing spike S3's code. **Passed 2026-10-17:** ADRs 0005–0007 and 0009 accepted; ADR-0008 pre-approved, pending S3-Q1 and S3-Q2; S3 authorized (Q1–Q5); AC5 amended |
| C | CP15 | ADR-0008, in the light of S3; merging `step-4` into `main`. **Decided 2026-10-18:** an S3 addendum (CP16); C1 is `SliceStack`, with no `unsafe` in `gc`; ADR-0008 accepted and `step-4` merged once the addendum's measurements are complete (§2). **Passed 2026-10-18,** when CP16's measurements were complete |

## 8. Method

- **Sources** follow the [research conventions](README.md): primary sources first, each dated, every claim labelled.
- **Specifications:**
  - WHATWG HTML: event loops, navigables, browsing context groups, agent clusters, "update the rendering";
  - DOM, CSSOM View and Fetch;
  - ECMAScript: agents and jobs.
- **Engines,** from their documentation and source:
  - Chromium: RenderingNG and BlinkNG, LayoutNG, property trees and Viz, out-of-process iframes and input routing, Mojo, the Blink scheduler;
  - Firefox: Fission, Stylo, WebRender, IPDL;
  - WebKit: site isolation, remote layer trees, its IPC;
  - Servo: the constellation, layout, WebRender, the embedding API, ipc-channel;
  - Ladybird: its event loop, process architecture and IPC compiler.
- **Apple:** developer documentation, and the SDK headers on this Mac (macOS 27.2, SDK 27.0).
- **Measurements:** only spike S3, after Gate B.

## 9. Risks to this plan

| Risk | Mitigation |
|---|---|
| Breadth: many coupled decisions | The checkpoint order follows the dependencies. CP11 reconciles the documents, and the ADRs group tightly coupled decisions |
| Specifying internals that belong to Step 5 | Step 4 fixes interfaces, ownership, threads and invariants. Algorithms stay in Step 5 |
| Precedent documentation is incomplete or out of date | Date every source. Prefer source code where documentation lags. Never invent history |
| A design question needs evidence that no source settles | Label it a hypothesis, and propose a spike to the owner. No spike runs without authorization |

## 10. Spike S3 design

Written at CP10. At Gate B (2026-10-17) the owner authorized S3's code: "Authorized for the full scope (questions S3-Q1 through S3-Q5 under spikes/s3/)". **Purpose:** test ADR-0003's obligation C1, and its assumption about where rooting traffic comes from, in an interpreter-shaped workload; and measure how stable Rust can dispatch bytecode on the M1, as evidence for J1. The design is in [script and heap](../architecture/script-and-heap.md) SH6–SH7.

### 10.1 Questions

| ID | Question | Required? |
|---|---|---|
| S3-Q1 | **C1:** can a root stack with 8-byte slots and no `RefCell` borrow tracking (an `UnsafeCell`-based stack in one allowlisted module, Miri-clean) cost **at most 1.0 ns per root and per read**, hot, on a performance core? It is measured the way `s1-probe roots` measured S1's stack: 4.08 ns per frame, 2.05 ns per root, 1.58 ns per read. How much does it cut S1's `dispatch` case (Pattern B: 1,073 ns per dispatch, 2.06× Pattern A)? | Yes |
| S3-Q2 | **Rooting traffic:** in a register-based bytecode interpreter over Pattern B arenas, with registers traced as whole regions and builtins that call back into bytecode while holding values, how many root operations, and what share of time, does each workload spend on rooting? Does ADR-0003's assumption hold that rooting comes from builtins, not from executing bytecode? | Yes |
| S3-Q3 | **Dispatch:** what does an executed operation cost under a `match` loop, a table of function pointers, and handlers that return the next handler, on stable Rust 1.94.1? Explicit tail calls (`become`) are measured on the pinned nightly as a reference, if available there. This is the language evaluation's original S3 | Yes |
| S3-Q4 | **C2:** on S1's `traverse` suite, do power-of-two strides or struct-of-arrays link columns remove the index arithmetic from the dependent chain, and by how much? | Optional; authorized at Gate B |
| S3-Q5 | **C3:** with generated setters for handle-valued fields, how does S1-Q2's ceremony count change, and what does a write cost with a no-op barrier and with a card-marking barrier? | Optional; authorized at Gate B |

### 10.2 What gets built

Refined during CP13, with reasons, in §11: a third root stack, without `unsafe`; the root stack borrowed rather than shared; a supplementary push measurement. **The CP16 addendum** adds C1's final stack, `SliceStack`, safe and contiguous, with a field-order control.

| Crate | Contents | `unsafe` |
|---|---|---|
| `spikes/s3/gc` (`s3-gc`) | A copy of S1's Pattern B heap (`s1-b-gc` stays untouched, as the baseline). It has two root stacks behind one interface: S1's `RefCell` stack, and the C1 stack in `src/raw/` | Only `src/raw/`, allowlisted in `tools/unsafe-policy.txt` with numbered invariants, and run under Miri (Stacked and Tree Borrows, strict provenance), as S1's Pattern A heap was |
| `spikes/s3/vm` (`s3-vm`) | Tagged 64-bit values with `u32` handles inside; about 20 register-machine operations (constants, moves, arithmetic, comparisons, jumps, objects with fixed slots, arrays, calls, returns, closures); builtins that call back into bytecode (`forEach`, `map` with allocation, `sortBy` with a comparator, a `valueOf`-style coercion hook); and a reference evaluator that serves as the oracle. The interpreter's dispatch technique is selectable | None (`forbid`) |
| `spikes/s3/bench` (`s3-bench`) | The harness and an `s3-probe` binary for unit costs; a port of S1's `dispatch` case onto `s3-gc` | None |
| `tools/s3-bench.sh`, `tools/s3-profile.sh` | Formal runs and `samply` profiles, on the model of S1's scripts | — |

No dependencies beyond the standard library. Results are written under `target/s3-results/`, and nothing outside the repository is touched.

### 10.3 Workloads and correctness

| Workload | Exercises |
|---|---|
| W1, arithmetic loop | Dispatch only; no allocation |
| W2, object churn | Allocation, slot access, frequent collection |
| W3, callback-heavy builtins | `forEach`, `map` and `sortBy` with closures, re-entering from Rust while values are held (rooting in builtins) |
| W4, tree traversal and mutation | Shapes from S1's scenarios, in bytecode |

**Correctness first:** every program's result must equal the reference evaluator's at normal collection pacing, and when collecting at every allocation (stress). Pattern B's debug epochs run throughout, as in S1, to catch stale handles. Miri covers `src/raw/`.

### 10.4 Measurements and pass criteria

- **Unit costs** (`s3-probe roots`): frame, root and read, for both root stacks.
- **Dispatch suite:** S1's case on both root stacks.
- **Per workload:** nanoseconds per executed operation; root operations per executed operation, from counters in a counting build; and the share of time spent rooting, from `samply` profiles, for each root stack and each dispatch technique.
- **Method:** release builds; three process runs with cool-downs; user-interactive QoS; formal runs from a clean tree, as in S1.

| Question | Passes if |
|---|---|
| S3-Q1 | The C1 stack measures ≤ 1.0 ns per root and ≤ 1.0 ns per read, hot, and its module is Miri-clean |
| S3-Q2 | Executing bytecode performs no per-value root operations, and with the C1 stack rooting takes ≤ 10% of run time in each of W1–W4. The owner confirmed the 10% threshold at Gate B |
| S3-Q3 | Report only: the costs, and a recommended technique for Step 5 |

**If S3-Q1 fails,** ADR-0003's first revisit condition applies, and the owner decides. **Outputs:** `docs/research/s3-results.md`; ADR-0008 updated (C1 settled, the dispatch technique recorded as input to J1); OQ-14 resolved.

### 10.5 Checkpoints

CP13 builds the crates, the oracle and the Miri runs (S3a). CP14 measures (S3b). CP15 writes the results (S3c).

## 11. Results log

### CP1 — completed 2026-10-17

- `main` pushed to `origin` at `da79e69`, as the owner authorized.
- Branch `step-4` created from `main` at `da79e69`.
- The owner's confirmation of S4's three refinements recorded in [ADR-0004](../adr/0004-process-model-and-sandbox.md)'s decision history.
- This plan written. [Status](../status.md), the [decision map](../decision-map.md) and the indexes ([docs](../README.md), [research](README.md)) updated.
- Every relative link in `docs/` resolves, and the four standard checks pass, with the code unchanged.

**Gate A.** The owner reviews this plan.

### Gate A — passed 2026-10-17

- The owner approved the plan as written: scope, criteria AC1–AC10, and the 16 checkpoints.
- Step 5's list in the [decision map](../decision-map.md) §5 now names "HTML parsing and DOM" and "JavaScript engine and bindings", as the owner instructed.
- Cadence: CP2–CP12 run without pausing. Gate B, after CP12, covers ADRs 0005–0009 and the authorization of S3's code together.
- `step-4` pushed to `origin`, as the owner authorized.

### CP2 — completed 2026-10-17

[macOS runtime note](macos-runtime-facilities.md) written, from SDK 27 headers, `sysctl`, `system_profiler`, the imports of Chrome Canary 158 and Tor Browser 16.0a11, and the Chromium, Firefox, WebKit and Ladybird sources. Main findings:
- **Frame clock.** `CVDisplayLink` is deprecated since macOS 15. Its replacement, `CADisplayLink`, comes only from `NSView`, `NSWindow` or `NSScreen`, so renderers can't have one. The browser owns the frame clock and sends frame signals, as WebKit's UI process does with `DisplayDidRefresh`. Chromium needs a private function to create a display link outside AppKit.
- **Waiting.** Helpers can wait on Mach messages, timers and descriptors in one `kevent64` call, receiving the message in the same call. The browser's engine sources go in the run loop's common modes, as Firefox and WebKit do.
- **Priority.** Mach messages can carry the sender's QoS and importance (a new input to OQ-17). Apple says background work is "more likely" to run on the efficiency cores, not that it always does.
- **Surfaces.** IOSurfaces cross only as Mach ports. Since macOS 14.4 a surface's memory can be charged to another task (hypothesis: useful for per-renderer accounting).
- **IME and accessibility** are synchronous queries on the browser's main thread, so the browser answers them from state it holds. Chromium instead blocks its UI thread for up to 1.5 s on the renderer.
- 16 implications for CP3–CP10, and five open questions.

Every relative link resolves; the four standard checks pass, with the code unchanged.

### Between CP2 and CP3 — 2026-10-17

At the owner's request, `CLAUDE.md` was written as the project's working instructions, with the rule that every decision put to the owner carries a recommendation and its rationale. On the owner's approval ("proceed with your recommendation"), `tools/check-doc-links.py` was added as the fifth check before every commit, and `CLAUDE.md` stays in the public repository.

### CP3 — completed 2026-10-17

[Event loop and threading model](../architecture/event-loop-and-threading.md) written (**Proposed**; rules EL1–EL24), from the HTML and ECMAScript specifications, Apple's run-loop and QoS guidance, XNU's scheduler documentation, and Chromium, Firefox, WebKit, Ladybird and Servo. Main proposals:
- **One wait per thread:** helper threads in one `kevent64` call; AppKit's main thread and the compositor in their run loops, with engine sources in the common modes on the main thread.
- **Endpoints bound to threads,** with no intermediate I/O thread.
- **Browser threads:** main (AppKit only), **core** (frame tree, navigation, process management, policy), compositor (display links, frame scheduling, compositing), and a worker pool.
- **Renderers: one thread per event loop,** with a heap each, plus a control thread and a helper pool that never touches a heap. Servo's script threads are the closest precedent.
- **Task selection:** discrete input, then a due rendering update, then other tasks oldest first, then idle work. Continuous input is coalesced into the next frame, as Chrome has done since 2017.
- **Frames:** the browser's compositor owns a paused-when-idle display link per window, offers frames only to subscribed event loops (one outstanding offer each), and never waits for a renderer.
- **Throttling** starts from Chrome's tested levels and covers whole agent clusters, as ECMAScript requires. QoS follows visibility, using public interfaces only.
- **Clarification:** "one agent cluster per process" (decision map §4, ADR-0004) means that an agent cluster never spans processes. A renderer may host several clusters of its own site. CP12 brings the decision map's wording into line.

**Housekeeping: a flaky check.** The first test run after a session restart failed once.
- Its totals traced the failure to `s4-sandbox`'s unit tests. Sixty repeated runs reproduced it once, in `this_process_has_a_plausible_footprint`.
- The test asserted that the lifetime-maximum footprint is at least the current one. A single `proc_pid_rusage` call doesn't snapshot both counters together, and the maximum trailed by one 16 KiB page, about once in 75 runs.
- The assertion now checks only the maximum's scale. Afterwards there were no failures in 150 runs. This maintains S4's authorized test suite, in its own commit.
- S4's reported footprints are unaffected (inference): a lag of one page in the peak is negligible beside MiB-scale results.

Every relative link resolves; the four standard checks pass.

### CP4 — completed 2026-10-17

[IPC: transport and interface definitions](../architecture/ipc.md) written (**Proposed**; rules IP1–IP22), from S4's measurements, the SDK 27 Mach headers, and Chromium (Mojo), Firefox (IPDL), WebKit, Servo (`ipc-channel`), Ladybird (LibIPC) and libdispatch. Main proposals:
- **OQ-17: Mach messages,** the only transport between processes. They carry every handle Axiom passes, IOSurfaces included, and an audit trailer per message. Small round trips are 24% faster than over sockets. Chromium, Servo and Ladybird use Mach on macOS. The same channels run in-process, always encoding and decoding.
- **Endpoints are capabilities** that only the browser creates and hands out. Launch pins each child's audit token. Ports are guarded and immovable, with queue limits.
- **An IDL and a safe-Rust generator** emit types, canonical encoders, checking decoders and a fuzzing entry point per decoder. Handles never sit in the bytes.
- **`Untrusted<T>`,** after WebKit's 2026 design: decoded values can't be used before a validator checks them against the browser's records. A violation is `Terminate` or, for a race, `Ignore`.
- **Synchronous requests** only toward more privilege, only where the platform requires them, with timeouts and no re-entry. Bulk data goes out of line or through single-writer shared memory. Every queue is bounded, and the browser never blocks.
- **Not settled by sources:** whether a priority override on a one-way message boosts a receiver waiting in `kevent64` (IP20). It is measured in the first implementation milestone, and the macOS note's open question 2 now says so.
- OQ-17's entry in the open questions is updated at CP12, with the ADRs.

Every relative link resolves; the four standard checks pass.

### CP5 — completed 2026-10-17

[Frame tree, navigation and loading](../architecture/frame-tree-and-navigation.md) written (**Proposed**; rules FT1–FT14), from the HTML standard (navigables, browsing context groups, session history) and from Chromium (navigation, out-of-process iframes), Firefox (BrowsingContext, process selection), WebKit (site isolation) and Servo (constellation). Main proposals:
- **The core thread holds the only authoritative frame tree:** tabs, frames, documents, browsing context groups with their agent-cluster maps, and processes. Every fact in it the browser derived itself. Tokens are scoped per process.
- **Renderers hold a skeleton replica:** the tree's shape, names and closed state, enough for script's synchronous questions and nothing more (WebKit's side-channel caution). Remote frames are remote `WindowProxy` objects with only the cross-origin operations.
- **Every operation between agents goes through the browser,** even within one process under SI3. So L3 changes only process selection, never an interface.
- **Navigation is browser-driven,** from request to acknowledged commit, with process and event-loop selection at the response (SI1–SI7). Session history lives in the browser, and same-document navigations stay synchronous and are validated afterwards.
- **Loader and storage endpoints are bound at commit** to the document's facts. Bodies stream from the network process to the renderer.
- **V9's lying-renderer tests** are specified (FT14).

Every relative link resolves; the four standard checks pass.

### CP6 — completed 2026-10-17

[Rendering pipeline](../architecture/rendering-pipeline.md) written (**Proposed**; rules PL1–PL16), from Chromium (RenderingNG data structures, LayoutNG), Firefox (WebRender, APZ), Stylo, and SpiderMonkey's Latin-1 strings. Main proposals:
- **Five stages, five owned structures:** the DOM in the GC heap; computed styles in a side table; the box tree; immutable fragments with their constraints, as in LayoutNG; a display list per frame. Everything after the DOM lives outside the heap.
- **Display lists** separate property trees (spatial, clip, effect) from items, and carry only validated primitives. Glyph images come from an atlas rasterized in the renderer with CoreText (S4-Q5), and images as decoded pixels. Child frames are embedded by token, and hit-test regions ride along. The browser never decodes or parses (AC4); its compositor validates every reference against its records.
- **Strings:** one 8-bit or 16-bit representation for JavaScript and the DOM. Atoms are static plus process-wide dynamic, comparable from any thread.
- **Style and layout run on the event loop's thread,** within AC5 as approved. The DOM arenas are built to be lendable: no interior mutability (checked by the compiler), and side tables for results.
- **For Gate B:** whether to amend AC5 and EL11 to allow read-only lending of the DOM arenas during a rendering stage frozen by the borrow checker, which would enable parallel style and layout without a second copy of the DOM.

Every relative link resolves; the four standard checks pass.

### CP7 — completed 2026-10-17

[Invalidation and incremental updates](../architecture/invalidation.md) written (**Proposed**; rules IV1–IV14), from Chromium (BlinkNG, style invalidation sets, LayoutNG), Firefox (APZ) and CSS Containment Level 2. Main proposals:
- **An explicit lifecycle per document,** whose phases are functions with immutable outputs. Phase order is checked by the compiler, through tokens that only completing a phase produces.
- **Two entry points:** the rendering update, and synchronous queries that run only the phases they need. **No synchronous flush crosses an agent,** so out-of-process frames behave like in-process cross-site frames.
- **Reuse at every stage:** pending, selector-driven style invalidation that over-invalidates rather than miss; box rebuilds only where needed; fragments reused by constraints, with containment as a boundary; display items reused by ID; damage-limited compositing.
- **The compositor scrolls and animates** from its property trees and display ports. Scroll offsets are reconciled by sequence numbers; listeners that may block scrolling are asked with a deadline; transform and opacity animations run on the compositor.
- **Late, hidden and dependent content:** parser yields; targeted invalidation for images, fonts and resizes; no frames for hidden or offscreen frames; one frame in flight; cross-frame sizes through the browser.
- **IV14:** a test mode compares every incremental update against a rebuild from scratch.

Every relative link resolves; the four standard checks pass.

### CP8 — completed 2026-10-17

[Input, focus, hit testing, scrolling and accessibility](../architecture/input-and-accessibility.md) written (**Proposed**; rules IN1–IN12), from the HTML standard (user activation, focus), Chromium (out-of-process iframes, RenderingNG), and Firefox (`ContentCache`, the Firefox 113 accessibility cache, APZ). Main proposals:
- **Input enters at the compositor,** which hit-tests the composited frames and routes to the target frame's event loop in that frame's coordinates. Imprecise regions are asked with a deadline.
- **Gestures are latched;** scrolling stays in the compositor, chaining across frames and processes.
- **Pointer capture and focus are browser records;** sequential focus moves between frames through the browser.
- **IME is answered from a mirror on the main thread,** kept current by the focused frame, as Firefox's `ContentCache` does. Chromium's 1.5 s block is avoided.
- **User activation comes from routed input,** propagated and consumed as the HTML standard specifies, decided by the browser.
- **Accessibility is a cache in the browser,** stitched across frames by token and enabled on first use. Firefox reports up to 20× gains on its worst pages.
- **Mediation and integrity:** the browser mediates the pasteboard, menus and dragging; hit-test claims are clipped to each frame's area; routing is tested across processes.

Every relative link resolves; the four standard checks pass.

### CP9 — completed 2026-10-17

[Engine/embedder API and the shell](../architecture/embedder-api.md) written (**Proposed**; rules EM1–EM12), from Chromium's content API, Apple's WKWebView delegates and Servo's constellation messages. Main proposals:
- **The shell is an embedder:** AppKit's main thread, talking to the engine's core through a message API defined in the IDL. The engine never depends on AppKit or the shell; the build enforces the rule, like Chromium's content/chrome layering.
- **Commands in; events and requests for decisions out** (new windows, permissions, dialogs, downloads, file choices, authentication, context menus). The engine applies its policy first, and every request has a default and never waits indefinitely.
- **Presentation, input, IME and accessibility** cross as objects the shell hosts and the engine keeps current.
- **The browser's own UI is native,** so there is no privileged web UI.
- **A headless embedder** with offscreen surfaces, synthetic input and virtual time, for WPT and reference tests.
- **OQ-11: pure Rust with objc2.** The shell milestone first validates an objc2-defined view class for text input and accessibility, which S2 never tried. A Swift shell can sit behind the same API if needed.

Every relative link resolves; the four standard checks pass.

### CP10 — completed 2026-10-17

[Script, agents and the heap](../architecture/script-and-heap.md) written (**Proposed**; rules SH1–SH14), from ADR-0003 and S1's measurements, V8's Orinoco, SpiderMonkey's collector, and the SDK's wait-on-address header. Spike S3's design is written in §10. Main proposals:
- **One heap per agent,** on its event loop's thread. DOM objects are the JavaScript objects, with no wrappers and no cross-heap references.
- **Between agents, only copies, transfers and shared buffers.** Backing stores live outside the heap. `Atomics.wait` uses the public `os_sync_wait_on_address` (macOS 14.4 or later).
- **Script runs only in tasks and microtasks.** An atomic interrupt flag stops runaway scripts.
- **The root set:** the interpreter's registers and frames traced as whole regions, the root stack for native code, and the agent's own roots.
- **C1–C5:**
  - **C1:** a root stack without borrow tracking, targeting about 1 ns; **spike S3 tests it**.
  - **C2:** struct-of-arrays columns or power-of-two strides for hot link fields.
  - **C3:** generated setters as the single place for a future barrier.
  - **C4:** stop-the-world now; incremental marking in idle periods and a generational nursery later, ordered by corpus measurements.
  - **C5:** memory that reaches platform APIs lives outside the heap and never moves.
- **Spike S3 design (§10):** three required questions (C1's root stack; rooting traffic in an interpreter; dispatch on stable Rust) and two optional ones (C2 layout; C3 barrier ceremony). It has `s3-gc` (a copy of S1's heap with an allowlisted `src/raw/` checked by Miri), `s3-vm` with a reference-evaluator oracle, and `s3-bench`. Pass criteria: ≤ 1.0 ns per root and per read, and rooting ≤ 10% of run time (a threshold for the owner to confirm).

Every relative link resolves; the four standard checks pass.

### CP11 — completed 2026-10-17

[Architecture overview v0](../architecture/overview.md) written (**Proposed**). It has:
- the system on one page;
- a crate structure whose `unsafe` stays in `platform`, `gc`'s raw module, `jit` (later) and the shell's `platform` module;
- five end-to-end scenarios traced through the rules: opening a URL, typing, scrolling across a cross-site iframe, forced layout, a renderer crash;
- the criteria check, where **AC1–AC10 are all met**: AC3 and AC7 by design, with measurement to come, and AC5 as written, with the lending amendment left to Gate B;
- the complete-model table (AC8);
- the L0–L3 staging, in which L3 changes only process selection.

It also regroups the ADRs: ADR-0007 now covers the frame tree, pipeline, invalidation and input across frames (SI4 as a whole).

Every relative link resolves; the four standard checks pass.

### CP12 — completed 2026-10-17

**ADRs written, all Proposed:**
- [ADR-0005](../adr/0005-ipc-transport-and-interfaces.md): IPC; resolves OQ-17.
- [ADR-0006](../adr/0006-event-loop-and-threading.md): event loop and threading, recording the clarification that an agent cluster never spans processes.
- [ADR-0007](../adr/0007-frame-tree-pipeline-and-input.md): frame tree, pipeline, invalidation and input (SI4).
- [ADR-0008](../adr/0008-script-agents-and-heap.md): script, agents and heap, recommended for acceptance after S3.
- [ADR-0009](../adr/0009-embedder-api-and-shell.md): embedder API and shell; OQ-11.

**Updates:**
- [Decision map](../decision-map.md): F4, P1–P3 and part of P4 Proposed. F5 (IPC) added. F2 notes ADR-0008. §4's agent-cluster wording corrected. Step 4 marked as in review.
- [Open questions](../open-questions.md): proposed resolutions for OQ-5, OQ-11, OQ-13 (design) and OQ-17. OQ-14 points to S3's design. **New:** OQ-19 (amend AC5 for lending) and OQ-20 (QoS on one-way Mach messages).
- Indexes and status updated.

Every relative link resolves; the four standard checks pass.

**Gate B.** The owner decides, as listed with recommendations in [status](../status.md), "Next steps": the five ADRs; spike S3's code, with its optional parts and threshold; and OQ-19.

### Gate B — passed 2026-10-17

The owner's decisions, each as recommended:
- **Accepted:** ADR-0005 (IPC), ADR-0006 (event loop and threading), ADR-0007 (frame tree, pipeline, invalidation and input) and ADR-0009 (embedder API and the pure-Rust shell). OQ-11 and OQ-17 are resolved.
- **ADR-0008:** "Pre-approved; will move to Accepted upon S3-Q1 and S3-Q2 passing."
- **Spike S3:** "Authorized for the full scope (questions S3-Q1 through S3-Q5 under spikes/s3/)". S3-Q2's threshold is "Confirmed at ≤10% of runtime spent rooting".
- **OQ-19:** "Amend criterion AC5 and rule EL11 to allow read-only scoped lending of DOM arenas to helper threads during borrow-checker-frozen rendering stages. Keep the actual parallel style/layout implementation deferred until profiling benchmarks justify it." AC5 (§5), EL11 and PL10 are amended, and ADRs 0006 and 0007 record the amendment.
- **The S4 test fix** (CP3's housekeeping) is approved.

Recorded in the ADRs, the design documents, the decision map, the open questions, the indexes and [status](../status.md). CP13 starts.

### CP13 — completed 2026-10-18 (S3a: implementation and correctness)

**Built,** with no dependencies beyond the standard library and S1's crates (`s1-b-gc` is untouched):
- **`spikes/s3/gc` (`s3-gc`):** S1's Pattern B heap, plus:
  - `Value`, a NaN-boxed 64-bit value with `u32` handles inside, carrying the debug collection epoch in spare bits;
  - **three root stacks behind one `RootStack` trait:** `RefCellStack` (S1's borrow-tracked design, with 8-byte slots); `CellStack` (8 KiB chunks of `Cell` slots, allocated on demand, no `unsafe`); and `RawStack`, C1's (one lazily committed, fixed-capacity array of `Cell` slots);
  - a write-barrier hook (`NoBarrier`, `CardBarrier`) and a `setters!` macro, for S3-Q5.

  Its only `unsafe` is 3 blocks in `src/raw/`, under invariants R1–R3, allowlisted in `tools/unsafe-policy.txt`. 1,411 lines, plus 586 of tests.
- **`spikes/s3/vm` (`s3-vm`, `forbid(unsafe_code)`):**
  - a register machine with 28 operations, Lua-style calls, and a register file of `Cell`s traced as one region;
  - builtins `forEach`, `map` and `sortBy`, which call back into bytecode and root what they hold in Pattern B's style, and a `valueOf`-style coercion hook;
  - four dispatch techniques: a `match` loop, a table of handlers, handlers that return the next handler, and `become` on nightly (feature `nightly-tail`);
  - the reference evaluator; workloads W1–W4 with native models; a `count` feature.

  3,580 lines, plus 639 of tests.
- **`spikes/s3/bench` (`s3-bench`, `forbid(unsafe_code)`):**
  - S1's harness, generalized to any number of variants;
  - `s3-probe` (`bench`, `aggregate`, `roots`, `profile`, `sizes`) and `s3-count`;
  - S1's Pattern B DOM ported to `s3-gc`, generic over stack and barrier, with handle-valued fields private behind generated setters, for S1's `dispatch` and `mutate` cases;
  - four node layouts for S3-Q4.

  2,271 lines.
- **Scripts:** `tools/s3-bench.sh`, `tools/s3-profile.sh` and `tools/s3-ceremony.sh`. `tools/samply-summary.py` gains `--share NAME=REGEX`: the share of samples whose inlined frames include a matching file, so that root-stack code counts wherever it was inlined.

**Correctness:**
- **The oracle.** Every workload, and 34 edge-case programs in 10 tests, give the reference evaluator's result on 3 techniques × 3 stacks × 2 pacings (default, and a collection at every allocation). The edge cases cover 8 operators on 144 operand pairs each, `-0`, errors, unbounded recursion, closures, deep returns, and builtins whose callbacks grow the array, count comparisons, return NaN or hooked objects, allocate, and nest builtins.
  - Debug builds check every handle's epoch; release builds pass too.
  - The reference evaluator agrees with every workload's native model.
- **Counted builds** (`--features count`): bytecode performs **zero** root operations, in every workload and technique. The interpreter executes exactly the reference evaluator's instructions.
- **The oracle bites.** A builtin changed to hold its callback across callbacks, instead of re-reading it, fails the oracle with a stale-handle panic.
- **Miri** (nightly 2026-04-08, strict provenance, Stacked Borrows and Tree Borrows; its leak checker is on by default): **no undefined behaviour and no leaks**, on `s3-gc`'s 44 tests (S1's heap tests on all three stacks, the root-stack tests, the doc example), and on `s3-vm`'s Miri test. That test runs every workload, small, on `RawStack` with two techniques and a collection at every allocation, about 190 s per borrow model.
- **Every benchmark suite verifies its inputs** (logs, digests, folds, results) in a quick pass under `cargo test`.

**Refinements made during implementation:**
1. **A third stack, `CellStack`, without `unsafe`.** ADR-0008 says the `gc` crate's only `unsafe` is the root stack, "only if C1 needs it". Measuring a stack that is safe and has no borrow tracking answers whether it does.
2. **C1's `unsafe` is narrower than planned.** The plan described "an `UnsafeCell`-based stack". `RawStack` uses `Cell` slots in one raw allocation, and its `unsafe` only allocates, views and frees that allocation.
3. **The root stack is borrowed, not shared.** `HeapCore` and frames borrow it for `'r`, instead of S1's `Rc`, so frames cost no reference counting. A thread-per-agent design (EL15) owns the stack in the thread's function, so the borrow costs nothing in structure.
4. **Values carry no lifetime.** A value held across a collection is caught by debug epochs under stress tests, not by the compiler. The engine's value type should carry the scope lifetime, as handles do.
5. **QoS.** §10.4 planned user-interactive QoS. `taskpolicy` can only lower a process's QoS, and raising it needs a pthread call: new `unsafe` FFI in the benchmark. Runs use the default QoS of a command-line process instead, as S1's did. Evidence that they run on the performance cores: the in-process S1 probe reproduces S1's published 4.08 ns frame cost.
6. **A supplementary push measurement** (below).

**Preliminary observation** (development runs, not formal):
- Under S1's probe method, **a push costs about 2.0 ns on every stack, C1's included**. Frames (0.54 ns) and reads (0.51 ns) are under the target.
- The cause is the method:
  - It puts a compiler barrier (`black_box`) after every push.
  - The stack is reachable from escaped pointers, so each push must reload the length that the previous push stored. The disassembly of `RawStack`'s loop shows that store-to-load chain.
  - A calibration on this Mac: a bare counter that has escaped costs **2.18 ns per increment** under the same barrier, against 0.67 ns when it hasn't escaped.
  - **Corrected at CP16:** the method does force the reload, but the 2 ns is not inherent to it. `RawStack` reloads its length in one 16-byte paired load together with its capacity. A stack whose length is loaded on its own pays about half as much in the same loop (CP16, below).
- So `s3-probe roots` adds `push-run`: 64 back-to-back pushes per frame with one barrier, the shape of a builtin rooting its arguments. Preliminary: C1 0.56 ns, `RefCellStack` 0.72 ns, S1 0.83–0.91 ns.
- S3-Q1's criterion stays as written: "measured the way `s1-probe roots` measured S1's stack". CP14 measures both formally.

The four standard checks and the link check pass.

### CP14 — completed 2026-10-18 (S3b: measurements)

**Formal runs,** from the clean tree at `05185cb`, on AC power with Low Power Mode off, with no thermal warning before or after any run:

| Run | Results (`target/`, not committed) |
|---|---|
| `tools/s3-bench.sh`: every suite, 3 process runs with 60 s cool-downs; `s3-count`; 3 nightly runs adding `become` | `s3-results/bench-20261018-005914` |
| S1's own `dispatch` and `traverse` suites, for Pattern A in the same session | `s1-results/bench-20261018-010643`, `bench-20261018-010948` |
| `tools/s3-profile.sh`: 39 profiles, 5 s each, at `11be3a7` (which changes only `samply-summary.py`) | `s3-results/profiles-20261018-011414` |
| `tools/s3-ceremony.sh` | `s3-results/ceremony-20261018-002529` |

**Two incidents:**
- **A first formal run was discarded** (`bench-20261018-001555`). The Mac was on battery, where its settings turn on Low Power Mode (`pmset -g custom`). Every timing came out about 1.64 times slower, S1's own included (6.69 against 4.08 ns), and the shortest micro-benchmarks spread widely. The owner plugged in the charger, and the runs above repeated the full set.
- **`samply-summary.py --share` crashed** on inlined frames that `atos` reports without a source path. Fixed in `11be3a7`; the profiles then ran in full.

**Main results** (median of 3 run medians; spreads mostly ≤ 1%):
- **S3-Q1.**
  - C1's stack: frame 0.75 ns, **read 0.51 ns**, push by S1's method **2.03 ns**, push back to back 0.56 ns. S1's stack measured 4.08, 1.64, 2.05 and 0.90 ns in the same process.
  - The safe `CellStack`: 0.53, 0.50, 2.32 and 0.86 ns.
  - S1's dispatch case: 1,017 ns per dispatch on S1's Pattern B, 753 ns on the port with C1's stack, and 472 ns on Pattern A. C1 closes 47–48% of B's gap to A.
- **S3-Q2.**
  - Bytecode performs no root operations. W3's 650,146 are all in builtins, 650,124 of them reads.
  - **Rooting takes 0.9–1.1% of W3's time on C1's stack** (profiles), and 8.1–9.0% on `RefCellStack`. W1, W2 and W4 spend 0%.
- **S3-Q3.**
  - Stable Rust: `match` 2.64 ns per instruction on W1. `table` and `threaded` are 33–65% slower wherever bytecode dominates.
  - Nightly `become` is 5–22% faster than `match` built by the same compiler.
- **S3-Q4.** One-based struct-of-arrays link columns: 5.1–6.5 ns per node in cache-friendly orders and 14.3 ns after random moves. That is faster than S1's Pattern B (8.0–9.4 and 40.4 ns) and Pattern A (6.5–8.5 and 44.3 ns) in every case.
- **S3-Q5.**
  - A setter without a barrier costs what a plain write does (1.26 against 1.28 ns). Card marking adds 0.03–0.05 ns per write, and 2.6–3.1% on S1's mutation case.
  - Setters replace S1's 14 link writes one for one.

The analysis is CP15's, in `docs/research/s3-results.md`. The four standard checks and the link check pass.

### CP15 — completed 2026-10-18 (S3c: results and synthesis)

**Written:** [S3 results](s3-results.md). Its answers:
- **S3-Q1: reads yes, pushes no, as S1's method measures them.**
  - C1's stack reads in 0.51 ns, opens and closes a frame in 0.75 ns, and is Miri-clean.
  - By S1's method a push costs 2.03 ns, S1's own figure. The method forces a store-to-load round trip on every stack. Back to back, a push costs 0.56 ns. (**Corrected at CP16:** the cost of that round trip depends on the stack's field order, and C1's final stack halves it. See CP16.)
  - S1's dispatch case runs 26% faster, closing about half of Pattern B's gap to Pattern A.
  - As written, S3-Q1 does not pass.
- **S3-Q2 passes.** Bytecode performs no root operations, and with C1's stack rooting takes 0.9–1.1% of the callback-heavy workload, against the 10% threshold. ADR-0003's assumption holds.
- **S3-Q3:** on stable Rust, a `match` loop is fastest. Nightly tail calls are faster again, but rustc flags the feature as incomplete.
- **S3-Q4:** one-based struct-of-arrays link columns are the fastest layout in every traversal case, faster than both of S1's patterns.
- **S3-Q5:** setters cost nothing without a barrier; card marking costs about 3%.

**Updated:**
- [ADR-0008](../adr/0008-script-agents-and-heap.md): the S3 results in its decision history, analysis, decisions 5–7 and 10, assumptions, risks and consequences. Decision 6 now names struct-of-arrays columns. **It stays Proposed:** the pre-approved condition, that S3-Q1 and S3-Q2 pass, is not met as written.
- [Script and heap](../architecture/script-and-heap.md): SH7–SH9, SH13 and open questions 1–2.
- [Open questions](../open-questions.md): OQ-14 answered, resolving with ADR-0008; OQ-13's barrier cost measured.
- The [decision map](../decision-map.md) (F2, Step 4) and the indexes ([docs](../README.md), [research](README.md)).
- `RawStack`'s lazy commitment is verified: an 8 MiB stack holding 1,000 roots adds 32 KiB to the peak footprint. A code comment's unverified comparison with V8's handle blocks is removed.

The four standard checks and the link check pass.

**Gate C.** The owner decides on ADR-0008 and on merging `step-4` into `main`. The recommendations are in [status](../status.md), "Next steps".

### Gate C — decisions 2026-10-18

The owner's decisions, quoted in full in §2:
- **The S3 addendum (CP16):** "Authorized."
- **C1's implementation:** "Adopt `SliceStack` (Safe Contiguous Stack)." "The `gc` crate remains strictly `#![forbid(unsafe_code)]`." "Keep `RawStack` archived in spikes/s3/ as benchmark history."
- **ADR-0008 and the merge:** "We will formally accept ADR-0008 and merge `step-4` into `main` (with a subsequent push to origin) immediately upon completion of the formal addendum measurements."

**Context.** The owner had asked whether `RawStack`'s `unsafe` was worth its 94 ns on S1's dispatch case. A probe outside the repository showed two things. A safe stack can have `RawStack`'s layout, and it compiles to the same push. Putting its length first halved S1's push and sped up dispatch (informal: 1.02 against 2.00 ns, and 19%). CP16 measures it formally.

### CP16 — completed 2026-10-18 (the S3 addendum)

**Built** (`17775c4`):
- **`s3-gc`:**
  - `SliceStack`, C1's stack: one contiguous array of `Cell<u64>` slots, borrowed from a zeroed buffer (`slot_buffer`), with its length first (`repr(C)`, checked by a compile-time assertion).
  - `SliceStackLenLast`, the control: the same with the length last.
  - Neither has `unsafe`. `RawStack` stays in `src/raw/`, archived, and `DEFAULT_CAPACITY` moves to `roots.rs`.
- **Tests:** the heap and root-stack tests take the stack by reference and run on every stack, the root-stack tests on the control too; the oracle runs every technique and pacing on `SliceStack`. 197 tests pass (180 before), and the count-feature tests.
- **`s3-bench`:**
  - Both new stacks in the `roots` and `dispatch` suites.
  - `SliceStack` in the `vm` suite, the profiles and the counts. The `vm` suite now keeps one stack per workload across runs.
  - A new `s3-probe footprint` command.
- **Tools:** `s3-profile.sh` profiles the new stacks, and `unsafe-policy.txt` marks `RawStack` as archived.

**Miri** (nightly 2026-04-08, strict provenance, Stacked Borrows and Tree Borrows): `s3-gc`'s 61 tests, on every stack, find no undefined behaviour and no leak.

**Formal runs** at `17775c4`, on AC power with Low Power Mode off, with no thermal warning before or after:

| Run | Results (`target/`, not committed) |
|---|---|
| `tools/s3-bench.sh`: every suite, 3 process runs with 60 s cool-downs; `s3-count`; 3 nightly runs | `s3-results/bench-20261018-083106` |
| S1's own `dispatch` suite, for Pattern A in the same session | `s1-results/bench-20261018-083905` |
| `tools/s3-profile.sh`: 53 profiles, 5 s each | `s3-results/profiles-20261018-084214` |
| `/usr/bin/time -l target/release/s3-probe footprint …`, three runs per case | [S3 results](s3-results.md) §3.4 |

**Two incidents:**
- **The Mac was in use.** The load average was 3.1–5.4 (VS Code and Chrome were running). Absolute times came out higher and noisier than CP14's: dispatch spreads reached ±20–23%. Every case rotates its variants, and the ratios within each run reproduce CP14's (`RawStack` at 0.74 of Pattern B in both runs).
- **S1's suite and the profiles ran with uncommitted Markdown changes in the tree.** These were this checkpoint's documents, written during the runs; the code was `17775c4`.

**Main results** ([S3 results](s3-results.md) §3):
- **`SliceStack`:** frame 0.77 ns, read 0.53 ns, push by S1's method 2.12 ns, push back to back 0.58 ns. The control and `RawStack` measure the same pushes (2.10 ns).
- **The informal 1.02 ns push did not reproduce in `s3-probe`.** In both binaries `SliceStack`'s push loads the length on its own; it is the harness's code that differs. In `s3-probe`, the handle and the rooted slot pass to `black_box` through one stack slot. So S1's method measures the loads and stores around a push as much as the push itself. CP15's explanation, a round trip that every stack keeping its top in memory pays, was wrong.
- **Dispatch:** `SliceStack` is 13–17% faster than `RawStack`, and 14–20% faster than the control, in each run. As a ratio to Pattern B in the same process: 0.64, against 0.74 and 0.78. The informal probe had measured 19%.
- **S3-Q1 still fails as written** (2.12 ns per push), with its explanation corrected. **S3-Q2:** rooting takes 0.8–1.0% of W3 on `SliceStack`. The counts are identical to CP14's.
- **Footprint:** 1,000 roots add 16 KiB on `SliceStack`, and 100,000 add 784 KiB on every stack.

**Documents updated:**
- The [S3 results](s3-results.md): summary, §1–§4 and §8–§10.
- [ADR-0008](../adr/0008-script-agents-and-heap.md), accepted below; [ADR-0003](../adr/0003-gc-handle-and-rooting-pattern.md)'s decision history (C1 settled).
- [Script and heap](../architecture/script-and-heap.md): now binding; SH7, AC9 and open question 1. The [overview](../architecture/overview.md): `gc` has no `unsafe` (AC9).
- The [decision map](../decision-map.md) and the [open questions](../open-questions.md): OQ-5 and OQ-14 resolved, and OQ-13's design part. OQ-5's row, which broke the table, is one row again.
- This tracker's corrections to CP13 and CP15, the indexes, the spikes README, and [status](../status.md).

The four standard checks and the link check pass.

### Gate C — passed 2026-10-18

The owner's condition is met: CP16's formal measurements are complete. **ADR-0008 is Accepted**, and its decision history quotes the owner. [Script and heap](../architecture/script-and-heap.md) is binding with it. CP17 merges.

### CP17 — completed 2026-10-18

`step-4` merged into `main` with `git merge --no-ff step-4`, as the owner instructed at Gate C, and `main` pushed to `origin`.
