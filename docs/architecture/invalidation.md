# Invalidation and Incremental Updates

| | |
|---|---|
| **Status** | **Proposed** (Step 4, CP7). Decided in ADR-0007 at Gate B |
| **Question** | What is the rendering lifecycle? How does a change propagate from a DOM mutation through style, layout, paint and compositing, reusing work at each stage? How are synchronous queries such as `offsetWidth` bounded? Which updates bypass the renderer's main thread, and how does that work across processes? |
| **Answers** | Step 4 questions Q3, Q11 and Q12 ([tracker](../research/step-4-architecture-proposal.md) §3): P3 |
| **Feeds** | ADR-0007; CP8 (input routing for scrolling); Step 5 (style, layout and graphics algorithms); Step 6 (budgets) |
| **Builds on** | [Event loop and threading](event-loop-and-threading.md) (EL16, EL19–EL22); [frame tree](frame-tree-and-navigation.md) (FT12); [rendering pipeline](rendering-pipeline.md) (PL1–PL16) |
| **Last updated** | 2026-10-17 |

Everything here is **planned**; nothing is implemented. Rules are numbered IV1–IV14. Precedent is sourced (§2); the design is inference unless marked otherwise.

## Summary

- **An explicit lifecycle per document** (IV1). Its phases are style, box tree, layout, pre-paint and paint. Each phase is a function with declared inputs and outputs that stay immutable for the rest of the update, as in Chromium's BlinkNG. Reading a phase's output requires a token that only finishing that phase produces, so phase order is checked by the compiler.
- **Two ways into the pipeline only** (IV2): the rendering update at an offered frame, and a synchronous query that runs just the phases it needs. **No synchronous flush crosses an agent:** a child frame uses the last viewport size it was sent.
- **Each stage reuses what didn't change** (IV3–IV7):
  - style is marked through invalidation sets derived from selectors, preferring too much to too little;
  - boxes are rebuilt only where box-affecting properties changed;
  - a fragment is reused when its inputs match its stored constraints, and containment stops propagation;
  - display items are reused by stable ID, and only changes travel;
  - the compositor redraws only what was damaged.
- **The compositor scrolls and animates without the renderer** (IV8, IV9).
  - It holds the scroll offsets and composites from a display port wider than the viewport.
  - It asks the renderer first only where a listener could block scrolling, and never waits beyond a deadline.
  - Transform and opacity animations tick on the compositor.
  - Content-driven scrolls are never lost.
- **Late content and hidden content are handled explicitly** (IV10–IV13). The parser yields between chunks; images and fonts invalidate only what they affect. Hidden and offscreen frames get no frames. One frame is in flight per event loop.
- **Incremental updates are checked against full rebuilds** (IV14): a test mode rebuilds from scratch after every update and compares.

## 1. Binding inputs

| Input | Consequence here |
|---|---|
| **EL16, EL19–EL22:** task selection; frames offered by the compositor, one outstanding per event loop; the compositor never waits | IV2, IV12 |
| **FT6, FT12, AC1:** each frame renders separately; agents interact only through the browser | IV2, IV13 |
| **PL5, PL14–PL16:** immutable fragments with constraints; display-list updates; scroll offsets and animations in the compositor's property trees; hit-test data | IV5, IV6, IV8, IV9 |
| **IR5:** the browser never blocks on a renderer | IV9 |
| **AC3:** idle costs nothing | IV12 |
| **Decision map §2, item 8:** invest in test oracles from the first milestone | IV14 |

## 2. Precedent

From documentation read on 2026-10-17 (Sources).

| Topic | What engines do |
|---|---|
| **Lifecycle** | Chromium's document lifecycle "keeps track of our progress through the rendering pipeline", enforced by assertions such as: "If we are modifying a ComputedStyle property, then the document lifecycle must be `kInStyleRecalc`". BlinkNG cut the entry points to two: "it is no longer possible to enter the pipeline starting at an intermediate phase". "Each stage should have well-defined inputs and outputs, and its behavior should be functional", and outputs "should be immutable for the remainder of the rendering update" [C1] |
| **Style invalidation** | Blink's invalidation sets find "a smaller set of elements which need recalculation". Changes become immediate or pending invalidations, pushed "when style is about to be read". They "err on the side of correctness, so we invalidate elements that do not need recalculation" [C2] |
| **Layout reuse** | LayoutNG stores each fragment's constraints as a cache key and diffs them to decide whether a child needs layout ([rendering pipeline](rendering-pipeline.md) §2) |
| **Paint reuse** | Chromium copies an unchanged object's display items from the previous list ([rendering pipeline](rendering-pipeline.md) §2) |
| **Asynchronous scrolling** | In Firefox's APZ, the compositor turns input into transforms applied at composite time. It composites from a "displayport" built beyond the viewport, and a user who scrolls past it sees "checkerboarding". Scroll offsets have "a second copy on the compositor side", reconciled when APZ requests a repaint, and "content-driven scroll position changes (e.g. scrollTo from JS) are never lost" [F1] |
| **Listeners that may block scrolling** | APZ gives content "a 'deadline'", 400 ms on desktop, to say whether `preventDefault()` was called, and otherwise assumes it wasn't. Passive listeners, and areas without blocking listeners, skip the round trip [F1] |
| **Compositor animation** | Chromium's compositor thread runs scrolling and animations; "even if a main thread rendering update is happening, scroll and animation can run in parallel" ([event loop](event-loop-and-threading.md) §3, [C1] there) |
| **Containment** | Under layout containment, "nothing outside can affect its internal layout"; size containment lets a box "be laid out without needing to examine its descendants"; `content-visibility` lets the user agent skip "an element's layout and painting entirely when not needed" [W1] |

## 3. The lifecycle

```
mutations ─▶ StyleDirty ─style─▶ StyleClean ─boxes─▶ BoxTreeClean ─layout─▶ LayoutClean
                                                                              │
             sent ◀── PaintClean ◀──paint── PrePaintClean ◀──pre-paint (property trees)
```

| Phase | Input | Output |
|---|---|---|
| Style | DOM; stylesheets; pending invalidations | Computed styles (side table; PL3) |
| Boxes | DOM; computed styles | Box tree (PL4) |
| Layout | Box tree; the viewport; cached fragments | Fragment tree (PL5) |
| Pre-paint | Fragments; computed styles | Property trees: transforms, clips, effects, scroll nodes (PL6) |
| Paint | Fragments; property trees | The frame's display-list update (PL14) |

## 4. Design

### 4.1 Lifecycle and entry points

| # | Rule |
|---|---|
| IV1 | **An explicit lifecycle per document** (§3). Each phase is a function with declared inputs and outputs, which stay immutable for the rest of the update (BlinkNG [C1]). **The compiler checks the order:** code that reads layout output takes a `LayoutClean` token, which only completing layout produces, and likewise for the other phases. Chromium enforces the same contract with runtime assertions |
| IV2 | **Two entry points only** (BlinkNG [C1]). (1) **The rendering update**, at an offered frame (EL21), runs every phase that has work. (2) **A synchronous query** runs only the phases it needs, from the first dirty one up to the one it reads, and for that document only. `getComputedStyle` needs style; `offsetWidth`, `getBoundingClientRect`, `elementFromPoint` and scroll positions need layout. **No synchronous flush crosses an agent.** A same-agent parent (a same-origin `iframe` on the same event loop) can be laid out on demand. A frame whose parent is another agent, in another process or (under SI3) on another event loop, uses the viewport size the browser last sent it (IV13). So moving a frame out of process doesn't change what a query can see (AC1) |

### 4.2 Reuse at each stage

| # | Rule |
|---|---|
| IV3 | **Style invalidation is pending and selector-driven.** A mutation of an attribute, class or ID, or an insertion or removal, records invalidations derived from the features of the selectors in effect: descendant and sibling invalidation sets, as in Blink [C2]. They wait until style is needed, because more mutations may come. Style recalculation then visits only marked elements, and stops descending where an element's inherited values didn't change. **Correctness first:** where the precise set is unknown, the design invalidates more. A stylesheet change invalidates by the features of the rules it added or removed. Step 5 designs the details |
| IV4 | **The box tree is rebuilt only where box-affecting values changed,** such as `display`, `position`, `float` or generated content, and where nodes were inserted or removed. Other style changes mark layout dirty, or only paint |
| IV5 | **A fragment is reused when its inputs match its stored constraints** (PL5). Dirtiness propagates up only to the nearest layout boundary: an element with layout or size containment [W1], or a scroll container whose size doesn't depend on its content |
| IV6 | **Display items are reused by stable ID.** Unchanged boxes keep their items, and a change to a property node, such as a transform, opacity or scroll offset, updates that node without repainting the items under it. Only changes travel to the compositor (PL14) |
| IV7 | **The compositor redraws only what was damaged.** It computes damage from changed items, property nodes and embedded child frames, and redraws only damaged regions where the graphics backend allows (Step 5, I2) |

### 4.3 Updates that bypass the renderer

| # | Rule |
|---|---|
| IV8 | **The compositor scrolls and animates from its property trees** (PL15). **Scrolling:** |

- **The compositor moves first.** On user scrolling it moves the scroll node's offset at once, and tells the renderer afterwards. The renderer then fires scroll events and updates intersection observers and sticky positioning.
- **Two copies of each offset are reconciled with sequence numbers.** The compositor's copy wins for user scrolling; the renderer's wins for scrolling by script, which is never lost (Firefox's APZ [F1]).
- **Display ports.** The renderer paints each scroll frame's content over a display port: the visible area plus a margin. The compositor asks for a new display port as the user moves. Past its edge, the compositor shows the scroll frame's background (checkerboarding), never stale or wrong content.
- **Compositor animations.** Animations and transitions of `transform` and `opacity` that affect nothing else tick on the compositor, from a description the renderer sends. The renderer hears when they start and end. Other animations run in the rendering update.

| # | Rule |
|---|---|
| IV9 | **Listeners that may block scrolling are honoured without blocking.** Hit-test regions say where non-passive wheel or touch listeners exist (PL16). Elsewhere, the compositor scrolls without asking. Where one exists, it asks the renderer and waits at most a deadline: Firefox uses 400 ms on desktop [F1], and Step 6 sets Axiom's. Past the deadline, it scrolls as if `preventDefault()` hadn't been called. The browser never blocks (IR5). CP8 designs the routing |

### 4.4 Late, hidden and dependent content

| # | Rule |
|---|---|
| IV10 | **Late content invalidates only what it affects.** The HTML parser yields between chunks, so rendering opportunities fall between them, and render-blocking stylesheets and fonts follow the specifications. **An image that arrives** repaints only, if its size was known from attributes or an aspect ratio, and relayouts otherwise. **A font that arrives** re-shapes only the text that uses it, under its `font-display` periods. **A viewport resize** re-evaluates media and container queries, invalidates only the rules that depend on them, and then lays out from the root |
| IV11 | **Hidden and offscreen content does no rendering work.** Hidden and occluded frames are offered no frames (EL20). An `iframe` scrolled out of view, as the parent's layout reports to the browser, stops being offered frames until it returns. Its document still runs tasks. `content-visibility: auto` skips style, layout and paint for content that is offscreen [W1], and every phase supports skipping a subtree |
| IV12 | **Frames are produced only when offered and needed.** An event loop answers an offer with an update only if something changed or animation-frame callbacks are due. Otherwise it says "no change", and the compositor keeps its last list. A rendering update that takes longer than a frame holds the next offer, since one is outstanding (EL21). Meanwhile the compositor keeps scrolling and animating, so the frame rate falls gracefully. Idle documents unsubscribe (AC3) |
| IV13 | **Dependencies between frames travel through the browser.** A parent's layout decides its child frame's size and position. The new rectangle goes to the browser in the parent's display-list update (PL7). The browser sends the child its new viewport, and the child invalidates as for any resize (IV10). Nothing waits synchronously across agents (IV2) |

### 4.5 Testing

| # | Rule |
|---|---|
| IV14 | **Incremental equals from scratch.** A test mode rebuilds style, boxes, fragments and the display list from scratch after each incremental update, and compares them. A difference is a failure: under-invalidation can't hide (decision map §2, item 8) |

## 5. Staging

| Level | What changes |
|---|---|
| L0 | All of it. Compositor-driven scrolling and animation run in the same process, through the same property trees |
| L1–L3 | Nothing in this design. Cross-agent dependencies were asynchronous from L0 (IV2, IV13), so an out-of-process frame behaves exactly as an in-process cross-site frame did |

## 6. Alternatives considered

| | Alternative | Assessment |
|---|---|---|
| A | **Dirty bits without an enforced lifecycle** (pre-BlinkNG) | Chromium's experience is the argument: many entry points, under- and over-invalidation, and use-after-free bugs [C1]. Rejected for IV1–IV2 |
| B | **Synchronous flushes across frames,** where a child forces its parent's layout | Possible in one agent; impossible across processes without blocking. Allowing it within a process would make out-of-process iframes behave differently. Rejected except within one agent (IV2) |
| C | **Main-thread scrolling** | Scrolling would stall behind long tasks. Rejected for IV8 |
| D | **Precise style invalidation at any memory cost** | Blink accepts over-invalidation for correctness and simplicity [C2]. Axiom does too |

## 7. Criteria check

| Criterion | How this design meets it |
|---|---|
| AC1 | No synchronous dependency crosses an agent, so L0–L3 behave alike (IV2, IV13) |
| AC3 | Frames only when offered and needed; idle documents unsubscribe (IV12) |
| AC6 | IV14: an oracle for every incremental update |

## 8. Open questions

| # | Question | Where it is settled |
|---|---|---|
| 1 | The deadline for listeners that may block scrolling, display-port margins, and starvation bounds | Step 6 |
| 2 | Which animations may run on the compositor beyond `transform` and `opacity` (filters, scroll-driven animations) | Step 5 |
| 3 | Damage tracking and partial presentation in the GPU backend | Step 5 (I2) |

## Sources

All checked on 2026-10-17.

- [C1] Chrome for Developers, S. Zager and C. Harrelson, "RenderingNG deep-dive: BlinkNG" (last updated 2022-04-19) — https://developer.chrome.com/docs/chromium/blinkng
- [C2] Chromium, "CSS Style Invalidation in Blink" — https://chromium.googlesource.com/chromium/src/+/HEAD/third_party/blink/renderer/core/css/style-invalidation.md
- [F1] Firefox source docs, "Asynchronous Panning and Zooming" — https://firefox-source-docs.mozilla.org/gfx/AsyncPanZoom.html
- [W1] W3C CSS Working Group, "CSS Containment Module Level 2" (Editor's Draft, 9 October 2026) — https://drafts.csswg.org/css-contain-2/
