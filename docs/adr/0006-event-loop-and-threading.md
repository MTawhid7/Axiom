# ADR-0006: Event loop and threading model

| | |
|---|---|
| **Status** | **Accepted** (2026-10-17), by the owner at Step 4's Gate B. **§4.2's threads amended by the architecture review's Round 1** (2026-10-21): the browser's compositor thread and the GPU/compositor process (RV1-02) |
| **Date** | 2026-10-17 |
| **Deciders** | Project owner (only the owner can move an ADR to Accepted) |
| **Related** | [Event loop and threading](../architecture/event-loop-and-threading.md) (the full design, EL1–EL24); [macOS runtime note](../research/macos-runtime-facilities.md); [process model](../security/process-model.md) (PM6, PM8, SI7, IR5, IR6); [ADR-0003](0003-gc-handle-and-rooting-pattern.md); [Step 4 tracker](../research/step-4-architecture-proposal.md); decision-map item P1 |

## Decision history

| Date | Event |
|---|---|
| 2026-10-17 | Proposed at Step 4, CP12 |
| 2026-10-17 | **Accepted** by the owner at Gate B: "ADR-0006 (Event Loop & Threading): Accepted." The same decisions amend rule EL11 (OQ-19): "Amend criterion AC5 and rule EL11 to allow read-only scoped lending of DOM arenas to helper threads during borrow-checker-frozen rendering stages. Keep the actual parallel style/layout implementation deferred until profiling benchmarks justify it." Decision 4 records the amendment |
| 2026-10-21 | **Amendment accepted** in the owner's dispositions of the architecture review's Round 1 ([the review](../review/README.md) §5.1), RV1-02: "Formally design the sandboxed GPU/compositor process topology now (Renderer → GPU Process → Browser CALayer tree)." "Build the separate GPU/compositor process at Milestone 2; split Budget B1 accordingly." "Ensure Level 3 (L3) security certification at Milestone 6 is performed on the final multi-process topology." The [event loop](../architecture/event-loop-and-threading.md)'s §4.2 is amended: from M2 the GPU/compositor process receives, validates and rasterizes display lists, and the browser's compositor thread keeps frame scheduling, layer commits, scrolling, animation and input. EL6 and the thread roles' interfaces are unchanged |

## Question

How does each Axiom process run its work: which threads exist, how each waits, how the HTML event loop is implemented and scheduled, what drives frame production, and how agents and workers map onto threads and heaps?

## Requirements and constraints

- **PM8 and AC3:** idle costs nothing. S2 measured an idle AppKit window at 0.0% CPU with no wakeups.
- **ADR-0003:** one single-threaded heap per agent.
- **SI7:** an agent cluster never spans processes.
- **IR5:** the browser never blocks on a child.
- **The specifications:** HTML ties one event loop to each agent but leaves threads, task selection and rendering opportunities to the implementation. ECMAScript lets agents share a thread only if none can block.
- **macOS:** `CADisplayLink` is vended only by AppKit objects; QoS classes set CPU, I/O and timer priority together.

## Alternatives considered

- **Renderer threads:** one main thread for all window agents (Chromium), or **one thread per event loop** (Servo's script threads).
- **Receiving IPC:** a central I/O thread per process, or **endpoints bound to their handling threads**.
- **Browser logic:** on AppKit's main thread (Chromium, Firefox, WebKit), or on **a separate core thread**.
- **Frame clock:** a renderer-side timer, or **the browser's display link with subscriptions**.

## Historical context and modern developments

- Chromium runs one main thread per renderer plus a compositor thread, and drives frames from Viz.
- WebKit's UI process owns the display link and sends `DisplayDidRefresh`.
- Servo keys script threads by browsing context group and registered domain.
- Ladybird's rendering stages run on threads that "never touch the heap".
- Chrome's tested timer throttling: once a second when hidden; once a minute after five minutes for chained timers.

All sourced in [event loop and threading](../architecture/event-loop-and-threading.md) §2–§3.

## Analysis

See [event loop and threading](../architecture/event-loop-and-threading.md) §4 and §5. In short:
- **One thread per event loop** isolates same-site tabs that share a process, lets QoS follow each agent's visibility, keeps each heap on one thread, and matches ECMAScript's rule. Its cost is a few threads, whose stacks are virtual.
- **A core thread** frees the engine from AppKit's nested run loops and lets it run headless. It costs one more hop of a few microseconds.

## Evidence and sources

[Event loop and threading](../architecture/event-loop-and-threading.md) Sources: the WHATWG HTML and ECMA-262 texts of October 2026; Apple's run-loop and QoS documentation; XNU's scheduler notes; Chromium, Firefox, WebKit, Ladybird and Servo.

## Decision

1. **One wait per thread** (EL1–EL3): kqueue for helper threads; run loops for AppKit's main thread and the compositor, with engine sources in the common modes on the main thread.
2. **Endpoints are bound to the thread that handles them,** with no intermediate I/O thread (EL4).
3. **Browser threads:** main (AppKit only), **core** (the engine's authority), compositor, and a worker pool. **Renderer threads:** one per event loop, a control thread, and a helper pool that never touches a heap. **Network threads:** I/O, storage, workers (EL5–EL9).
4. **GC heaps never cross threads.** Other threads see owned messages, immutable snapshots, or data lent for a bounded scope (EL10–EL13). **As amended at Gate B** (OQ-19), the only GC-managed data that may be lent is a DOM arena, read-only, for a rendering stage during which the borrow checker freezes the heap. Parallel style and layout are built only when profiling justifies them.
5. **One event loop per agent, and one thread per event loop** (EL14–EL15). Tasks are chosen in this order: discrete input, a due rendering update, other tasks oldest first, then idle work. Continuous input is coalesced per frame. There are no nested event loops (EL16–EL17).
6. **Timers are aligned and throttled** from Chrome's tested levels, per agent cluster (EL18, §4.8).
7. **The browser's compositor owns a display link per window,** paused when unneeded. It offers frames only to subscribed event loops, one outstanding each, and never waits for a renderer (EL19–EL22).
8. **QoS follows role and visibility,** through public interfaces only (EL23).
9. **The same loops run under a deterministic scheduler with a virtual clock in tests** (EL24).
10. **Clarification of SI7.** ADR-0004's consequence "one agent cluster per process" means that **an agent cluster never spans processes**. A renderer may host several agent clusters of its own site (SI6).

## Rationale

- Idle cost, responsiveness and memory (charter G4) all follow from threads that block in one place and wake only for work.
- One thread per agent is the simplest model that the specifications allow and that keeps AC5.
- A browser-owned frame clock is forced by macOS (`CADisplayLink` needs AppKit), and is what WebKit does.

## Assumptions

- A thread per agent stays cheap at the process counts B5 allows (to measure).
- Chrome's throttling levels transfer to Axiom's single-user workload.

## Risks and unresolved issues

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Many dedicated workers create many threads | Medium | Low | A per-process thread cap (Step 6) |
| Starvation between task classes | Medium | Medium | Starvation bounds per class (Step 6) |
| Display-link behaviour across display reconfiguration | Low | Medium | The browser has a full AppKit loop; tested at implementation (macOS runtime note §12) |

## Conditions for revisiting

- Measurements show per-agent threads costing memory beyond B2 or B3.
- macOS offers a public display link for processes without AppKit, which would allow other designs, though it wouldn't force them.

## Consequences

- The `event-loop` crate provides thread loops, the HTML scheduler and the test scheduler.
- Every IDL interface names its owning thread.
- The decision map's §4 wording on agent clusters is corrected (CP12).
