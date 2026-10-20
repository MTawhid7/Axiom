# Axiom — Design Documentation

Axiom is a from-first-principles browser engine for personal use on macOS. **The project is in Phase Zero: research, requirements and architecture.**
- **Authorized:** design documents, now Step 6's (2026-10-20), whose plan passed Gate A (2026-10-20), and Step 6's census spike, S8 (authorized at Gate A). Step 5 is done and merged (2026-10-20). The spikes of Steps 2b to 5 (S1–S7, all complete) remain in `spikes/` as throwaway code.
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
| [corpus.md](corpus.md) | The target-site corpus: the owner's 13 pages, their tiers, needs, logins and earliest security levels; what each site's terms allow automation to do; how each page is measured, logged out and signed in, and how repeatably | Draft (Step 6, CP2) |
| [decision-map.md](decision-map.md) | Decision layers, couplings, Phase Zero sequence | Living |
| [open-questions.md](open-questions.md) | Unresolved questions and the evidence each needs | Living |
| [status.md](status.md) | Current state and session handoffs | Living |
| [adr/](adr/) | Architecture Decision Records | See each ADR |
| ↳ [0000-template.md](adr/0000-template.md) | ADR template | — |
| ↳ [0001-scope-and-target.md](adr/0001-scope-and-target.md) | Target, JavaScript strategy, security posture, defaults | **Accepted** |
| ↳ [0002-implementation-language.md](adr/0002-implementation-language.md) | Implementation language (Rust) | **Accepted** (final, 2026-10-17) |
| ↳ [0003-gc-handle-and-rooting-pattern.md](adr/0003-gc-handle-and-rooting-pattern.md) | GC handle and rooting pattern (Pattern B, Nova style; obligations C1–C5) | **Accepted** (2026-10-17) |
| ↳ [0004-process-model-and-sandbox.md](adr/0004-process-model-and-sandbox.md) | Process model, sandbox and JIT policy | **Accepted** (2026-10-17), after spike S4 |
| ↳ [0005-ipc-transport-and-interfaces.md](adr/0005-ipc-transport-and-interfaces.md) | IPC: Mach transport, capabilities, IDL and generated decoders, `Untrusted<T>` | **Accepted** (2026-10-17, Gate B) |
| ↳ [0006-event-loop-and-threading.md](adr/0006-event-loop-and-threading.md) | Event loop and threading model | **Accepted** (2026-10-17, Gate B) |
| ↳ [0007-frame-tree-pipeline-and-input.md](adr/0007-frame-tree-pipeline-and-input.md) | Frame tree, rendering pipeline, invalidation and input across frames (SI4) | **Accepted** (2026-10-17, Gate B); IV1 and IV2 amended at Step 5's Gate C (2026-10-20) |
| ↳ [0008-script-agents-and-heap.md](adr/0008-script-agents-and-heap.md) | Script, agents and the heap; ADR-0003's C1–C5 | **Accepted** (2026-10-18, Gate C), with C1 met by the safe `SliceStack`; SH10 amended at Step 5's Gate C (2026-10-20) |
| ↳ [0009-embedder-api-and-shell.md](adr/0009-embedder-api-and-shell.md) | Engine/embedder API and the shell (OQ-11) | **Accepted** (2026-10-17, Gate B) |
| ↳ [0010-dependency-and-supply-chain-policy.md](adr/0010-dependency-and-supply-chain-policy.md) | Dependency and supply-chain policy (X4) | **Accepted** (2026-10-18, Step 5's Gate B) |
| ↳ [0011-text-fonts-and-unicode.md](adr/0011-text-fonts-and-unicode.md) | Text, fonts and Unicode (I1) | **Accepted** (2026-10-20, Step 5's Gate C), after the owner's visual check of S5's sheets; CoreText for system fonts (TX12) |
| ↳ [0012-graphics-images-and-media.md](adr/0012-graphics-images-and-media.md) | Graphics, images and media (I2, I4) | **Accepted** (2026-10-20, Step 5's Gate C), with AVIF in the media utility process (IM3) |
| ↳ [0013-networking-storage-and-workers.md](adr/0013-networking-storage-and-workers.md) | Networking, storage and workers (I3) | **Accepted** (2026-10-19), the owner's Gate B condition met by S7; NW6's verification in the worker pool confirmed at Gate C (2026-10-20) |
| ↳ [0014-accessibility.md](adr/0014-accessibility.md) | Accessibility, and the shell's settings and reserved keys (I5) | **Accepted** (2026-10-18, Step 5's Gate B) |
| ↳ [0015-html-parsing-and-the-dom.md](adr/0015-html-parsing-and-the-dom.md) | HTML parsing and the DOM (P5), with an amendment to ADR-0008 (HD11) | **Accepted** (2026-10-20, Step 5's Gate C), with the amendment to ADR-0008 |
| ↳ [0016-style-layout-and-paint.md](adr/0016-style-layout-and-paint.md) | Style, layout and paint (P6), with an amendment to ADR-0007 (ST25, ST31) | **Accepted** (2026-10-20, Step 5's Gate C), with the amendment to ADR-0007 and `:visited` by partitioned history |
| ↳ [0017-javascript-engine-and-bindings.md](adr/0017-javascript-engine-and-bindings.md) | The JavaScript engine and WebIDL bindings (P7) | **Accepted** (2026-10-20, Step 5's Gate C) |
| [security/threat-model.md](security/threat-model.md) | Assets, adversaries, attack surfaces, trust boundaries, security goals, security levels L0–L3 | **Accepted** (2026-10-17), amended: out-of-process iframes required for L3 |
| [security/process-model.md](security/process-model.md) | Process types and privileges, site isolation, IPC trust rules, memory budgets | Binding (ADR-0004 accepted, 2026-10-17); refined at Step 5's Gate C (2026-10-20), in its change record |
| [technology/language-evaluation.md](technology/language-evaluation.md) | Detailed language comparison and recommendation | Reviewed |
| [technology/dependency-policy.md](technology/dependency-policy.md) | Dependency and supply-chain policy (X4): what Axiom writes and may reuse; classes, soundness grades and groups for dependencies; audits, licenses, Apple's frameworks by process | Binding (ADR-0010 accepted, 2026-10-18) |
| [technology/dependency-register.md](technology/dependency-register.md) | Every dependency proposed for the engine, with the record the policy requires | Living |
| [research/step-2b-validation-spikes.md](research/step-2b-validation-spikes.md) | Step 2b plan and checkpoint tracker (spikes S1 and S2) | Complete (2026-10-17) |
| [research/s1-results.md](research/s1-results.md) | S1 results: `unsafe` containment, ceremony, runtime, memory and build cost of the two GC patterns; recommendation for ADR-0003 | Reviewed (2026-10-17) |
| [research/s2-results.md](research/s2-results.md) | S2 results: AppKit, Metal and CoreText through objc2 without Xcode; `unsafe` surface; build and run-time cost; proposed build thresholds for ADR-0002 | Reviewed (2026-10-17) |
| [research/step-3-security-and-process-model.md](research/step-3-security-and-process-model.md) | Step 3 plan and checkpoint tracker, including spike S4 | Complete (2026-10-17) |
| [research/macos-process-confinement.md](research/macos-process-confinement.md) | Process confinement on macOS: Seatbelt, App Sandbox, hardened runtime, JIT, launch and IPC options; browser precedent; spike S4 design | Reviewed (2026-10-17) |
| [research/s4-results.md](research/s4-results.md) | S4 results: custom Seatbelt profiles on macOS 27; probes; start-up, footprint and IPC cost; the JIT matrix; CoreText in a sandbox | Complete (2026-10-17) |
| [research/s3-results.md](research/s3-results.md) | S3 results: root-stack costs (C1), rooting traffic in an interpreter, dispatch techniques, node layouts (C2), setters and write barriers (C3); recommendations for ADR-0008 | Complete (2026-10-18), with the CP16 addendum (C1's `SliceStack`) |
| [research/macos-runtime-facilities.md](research/macos-runtime-facilities.md) | macOS facilities for the runtime: run loops, frame timing, QoS, surfaces, IME and accessibility; browser precedent | Complete (Step 4, CP2) |
| [research/step-4-architecture-proposal.md](research/step-4-architecture-proposal.md) | Step 4 plan and checkpoint tracker: the architecture proposal v0, including spike S3's design | Done (CP1–CP17; merged 2026-10-18) |
| [research/s5-results.md](research/s5-results.md) | S5 results: HarfRust and Rust rasterizers against CoreText; web fonts through a Rust-only path in the renderer's sandbox; footprint, speed and size; ImageIO in the sandbox against the safe image decoders | Complete (2026-10-18); the owner's visual checks of S5-Q1 and S5-Q2 done at Gate C (2026-10-20) |
| [research/s6-results.md](research/s6-results.md) | S6 results: compositing on Metal into tiled IOSurface layers; damage-limited redraws; scrolling and animation by layers; CPU against GPU path filling; the compositor's memory against B1, and Display P3 tags | Complete (2026-10-20), two formal runs: S6-Q1 and S6-Q3 pass at the median; S6-Q5's colour condition met, by the owner's screenshot, and its memory condition not met as measured (B1 revised); ADR-0012 accepted |
| [research/s7-results.md](research/s7-results.md) | S7 results: the network process's sandbox profile, and TLS verified by Security inside it; rustls against Network framework; DNS-SD from kqueue; one `kevent64` wait for every protocol | Complete (2026-10-19): S7-Q1, S7-Q3 and S7-Q4 pass; ADR-0013 accepted |
| [research/step-5-subsystems.md](research/step-5-subsystems.md) | Step 5 plan and checkpoint tracker: the subsystems behind Step 4's interfaces, the subsystem criteria SC1–SC10, and candidate spikes S5–S7 | Done (CP1–CP19; merged 2026-10-20) |
| [research/step-6-standards-budgets-roadmap.md](research/step-6-standards-budgets-roadmap.md) | Step 6 plan and checkpoint tracker: the owner's 13-site corpus, the standards baseline (X1), the testing strategy (X2), performance budgets (X3), the roadmap and Milestone 1, the readiness criteria RC1–RC10, and spike S8, the corpus census, with its logged-in mode | **In progress:** CP1–CP2 ✅; Gate A passed (2026-10-20) |
| [architecture/overview.md](architecture/overview.md) | **Architecture overview v1:** the system on one page, crates with their `unsafe` map and dependencies, the register summarized, seven end-to-end scenarios through the subsystems' rules, the checks of AC1–AC10 and SC1–SC10, staging, and the decisions for Step 5's Gate C | **Proposed** (v0 at Step 4, CP11; v1 at Step 5, CP18); Step 5's ADRs accepted at Gate C (2026-10-20) |
| [architecture/event-loop-and-threading.md](architecture/event-loop-and-threading.md) | Event loop and threading model: thread loops, the threads of each process, the HTML event loop, frame scheduling, agents and workers | Binding (ADR-0006 accepted, 2026-10-17) |
| [architecture/ipc.md](architecture/ipc.md) | IPC: Mach transport, channels and capabilities, the IDL and generated decoders, `Untrusted<T>` validation, synchronous requests, bulk data, backpressure | Binding (ADR-0005 accepted, 2026-10-17) |
| [architecture/frame-tree-and-navigation.md](architecture/frame-tree-and-navigation.md) | Frame tree, navigation and loading: the browser's records, skeleton replicas, remote window proxies, browser-routed operations, browser-driven navigation, bound loader and storage endpoints | Binding (ADR-0007 accepted, 2026-10-17) |
| [architecture/rendering-pipeline.md](architecture/rendering-pipeline.md) | Rendering pipeline: DOM, style, box tree, immutable fragments, per-frame display lists over property trees, validation and compositing, strings and atoms | Binding (ADR-0007 accepted, 2026-10-17) |
| [architecture/invalidation.md](architecture/invalidation.md) | Invalidation and incremental updates: the lifecycle, entry points, reuse at each stage, compositor-driven scrolling and animation, late and hidden content | Binding (ADR-0007 accepted, 2026-10-17); IV1 and IV2 amended at Step 5's Gate C (2026-10-20) |
| [architecture/input-and-accessibility.md](architecture/input-and-accessibility.md) | Input routing, gesture latching, scrolling, focus, IME mirror, user activation, accessibility cache, mediation | Binding (ADR-0007 accepted, 2026-10-17) |
| [architecture/embedder-api.md](architecture/embedder-api.md) | Engine/embedder API and the shell: the message boundary, commands, events, decisions, hosted objects, the headless embedder, the shell's language (OQ-11) | Binding (ADR-0009 accepted, 2026-10-17) |
| [architecture/script-and-heap.md](architecture/script-and-heap.md) | Script, agents and the heap: agents and realms, messages and transfers, `SharedArrayBuffer`, the root set, ADR-0003's obligations C1–C5, bindings, room for a JIT | Binding (ADR-0008 accepted, 2026-10-18); SH10 amended at Step 5's Gate C (2026-10-20) |
| [architecture/text-and-fonts.md](architecture/text-and-fonts.md) | Text, fonts and Unicode: ICU4X as the one Unicode source, our own encodings, CoreText's font database, HarfRust shaping, a Rust-only path for web fonts, glyph atlases in the renderer | Binding (ADR-0011 accepted, 2026-10-20); TX12: CoreText for system fonts |
| [architecture/graphics.md](architecture/graphics.md) | Graphics: Metal through `platform`, CoreAnimation layer trees with damage-limited tiles, our own pipelines, paths in the renderer (GR5's amendment deferred to the graphics milestone), Display P3, compositor animations, the GPU process later | Binding (ADR-0012 accepted, 2026-10-20); S6 reported |
| [architecture/images-and-media.md](architecture/images-and-media.md) | Images and media: safe Rust decoders in the renderer, AVIF through ImageIO in the media utility process, decoding at display size and discarding, color management, a sandboxed media process | Binding (ADR-0012 accepted, 2026-10-20); IM3 amended |
| [architecture/networking.md](architecture/networking.md) | Networking: state-machine protocols on one I/O thread, our own URL parser and HTTP framing, rustls with system certificate verification, DNS-SD, partitioned caches and cookies, Fetch's policy from bound endpoints | Binding (ADR-0013 accepted, 2026-10-19); NW6's verification in the worker pool confirmed at Gate C (2026-10-20) |
| [architecture/storage-and-workers.md](architecture/storage-and-workers.md) | Storage and workers: web storage in the network process by storage key, one engine (`redb` or SQLite), a renderer cache for `localStorage`, serialized IndexedDB values, quotas, service-worker interception, worker placement | Binding (ADR-0013 accepted, 2026-10-19) |
| [architecture/accessibility.md](architecture/accessibility.md) | Accessibility: trees built in renderers under HTML-AAM and Core-AAM, the browser's cache and our own `NSAccessibility` bridge, text markers, system settings as media features; the settings and reserved keys, for the owner | Binding (ADR-0014 accepted, 2026-10-18) |
| [architecture/html-and-dom.md](architecture/html-and-dom.md) | HTML and the DOM: our own parser on the event loop thread in budgeted tasks, with a preload scanner on the helper pool; sniffing and decoding straight into the DOM's 8- and 16-bit strings; one node arena of columns whose slots never move; one mutation path with fixed hooks; events, observers, ranges, Shadow DOM, custom elements, forms and editing | Binding (ADR-0015 accepted, 2026-10-20) |
| [architecture/style.md](architecture/style.md) | Style: our own CSS parser over the DOM's 8- and 16-bit strings; one property table that generates parsers, storage and accessors; right-to-left matching with rule buckets and an ancestor Bloom filter, and no selector JIT; the cascade of Cascade 5 and 6; immutable, shared computed styles with a rule tree, a sharing cache and an evictable cascade cache; invalidation sets; container queries during layout; animations on Web Animations; partitioned `:visited` | Binding (ADR-0016 accepted, 2026-10-20) |
| [architecture/layout-and-paint.md](architecture/layout-and-paint.md) | Layout and paint: a fixed-point layout unit (1/64 px); boxes as CSS Display generates them; LayoutNG's contract of immutable fragments cached by constraint space; block, inline, flex, grid, positioning and fragmentation of our own, with tables marked approximate; scroll nodes with snapping, sticky constraints and compositor-moved scrollbar thumbs; CSS 2.1's painting order, property trees and display items reused by identity; hit testing in the renderer | Binding (ADR-0016 accepted, 2026-10-20) |
| [architecture/javascript-engine.md](architecture/javascript-engine.md) | The JavaScript engine, all our own: a parser producing heap-free compiled scripts for off-thread compilation; a register machine with an accumulator and a `match` loop; a NaN-boxed value type that carries the scope lifetime; shapes and inline caches as data; ropes and slices; `RegExp` interpreted with a linear-time fallback; `Intl` on ICU4X; Temporal over ICU4X's calendars; test262 | Binding (ADR-0017 accepted, 2026-10-20) |
| [architecture/bindings.md](architecture/bindings.md) | WebIDL bindings: our own generator over webref's curated IDL, emitting safe Rust; platform objects as heap entries with no wrappers; brand checks as range checks; lazy per-realm interfaces; Web IDL's conversions, with rooting only where a later conversion may collect; `[CEReactions]`, callbacks and the dispatch hot path; `WindowProxy` and remote window proxies | Binding (ADR-0017 accepted, 2026-10-20) |
| [research/](research/README.md) | Research conventions and notes | Living |

**Planned documents**, each created when its Phase Zero step begins:

| Document | Step |
|---|---|
| `standards.md` | Step 6 (CP4) |
| `testing.md` | Step 6 (CP5) |
| `performance.md` | Step 6 (CP6) |
| `roadmap.md` | Step 6 (CP8) |
| `milestone-1.md` | Step 6 (CP9) |
| `requirements.md` | Steps 1→6 (CP10) |

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
