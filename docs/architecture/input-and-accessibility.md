# Input, Focus, Hit Testing, Scrolling and Accessibility Across Frames

| | |
|---|---|
| **Status** | **Binding** since ADR-0007's acceptance (2026-10-17, Gate B). Written at Step 4, CP8. **Amended by the architecture review's Round 1** (the owner's dispositions, 2026-10-21; [the review](../review/README.md) §5): IN1 for the GPU/compositor process (RV1-02; CP2) |
| **Question** | How does the browser route pointer, wheel, keyboard, IME and gesture input to frames that may live in different processes? How are focus, scrolling, user activation and accessibility handled, when AppKit asks synchronous questions and the browser may never block on a renderer? |
| **Answers** | Step 4 question Q13 ([tracker](../research/step-4-architecture-proposal.md) §3); the input-routing part of SI4 |
| **Feeds** | ADR-0007; CP9 (embedder API: input and accessibility at the shell boundary); Step 5 (the accessibility bridge, I5) |
| **Builds on** | [macOS runtime note](../research/macos-runtime-facilities.md) §7, §8, §11 items 13–14; [event loop and threading](event-loop-and-threading.md); [IPC](ipc.md); [frame tree](frame-tree-and-navigation.md); [rendering pipeline](rendering-pipeline.md) (PL16); [invalidation](invalidation.md) (IV8, IV9) |
| **Last updated** | 2026-10-21: Round 1's accepted findings |

Everything here is **planned**; nothing is implemented. Rules are numbered IN1–IN12. Precedent and specification text are sourced (§2); the design is inference unless marked otherwise.

## Summary

- **Input enters at the compositor** (IN1, IN2). AppKit's main thread hands every event to the compositor thread at once. The compositor hit-tests the latest composited picture, using each frame's hit-test regions mapped through its parent's embedding, and sends the event to the target frame's event loop in that frame's coordinates. The renderer finds the element and dispatches. Where a region is too complex to decide, the compositor asks the renderer, with a deadline.
- **Gestures are latched** (IN3): a press, drag, scroll or touch sequence goes to the frame chosen when it began, momentum included.
- **Scrolling stays in the compositor** (IN4), chaining from a child frame's scroller to its parent's across processes. The renderer is asked first only where a listener could block (IV9).
- **Pointer capture and focus are the browser's records** (IN5, IN6). Keyboard input goes to the focused frame, and sequential focus moves between frames through the browser.
- **IME is answered from a mirror** (IN7). The focused frame's renderer keeps the browser's copy of the text, selection, composition and character rectangles current, so `NSTextInputClient`'s synchronous queries never wait. This is Firefox's ContentCache model, not Chromium's 1.5 s blocking wait.
- **User activation comes from routed input** (IN8). It propagates as the HTML standard specifies, and is consumed only with the browser's consent.
- **Accessibility is a cache in the browser** (IN9). Renderers send incremental accessibility trees, stitched across frames. The browser answers assistive technologies from the cache, which Firefox reports made it up to 20× faster on the worst pages.
- **The pasteboard, menus and dragging are mediated by the browser** (IN10). Hit-test claims are clipped to each frame's own area (IN11), and routing across processes is tested (IN12).

## 1. Binding inputs

| Input | Consequence here |
|---|---|
| **SI4:** the browser routes input to frames from the first line of code | IN1–IN6 |
| **IR5:** the browser never blocks on a renderer | IN1, IN7, IN9 |
| **EL6:** AppKit's main thread does UI only; the compositor thread does hit testing; the core thread holds the frame tree | IN1, IN6 |
| **FT2, FT6:** facts come from the browser's records; focus and activation are decided by the browser | IN6, IN8 |
| **PL12, PL16:** accessibility from the DOM and box tree; hit-test regions in display lists | IN1, IN9 |
| **IV8, IV9:** the compositor scrolls; listeners that may block scrolling are asked with a deadline | IN4 |
| **Process model §4.1:** renderers have no pasteboard; the browser mediates it | IN10 |
| **macOS runtime note §11, items 13 and 14:** IME and accessibility queries are synchronous on the main thread; scroll gestures have phases | IN3, IN7, IN9 |

## 2. Precedent and specification

From sources read on 2026-10-17 (Sources).

| Topic | What engines and specifications do |
|---|---|
| **Routing across processes** | Chromium hit-tests in the browser process to "deliver input events directly to the intended frame's renderer process", and manages focus in the browser [C1]. Its compositor "determines if the main thread needs to know about the event"; the browser "converts it to a scroll gesture" [C2]. Firefox routes "UI events ... to APZ first, to minimize latency", hit-testing with WebRender [F3] |
| **IME** | Firefox's `ContentCache` "stores various information of the child content": the whole text, the composition, the selection's rectangle, the first character's rectangle ("because Yosemite's Japanese IME sometimes tries to query it"), and the character rectangles around the selection. The parent handles IME queries from it [F1]. Chromium instead blocks its UI thread for up to 1,500 ms on the renderer ([macOS runtime note](../research/macos-runtime-facilities.md) §10) |
| **Accessibility** | Firefox 113 (2023) provides "a cache of all tab and browser UI content to screen readers in the browser's parent process". Before, "screen readers had to do a lot of expensive work to retrieve and relay content". Afterwards it is "up to 20 times faster" in the worst cases. "We were inspired by Chrome's approach" [F2]. Chromium's browser wraps the tree the renderer sends ([macOS runtime note](../research/macos-runtime-facilities.md) §10) |
| **User activation** | Activation-triggering input: trusted `keydown` (not Esc or a reserved shortcut), `mousedown`, `pointerdown` for mice, `pointerup` for other pointers, `touchend`. Notification updates the document's window, its ancestors' windows, and descendants "whose active document's origin is same origin". Consuming activation clears transient activation for the whole page [H1] |
| **Focus** | Each document has one focused area. A top-level traversable's focused area is found by descending through containers into their content navigables. Key events sent to a container "get immediately routed to its content navigable's active document" [H1] |

## 3. The path of an event

```
NSEvent (main thread)
  └─▶ compositor thread: hit test over composited frames ─▶ target frame, scroll node
        ├─ scroll with no blocking listener ─▶ compositor scrolls (IV8)
        └─ otherwise ─▶ target frame's event loop (renderer)
               └─ DOM hit test and dispatch ─▶ acknowledgement (handled, prevented)
                      └─▶ compositor: scroll if not prevented; core: activation, focus
```

## 4. Design

### 4.1 Routing

| # | Rule |
|---|---|
| IN1 | **Input enters at the compositor.** The main thread passes each `NSEvent` to the compositor thread at once, without engine logic (EL6). The compositor hit-tests the latest composited picture of that window. **As amended in Round 1** (RV1-02): from M2 this compositor thread stays in the browser, holding the layer tree and the hit-test regions that arrive with the GPU process's layer commits, so input never waits on the GPU process. It walks the frames' hit-test regions (PL16), each mapped through its parent's embedding transform and clip, and finds the target frame and the scroll node under the point. **Regions too complex to decide,** such as those under a clip path or an unusual transform, are marked imprecise by their renderer. For those, the compositor asks the candidate frame's renderer and waits at most a deadline, queueing the gesture's later events meanwhile (IR5) |
| IN2 | **The renderer finishes the job.** The target frame's event loop receives the event in that frame's coordinates, finds the DOM element, and dispatches. It then acknowledges: handled, or default prevented. The browser measures acknowledgement time for hang detection ([event loop](event-loop-and-threading.md) §4.7) |
| IN3 | **Gestures are latched.** A gesture is routed to the frame chosen at its start until it ends: a button press through its release, a scroll from its began phase through the end of its momentum, a touch sequence. This uses the phases that `NSEvent` carries (macOS runtime note §7). Scroll gestures also latch to a scroll node |

### 4.2 Scrolling and capture

| # | Rule |
|---|---|
| IN4 | **Scrolling stays in the compositor** (IV8). The compositor scrolls the latched node. Where hit-test regions show a listener that may block (PL16), the event goes to the renderer first, under IV9's deadline. **Scroll chaining:** when a node reaches its limit, the remaining distance passes to its ancestor scroll node, from a child frame's root scroller to the parent frame's node at the embedding point. That works across processes, entirely within the compositor's property trees, and respects `overscroll-behavior` |
| IN5 | **Pointer capture is a browser record.** A renderer that calls `setPointerCapture` asks the browser. The browser accepts only if that frame is currently receiving that pointer's events, and then routes the pointer to it until it is released. The implicit capture of a mouse drag follows the same rule: events go to the frame where the button went down until it comes up |

### 4.3 Focus, keyboard and text input

| # | Rule |
|---|---|
| IN6 | **The browser tracks the focused frame** (FT6), as Chromium does [C1]. Keyboard events go to the focused frame's event loop, after the shell has had its key equivalents (menu shortcuts). A click makes the frame under it focused. `focus()` and `blur()` from script are requests the browser validates. **Sequential navigation** (Tab) crosses frames through the browser: a renderer that reaches the end of its sequence asks the browser to continue, and the browser moves focus to the next frame in tree order. The specification treats a container as "a placeholder for its content navigable's active document" [H1] |
| IN7 | **IME is answered from a mirror on the main thread.** While an editable element has focus, its frame's renderer keeps the browser's copy current: the text around the selection, the selection and composition ranges, and the caret and character rectangles in screen coordinates. `NSTextInputClient`'s synchronous queries read the mirror without waiting (macOS runtime note item 13). Edits from the IME (`insertText`, `setMarkedText`, `unmarkText`) go to the renderer as messages. The renderer applies them and sends the new state. Firefox's `ContentCache` is the precedent [F1]. A query that arrives before the mirror has caught up gets the latest state, as the platform tolerates; it never blocks |

### 4.4 Activation, accessibility and mediation

| # | Rule |
|---|---|
| IN8 | **User activation comes from routed input.** When the compositor routes an activation-triggering event [H1] to a frame, the core records activation for that document. It propagates the activation to the windows the specification names (ancestors, and same-origin descendants) and tells their renderers. **Consuming activation** (`window.open`, fullscreen, other gated APIs) is decided by the browser from its records, and clears transient activation for the whole page, as specified. A renderer can't create activation (FT2, FT14) |
| IN9 | **Accessibility is a cache in the browser.** After the first query from an assistive technology (WebKit also enables its cache only then; macOS runtime note §10; **as amended in Round 2,** RV2-07, enablement has tiers, full for VoiceOver and Switch Control and basic for other clients: [accessibility](accessibility.md) AX7), each renderer builds an accessibility tree per document from the DOM and box tree (PL12). It sends incremental updates to the browser's main thread. There the trees are stitched across frames: an `iframe` node refers to its child frame's tree by token, and only to children the browser's frame tree lists. The browser answers `NSAccessibility` queries from the cache, never by asking a renderer (IR5). Actions (press, focus, set value, scroll into view) go to the renderer as messages. Updates are validated like any untrusted input (IP10): tree shape, bounded sizes, references. Firefox reports this design made it up to 20× faster on the worst pages [F2]. Step 5 designs the bridge to `NSAccessibility` (I5) |
| IN10 | **The browser mediates the pasteboard, menus and dragging.** Renderers have no pasteboard ([process model](../security/process-model.md) §4.1). Copying sends the selection's data to the browser, which writes the pasteboard. Pasting happens only on a user action: the browser reads the pasteboard and sends the data to the focused frame. A context menu is requested by the renderer, with what it hit-tested. The browser shows the menu, checks any URL in it against its own records before acting on it, and sends the chosen action back. Dragging into and out of frames, later, follows the same pattern |

### 4.5 Integrity and tests

| # | Rule |
|---|---|
| IN11 | **Hit-test claims are clipped.** A renderer's regions count only inside the area its frame occupies, as its parent's display list placed it. So a renderer can't attract input meant for other frames by claiming their area. What a parent may do to its children by layout, such as covering them, is what the web platform allows, and is left to the specifications |
| IN12 | **Routing is tested across processes** (extending V9): events reach the right frame in the right coordinates; a renderer receives nothing routed to another frame; capture can't be taken by a frame that isn't receiving the pointer; activation can't be forged; the IME mirror stays consistent through compositions |

## 5. Staging

| Level | What changes |
|---|---|
| L0 | All of it: the browser's compositor thread routes events to event loops, which run in renderer processes from M1 (Round 1, RV1-07), and the mirrors and caches exist. From M2 the compositor thread keeps routing while the GPU/compositor process rasterizes (RV1-02) |
| L1–L3 | Nothing in this design. A cross-site frame already received input through the browser at L0, so moving it to another process changes no path (AC1) |

## 6. Alternatives considered

| | Alternative | Assessment |
|---|---|---|
| A | **Renderers route input to their children** | Input for an out-of-process child would have to pass through the parent's renderer, which would see it. Rejected for IN1 |
| B | **Synchronous IME queries to the renderer, with a timeout** (Chromium) | Up to 1.5 s of frozen UI, and blocking contradicts IR5. Rejected for IN7 |
| C | **Accessibility answered live by renderers** (WebKit's private route; Firefox before 113) | It needs private SPI and AppKit in the renderer, and Firefox measured it as slow. Rejected for IN9 |

## 7. Criteria check

| Criterion | How this design meets it |
|---|---|
| AC1 | The same routing, focus, IME and accessibility paths from L0 to L3 (§5) |
| AC2 | Focus, capture, activation and stitching are the browser's records (IN5, IN6, IN8, IN9) |
| AC8 | Accessibility and IME are designed in, not deferred (IN7, IN9) |

## 8. Open questions

| # | Question | Where it is settled |
|---|---|---|
| 1 | The deadlines for imprecise hit tests and for listeners that may block scrolling | Step 6 |
| 2 | Which key equivalents a page may override | Step 5 (shell), with the owner |
| 3 | The mapping from accessibility trees to `NSAccessibility` roles and actions | Step 5 (I5) |
| 4 | Dragging into and out of frames, and between processes | A later milestone |

## Sources

All checked on 2026-10-17.

- [H1] WHATWG HTML, §6 "User interaction": activation-triggering input events, activation notification and consumption, focus (Last Updated 16 October 2026) — https://html.spec.whatwg.org/multipage/interaction.html
- [C1] Chromium, "Out-of-Process iframes (OOPIFs)" (undated) — https://www.chromium.org/developers/design-documents/oop-iframes/
- [C2] Chrome for Developers, "RenderingNG architecture" (last updated 2021-07-26) — https://developer.chrome.com/docs/chromium/renderingng-architecture
- [F1] Firefox, `widget/ContentCache.h` — https://github.com/mozilla-firefox/firefox/blob/main/widget/ContentCache.h
- [F2] Mozilla, "Firefox 113 significantly boosts accessibility performance" (2023-05-09) — https://blog.mozilla.org/en/uncategorized/firefox-accessibility-boost/
- [F3] Firefox source docs, "Asynchronous Panning and Zooming" and "Rendering Overview" — https://firefox-source-docs.mozilla.org/gfx/AsyncPanZoom.html ; https://firefox-source-docs.mozilla.org/gfx/RenderingOverview.html
