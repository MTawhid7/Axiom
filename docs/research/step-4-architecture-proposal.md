# Step 4 — Architecture Proposal v0 (Event Loop, Threading, Pipeline Data Model, Invalidation, Embedder API): Plan and Checkpoint Tracker

| | |
|---|---|
| **Status** | **CP1 ✅ (2026-10-17). The plan is submitted for the owner's review** (Gate A), as the owner asked. CP2 starts when the owner approves it |
| **Authorized** | 2026-10-17, by the owner: "Proceed with Step 4 design work on a new branch `step-4`." Design documents only. Spike S3's code needs a separate go-ahead once its design is ready (§2) |
| **Branch** | `step-4`, from `main` at `da79e69` |
| **Feeds** | Decision-map items F4, P1, P2, P3 and part of P4; OQ-5, OQ-11, OQ-13, OQ-14, OQ-17; ADR-0003 obligations C1–C5; ADRs 0005–0009 (provisional, §4); Steps 5 and 6 |
| **Last updated** | 2026-10-17 |

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
  - **Proposal:** add "HTML parsing and DOM" and "JavaScript engine and bindings" to Step 5's list of subsystems, which doesn't name them today (decision map §5).
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
| 2 | `docs/research/macos-runtime-facilities.md`, a research note on the macOS facilities the runtime needs, with observations of this Mac | CP2 |
| 3 | `docs/architecture/event-loop-and-threading.md` (P1) | CP3 |
| 4 | `docs/architecture/ipc.md` (OQ-17) | CP4 |
| 5 | `docs/architecture/frame-tree-and-navigation.md` (SI4) | CP5 |
| 6 | `docs/architecture/rendering-pipeline.md` (P2, part of P4) | CP6 |
| 7 | `docs/architecture/invalidation.md` (P3) | CP7 |
| 8 | `docs/architecture/input-and-accessibility.md` (SI4) | CP8 |
| 9 | `docs/architecture/embedder-api.md` (F4, OQ-11) | CP9 |
| 10 | `docs/architecture/script-and-heap.md` (OQ-5, OQ-13, OQ-14; ADR-0003 C1–C5), and spike S3's design in this tracker (§10) | CP10 |
| 11 | `docs/architecture/overview.md`: the architecture proposal v0 | CP11 |
| 12 | ADRs, Proposed. Provisional set: 0005, IPC transport and interfaces; 0006, event loop and threading; 0007, frame tree, pipeline and invalidation; 0008, script, agents and heap (accepted after S3); 0009, engine/embedder API and shell. CP12 may regroup them, and Gate B reports any change | CP12 |
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
| AC5 | **One single-threaded heap per agent.** Nothing GC-managed crosses a thread or a process. Data moves between agents by structured clone or transfer | ADR-0003, SI7 |
| AC6 | **A deterministic, testable core.** Every stage can run headless and in one process, with virtual time and deterministic scheduling for tests | R6; decision map §2, item 8 |
| AC7 | **Memory is designed.** Each long-lived structure has an owner and a size estimate, consistent with B1–B7 | PM7 |
| AC8 | **The complete model fits.** For each app-web capability, the proposal names where it lives and shows that it needs no architectural change: dedicated, shared and service workers; `SharedArrayBuffer` and Wasm threads; WebGL and WebGPU; media; accessibility; IME | ADR-0001 R1, charter principle 7 |
| AC9 | **`unsafe` stays where ADR-0002 allows it.** Any new allowlisted module is named and justified | ADR-0002, evaluation §15.3 |
| AC10 | **Every decision traces to evidence:** a source, a measurement, or a labelled inference or hypothesis | Research conventions |

## 6. Checkpoints

**Every checkpoint ends with:**
- every relative link in `docs/` resolving. Planned documents are named in code font, not linked, until they exist;
- the four standard checks green: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `tools/check-unsafe-policy.sh`. CP1–CP12 change no code;
- a commit on `step-4`, and an update to this tracker.

| CP | Scope | Status |
|---|---|---|
| **CP1** | This plan; branch `step-4`; the owner's decisions recorded | ✅ 2026-10-17 (§11). **Gate A:** the owner reviews the plan |
| CP2 | Research note: macOS facilities for the runtime | |
| CP3 | Event loop and threading model (P1) | |
| CP4 | IPC: transport and interface definitions (OQ-17) | |
| CP5 | Frame tree, navigation and loading (SI4) | |
| CP6 | Rendering pipeline: data model, per-frame display lists, compositing, strings (P2, part of P4) | |
| CP7 | Invalidation and incremental updates (P3) | |
| CP8 | Input, focus, hit testing, scrolling and accessibility across frames (SI4) | |
| CP9 | Engine/embedder API and shell (F4, OQ-11) | |
| CP10 | Script, agents and heap (OQ-5, OQ-13, OQ-14; ADR-0003 C1–C5); spike S3's design | |
| CP11 | Architecture overview v0, and the criteria check (§5) | |
| CP12 | Synthesis: ADRs (Proposed); decision map, open questions, indexes, status. **Gate B** | |
| CP13 | **S3a**, if authorized: implementation and correctness | |
| CP14 | **S3b:** measurements | |
| CP15 | **S3c:** `docs/research/s3-results.md`; ADR-0008 updated; docs synthesis. **Gate C** | |
| CP16 | Merge `step-4` into `main` (`--no-ff`), on the owner's instruction | |

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

**CP13–CP15. Spike S3**, if authorized:
- It follows S1's methods: `s1-probe roots`, the dispatch suite, three process runs, and a clean tree for formal runs.
- CP15 writes the results and updates ADR-0008.
- **Gate C** (§7).

**CP16. Merge** `step-4` into `main` with `--no-ff`, on the owner's instruction.

## 7. Review gates

Between gates, checkpoints run without pausing, as in Steps 2b and 3.

| Gate | After | The owner decides |
|---|---|---|
| A | CP1 | This plan, including its scope (§2) |
| B | CP12 | ADRs 0005–0009 (Proposed). ADR-0008's acceptance may wait for S3, as ADR-0004's waited for S4. Also: authorizing spike S3's code |
| C | CP15 | ADR-0008, in the light of S3; merging `step-4` into `main` |

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

Written at CP10 (§6.1).

## 11. Results log

### CP1 — completed 2026-10-17

- `main` pushed to `origin` at `da79e69`, as the owner authorized.
- Branch `step-4` created from `main` at `da79e69`.
- The owner's confirmation of S4's three refinements recorded in [ADR-0004](../adr/0004-process-model-and-sandbox.md)'s decision history.
- This plan written. [Status](../status.md), the [decision map](../decision-map.md) and the indexes ([docs](../README.md), [research](README.md)) updated.
- Every relative link in `docs/` resolves, and the four standard checks pass, with the code unchanged.

**Gate A.** The owner reviews this plan.
