# Axiom — Design Documentation

Axiom is a from-first-principles browser engine for personal use on macOS. **The project is in Phase Zero: research, requirements and architecture.**
- **Authorized:** design documents, now Step 4's (2026-10-17). The spikes of Steps 2b and 3 (S1, S2, S4; all complete) remain in `spikes/` as throwaway code.
- **Not yet authorized:** production engine code.

These documents are the project's source of truth. Conversation history is not.

## Start here

1. [status.md](status.md): current state, next steps, and the session handoff log
2. [charter.md](charter.md): vision, goals, environment, non-goals, constraints
3. [decision-map.md](decision-map.md): every major decision, its status, and how the decisions constrain each other

## Index

| Document | Purpose | Status |
|---|---|---|
| [charter.md](charter.md) | Goals, users, environment, scope, non-goals, success criteria | Draft (scope sections Accepted) |
| [decision-map.md](decision-map.md) | Decision layers, couplings, Phase Zero sequence | Living |
| [open-questions.md](open-questions.md) | Unresolved questions and the evidence each needs | Living |
| [status.md](status.md) | Current state and session handoffs | Living |
| [adr/](adr/) | Architecture Decision Records | See each ADR |
| ↳ [0000-template.md](adr/0000-template.md) | ADR template | — |
| ↳ [0001-scope-and-target.md](adr/0001-scope-and-target.md) | Target, JavaScript strategy, security posture, defaults | **Accepted** |
| ↳ [0002-implementation-language.md](adr/0002-implementation-language.md) | Implementation language (Rust) | **Accepted** (final, 2026-10-17) |
| ↳ [0003-gc-handle-and-rooting-pattern.md](adr/0003-gc-handle-and-rooting-pattern.md) | GC handle and rooting pattern (Pattern B, Nova style; obligations C1–C5) | **Accepted** (2026-10-17) |
| ↳ [0004-process-model-and-sandbox.md](adr/0004-process-model-and-sandbox.md) | Process model, sandbox and JIT policy | **Accepted** (2026-10-17), after spike S4 |
| [security/threat-model.md](security/threat-model.md) | Assets, adversaries, attack surfaces, trust boundaries, security goals, security levels L0–L3 | **Accepted** (2026-10-17), amended: out-of-process iframes required for L3 |
| [security/process-model.md](security/process-model.md) | Process types and privileges, site isolation, IPC trust rules, memory budgets | Binding (ADR-0004 accepted, 2026-10-17) |
| [technology/language-evaluation.md](technology/language-evaluation.md) | Detailed language comparison and recommendation | Reviewed |
| [research/step-2b-validation-spikes.md](research/step-2b-validation-spikes.md) | Step 2b plan and checkpoint tracker (spikes S1 and S2) | Complete (2026-10-17) |
| [research/s1-results.md](research/s1-results.md) | S1 results: `unsafe` containment, ceremony, runtime, memory and build cost of the two GC patterns; recommendation for ADR-0003 | Reviewed (2026-10-17) |
| [research/s2-results.md](research/s2-results.md) | S2 results: AppKit, Metal and CoreText through objc2 without Xcode; `unsafe` surface; build and run-time cost; proposed build thresholds for ADR-0002 | Reviewed (2026-10-17) |
| [research/step-3-security-and-process-model.md](research/step-3-security-and-process-model.md) | Step 3 plan and checkpoint tracker, including spike S4 | Complete (2026-10-17) |
| [research/macos-process-confinement.md](research/macos-process-confinement.md) | Process confinement on macOS: Seatbelt, App Sandbox, hardened runtime, JIT, launch and IPC options; browser precedent; spike S4 design | Reviewed (2026-10-17) |
| [research/s4-results.md](research/s4-results.md) | S4 results: custom Seatbelt profiles on macOS 27; probes; start-up, footprint and IPC cost; the JIT matrix; CoreText in a sandbox | Complete (2026-10-17) |
| [research/macos-runtime-facilities.md](research/macos-runtime-facilities.md) | macOS facilities for the runtime: run loops, frame timing, QoS, surfaces, IME and accessibility; browser precedent | Complete (Step 4, CP2) |
| [research/step-4-architecture-proposal.md](research/step-4-architecture-proposal.md) | Step 4 plan and checkpoint tracker: the architecture proposal v0, including spike S3 | In progress (plan approved at Gate A) |
| [architecture/event-loop-and-threading.md](architecture/event-loop-and-threading.md) | Event loop and threading model: thread loops, the threads of each process, the HTML event loop, frame scheduling, agents and workers | **Proposed** (Step 4, CP3) |
| [architecture/ipc.md](architecture/ipc.md) | IPC: Mach transport, channels and capabilities, the IDL and generated decoders, `Untrusted<T>` validation, synchronous requests, bulk data, backpressure | **Proposed** (Step 4, CP4) |
| [architecture/frame-tree-and-navigation.md](architecture/frame-tree-and-navigation.md) | Frame tree, navigation and loading: the browser's records, skeleton replicas, remote window proxies, browser-routed operations, browser-driven navigation, bound loader and storage endpoints | **Proposed** (Step 4, CP5) |
| [architecture/rendering-pipeline.md](architecture/rendering-pipeline.md) | Rendering pipeline: DOM, style, box tree, immutable fragments, per-frame display lists over property trees, validation and compositing, strings and atoms | **Proposed** (Step 4, CP6) |
| [architecture/invalidation.md](architecture/invalidation.md) | Invalidation and incremental updates: the lifecycle, entry points, reuse at each stage, compositor-driven scrolling and animation, late and hidden content | **Proposed** (Step 4, CP7) |
| [architecture/input-and-accessibility.md](architecture/input-and-accessibility.md) | Input routing, gesture latching, scrolling, focus, IME mirror, user activation, accessibility cache, mediation | **Proposed** (Step 4, CP8) |
| [architecture/embedder-api.md](architecture/embedder-api.md) | Engine/embedder API and the shell: the message boundary, commands, events, decisions, hosted objects, the headless embedder, the shell's language (OQ-11) | **Proposed** (Step 4, CP9) |
| [architecture/script-and-heap.md](architecture/script-and-heap.md) | Script, agents and the heap: agents and realms, messages and transfers, `SharedArrayBuffer`, the root set, ADR-0003's obligations C1–C5, bindings, room for a JIT | **Proposed** (Step 4, CP10) |
| [research/](research/README.md) | Research conventions and notes | Living |

**Planned documents**, each created when its Phase Zero step begins:

| Document | Step |
|---|---|
| `requirements.md` | Steps 1→6 |
| `architecture/overview.md`, plus one file per subsystem | Steps 4–5 |
| `standards.md` | Step 6 |
| `performance.md` | Step 6 |
| `testing.md` | Step 6 |
| `roadmap.md` | Step 6 |

## Document status legend

| Status | Meaning |
|---|---|
| **Draft** | Work in progress; may change freely |
| **Proposed** | Complete recommendation awaiting the owner's decision |
| **Accepted** | Confirmed explicitly by the owner; change it only through a new ADR |
| **Superseded** | Replaced; links to its successor |
| **Living** | Continuously updated tracking document |

## Conventions

- Dates are absolute (YYYY-MM-DD).
- Claims are labelled as sourced fact, inference, hypothesis or background ([research conventions](research/README.md)).
- Implemented behaviour is kept distinct from planned behaviour, and standards compliance from approximation.
- Every dependency carries a justification and is classed as one of:
  - a required platform facility;
  - a replaceable implementation detail;
  - a foundational dependency.
