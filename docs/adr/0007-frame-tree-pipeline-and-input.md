# ADR-0007: Frame tree, rendering pipeline, invalidation and input across frames

| | |
|---|---|
| **Status** | **Accepted** (2026-10-17), by the owner at Step 4's Gate B. **IV1 and IV2 amended** at Step 5's Gate C (2026-10-20), for size container queries ([style](../architecture/style.md) ST25, ST31). **PL7–PL9, PL15, PL16 and IN1 amended by the architecture review's Round 1** (2026-10-21): the GPU/compositor process from M2, with scrolling, animation and input in the browser; pixels in memory entries (RV1-02, RV1-09) |
| **Date** | 2026-10-17 |
| **Deciders** | Project owner (only the owner can move an ADR to Accepted) |
| **Related** | [Frame tree and navigation](../architecture/frame-tree-and-navigation.md) (FT1–FT14); [rendering pipeline](../architecture/rendering-pipeline.md) (PL1–PL16); [invalidation](../architecture/invalidation.md) (IV1–IV14); [input and accessibility](../architecture/input-and-accessibility.md) (IN1–IN12); [process model](../security/process-model.md) §5 (SI1–SI7, SI4); [threat model](../security/threat-model.md) §8 (V9); [overview](../architecture/overview.md); decision-map items P2, P3 and part of P4 |

## Decision history

| Date | Event |
|---|---|
| 2026-10-17 | Proposed at Step 4, CP12. It decides SI4 as a whole: the browser-owned frame tree, per-frame display lists and browser-routed input that the owner required "from the first line of code" |
| 2026-10-17 | **Accepted** by the owner at Gate B: "ADR-0007 (Frame Tree, Pipeline, Input): Accepted." The open question in its consequences was decided at the same time (OQ-19): "Amend criterion AC5 and rule EL11 to allow read-only scoped lending of DOM arenas to helper threads during borrow-checker-frozen rendering stages. Keep the actual parallel style/layout implementation deferred until profiling benchmarks justify it." Decision 6 records it |
| 2026-10-19 | **Amendment proposed, for the owner at Step 5's Gate C** ([style](../architecture/style.md) ST25, ST31). (1) **IV1 gains nested lifecycles:** a size container's subtree is styled during layout, once the container's size is known, then gets its own box construction and layout, each phase with its own token, and runs again if a later layout pass resizes the container. Phase outputs still never change once produced; what is new is that style's output can have deferred subtrees. Reason: size container queries depend on the container's laid-out size, and Blink resolves them the same way. (2) **IV2's examples:** `getComputedStyle` reads that resolve to used values (CSSOM §9), and reads inside deferred subtrees, need layout; IV2's rule, to run only the phases a query needs, is unchanged. Until the owner decides, IV1 and IV2 stand as written |
| 2026-10-20 | **Amendment accepted** by the owner at Step 5's Gate C: "ADR-0007 Amendment (ST25, ST31): Accepted. Nested lifecycle phases for size container queries are adopted." IV1 and IV2 are amended in [invalidation](../architecture/invalidation.md), and decision 9 records it |
| 2026-10-21 | **Amendment accepted** in the owner's dispositions of the architecture review's Round 1 ([the review](../review/README.md) §5.1), RV1-02: "Formally design the sandboxed GPU/compositor process topology now (Renderer → GPU Process → Browser CALayer tree)." "Build the separate GPU/compositor process at Milestone 2; split Budget B1 accordingly." "Ensure Level 3 (L3) security certification at Milestone 6 is performed on the final multi-process topology." And RV1-09: "Renderers write pixels into shared memory entries; only the compositor owns IOSurfaces; renderer profiles explicitly deny iokit-open." PL7–PL9, PL15 and PL16 are amended in the [rendering pipeline](../architecture/rendering-pipeline.md), and IN1 in [input](../architecture/input-and-accessibility.md): the GPU/compositor process validates and rasterizes from M2, the browser scrolls and animates layers and routes input with no hop, and images arrive in memory entries sampled in place. Decisions 7 and 10 record it |

## Question

How are frames, documents and navigation represented and decided? What data structures make up the rendering pipeline, and how do they update incrementally? How does input reach frames that may live in different processes? All of it must let out-of-process iframes, mandatory before L3, arrive without an architectural rewrite.

## Requirements and constraints

- **SI4 (the owner, 2026-10-17):** "the Step 4 data model must design a browser-owned frame tree, frame-level display lists, and browser-routed input from the first line of code".
- **SI1–SI7, V9:** site locks; the browser alone decides process switches; out-of-process iframes are tested.
- **The Rule of 2 (PM2):** the browser never decodes images or parses fonts.
- **IR2, IR5:** decisions from the browser's records; the browser never blocks.
- **AC1:** no rewrite from L0 to L3.
- **The HTML standard:** navigables, browsing context groups, session history in the traversable, user activation and focus.

## Alternatives considered

- **Authority:** renderers hold the frame tree, or **the browser does**.
- **Replication:** full replication of frame state, or **a skeleton replica**.
- **Same-process cross-site frames:** direct calls, or **browser-routed operations**.
- **Rendering:** renderer rasterization with tiles, or **display lists of validated primitives**; a mutable layout tree, or **immutable fragments**.
- **Synchronous queries:** allowed across frames, or **never across agents**.
- **IME and accessibility:** synchronous queries to the renderer, or **browser-side mirrors and caches**.

## Historical context and modern developments

- Chromium, Firefox, WebKit and Servo all keep an authoritative tree in the privileged process, with little-state placeholders for remote frames.
- Chromium and WebKit report that the cost of out-of-process iframes lay in retrofitting code that assumed frames were local. WebKit was still at step 2 of 3 in January 2025.
- LayoutNG's immutable fragments and BlinkNG's enforced lifecycle retired classes of invalidation bugs.
- Firefox's APZ scrolls in the compositor.
- Firefox's `ContentCache` answers IME queries in the parent, and its 2023 accessibility cache in the parent was "up to 20 times faster" on its worst pages.

All sourced in the four documents above.

## Analysis

Each alternative's assessment is in the documents' "Alternatives considered" sections. The common thread: **every design choice that would make same-process frames behave differently from out-of-process frames is rejected**, because that difference is what made the retrofits expensive elsewhere.

## Evidence and sources

The Sources sections of the four architecture documents: the WHATWG HTML text of October 2026; Chromium's navigation, out-of-process-iframe, RenderingNG, BlinkNG, LayoutNG and style-invalidation documents; Firefox's BrowsingContext, process-model, APZ, rendering-overview and `ContentCache` sources and its accessibility post; WebKit's site-isolation documentation; Servo's constellation; CSS Containment Level 2.

## Decision

1. **The browser's core holds the only authoritative frame tree,** built from facts it derived itself, with tokens scoped per process (FT1–FT3).
2. **Renderers hold skeleton replicas and remote window proxies:** only what synchronous script needs (FT4–FT5).
3. **Every operation between agents goes through the browser,** even within one process (FT6).
4. **Navigation is browser-driven, through to an acknowledged commit.** Process and event loop are chosen at the response, and session history lives in the browser (FT7–FT10).
5. **Loader and storage endpoints are bound at commit;** bodies stream from the network process (FT11).
6. **The pipeline has five owned stages** (PL1–PL5): the DOM in the heap, then computed styles, the box tree and immutable fragments outside it. DOM arenas are built to be lendable. **As amended at Gate B** (AC5, EL11, OQ-19), they may be lent read-only to helper threads for a rendering stage frozen by the borrow checker. Parallel style and layout are built only when profiling justifies them (PL10).
7. **Each frame has a display list of validated primitives over property trees.** **As amended in Round 1** (RV1-02, RV1-09): the GPU/compositor process validates and composites them from M2, sending the browser layer commits, and images arrive in Mach memory entries. Glyphs are rasterized in the renderer. Child frames are embedded by token, and the browser validates every reference (PL6–PL9, PL14–PL16).
8. **Strings use one 8-bit or 16-bit representation** for JavaScript and the DOM, and names are atoms (PL11).
9. **An explicit lifecycle checked by the compiler,** with two entry points and no synchronous flush across agents. Reuse at every stage, and compositor-driven scrolling and animation (IV1–IV13). Every incremental update is checked against a rebuild in tests (IV14). **As amended at Step 5's Gate C** (ST25, ST31): a size container's subtree runs a nested lifecycle inside layout, each phase with its own token, and `getComputedStyle` reads that resolve to used values need layout (IV1, IV2).
10. **Input enters at the compositor and is routed by hit testing.** **As amended in Round 1** (RV1-02): the browser's compositor thread keeps the layer tree, hit testing, scrolling and animation from M2, so input never waits on the GPU process. Gestures latch, and focus, capture and activation are browser records. IME is served from a mirror, and accessibility from a browser-side cache (IN1–IN12).

## Rationale

- These choices make an out-of-process iframe a matter of process selection alone (FT10; [overview](../architecture/overview.md) §5).
- They keep the browser free of web-content parsing (PL7–PL9).
- They answer AppKit's synchronous questions without ever blocking on a renderer (IN7, IN9).

## Assumptions

- Skeleton replicas suffice for synchronous script semantics (to verify at the frames milestone).
- Validating display lists in the browser is cheap enough at display rate (to measure).

## Risks and unresolved issues

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Per-frame overhead when frames outnumber tabs (Chromium's lesson) | Medium | Medium | Small records and replicas; measured in Step 6 |
| Display-list validation costs at high frame rates | Medium | Medium | Incremental updates (PL14); bounded per frame (IP17) |
| Stale replicas cause visible races | Low | Low | Expected races are `Ignore`, not `Terminate` (IP11) |

## Conditions for revisiting

- A web-platform feature requires a synchronous dependency across agents that the specifications don't allow to be asynchronous.
- Measurements show compositor-side validation can't keep up with display rate on this Mac.

## Consequences

- The core, frame-tree, compositor, display-list, style, layout and paint crates follow these rules ([overview](../architecture/overview.md) §2).
- V9's tests are specified (FT14, IN12).
- Step 5 designs the algorithms behind the stages, and where glyphs are rasterized in the long run.
- **AC5 and EL11 were amended at Gate B** (OQ-19): read-only lending of DOM arenas during frozen rendering stages is allowed, so parallel style and layout need no further architectural decision. They are built only when profiling justifies them ([rendering pipeline](../architecture/rendering-pipeline.md) PL10).
- **IV1 and IV2 were amended at Step 5's Gate C** (2026-10-20): nested lifecycles for size-container subtrees, and the `getComputedStyle` reads that need layout ([style](../architecture/style.md) ST25, ST31; [ADR-0016](0016-style-layout-and-paint.md)).
