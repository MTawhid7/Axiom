# Rendering Pipeline: Data Model, Per-Frame Display Lists and Compositing

| | |
|---|---|
| **Status** | **Binding** since ADR-0007's acceptance (2026-10-17, Gate B). Written at Step 4, CP6. PL10 updated for the amendment of AC5 (OQ-19). **Amended by the architecture review's Round 1** (the owner's dispositions, 2026-10-21; [the review](../review/README.md) §5): PL7–PL9, PL15, PL16 and §5 for the GPU/compositor process and renderers without IOKit (RV1-02, RV1-09; CP2); PL9 against pixel stealing (RV1-19; CP5) |
| **Question** | What data structures make up the pipeline, from the DOM through style, the box tree, the fragment tree and the per-frame display list to the browser's compositor? Who owns each one, on which thread, and what crosses a process boundary? How are strings and atoms represented? |
| **Answers** | Step 4 questions Q2, Q8, Q9 and Q10 ([tracker](../research/step-4-architecture-proposal.md) §3): P2, and the representation part of P4 |
| **Feeds** | ADR-0007; CP7 (invalidation), CP8 (hit testing and accessibility), CP10 (the heap's DOM arenas); Step 5 (text, graphics, style, layout) |
| **Builds on** | [Process model](../security/process-model.md) §3 (the browser composites; the Rule of 2); [ADR-0003](../adr/0003-gc-handle-and-rooting-pattern.md); [event loop and threading](event-loop-and-threading.md); [IPC](ipc.md); [frame tree](frame-tree-and-navigation.md); [S4 results](../research/s4-results.md) (CoreText in a sandbox) |
| **Last updated** | 2026-10-21: Round 1's accepted findings |

Everything here is **planned**; nothing is implemented. Rules are numbered PL1–PL16. Precedent is sourced (§2); the design is inference unless marked otherwise. The algorithms behind each stage belong to Step 5.

## Summary

- **Five stages, five owned structures** (PL1):
  - the DOM, in the agent's GC heap;
  - computed styles, in a side table;
  - the box tree;
  - the immutable fragment tree;
  - a display list per frame.

  Everything after the DOM lives outside the GC heap.
- **The DOM's arenas are built to be lendable** (PL2). They hold no interior mutability, which the compiler checks, and style results go in side tables that can be written in disjoint slices. So parallel style and layout can be switched on later without redesign.
- **Style and layout run on the event loop's thread for now** (PL10). At Gate B the owner amended criterion AC5 and rule EL11 to allow read-only lending of the DOM arenas during a rendering stage that the borrow checker freezes (OQ-19). Parallel style and layout are built only when profiling justifies them.
- **Fragments are immutable** (PL5), each with the constraints that produced it, as in LayoutNG, which retired whole classes of invalidation bugs.
- **Each frame has its own display list** (PL6–PL9). Items carry only validated primitives: geometry, glyph images from an atlas the renderer rasterized, decoded image pixels, references to child frames. They point into property trees (transforms, scrolling, clips, effects) instead of nesting them, so the compositor can scroll and animate without the renderer. **The browser never decodes an image or parses a font** (AC4).
- **The compositor composites** (PL9): from M2 in the sandboxed GPU/compositor process, which sends the browser layer commits; at M1 as a browser thread (Round 1, RV1-02). It embeds each child frame's latest output where its parent's display list references it, as Chromium's surfaces do.
- **Strings** (PL11): one representation for JavaScript and the DOM, with 8-bit or 16-bit storage per string, as SpiderMonkey has done since 2014. Names are atoms that every thread can compare.

## 1. Binding inputs

| Input | Consequence here |
|---|---|
| **Rule of 2 and process model §3, note 1:** the browser composites renderer display lists of simple primitives, and never decodes images or parses fonts. **As amended in Round 1** (RV1-02): from M2 the GPU/compositor process composites them, and the browser receives layer commits | PL6–PL9, PL15, PL16 |
| **SI4, FT12:** each frame produces its own display list | PL6, PL9 |
| **ADR-0003:** one GC heap per agent for JavaScript and the DOM; Pattern B's typed arenas and `u32` handles; obligation C5, that glyph and pixel buffers need not live in the GC heap | PL2, PL7 |
| **AC5, EL10–EL11**, as amended at Gate B (OQ-19): nothing GC-managed crosses a process; a GC heap is never lent as a whole, but DOM arenas may be lent read-only for a rendering stage frozen by the borrow checker | PL2, PL10 |
| **S4-Q5:** CoreText renders correctly inside the renderer's sandbox | PL7 |
| **Decision map §2, item 6:** strings, text and accessibility are data-model properties | PL11, PL12 |

## 2. Precedent

From documentation read on 2026-10-17 (Sources).

| Topic | What engines do |
|---|---|
| **Layout output** | Chromium's LayoutNG produces "a completely new, immutable object called the fragment tree". Each fragment keeps the constraints that produced it as a "cache key", which brings incremental layout "back to O(n)". The legacy mutable tree suffered under-invalidation, over-invalidation and hysteresis: it was "incredibly difficult to ensure that layout was idempotent" [C2] |
| **Property trees** | Chromium describes "how visual and scrolling effects apply to DOM elements" with separate clip, transform, effect and scroll trees: "It's much easier and faster to send property trees around to different threads and processes than full DOM state" [C1] |
| **Display lists** | Chromium groups display items into paint chunks that share a property-tree state, and copies an unchanged object's items from the previous list [C1]. Firefox serializes WebRender's display list into "a completely self-contained binary blob that will survive Interprocess Communication", built on stacking contexts, clips and scroll frames [F1] |
| **Frames from other processes** | Chromium aggregates one compositor frame per local frame root. A parent embeds a child with a surface draw quad, which contains "surface IDs only, and not textures" [C1] |
| **Text** | Firefox's WebRender rasterizes glyphs into an atlas "via a thread pool from within Rust itself", in its GPU or parent process [F1]. So the compositor process handles font data |
| **Scrolling** | Firefox's APZ updates an offset and recomposites, routing UI events to it first [F1] |
| **Parallel style** | Stylo splits style computation "for the different DOM nodes across the different cores", with work stealing, relying on Rust to rule out data races. Its authors estimated close to 4× on 4 cores [M1] |
| **Strings** | SpiderMonkey stores strings that fit in Latin-1 at one byte per character, "purely an internal optimization". On Gmail, "the malloc-heap size was more than halved and the total string memory was reduced by about 4 MB (40%)" [M2] |

**Observation** (inference). Chromium's and Firefox's display lists both separate what to draw from where it sits, using property trees or spatial nodes. That separation is what lets a compositor scroll and animate on its own. Firefox's choice to rasterize glyphs in the compositor's process is the one Axiom can't copy into the browser (AC4).

## 3. The stages

| # | Stage | Structure | Lives in | Owner | Thread |
|---|---|---|---|---|---|
| 1 | Parse and script | DOM | The agent's GC heap, in typed arenas | The agent | Event loop |
| 2 | Style | Computed styles: immutable, shared, reference-counted values | A side table indexed by node | The document | Event loop (PL10) |
| 3 | Box construction | Box tree: block, inline, anonymous and replaced boxes, with a child frame as a replaced box | Per-document arenas, outside the heap | The document | Event loop |
| 4 | Layout | Fragment tree: immutable fragments, each with the constraints that produced it | Per-document arenas, outside the heap | The document | Event loop (PL10) |
| 5 | Paint | Display list for one frame, with its property trees | Built in the renderer, sent to the compositor through a ring (IP14) | The frame | Event loop builds it; a helper may encode it |
| 6 | Composite | One layer tree per window from the frames' display lists, its tiles rasterized into IOSurfaces | The compositor: the GPU/compositor process from M2, the browser at M1 (Round 1, RV1-02) | That process | Its raster thread; the browser's compositor thread applies the layer commits (EL6) |

## 4. Design

### 4.1 The DOM and style

| # | Rule |
|---|---|
| PL1 | **Each stage has one output structure and one owner** (§3). Everything after the DOM lives outside the GC heap and contains no GC handles except the DOM node handles it refers back to. Those are plain indices, never dereferenced outside the event loop's thread |
| PL2 | **The DOM's arenas are built to be lendable.** DOM node types hold no interior mutability: every mutation goes through the heap's mutable borrow (ADR-0003). A compile-time assertion keeps the DOM arenas `Sync`. Data that style or layout write, such as computed styles and dirty bits, goes in side tables indexed by node, which can be split into disjoint mutable slices. Node identity is the handle plus Pattern B's generation, stable for the node's life |
| PL3 | **Computed styles are immutable and shared.** Each is a reference-counted value, shared among elements with equal styles (Stylo's style sharing cache and rule tree are the precedent [M1]). The side table holds one reference per element. Step 5 designs the cascade and its caches |

### 4.2 Boxes and fragments

| # | Rule |
|---|---|
| PL4 | **The box tree** is built from the DOM and computed styles, in per-document arenas outside the heap. Each box records the DOM node it came from, for hit testing and accessibility (PL12, CP8). An `iframe` is a replaced box that carries its child frame's token (FT3) |
| PL5 | **Fragments are immutable.** Layout reads a box and the constraints offered by its parent, and produces a fragment that is never changed afterwards. The fragment keeps those constraints, so the next layout can reuse it when the inputs are unchanged (CP7). This is LayoutNG's contract, adopted because its absence caused whole classes of bugs [C2]. Step 5 chooses the layout unit (fixed-point or float) |

### 4.3 Display lists

| # | Rule |
|---|---|
| PL6 | **One display list per frame,** built by painting that frame's fragment tree (FT12). It has two parts. **Property trees:** spatial nodes (transforms, scroll frames, sticky and fixed positioning), clip nodes (rectangles and rounded rectangles; masks later), and effect nodes (opacity, blend modes, filters). **Items**, each referring to the property-tree state it draws in, as in Chromium's paint chunks [C1]. Items never nest effects themselves |
| PL7 | **Items carry validated primitives only:** |

- rectangles, rounded rectangles, borders, shadows and gradients;
- **text as glyph images** from an atlas that the renderer rasterized with CoreText inside its sandbox (S4-Q5). The item gives atlas entries and positions, never font data or glyph IDs to look up;
- **images as decoded pixels** in a resource that the renderer decoded and shares, a Mach memory entry, referenced by ID (**as amended in Round 1,** RV1-09: never an IOSurface);
- **child frames** by token (FT3), with a rectangle;
- **hit-test regions** (CP8);
- later, video frames from a media utility process and canvas contents, each as a resource.

| # | Rule |
|---|---|
| PL8 | **Resources belong to their renderer.** Images and glyph atlases are registered with the compositor by resource messages (IP14), reference-counted by the display lists that use them, and released when unused. They count against the renderer's memory budget, charged to it where macOS allows (macOS runtime note §6.1, a hypothesis). **The compositor never decodes or parses them** (AC4). **As amended in Round 1** (RV1-05, RV1-09): renderers write pixels only into Mach memory entries, and **the compositor samples them in place** where Metal allows (a buffer made over the page-aligned mapping without a copy, read as a linear texture), or else with one blit into a texture it owns; the renderer's copy stays the only long-lived one. Spike S10 measures both paths. If Step 5 moves glyph rasterization into a GPU process, glyph runs by font are an item only that process may receive, never the browser |
| PL9 | **The compositor validates, then composites** (**as amended in Round 1,** RV1-02: in the GPU/compositor process from M2, a browser thread at M1, with the frame tree's parent and child facts sent by the browser). It decodes each display list with generated decoders (IP8). Then it checks that every resource belongs to the sending renderer, every property-tree reference exists and the trees are acyclic, every count and size is within bounds, every number is finite, and every child-frame token names a child of that frame in the browser's frame tree (FT1), so no renderer can embed another tab's frame. A violation is `Terminate` (IP11). It then composes each window: the root frame's list, with each child frame's latest output embedded where its parent refers to it, under the parent's clip and transform at that point. Chromium's surface draw quads work the same way [C1]. Step 5 designs the rasterizer behind it (I2). From M2 the result goes to the browser as layer commits, which the browser validates again in bounded form (process model §3, note 1). **From M5** (Round 1, RV1-19): effects whose cost depends on pixel values (filters, `backdrop-filter`, blend modes) are not applied to a cross-origin child frame's output, or only in forms whose cost doesn't depend on its pixels, against pixel stealing (threat model §9); V4 tests it |

### 4.4 Threads

| # | Rule |
|---|---|
| PL10 | **For now, style, layout and paint run on the event loop's thread.** Image decoding and glyph rasterization run on the helper pool: their inputs (response bytes, fonts) and outputs (pixels, atlas entries) are not GC-managed (EL9, ADR-0003 C5). Encoding a finished display list may also use a helper, since it is outside the heap. **Parallel style and layout** over the DOM would lend the DOM arenas read-only to helper threads for one rendering stage. During that stage, the event loop's thread holds the heap's borrow, so the compiler forbids mutation and collection. That is safe Rust, and the owner amended AC5 and EL11 at Gate B (2026-10-17) to allow it (OQ-19). It is built only when profiling shows that style or layout limit frame production; PL2 keeps the option open at no cost |

### 4.5 Strings, atoms, accessibility and memory

| # | Rule |
|---|---|
| PL11 | **Strings: one representation for JavaScript and the DOM** (part of P4). ECMAScript strings are sequences of 16-bit code units, and DOM strings are the same (background). So the heap holds immutable strings stored as 8-bit Latin-1 or 16-bit UTF-16, chosen per string, with constant-time indexing by code unit. Lone surrogates stay representable. SpiderMonkey's measurements argue for the 8-bit form [M2]. The JavaScript engine's ropes and slices belong to Step 5. Text reaches Rust code that needs UTF-8 only at the boundaries that need it, and CoreText receives UTF-16, its native form (background). **Atoms:** element, attribute, event and CSS names are atoms. Static atoms are generated at build time, and dynamic atoms live in a process-wide, thread-safe table, so style can compare names by value from any thread |
| PL12 | **Accessibility from the same data.** A node's identity is its document ID plus its node handle and generation (PL2). Its geometry comes from fragments. The accessibility tree is built on the event loop's thread from the DOM and the box tree, and sent to the browser as incremental updates (CP8) |
| PL13 | **Memory has owners.** Box and fragment arenas belong to their document and are freed with it. Display lists are bounded per frame, and resources are budgeted per renderer (PL8). Step 6 measures them against B2 and B3 |

### 4.6 Transport and timing

| # | Rule |
|---|---|
| PL14 | **Display lists travel as updates.** A frame's display list goes to the compositor as changes to the previous one: items keyed by stable IDs, and property-tree nodes added, changed or removed. An unchanged object's items are reused, as Chromium copies them [C1]. Updates travel through a single-writer ring (IP14). At most one update is sent per offered frame (EL21, IP17) |
| PL15 | **Scroll offsets and compositor animations live in the property trees** that the compositor holds. So the compositor can scroll and animate between updates without the renderer (CP7). **As amended in Round 1** (RV1-02): the GPU process holds the full trees; each layer commit carries the browser the scroll and animation nodes its layers need, and **the browser scrolls and animates layers** between commits, without the renderer or the GPU process. It sends new offsets to the GPU process, which rasterizes the tiles they expose (IV8), and to the renderer, which runs `scroll` events |
| PL16 | **Hit-test data rides with the display list:** regions per frame, tagged with the frame, the scroll node, and whether listeners may block scrolling. The browser routes input from it (CP8). **As amended in Round 1** (RV1-02): from M2 the GPU process checks the regions and forwards them with the layer commit, and the browser checks that each frame's regions lie within that frame's bounds before routing by them |

## 5. Staging

| Level | What changes |
|---|---|
| L0 | Renderers are sandboxed processes from M1 (Round 1, RV1-07); the compositor runs as a browser thread behind its interface. Display lists cross from renderers through rings and memory entries, through the same encoder, decoder and validator |
| L1–L2 | From M2, display lists and resources go to the GPU/compositor process, and the browser receives layer commits (RV1-02). Nothing in the format changes |
| L3 | Child frames in other processes are embedded by token, as every child frame always was (FT12, PL9) |
| GPU process (M2) | The compositor's validation and rasterizing move to it; the browser keeps the layer tree, scrolling, animation and input. The display-list format and validation stay. Glyph runs by font may then be allowed to that process only (PL8) |

## 6. Alternatives considered

| | Alternative | Assessment |
|---|---|---|
| A | **The renderer rasterizes and sends tiles** | The browser would never see display lists, only pixels. That would be simpler for the browser, but costs memory for tiles of every frame on an 8 GB machine, and loses the compositor's ability to scroll and animate from property trees. Rejected |
| B | **The browser rasterizes glyphs, as WebRender does** | Putting font parsing in the browser breaks the Rule of 2 (AC4). Rejected |
| C | **A mirror of the DOM outside the heap**, for parallel style and layout within AC5 as first written | It allows parallel stages without lending, but duplicates DOM data and needs a synchronization path that could diverge. Not needed since the owner amended AC5 at Gate B to allow lending (OQ-19). Rejected |
| D | **A mutable layout tree** (pre-LayoutNG) | Chromium's experience is the counter-argument [C2]. Rejected |
| E | **UTF-8 strings for the DOM** | They would make JavaScript's code-unit indexing slow, or force conversions between JavaScript and the DOM in one heap. Rejected for PL11 |

## 7. Criteria check

| Criterion | How this design meets it |
|---|---|
| AC1 | Per-frame display lists and token-embedded child frames from L0 (§5) |
| AC4 | PL7–PL9: only validated primitives; no decoding or font parsing in the browser |
| AC5 | Met, as amended at Gate B: style and layout stay on the event loop's thread for now, and may later read DOM arenas lent for a frozen stage (PL10) |
| AC7 | PL13 |

## 8. Open questions

| # | Question | Where it is settled |
|---|---|---|
| 1 | Amend AC5 and EL11 to allow read-only lending of DOM arenas during a frozen rendering stage, enabling parallel style and layout? | **Resolved at Gate B** (2026-10-17): amended. Parallel stages are built only when profiling justifies them (OQ-19) |
| 2 | Where glyphs are rasterized long-term: the renderer, or a GPU process | Step 5 (I1, I2) |
| 3 | The rasterizer and the GPU backend behind the compositor | Step 5 (I2) |
| 4 | The layout unit, and the cascade's caches | Step 5 (layout, style) |

## Sources

All checked on 2026-10-17.

- [C1] Chrome for Developers, "Key data structures in RenderingNG" (last updated 2021-08-24) — https://developer.chrome.com/docs/chromium/renderingng-data-structures
- [C2] Chrome for Developers, "RenderingNG deep-dive: LayoutNG" (last updated 2021-10-08) — https://developer.chrome.com/docs/chromium/layoutng
- [F1] Firefox source docs, "Rendering Overview" — https://firefox-source-docs.mozilla.org/gfx/RenderingOverview.html
- [M1] L. Clark, "Inside a super fast CSS engine: Quantum CSS (aka Stylo)", Mozilla Hacks (2017-08-22) — https://hacks.mozilla.org/2017/08/inside-a-super-fast-css-engine-quantum-css-aka-stylo/
- [M2] J. de Mooij, "Slimmer and faster JavaScript strings in Firefox", Mozilla JavaScript blog (2014-07-21) — https://blog.mozilla.org/javascript/2014/07/21/slimmer-and-faster-javascript-strings-in-firefox/
- Background: ECMA-262 §6.1.4, "The String Type" — https://tc39.es/ecma262/#sec-ecmascript-language-types-string-type
