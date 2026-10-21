# Accessibility: the Tree and the Bridge, and the Shell's Questions

| | |
|---|---|
| **Status** | **Binding** since [ADR-0014](../adr/0014-accessibility.md)'s acceptance (2026-10-18, Gate B). Written at Step 5, CP8. The owner approved §6's two answers as written |
| **Question** | How is the accessibility tree built in the renderer, and how does the browser expose it through `NSAccessibility`? With the owner: the settings list, and the key equivalents a page may override |
| **Answers** | Step 5 question Q8 ([tracker](../research/step-5-subsystems.md) §3); decision-map item I5; the tracker's handed-over questions 12–14 (§2.2); [input and accessibility](input-and-accessibility.md) open questions 2 and 3; [embedder API](embedder-api.md) open question 2 |
| **Feeds** | ADR-0014 (CP9); CP13 (the DOM's accessibility hooks); CP14 (media features from system settings); CP15 (geometry and text runs); the shell milestone (EM11) |
| **Builds on** | [Input and accessibility](input-and-accessibility.md) IN6–IN9; [rendering pipeline](rendering-pipeline.md) PL12; [embedder API](embedder-api.md) EM5, EM6, EM8, EM11; [IPC](ipc.md) IP7–IP11; [frame tree](frame-tree-and-navigation.md) FT1, FT3; [text and fonts](text-and-fonts.md) TX16; the [macOS runtime note](../research/macos-runtime-facilities.md) §10; the [dependency policy](../technology/dependency-policy.md) DP9 |
| **Last updated** | 2026-10-18 |

Everything here is **planned**; nothing is implemented. Rules are numbered AX1–AX12. Precedent is sourced (§3); the design is inference unless marked otherwise.

## Summary

- **The renderer builds the tree; the browser answers the questions.** The renderer turns each document's DOM and boxes into an accessibility tree once an assistive technology first asks, as WebKit enables its cache. It follows HTML-AAM, Core-AAM, the accessible-name computation and WAI-ARIA (AX1). It pushes a complete tree, then incremental updates (AX3), the model Chromium uses and AccessKit borrows. The browser validates the updates, stitches frames together, and answers every `NSAccessibility` query from its cache, never by waiting on a renderer (IN9).
- **Text navigation uses macOS's text-marker API,** which the SDK 27 headers document for "custom web browser engines" (AX5). Chromium, WebKit and Firefox all implement text markers. The renderer precomputes word, sentence and line boundaries, so the browser never runs Unicode segmentation on a renderer's text.
- **The bridge is Axiom's own,** implemented in the shell's `platform` module (AX8). AccessKit's macOS adapter doesn't yet support "rich text or hypertext", which web content needs.
- **System settings reach pages as media features** (AX10): reduced motion, increased contrast, reduced transparency, and appearance.
- **Two questions for the owner** (§6): the settings list, with how the sensitive-site list and the JIT switches appear; and which key equivalents a page may never override. The recommendation follows Chromium's reserved commands: tab, window and quit commands, and the address bar, are never delivered to pages. **The owner approved both as written at Gate B** (2026-10-18).

## 1. Binding inputs

| Input | Consequence here |
|---|---|
| **IN9:** accessibility is a cache in the browser, built after the first query from an assistive technology; renderers send incremental updates; the browser stitches frames by token and answers `NSAccessibility` from the cache, never asking a renderer (IR5); actions go to renderers as messages | AX1–AX7 |
| **PL12:** node identity is the document ID with the node handle and generation; geometry comes from fragments; the tree is built on the event loop's thread from the DOM and the box tree | AX1, AX2 |
| **IN6–IN8:** the browser tracks focus; IME is answered from a mirror; user activation comes from routed input | AX4, AX7 |
| **EM6, EM8, EM11:** accessibility crosses the embedder API as hosted objects; the browser's own interface is native; the shell is pure Rust with objc2, and its first milestone validates objc2-defined classes for text input and accessibility | AX4, AX8 |
| **IP7–IP11:** interfaces in the IDL; generated decoders; `Untrusted<T>` validated against the browser's records; typed violations | AX2, AX6 |
| **DP9:** a crate in the browser that handles renderer-supplied data needs `ub-risk-1`. ICU4X's data crates are graded `ub-risk-2` | AX5 |
| **ADR-0001:** accessibility is designed in | §8 |

## 2. Specifications

- **WAI-ARIA**, roles, states and properties.
- **Core Accessibility API Mappings 1.2,** a W3C Candidate Recommendation Draft dated 2026-10-12, whose platforms include the "macOS Accessibility Protocol". For example, a button maps to "AXRole: `AXButton`" [S1].
- **HTML Accessibility API Mappings 1.0,** a W3C Working Draft dated 2026-10-12, which maps HTML elements to the same platforms and "adapts" the Accessible Name and Description Computation 1.2 for HTML [S2].
- **Accessible Name and Description Computation 1.2.**
- **Media Queries Level 5:** `prefers-reduced-motion`, `prefers-contrast`, `prefers-reduced-transparency`, `prefers-color-scheme`.

## 3. Precedent

Read on 2026-10-18; dates in brackets are the last commits to each file, from GitHub's API.

| Engine or library | Accessibility on macOS |
|---|---|
| **Chromium** | Its platform nodes implement Cocoa accessibility (`ax_platform_node_cocoa.mm`, `ax_platform_node_mac.mm`). `browser_accessibility_cocoa.mm` implements text markers, with 71 mentions of `AXTextMarker` [C1]. It reserves a fixed set of commands that pages never receive: closing a tab or window, new windows and tabs, restoring a tab, switching tabs, and quitting (`kReservedCommandIds` in `accelerator_table.cc` [2026-10-15]). In fullscreen, all commands but leaving fullscreen and quitting go to the page (`browser_command_controller.cc` [2026-10-15]) [C2] |
| **WebKit** | `WebAccessibilityObjectWrapperMac.mm` [W1] |
| **Firefox** | `mozAccessible.mm`, with text markers in `GeckoTextMarker.mm` and `MOXTextMarkerDelegate.mm` [F1] |
| **AccessKit** (0.25.1, 2026-09-25) | "The schema is based largely on Chromium's cross-platform accessibility abstraction." It works by push: "the renderer process pushes data to the browser process ... initially ... a complete accessibility tree, then it pushes incremental updates", and the browser asks only for actions. Its macOS adapter "implements the NSAccessibility protocols". Its adapters "don't yet support rich text or hypertext" [K1] |

**macOS's SDK 27** (observed in HIServices' headers):
- **`AXWebConstants.h`** describes "the text marker API: a set of attributes that custom web browser engines and other rich-text engines implement to expose fine-grained text positions to assistive technologies such as VoiceOver, Speak Selection, and Hover Text". With markers and ranges, assistive technologies "navigate by character, word, line, sentence, and paragraph". A marker is "an opaque payload: your engine creates one with AXTextMarkerCreate".
- **`AXUIElement.h`** declares `AXTextMarkerCreate`, `AXTextMarkerGetBytePtr` and `AXTextMarkerRangeCreate`.
- **`AXWebConstants.h`** defines 161 attribute constants for web content, among them `kAXARIALiveAttribute`, `kAXDOMIdentifierAttribute` and `kAXCaretBrowsingEnabledAttribute`.

**Observations** (inference):
- **All three engines implement text markers,** which VoiceOver uses to read web content.
- **Chromium's push model** is the one IN9 already adopted.

## 4. Build or reuse

| Need | Candidates | Decision |
|---|---|---|
| **Building the tree** | **Our own:** it is the DOM's and layout's own data, under HTML-AAM's rules | **Our own** (AX1) |
| **The tree's format across IPC** | **Our own,** in the IDL, modeled on Chromium's abstraction, as AccessKit's schema is; **AccessKit's schema crate** | **Our own, in the IDL** (AX2): its decoders must be generated and validated (IP8, IP10) |
| **The `NSAccessibility` bridge** | **AccessKit's macOS adapter** 0.27.1: no rich text or hypertext yet [K1]. **Our own,** in the shell's `platform` module | **Our own** (AX8). AccessKit is reconsidered if its adapter gains hypertext |

## 5. Design

### 5.1 The tree, in the renderer

| # | Rule |
|---|---|
| AX1 | **The renderer builds an accessibility tree per document,** on the event loop's thread after each rendering update, from the DOM and the box tree (PL12). It does so only once accessibility is enabled (AX7). Roles, names, descriptions, values, states and relations follow HTML-AAM, Core-AAM, the accessible-name computation and WAI-ARIA (§2). The tree is maintained incrementally, from DOM mutations, style changes that affect it (`display`, `visibility`, `content`) and layout's geometry |
| AX2 | **The tree's format is defined in the IDL** (IP7). Each node has:<br>• a stable ID: the document, the node handle and its generation (PL2);<br>• a role, name, description, value and states;<br>• relations by ID: labelled by, described by, controls, owns;<br>• the actions it supports;<br>• its geometry in frame coordinates, with the property-tree node it sits in, so that the browser can place it on screen after scrolling or zooming without asking;<br>• for text, runs with offsets and their word, sentence and line boundaries (AX5).<br>Child frames are referenced by frame token (FT3). **Limits:** node counts, depth, string sizes and update rates are bounded, and a violation is typed (IP11).<br>**As amended in Round 2** ([RV2-06](../review/round-2/03-findings.md#rv2-06-accessibility-geometry-after-the-gpu-process)): since Round 1 the browser holds only the property-tree nodes its layers need (PL15), so **each node's bounds are in the space of the nearest scroll or animation node that has a layer,** with every other transform, clip and effect applied by the renderer, as PL16 tags hit-test regions. The browser then applies only the offsets and transforms it holds. **Text runs also carry an advance per grapheme cluster,** so the browser can bound any text-marker range from its cache; they are sent only in AX7's full tier, within the limits above |
| AX3 | **Push, then update.** When accessibility is enabled, a document sends its complete tree, then incremental updates (nodes created, changed, removed) with each rendering update, coalesced. The browser asks for nothing; it only sends actions. This is Chromium's model, which AccessKit describes (§3), and which IN9 adopted |

### 5.2 The cache and the bridge, in the browser

| # | Rule |
|---|---|
| AX4 | **The browser answers every `NSAccessibility` query from its cache,** on the main thread, without waiting on a renderer (IN9, IR5). Roles, subroles and attributes follow Core-AAM's and HTML-AAM's AX API mappings (§2). This answers input and accessibility open question 3. **Actions** go to the renderer as messages: press, focus, set value, set selection, scroll into view, increment and decrement. An assistive technology's press is routed like input, so it counts as user activation (IN8): the request comes from the user, through the system. **Notifications** are posted from cache updates: value, selection, focus, layout and live-region changes.<br>**As amended in Round 2:**<br>• **Hit testing** (RV2-06): AppKit's hit test is synchronous, and only the renderer is exact. The browser answers at once from the cached bounds, asks the renderer asynchronously for the exact node, and uses that answer for the next query that falls within it, as Chromium does since 2016.<br>• **The notifications** (RV2-09) are the SDK's eight for web content (`AXWebConstants.h`, which gives their names only), each posted from a cache update. Their sources below are inferred from the names and are checked against Core-AAM's table of state and property changes when built: `AXActiveElementChanged` (focus), `AXCurrentStateChanged` (`aria-current`), `AXExpandedChanged` (`aria-expanded`), `AXInvalidStatusChanged` (`aria-invalid`), `AXLayoutComplete` (after a layout that changed the tree's geometry), `AXLiveRegionCreated` and `AXLiveRegionChanged` (live regions), and `AXLoadComplete` (a document's load); with AppKit's value and selection notifications, and `NSAccessibilityAnnouncementRequestedNotification` for `ariaNotify` |
| AX5 | **Text navigation uses the text-marker API** that SDK 27 documents for web engines (§3). A marker's payload is opaque to assistive technologies, and encodes a frame token, a node ID, a text offset and an affinity. **Boundaries come from the renderer:** each text run carries its word, sentence and line boundaries, computed with ICU4X (TX16) and layout. The browser only indexes them. So ICU4X never runs in the browser on renderer-supplied text (DP9) |
| AX6 | **Frames are stitched by token.** An `iframe` node refers to its child frame's tree by frame token, and only to a child that the browser's frame tree lists for that frame (FT1). A child frame in another process is another renderer's tree in the same cache. A renderer's claim about a frame that isn't its child is dropped, and is a violation (IN9, IP11) |
| AX7 | **Enabled on first use,** as WebKit enables its cache only after an assistive technology's first query ([macOS runtime note](../research/macos-runtime-facilities.md) §10). The first query tells every renderer with a visible document to build its tree. Documents that load later build theirs at load. The cost is paid only by users of assistive technologies. The cache's and the trees' memory is measured at the shell milestone.<br>**As amended in Round 2** ([RV2-07](../review/round-2/03-findings.md#rv2-07-accessibility-enabled-by-any-client)): many clients of the accessibility API aren't assistive technologies (password managers, window managers, antivirus tools), so **enablement has tiers:**<br>• **Full,** while VoiceOver or Switch Control runs, which the browser observes through `NSWorkspace`'s `voiceOverEnabled` and `switchControlEnabled` by KVO: every visible document's tree, with text markers, advances and notifications, as above.<br>• **Basic,** for any other client's query: the tree of the document queried, built on demand, with roles, names, states and bounds, as Chromium turns on only a basic mode for such clients since 2024.<br>• **Off again** after a period with no queries (a hypothesis, 60 s, tuned at the shell milestone): the trees are discarded and the renderers stop sending updates.<br>Each tier's memory is measured at the shell milestone |
| AX8 | **The bridge is Axiom's own,** in the shell's `platform` module, through objc2-defined classes that implement `NSAccessibility`. Those are the classes whose validation opens the shell milestone (EM11). The browser's own interface is native AppKit, accessible by AppKit itself (EM8), so it needs no bridge |

### 5.3 System settings, tests and memory

| # | Rule |
|---|---|
| AX9 | **Focus and caret.** The browser's focused-frame record (IN6) and each renderer's focused node give assistive technologies one focus. Caret browsing, which `kAXCaretBrowsingEnabledAttribute` exposes (§3), comes later |
| AX10 | **System settings reach pages as media features.** The browser reads macOS's accessibility display options and appearance: reduce motion, increase contrast, reduce transparency, dark mode, and, **as amended in Round 2** (RV2-09), invert colours (`accessibilityDisplayShouldInvertColors`). It sends them to renderers as inputs to `prefers-reduced-motion`, `prefers-contrast`, `prefers-reduced-transparency`, `prefers-color-scheme` and `inverted-colors` (CP14). A change is a style invalidation like any other (IV3) |
| AX11 | **Tests:**<br>• WPT's `accname`, `html-aam` and `core-aam` directories, through the headless embedder, which exposes each document's computed roles and names (EM9);<br>• the same tree checked incrementally against a tree rebuilt from scratch (IV14);<br>• VoiceOver checks with the owner at the shell milestone.<br>**As amended in Round 2** ([RV2-08](../review/round-2/03-findings.md#rv2-08-accessibility-tests-that-cant-run-as-written)), **which suite runs where:**<br>• **Roles and names** (WPT's `wai-aria`, `accname` and `html-aam` tests that call `test_driver.get_computed_role` or `get_computed_label`): in the headless embedder, through the WebDriver commands Get Computed Role and Get Computed Label, which TS2's subset gains at M4, answered from the renderer's tree.<br>• **The platform mapping** (`core-aam/aamtests` and `html-aam/aamtests`, which query macOS's accessibility API by the browser's process ID): against the shell's build, or a test embedder with the AppKit bridge, never the headless one, which has no AppKit. wptrunner's `axiom` product reports the process ID in an `axiom:processID` capability, with a matching entry in the fixtures' `pid_from`; the runner needs the Accessibility permission, which the owner grants by hand, as for S15's privacy decisions.<br>• **`ariaNotify`** (WPT's `core-aam/arianotify`, eleven manual tests) with the owner, from the stage that brings it |
| AX12 | **Memory and idle.** With no assistive technology, there is no tree and no cache (AX7). **As amended in Round 2** (RV2-07): with no client of the accessibility API at all, there is none; another client gets AX7's basic tier for the document it queries, discarded when its queries stop. With one, trees are bounded per document, and hidden documents stop updating (IV11) until they are shown again |

## 6. Questions for the owner

These were handed to Step 5 "with the owner" ([embedder API](embedder-api.md) open question 2; [input and accessibility](input-and-accessibility.md) open question 2). They were put to the owner at Gate B, with these recommendations. **Both were approved as written** (2026-10-18): "Settings List: Approved as written in docs/architecture/accessibility.md §6.1." "Reserved Key Equivalents: Approved as written in docs/architecture/accessibility.md §6.2 (Chromium reserved set, plus ⌘L, ⌘,, and Esc un-overridable even in fullscreen)."

**6.1 The settings list, and how the sensitive-site list and the JIT switches appear.** Recommended for the shell's first milestone, with defaults; everything else waits until it is needed:

| Group | Settings (default) |
|---|---|
| **Security** | **Sensitive sites:** the owner's list, plus the sites the browser's heuristics add (password entered, OAuth, a COOP header; SI5), each shown with why it is there and removable. Also in each site's information panel as "Treat as sensitive". A sensitive site always gets a dedicated, no-JIT renderer (SI5, J4). **JavaScript JIT:** a global switch, and a per-site switch in the site's panel, shown disabled, and explained, while the build has no JIT (J1–J7). **The current security level** (L0–L3), shown read-only |
| **Privacy** | Third-party cookies: blocked (fixed while SI3 needs it; no exceptions offered at first). Clear browsing data, per site and for everything |
| **Permissions, per site** | Camera, microphone, location, notifications, local network access (NW12), persistent storage (SW6), pop-ups, automatic downloads. All "ask" by default |
| **Content** | Default zoom; minimum font size; the families for CSS's generic fonts (TX6); preferred languages, for `Accept-Language` and for choosing Han fonts |
| **Downloads** | Folder (`~/Downloads`); ask where to save (off) |
| **Startup and search** | Restore the previous session (on); the address bar's search engine, as a URL template the owner sets |

**Recommendation: approve this list for the first shell milestone,** with the security group as specified. It covers every switch the architecture already requires (SI5, J1–J7, NW12, SW6) and the content settings that text and layout need, and nothing speculative.

**6.2 Key equivalents a page may never override.** Recommended:
- **Reserved, never delivered to pages:**
  - Quit (⌘Q);
  - close tab and window (⌘W, ⇧⌘W);
  - new tab and window (⌘T, ⌘N), and reopen the closed tab (⇧⌘T);
  - switch tabs (⌃Tab, ⌃⇧Tab, ⇧⌘[, ⇧⌘], ⌘1–⌘9);
  - **focus the address bar (⌘L);**
  - Settings (⌘,).

  AppKit and the system already handle Hide (⌘H), Minimize (⌘M) and switching apps before the page sees them.
- **Delivered to the page first;** the browser's default runs only if the page doesn't cancel it: every other key equivalent. That includes ⌘S, ⌘P, ⌘F, ⌘K, ⌘Z, ⌘B and ⌘I, which web applications use.
- **In fullscreen,** Esc always leaves fullscreen, and the reserved set stays reserved. The Keyboard Lock API, which would let a fullscreen page capture more, comes later, behind the browser's prompt.

**Recommendation: approve.** It is Chromium's reserved set (§3), plus ⌘L and ⌘,. ⌘L is added because a page that could block the address bar could keep the user from seeing or changing where they are, a spoofing risk the threat model lists among the browser interface's attack surfaces (§5 there). Chromium instead hands fullscreen pages almost everything (§3). The recommendation keeps the reserved set in fullscreen too, because a single user gains nothing from pages capturing ⌘W.

## 7. Interfaces with neighbouring subsystems

| With | Interface |
|---|---|
| **The DOM and layout** (CP13, CP15) | ARIA attributes, names, and the box tree's geometry and text runs, for AX1–AX2 |
| **Text** (CP3) | Word and sentence boundaries from ICU4X; line boundaries from layout (AX5) |
| **Style** (CP14) | Media features from system settings (AX10) |
| **The shell** (EM6, EM11) | The objc2-defined accessibility classes; the settings (§6.1); key handling (§6.2) |

## 8. Security

- **Renderer trees are untrusted.** They are decoded by generated code (IP8), validated (IP10), bounded (AX2), and stitched only to the browser's own frame records (AX6).
- **The browser runs no Unicode segmentation on renderer data** (AX5, DP9).
- **Reserved key equivalents** keep a page from trapping the user, or hiding the address bar (§6.2).

## 9. Staging and the complete model

| Stage | What exists |
|---|---|
| The shell milestone | The objc2 classes validated (EM11); the cache; roles, names, actions and notifications; text markers |
| By M7 (Round 2, RV2-09) | Live regions in full (`aria-live`, `aria-atomic`, `aria-relevant`, `aria-busy`) and `ariaNotify`, for Twitch's chat (M7's target) |
| Later | Caret browsing; math (MathML-AAM); `graphics-*` roles for SVG |
| L0–L3 | No change: trees are pushed by renderers and stitched by token at every level |

## 10. Alternatives considered

| | Alternative | Assessment |
|---|---|---|
| A | **Synchronous queries to renderers,** answered on demand | Chromium's experience behind IN9 rules it out: the browser's main thread would wait on renderers. Rejected (IN9) |
| B | **AccessKit's macOS adapter** | It lacks rich text and hypertext (§3). Rejected for now; reconsidered when it has them |
| C | **Segmenting text in the browser** | It would put ICU4X on renderer-supplied text in the most privileged process (DP9). Rejected (AX5) |

## 11. Open questions

| # | Question | Where it is settled |
|---|---|---|
| 1 | The settings list (§6.1) | **Resolved:** approved as written by the owner at Gate B (2026-10-18) |
| 2 | Reserved key equivalents (§6.2) | **Resolved:** approved as written by the owner at Gate B (2026-10-18) |
| 3 | The memory cost of trees and the cache | The shell milestone |

## Sources

All read on 2026-10-18. Dates in brackets are the last commits to each file, from GitHub's API.

- [S1] W3C, "Core Accessibility API Mappings 1.2", Candidate Recommendation Draft (2026-10-12) — https://www.w3.org/TR/core-aam-1.2/
- [S2] W3C, "HTML Accessibility API Mappings 1.0", Working Draft (2026-10-12) — https://www.w3.org/TR/html-aam-1.0/
- [C1] Chromium, `ui/accessibility/platform` (`ax_platform_node_cocoa.mm`, `browser_accessibility_cocoa.mm`) — https://github.com/chromium/chromium/tree/main/ui/accessibility/platform
- [C2] Chromium, `chrome/browser/ui/accelerator_table.cc` and `browser_command_controller.cc` — https://github.com/chromium/chromium/blob/main/chrome/browser/ui/accelerator_table.cc
- [W1] WebKit, `Source/WebCore/accessibility/mac/WebAccessibilityObjectWrapperMac.mm` — https://github.com/WebKit/WebKit/tree/main/Source/WebCore/accessibility/mac
- [F1] Firefox, `accessible/mac` — https://github.com/mozilla-firefox/firefox/tree/main/accessible/mac
- [K1] AccessKit, README — https://github.com/AccessKit/accesskit ; crates.io's API, for `accesskit` and `accesskit_macos`
- Local: the SDK's `ApplicationServices.framework/Frameworks/HIServices.framework/Headers` (`AXWebConstants.h`, `AXUIElement.h`)
