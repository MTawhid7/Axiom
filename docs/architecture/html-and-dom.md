# HTML and the DOM: Parsing, Nodes, Events and the Rest of the DOM

| | |
|---|---|
| **Status** | **Proposed** (Step 5, CP13). To be decided with part 2's ADRs at CP18, for the owner at Gate C. Section 5.2 proposes an amendment to [ADR-0008](../adr/0008-script-agents-and-heap.md) (HD11) |
| **Question** | How is HTML parsed, from bytes to tree? How is the DOM represented in Pattern B arenas: nodes, links, attributes, text, events, mutation observers, Shadow DOM, custom elements, ranges and selection, forms and editing? |
| **Answers** | Step 5 question Q9 ([tracker](../research/step-5-subsystems.md) §3); the handed-over question 18, on S3's listener snapshot (§2.2); HTML's encoding sniffing, which [text and fonts](text-and-fonts.md) TX3 left to CP13 |
| **Feeds** | Part 2's ADRs (CP18); CP14 (selectors over the DOM, style's side tables, invalidation hooks); CP15 (boxes built from the DOM, hit tests back to nodes); CP16 (the value type for nodes, microtasks, script elements); CP17 (the DOM's JavaScript facet: prototypes, expando properties, `[CEReactions]`, event handler attributes) |
| **Builds on** | [ADR-0003](../adr/0003-gc-handle-and-rooting-pattern.md) and [script and heap](script-and-heap.md) (SH1, SH6, SH8–SH12); the [rendering pipeline](rendering-pipeline.md) (PL1, PL2, PL11, PL12, PL16); [invalidation](invalidation.md) (IV1–IV4, IV9, IV10, IV14); [input and accessibility](input-and-accessibility.md) (IN2, IN7, IN8, IN10); the [frame tree](frame-tree-and-navigation.md) (FT3, FT7, FT11); the [event loop](event-loop-and-threading.md) (EL9–EL11, EL14–EL17, EL24); [IPC](ipc.md) (IP14, IP16); [text and fonts](text-and-fonts.md) (TX1, TX3); [networking](networking.md) (NW12, NW14); [accessibility](accessibility.md) (AX1–AX3); the [dependency policy](../technology/dependency-policy.md) (DP1) |
| **Last updated** | 2026-10-19 |

Everything here is **planned**; nothing is implemented. Rules are numbered HD1–HD33. Precedent is sourced (§3); the design is inference unless marked otherwise.

## Summary

- **The HTML parser is Axiom's own** (DP1 (a)): the HTML standard's tokenizer and tree builder, followed state by state, in safe Rust (HD1). It runs on the document's event-loop thread in time-budgeted tasks, so rendering and input get their turns between chunks (HD5). A preload scanner on the helper pool fetches resources speculatively while the parser waits for a script (HD6). Off-thread tree building, as Firefox does it, is left possible but not built.
- **No conversion to UTF-8.** Bytes stream from the network process (FT11), are sniffed (HTML §13.2.3) and decoded by TX3's decoders into 8-bit or 16-bit code units, which are the DOM's own string forms (PL11), so text and attribute values never pass through UTF-8 (HD2, HD3). html5ever, which keeps every string in UTF-8, is the differential reference, not a dependency.
- **Every node kind shares one node arena of columns** (SH8): five links, the owner document, kind and flags, a rare-data index and a generation. Kind-specific data lives in per-kind arenas (HD10). **Node slots never move:** a freed slot is reused with a new generation, so a node's handle and generation identify it for its life, as PL2 and AX2 need. That narrows SH10's compaction to the other arenas, **an amendment to ADR-0008 for the owner** (HD11).
- **One mutation path.** The DOM standard's algorithms (insert, remove, move, replace, adopt, attribute and data changes) call a fixed, ordered list of internal hooks: mutation records, live ranges, slots, custom element reactions, form owners, element insertion steps, style invalidation (IV3), box-tree dirtiness (IV4) and accessibility (AX3). No subsystem watches the DOM any other way (HD15).
- **Events:** listeners live in a side table. Dispatch builds the event path in one heap array and snapshots each target's listeners into another, so a dispatch roots two values instead of every listener, as S3 recommended (HD18). Non-passive wheel and touch listeners are counted for the hit-test regions the compositor routes by (HD20). Trusted events and user activation come only from routed input (HD19).
- **The rest of the DOM follows the specifications** in the same model: mutation observers (HD22), live ranges and selection (HD23), Shadow DOM with slots and declarative shadow roots (HD24), custom elements with scoped registries (HD25), forms whose submission is a navigation request to the browser (HD26), and editing through Input Events and the IME mirror (HD28). XML documents get an XML parser of Axiom's own (HD9).
- **The DOM's JavaScript facet is an interface** that CP17 closes: a node's handle is its platform object (SH1), its prototype comes from its realm, and expando properties and `[SameObject]` values live in its rare data (HD30).
- **Oracles:** html5lib-tests; WPT's `html/syntax`, `dom`, `shadow-dom`, `custom-elements`, `html/semantics/forms`, `editing` and `selection` directories; html5ever for differential fuzzing; and IV14's from-scratch comparison (HD33).

## 1. Binding inputs

| Input | Consequence here |
|---|---|
| **DP1 (a):** parsing HTML and CSS, and the DOM, are Axiom's own | HD1, HD9; §4 |
| **SH1:** DOM objects are JavaScript objects: a node is its own platform object, in a typed arena, with no wrapper | HD10, HD30 |
| **SH6, SH9, SH11:** native code roots what it holds across a possible collection; fields that hold handles are written only through generated setters; memory that reaches platform APIs lives outside the heap | HD7, HD14, HD18 |
| **SH8:** hot links as one-based `u32` columns in a struct-of-arrays layout, which S3 measured fastest | HD10 |
| **SH10:** version 0 of the collector is stop-the-world mark–compact | HD11 (an amendment) |
| **PL1, PL2:** stages after the DOM refer to nodes by plain index; DOM arenas hold no interior mutability and stay `Sync`; data that style and layout write goes in side tables; node identity is the handle plus a generation, stable for the node's life | HD11, HD14 |
| **PL11:** one string representation, Latin-1 or UTF-16 chosen per string; names as atoms, static and process-wide | HD2, HD3, HD10, HD12 |
| **PL12, AX1–AX3:** the accessibility tree is built from the DOM, and its node IDs go to the browser | HD11, HD15 |
| **PL16, IV9:** hit-test regions say where listeners may block scrolling | HD20 |
| **IV1–IV4, IV10, IV14:** a compiler-checked lifecycle; pending, selector-driven style invalidation; box rebuilds only where needed; the parser yields between chunks; incremental output equals a rebuild | HD5, HD15, HD33 |
| **IN2, IN7, IN8, IN10:** the renderer dispatches routed input; an IME mirror in the browser; user activation from routed input only; the browser mediates the pasteboard, menus and dragging | HD19, HD21, HD26–HD28 |
| **FT3, FT7, FT11:** frame tokens issued by the browser; navigation driven by the browser; loader endpoints bound at commit, with bodies streamed through a ring | HD2, HD6, HD26 |
| **EL9–EL11, EL14–EL17:** the helper pool never touches a heap except arenas lent for a frozen stage; heaps never cross threads; the event loop owns the task queues; no nested event loops | HD5–HD7, HD31 |
| **TX3:** the Encoding Standard's decoders and encoders are Axiom's own; HTML's sniffing is CP13's | HD3 |
| **NW12, NW14:** ORB runs in the network process before any byte reaches a renderer; bodies stream with backpressure; sniffing for rendering happens in the renderer | HD2 |
| **S3 results §9, item 6:** code that roots many values at once, like event dispatch's listener snapshot, can root one heap array | HD18 |

## 2. Specifications

- **WHATWG HTML,** last updated 2026-10-16 [H1]:
  - parsing (§13.2): the input byte stream and encoding sniffing (§13.2.3), tokenization (§13.2.5, 84 states), tree construction (§13.2.6) and speculative parsing (§13.2.8); serializing and parsing fragments (§13.3, §13.4);
  - **processing instructions are now parsed into nodes** (§13.2.5.72–76), except the targets `xml` and `xml-stylesheet`; html5lib-tests added their tokenizer tests on 2026-10-01 [T1];
  - dynamic markup insertion (§8.4: `document.open`, `write`); the DOM parsing and insertion methods (§8.5): `setHTML`, `appendHTML`, `prependHTML`, `beforeHTML`, `afterHTML`, `replaceWithHTML` and their `Unsafe` forms, `getHTML` and `Document.parseHTML`, which **sanitize while parsing**, through the `Sanitizer` of §8.6. The Sanitizer API's own draft now redirects to HTML (its repository, 2026-08-31);
  - custom elements (§4.13), including scoped registries (§4.13.1.7); forms (§4.10); user activation (§6.4) and editing (§6.8); the XML syntax and parsing XML documents (§14);
  - the named character references: 2,231 names, 2,125 of them ending in a semicolon (observed, `entities.json`).
- **WHATWG DOM,** last updated 2026-10-16 [D1]: events (§2), with the "default passive value" for touch and wheel listeners; aborting (§3); nodes (§4), with mutation observers (§4.3), `ShadowRoot` (§4.8) and the atomic `moveBefore()`; ranges (§5); traversal (§6); `DOMTokenList` (§7); XPath (§8) and `XSLTProcessor` (§9).
- **WHATWG Encoding,** last updated 2026-05-21 [E1].
- **W3C:** XML 1.0, Fifth Edition (2008-11-26) [X1], and Namespaces in XML 1.0, Third Edition (2009-12-08) [X2]; UI Events and Pointer Events, for event types.
- **The Web Editing Working Group's drafts:** Selection API [W1], Input Events Level 2 [W2], EditContext [W3], and execCommand [W4].
- **Approximate behaviour, kept apart:** `execCommand`'s draft says it "is incomplete and it is not expected that it will advance beyond draft status", and that its features "are not implemented consistently or fully by user agents" [W4]. Where engines agree, Axiom follows them; elsewhere it follows the draft, and such behaviour is marked approximate in the code and the tests. Everything else in this document is specified precisely enough to implement as written.

## 3. Precedent

Read on 2026-10-19; dates in brackets are the last commits to each file, from GitHub's API.

| Engine | Parsing | The DOM |
|---|---|---|
| **Chromium** (Blink) | **Parses on the main thread,** in chunks under time budgets: 10 ms for the first six yields on desktop (two on Android), then 500 ms (50 ms on Android). The constants "were chosen using experiment data from the field". It also yields for high-priority work, and doesn't yield at all before leaving the document's head. **A preload scanner runs on a background thread** (`kThreadedPreloadScanner`, main frames only by default), and another background scanner precompiles inline scripts (`html_document_parser.cc` [2026-10-17], `background_html_scanner.h` [2026-08-25]) [B1] | Nodes keep a parent, a previous and a next sibling; containers keep only a first child, because the previous-sibling link is circular, so the first child's points to the last (`node.h` [2026-10-07], `container_node.h` [2026-10-16]). XML documents are parsed with libxml2 (`xml_document_parser.cc` [2026-10-05]) [B1] |
| **Firefox** (Gecko) | **Tokenizes and builds the tree on a parser thread,** producing tree operations that the main thread executes. It parses speculatively past scripts that block, and stops speculating after 100 failures in a document (`nsHtml5StreamParser.h` [2026-06-05], `nsHtml5TreeOpExecutor.h` [2026-08-25]). The C++ is translated from Java sources kept in the repository (`parser/html/javasrc/`) [F1] | `nsINode` holds a parent, a first child, a next sibling, and a weak previous-sibling link; the first child's points to the last child. The selector flags that style matching writes sit on the node, in a `RustCell` (`nsINode.h` [2026-09-23]). XML goes through expat, run inside an RLBox sandbox (`nsExpatDriver.cpp` [2026-04-26]) [F1] |
| **WebKit** | **Parses on the main thread,** yielding after 500 ms by default, and before a script once the parsing session has run 16 ms and processed 256 tokens; before the first paint, it yields before scripts whenever there is visible content (`HTMLParserScheduler.cpp` [2026-10-13]) [K1] | Nodes keep a parent, both siblings and their tree scope; containers keep a first and a last child, at offsets exposed to the JIT (`Node.h`, `ContainerNode.h` [2026-10-17]). XML documents are parsed with libxml2 (`XMLDocumentParserLibxml2.cpp` [2026-10-14]) [K1] |
| **Servo** | **html5ever:** "passes all tokenizer tests from html5lib-tests, with most tree builder tests outside of the unimplemented features"; it "exclusively uses UTF-8 to represent strings" and reports to the DOM through callbacks (`README.md` [2025-07-02]). **An optional mode runs it on a parser thread** and sends parse operations over a channel to the script thread (`async_html.rs` [2026-09-15]), behind the `dom_servoparser_async_html_tokenizer_enabled` preference (`servoparser/mod.rs` [2026-10-12]). XML goes through xml5ever (`xml.rs` [2026-01-22]) [S1] | `Node` holds five links as `MutNullableDom` cells, rare data and layout data behind `DomRefCell`, and flags in a `Cell`: interior mutability throughout (`node/node.rs` [2026-10-17]) [S1] |
| **Ladybird** | Its own tokenizer and tree builder (`HTML/Parser/`) [L1]. XML goes through libxml2, behind an adapter (`LibXML/Parser/Parser.cpp` [2026-09-17]) [L1] | `TreeNode` holds five links (`TreeNode.h` [2026-08-25]). **Text is UTF-16:** `CharacterData` stores a `Utf16String` (`CharacterData.h` [2026-10-05]) [L1] |

**Observations** (inference):
- **Every engine but Firefox builds the tree on the thread that runs script, by default,** and Firefox pays for its parser thread with a speculation mechanism and a cap on failures. Blink keeps the parser on the main thread and moves only the scanning off it. Its yield budgets come from field data, which Axiom can start from.
- **Link layouts differ in one field:** whether the last child is stored or found through a circular link. Axiom's columns are cheap enough to keep both (HD10).
- **Of the five engines, only Servo parses XML in a memory-safe language;** Blink, WebKit and Ladybird use libxml2, and Firefox sandboxed its C parser a second time inside the renderer. Axiom's rule 5 asks for its own (HD9).
- **Servo's DOM shows what a shared, garbage-collected DOM costs in Rust without Pattern B:** every field is a cell. Axiom's arenas take the heap's mutable borrow instead (PL2), which is what makes them lendable to style and layout.

## 4. Build or reuse

| Need | Candidates | Decision |
|---|---|---|
| **HTML tokenizer and tree builder** | **html5ever** (Servo's), in Rust, which keeps strings in UTF-8; **our own** | **Our own** (DP1 (a)), producing PL11's strings directly (HD1–HD3). html5ever is the differential reference |
| **Encoding detection and decoders** | TX3's decoders, Axiom's own | **TX3's,** with the sniffing algorithm here (HD3) |
| **Named character references** | The standard's table (`entities.json`) | **Data, compiled in at build time** into a trie (HD1) |
| **XML parser** | **libxml2** (C), as in Blink, WebKit and Ladybird; **expat** (C), sandboxed with RLBox in Firefox; **xml5ever** (Rust, Servo's); **our own** | **Our own** (DP1 (a); rule 5): a non-validating parser for XML 1.0 and its namespaces, which is far smaller than HTML's (HD9). xml5ever is a differential reference |
| **Sanitizer** | The standard's algorithm (HTML §8.6) | **Our own,** inside the tree builder (HD8) |
| **Atom tables** | PL11's static atoms, generated at build time | **Generated** from the specifications' name lists: HTML, SVG and MathML elements and attributes, and event types (HD10) |

No new dependency is proposed.

## 5. Design

### 5.1 Parsing

| Rule | Decision |
|---|---|
| HD1 | **The parser is Axiom's own,** a direct implementation of HTML §13.2: the tokenizer's 84 states, including CDATA sections in foreign content and processing instructions, and the tree builder's insertion modes, written as explicit state machines in safe Rust (rule 5). Parse errors are recorded for developer tools and tests, never shown to users. The 2,231 named character references are compiled into a trie at build time. **The oracles** are html5lib-tests (`tokenizer/` and `tree-construction/`) [T1] and WPT's `html/syntax` [T2]. html5ever is the differential reference wherever it implements a feature |
| HD2 | **Input arrives as bytes from the loader ring** (FT11, IP14). ORB has already run in the network process (NW12). While the parser is paused, the renderer stops reading the ring past a bounded read-ahead, so the network process's backpressure applies (NW14, IP16). Step 6 sets the bound: it trades how far the preload scanner can see (HD6) against buffered memory |
| HD3 | **Encoding, as HTML §13.2.3 decides it:** a byte-order mark; the transport layer's `charset`; a prescan of the first 1,024 bytes for `<meta charset>`, waiting for at most those bytes or 500 ms, as the standard suggests; for a same-origin frame, its container document's encoding; then the default for the user's locale. The standard's optional steps (a user's override, the encoding seen on a previous visit, statistical detection) are staged. **A late `<meta charset>`** (§13.2.3.4) changes the decoder on the fly when the bytes decoded so far decode identically, as the standard allows. Otherwise the navigation restarts with the new encoding, re-parsing the bytes already received rather than fetching again, as the standard asks: the renderer asks the browser to restart it (FT7). Where re-fetching can't be avoided and would repeat a request other than `GET`, the new encoding is ignored, as specified. **Decoding** uses TX3's decoders, writing PL11's forms directly (TX4): 8-bit chunks when every code point fits in a byte, 16-bit chunks otherwise. The tokenizer creates text and attribute values directly as PL11 strings, choosing the 8-bit form when every code unit fits (tracked while building, so nothing is scanned twice). windows-1252 maps bytes 0x80–0x9F above U+00FF, so its output isn't always 8-bit (background) |
| HD4 | **The input stream is a list of chunks with an insertion point,** which `document.write` inserts into (HTML §8.4). Its counters (script nesting level, throw-on-dynamic-markup-insertion, ignore-destructive-writes) and its implicit `document.open` behave as specified. Strings from script may contain lone surrogates; they are kept, after the parse error the standard names, because PL11 keeps them representable |
| HD5 | **The parser runs on the document's event-loop thread, in budgeted tasks.** A parse task runs until its time budget ends, input is waiting (EL16 puts discrete input first), or the buffered input runs out. Rendering opportunities fall between tasks (IV10). Blink's measured budgets are the starting point: 10 ms per task, longer after the first yields; Step 6 sets Axiom's on the corpus. **Blocking follows the standard:** a parser-blocking script pauses the tree builder; render-blocking stylesheets and `blocking="render"` hold rendering, not parsing. Algorithms that "spin the event loop" are resumable parser states (EL17).<br>**Off-thread tree building is left possible, not built.** The tree builder reports through a sink interface of tree operations, so it could later run on a parser thread with its own node IDs, as in Firefox and Servo's optional mode. It isn't built now, because the tree builder needs synchronous answers from the DOM (custom element definitions, `document.write`, the current node in foreign content), and a parser thread needs speculation that can fail (§3). Step 6 profiling decides (hypothesis: a parse costs well under a frame per chunk at budgeted sizes) |
| HD6 | **Speculative fetching** (HTML §13.2.8) is a preload scanner on the helper pool (EL9). It tokenizes a copy of the decoded input ahead of the parser, with just enough simulated tree-builder state to find `<base>`, scripts, stylesheets, preloads, module preloads, images with `srcset` and `sizes` (the viewport is known from the browser), and `<meta>` policies that constrain fetching. It requests each resource through the document's loader endpoint (FT11), with its fetch priority. It never touches the heap: its input is an owned copy and its output is requests. As the standard requires, it fetches nothing the real parser wouldn't, assuming the blocking script does nothing. The standard leaves duplicate fetches to caching rules it doesn't yet fully specify; the renderer's memory cache answers the parser's later requests for the same resources |
| HD7 | **Tree construction** holds the stack of open elements, the list of active formatting elements, the head and form pointers and the template insertion modes as node handles. The parser is an object in the document's heap, traced from the document, so these stay rooted without root-stack traffic (SH6). Element names and attribute names resolve to static atoms through a perfect hash generated at build time, with SVG's and MathML's case adjustments applied as §13.2.6.5 lists them. **Re-entry happens only where the standard allows it:** outside fragment parsing, creating an element that has a custom element definition runs its constructor synchronously (raising the throw-on-dynamic-markup-insertion counter and checkpointing microtasks first, §13.2.6.1); a script end tag runs the script; `document.write` re-enters the tokenizer at the insertion point. Templates parse into their inert contents document, and `<template shadowrootmode>` attaches a declarative shadow root (§13.2.6.4). **Text:** character tokens collect in a buffer that becomes, or extends, the text node when a non-character token arrives or the chunk ends, so a long text body is copied at most once per chunk (HD13) |
| HD8 | **One tree builder parses fragments** (§13.4), given a context element: `innerHTML`, `outerHTML`, `insertAdjacentHTML`, `createContextualFragment`, `DOMParser`, and the insertion methods of §8.5. **Sanitization happens during parsing,** in the tree builder's insertion steps, where the standard places it (§8.6), so unsafe content is never inserted rather than removed afterwards. Because the methods turn a string into nodes in one step, with the context they imply, the standard notes that they aren't directly affected by mutation XSS, which exploits serializing and parsing again; a sanitized tree that is serialized is no longer sanitized (§8.6.6.4). Serialization (§13.3, `getHTML`) is Axiom's own |
| HD9 | **XML documents get an XML parser of Axiom's own:** non-validating, XML 1.0 with namespaces [X1, X2], for XML media types (XHTML and SVG documents), for `DOMParser` with XML types, and for `XMLHttpRequest`'s `responseXML`. **No external entity is fetched.** HTML §14.2 lists XHTML's and MathML's public identifiers, which should resolve to a DTD declaring HTML's named character references, and says user agents "should not attempt to retrieve any other external entity's content". Axiom compiles that DTD's entities in, so even the listed ones need no network access. Internal entity expansion is bounded, against exponential expansion. The oracle is WPT's XML tests, with xml5ever as a differential reference. **XPath** (DOM §8) is staged. **XSLT** (DOM §9) is an open question (§10) |

### 5.2 Nodes in the heap

| Rule | Decision |
|---|---|
| HD10 | **One node arena, stored as columns** (SH8), one-based `u32` per field:<br>• the five links: parent, first child, last child, next sibling, previous sibling;<br>• the owner document;<br>• the kind and flags: node type, namespace class (HTML, SVG, MathML, other), connected, in a shadow tree, has rare data, custom element state, and similar bits the DOM writes;<br>• the rare-data index, 0 when there is none;<br>• the generation (HD11).<br>That is 36 bytes a node. **Kind-specific data** lives in per-kind arenas, indexed from the node: elements (local name atom, prefix, attribute storage, ID atom, class atoms, inline style declarations for CP14), character data (the string handle; a processing instruction's target atom), documents (URL, mode, encoding, readiness, the active parser), document types, fragments, shadow roots (mode, `delegatesFocus`, slot assignment, `clonable`, `serializable`, host) and `Attr` nodes (HD12). **Rare data** holds what most nodes lack: mutation observer registrations, a host's shadow root, custom element definition and registry, element internals, listeners (HD17), cached `[SameObject]` objects and expando properties (HD30). Atoms are PL11's: static ones generated at build time, dynamic ones in the process-wide table |
| HD11 | **Node slots never move.** A freed slot goes on a free list, and its generation is incremented. Inside the heap, handles are Pattern B's `u32` handles, kept alive by the collector. **Identities that leave the heap are pairs of index and generation:** style and layout side tables (PL1), the accessibility tree's IDs in the browser (AX2), hit-test results, and messages. Each is checked on use, so a stale one is detected rather than misread. Nothing outside the DOM ever has to be told that a node moved.<br>**This narrows SH10:** the mark–compact collector compacts the other arenas, but not the node arena. **Proposed as an amendment to ADR-0008, for the owner at Gate C.** The evidence: PL2 and PL12 make node identity stable for the node's life, and AX2 sends it to the browser, so moving a node would invalidate identities held outside the heap. **The cost is fragmentation after churn.** Freed slots are reused last in, first out. Step 6 measures the arena's density on the corpus; if it degrades, compacting one document's nodes while remapping its side tables is the fallback (hypothesis) |
| HD12 | **Attributes without `Attr` nodes.** An element stores its attributes as a short vector of (qualified name atoms, value string): up to two inline, more in an attribute arena. `Attr` nodes exist only once script asks for them (`getAttributeNode`, the `attributes` map). They are cached in rare data and kept consistent by the standard's "change an attribute" steps. `id` and `class` are parsed into atoms when set, for selector matching (CP14); `style` is parsed when first needed (CP14) |
| HD13 | **Character data is an immutable string** (PL11). Each change (`appendData`, `insertData`, a `data` write) installs a new string. The parser coalesces text before creating a node (HD7), so pages don't pay for that. If script that appends text repeatedly turns out to matter, the JavaScript engine's ropes (CP16) can back it (hypothesis, measured at that milestone) |
| HD14 | **No interior mutability; one way to write.** Every DOM write takes the heap's mutable borrow (PL2), and a compile-time assertion keeps the DOM arenas `Sync`, so they can be lent read-only to style and layout (EL11). Columns and fields that hold handles (links, rare-data references, attribute values) are written only through the generated setters (SH9), which are where a collector's barrier will go. State that style, layout and accessibility write lives in their side tables, indexed by node (PL2) |
| HD15 | **The mutation algorithms, and their hooks.** The DOM standard's insert, remove, `moveBefore`, replace-all, adopt, attribute changes, data replacement and text splitting are implemented as written. Each calls **a fixed, ordered list of internal hooks** at the points the standards define:<br>• mutation records (HD22);<br>• live ranges and node iterators (HD23);<br>• slot assignment (HD24);<br>• custom element reactions (HD25);<br>• form owners and controls (HD26);<br>• the insertion, removal and post-connection steps of `script`, `img`, `link`, `style`, `iframe`, `template` and other elements;<br>• style invalidation sets (IV3), box-tree dirtiness (IV4) and accessibility updates (AX3);<br>• the tree version (HD16).<br>The list is fixed at compile time, so the order is deterministic and visible in one place. Script runs inside a mutation only where the standards allow it: reactions are queued, and post-connection steps run after the insertion completes |
| HD16 | **Live collections cache against a tree version.** `HTMLCollection`, live `NodeList`s and similar objects cache their length and their last position, keyed by the document's tree version, a counter that every tree mutation and every relevant attribute change increments. `childNodes[i]` and `children` walk from the cached position. Caches live in the collection objects, so the nodes carry nothing for them |

### 5.3 Events

| Rule | Decision |
|---|---|
| HD17 | **Listeners live in a side table,** reached through rare data, since most targets have none. Each entry maps an event type atom to an ordered list of listeners: callback, capture, once, passive and signal (DOM §2.7). An `AbortSignal` removes its listeners (§3). Event handler attributes such as `onclick` hold their place in the list, as HTML specifies, and are compiled when first needed (CP16, CP17) |
| HD18 | **Dispatch follows DOM §2.9,** including shadow-including retargeting, `relatedTarget` and touch-target retargeting, `composedPath`, legacy pre-activation and activation behaviour, and the canceled flag. **The path is built into one heap array, and each target's listeners are snapshotted into another,** so a dispatch roots two values on the root stack, not one per listener or target. That is S3's recommendation (results §9, item 6), which removes most of the root-stack pushes left in its dispatch case |
| HD19 | **Trust comes from the user agent.** `isTrusted` is true only for events the user agent creates: input routed by the browser (IN2), and the user agent's own events. Events from `dispatchEvent` are untrusted, and run activation behaviour only where the standards say so. **User activation** is the renderer's copy of the browser's record (IN8). The renderer can read it, and consumes it only through the browser |
| HD20 | **Listeners that can block scrolling are counted.** Non-passive `wheel`, `mousewheel`, `touchstart` and `touchmove` listeners are counted per node. The DOM standard's default passive value makes them passive on the window, the document, the root element and the body unless declared otherwise (§2.7). Paint turns the counts into hit-test region flags (CP15, PL16), so the compositor waits for the renderer only where such a listener exists (IV9) |
| HD21 | **Input becomes tasks.** Each event the compositor routes is a task in the user-interaction task source (EL16). It is dispatched to the node found by hit testing, then acknowledged (IN2). Continuous events such as pointer and mouse moves are coalesced and delivered once per rendering opportunity, with `getCoalescedEvents()` returning the merged samples |

### 5.4 Observers, ranges, Shadow DOM and custom elements

| Rule | Decision |
|---|---|
| HD22 | **Mutation observers,** as DOM §4.3 specifies: registrations in rare data, including the transient registrations that follow a removed subtree; records queued per observer by HD15's hook; delivery in the "notify mutation observers" microtask, which runs at the microtask checkpoint (SH4) |
| HD23 | **Live ranges and iterators.** Each document keeps its live ranges and its `NodeIterator`s in side lists, which HD15's hook updates by the standard's steps (DOM §5–§6). `StaticRange` is never updated. **Selection** (Selection API [W1]): one per document, holding a live range and a direction. It is painted by CP15, read by accessibility (AX5), and copied into the IME mirror while editing (IN7) |
| HD24 | **Shadow DOM and slots.** Shadow roots carry their mode, `delegatesFocus`, slot assignment (named or manual), `clonable` and `serializable` (DOM §4.8). Slot assignment runs when HD15's hook says a slot or slottable changed, and "signal a slot change" queues `slotchange` as a microtask. Declarative shadow roots come from the parser (HD7). Style scoping is CP14's; the flat tree, CP15's |
| HD25 | **Custom elements** (HTML §4.13). Registries are global or scoped (§4.13.1.7); a node's scoped registry sits in its rare data. The custom element reactions stack belongs to the agent, with its backup element queue. Upgrades, the element states (undefined, failed, uncustomized, precustomized, custom), customized built-in elements, and form-associated custom elements with `ElementInternals` are implemented as specified. Invoking constructors and callbacks, and `[CEReactions]`, are CP17's |

### 5.5 Forms and editing

| Rule | Decision |
|---|---|
| HD26 | **Forms** (HTML §4.10): form owners, the `form` attribute, value modes and dirty flags, selection in text controls, constraint validation, and the entry list. **Submission is a navigation request:** the renderer builds the request, encodes its body with the form's encoding (TX3's encoders), and sends it to the browser, which navigates (FT7); a large body travels out of line (IP14). **Files:** a file chooser is browser interface (IN10). The renderer receives file contents only for files the user picked, through the browser, never paths it could reuse (SG1). **Autofill and saved passwords** are browser features, staged: the browser fills a form on the user's action, and offers credentials only for the origin in its own records for that frame (FT2), so a compromised renderer can't collect another site's |
| HD27 | **Form controls are drawn by Axiom,** not as native widgets in content, with their appearance controlled by CSS (`appearance`, CP14). **Popups:** a `<select>` list, a date picker or a colour picker is requested from the browser, which shows native interface and returns the choice (IN10's pattern); a `<select>` rendered with base appearance opens its picker as a popover, in its own document's top layer, as HTML §4.10.7 specifies |
| HD28 | **Editing** (HTML §6.8). `contenteditable` and `designMode` mark editing hosts. Text input arrives from the IME through the browser's mirror (IN7). Each edit fires `beforeinput`, with its input type and target ranges (Input Events [W2]), then applies the change, then fires `input`. **`execCommand` is approximate** (§2): it follows the engines where they agree, and its draft otherwise. Spelling and grammar checking need AppKit, so they are browser features: the renderer sends text runs of the focused editable element, and the browser returns marked ranges (staged). **EditContext** [W3], which hands text input to script without DOM editing, maps onto the same mirror (staged) |
| HD29 | **The top layer and inertness** (`dialog`, popovers, fullscreen, `inert`) are DOM state here: membership and order of the top layer, the modal dialog's inert subtree, and focus changes. Rendering them is CP15's, and fullscreen is a browser decision, granted only with user activation (IN8) |

### 5.6 The JavaScript facet

| Rule | Decision |
|---|---|
| HD30 | **An interface that CP17 closes.** A node's handle is its platform object (SH1): JavaScript values carry node handles directly, with no wrapper. The bindings get, per node:<br>• its interface, from the kind column and the element's local name and namespace (for brand checks);<br>• its prototype, from its realm, which its document records;<br>• **expando properties,** in a property table created in rare data on the first write;<br>• `[SameObject]` values (`classList`, `dataset`, `style`, `attributes`, `childNodes`), cached in rare data;<br>• its custom element state and definition (HD25).<br>Objects that aren't nodes (`Event`, `Range`, `MutationObserver`, collections) are ordinary platform objects in their own arenas |

### 5.7 Threads, memory, determinism and tests

| Rule | Decision |
|---|---|
| HD31 | **Threads.** Everything here runs on the document's event-loop thread, except the preload scanner, which works on owned copies on the helper pool (HD6). Style and layout may read the DOM arenas while the event loop lends them for a frozen stage (EL11) |
| HD32 | **Memory has owners.** Nodes belong to their agent's heap, and are collected when unreachable; a document's own side lists (ranges, iterators, the preload map) go with it. **Per node** (inference from HD10–HD13): 36 bytes of columns; about 24 more for an element's data, plus its attributes (8 bytes each, plus their value strings); 4 more for character data, plus its string. S1 measured 236 bytes per live node for Pattern B, everything included, on its synthetic DOM ([S1 results](../research/s1-results.md)). Axiom's target is to stay at or below that, which makes a 10,000-node page's DOM about 2.4 MB, well inside B3's 50 MB per renderer. Step 6 measures the real figure on the corpus |
| HD33 | **Deterministic, and tested at boundaries.** Parsing is deterministic under the test scheduler (EL24), and **its output must not depend on how the input is split:** the tests feed every document in randomly sized chunks, and compare with a single-chunk parse. **Oracles:**<br>• html5lib-tests' tokenizer and tree-construction suites [T1];<br>• WPT's `html/syntax`, `dom`, `shadow-dom`, `custom-elements`, `html/semantics/forms`, `editing` and `selection` directories [T2];<br>• differential fuzzing of the HTML parser against html5ever, and of the XML parser against xml5ever (Step 6);<br>• IV14's comparison of every incremental update with a rebuild, which covers HD15's hooks;<br>• tests that the node arena's identities stay stable across collections, and that stale identities are detected (HD11) |

## 6. Interfaces with neighbouring subsystems

| Neighbour | Interface |
|---|---|
| **Networking** (NW12, NW14; FT11) | Document bytes and subresources through the loader endpoint, streamed through a ring with backpressure; speculative fetches from the preload scanner (HD2, HD6) |
| **Text** (TX1, TX3) | Decoders for parsing and encoders for form submission (HD3, HD26); ICU4X for case-insensitive comparisons |
| **Style** (CP14) | Element names, IDs and classes as atoms; HD15's invalidation hook; computed styles in side tables (PL2); shadow trees for scoping |
| **Layout and paint** (CP15) | Boxes built from the DOM and its flat tree; listener counts for hit-test regions (HD20); selection and the top layer for painting |
| **Script and bindings** (CP16, CP17) | Script elements' processing; microtasks for observers and reactions; the JavaScript facet (HD30); event handler compilation |
| **Accessibility** (AX1–AX3) | Built from the DOM after each rendering update, keyed by stable identities (HD11) |
| **Input and the frame tree** (IN2, IN7, IN8, IN10; FT3, FT7) | Routed events become tasks; the IME mirror; user activation; browser popups; `iframe` elements hold their child frame's token; navigations and form submissions go to the browser |
| **Images and media** (IM9) | `img`, `picture` and media elements request decoding through IM9's interface, from their insertion steps (HD15) |

## 7. Security

- **Rule of 2:** every parser here (HTML, XML, sanitizer, character references, encodings) handles untrusted input inside the sandboxed renderer, as safe Rust with no `unsafe` (rule 5, SC2, SC3). No C parser sees web content in the renderer, unlike libxml2 in Blink, WebKit and Ladybird, and expat in Firefox (§3).
- **Sanitization during parsing** removes unsafe content before insertion, as the standard specifies, so re-parsing can't revive it (HD8).
- **Trust and activation** come from the browser's records (HD19), and every navigation, form submission and file comes through the browser (HD26).
- **Stale identities are detected,** not misread, by their generation (HD11). That matters because side tables and the browser act on node identities.
- **Trusted Types,** which guard the DOM's injection sinks (`innerHTML`, script URLs), hook into HD8's insertion methods and HD7's script handling; they are staged with CSP (CP16, CP17).

## 8. Staging and the complete model

| Stage | What exists |
|---|---|
| First document milestone | The HTML parser with encoding sniffing, `document.write` and fragments; the node arena, attributes, text and the mutation algorithms with their hooks; events and dispatch; mutation observers; forms without autofill; ranges and selection |
| Next | Shadow DOM, slots and declarative shadow roots; custom elements with scoped registries; the preload scanner; the sanitizer and the new insertion methods; the XML parser |
| Later | Editing with `beforeinput`, `execCommand` and EditContext; spelling; autofill and saved passwords; XPath; the top layer's full behaviour |

**The complete model** (SC9): every element and API in the HTML and DOM standards fits these rules without architectural change. New element kinds add per-kind arenas or rare-data fields; new mutation-driven behaviour adds a hook to HD15's list; new parsing features (processing instructions are the latest) extend HD1's state machines. Off-thread parsing stays possible behind HD5's sink interface.

## 9. Alternatives considered

| | Alternative | Assessment |
|---|---|---|
| A | **html5ever** as the parser | Mature and in Rust, but DP1 (a) makes parsing core, and its UTF-8 strings would mean converting every text node and attribute value into PL11's forms. Rejected; kept as the differential reference |
| B | **A parser thread** that tokenizes and builds tree operations, as Firefox does | It frees the event loop during large parses, but needs speculation past scripts, with failures, and synchronous answers from the DOM. Left possible behind HD5's sink; Step 6 decides |
| C | **A compacting node arena,** with side tables and the browser told when nodes move | Denser after churn, but every identity outside the heap would need remapping, and AX2's IDs in the browser would change under it. Rejected (HD11) |
| D | **Nodes as individual heap objects with pointer fields,** like other engines' nodes | Against SH8's measured columns and PL2's lending. Rejected |
| E | **libxml2 or expat** for XML | C parsers on web content in the renderer. Firefox runs expat inside an RLBox sandbox, within its already sandboxed content process (§3). Rejected (rule 5) |
| F | **Native widgets** for form controls | They would put AppKit in the renderer, which the process model denies. Rejected (HD27) |

## 10. Open questions

| # | Question | Where it is settled |
|---|---|---|
| 1 | The amendment to ADR-0008: node arenas excluded from compaction (HD11) | The owner, at Gate C |
| 2 | Whether to support XSLT (DOM §9). It needs an XSLT 1.0 processor of Axiom's own, for a feature little used on modern pages (inference) | The owner, at Gate C; until then not planned |
| 3 | The parser's time budget and the read-ahead bound (HD2, HD5) | Step 6, on the corpus |
| 4 | Statistical encoding detection, for legacy pages without a declared encoding (HD3) | A later milestone, with the corpus |
| 5 | Whether DOM text needs ropes (HD13) | CP16, and measurement at the first document milestone |

## Sources

All checked 2026-10-19.
- [H1] WHATWG, *HTML Living Standard*, last updated 2026-10-16: parsing (§13.2–§13.4), dynamic markup insertion, parsing and insertion methods, and sanitization (§8.4–§8.6), custom elements (§4.13), forms (§4.10), user interaction (§6.4, §6.8), the XML syntax (§14) — https://html.spec.whatwg.org/multipage/ ; the named character references — https://html.spec.whatwg.org/entities.json ; the Sanitizer API's former draft, rewritten to redirect to HTML on 2026-08-31 — https://github.com/WICG/sanitizer-api
- [D1] WHATWG, *DOM Living Standard*, last updated 2026-10-16 — https://dom.spec.whatwg.org/
- [E1] WHATWG, *Encoding Standard*, last updated 2026-05-21 — https://encoding.spec.whatwg.org/
- [X1] W3C, *Extensible Markup Language (XML) 1.0 (Fifth Edition)*, Recommendation (2008-11-26) — https://www.w3.org/TR/xml/
- [X2] W3C, *Namespaces in XML 1.0 (Third Edition)*, Recommendation (2009-12-08) — https://www.w3.org/TR/xml-names/
- [W1] W3C Web Editing Working Group, *Selection API*, editor's draft [2026-06-11] — https://w3c.github.io/selection-api/
- [W2] W3C Web Editing Working Group, *Input Events Level 2*, editor's draft [2026-10-15] — https://w3c.github.io/input-events/
- [W3] W3C Web Editing Working Group, *EditContext API*, editor's draft [2026-09-10] — https://w3c.github.io/edit-context/
- [W4] W3C Web Editing Working Group, *execCommand*, draft [2026-10-06] — https://w3c.github.io/editing/docs/execCommand/
- [T1] html5lib-tests, at `c777c408b61078ea2eb4acefc2535f54dbc8b28a` (2026-10-01, "Add tokenizer tests for processing instructions") — https://github.com/html5lib/html5lib-tests
- [T2] web-platform-tests, `html/syntax` at `4aa56d3beafc5655fa55491169eb14845b44eab3` (2026-10-16) — https://github.com/web-platform-tests/wpt
- [B1] Chromium: `third_party/blink/renderer/core/html/parser/html_document_parser.cc`, `background_html_scanner.h`, `html_preload_scanner.h`; `core/dom/node.h`, `container_node.h`; `core/xml/parser/xml_document_parser.cc` — https://github.com/chromium/chromium
- [F1] Firefox: `parser/html/nsHtml5StreamParser.h`, `nsHtml5TreeOpExecutor.h`, `parser/html/javasrc/`; `dom/base/nsINode.h`; `parser/htmlparser/nsExpatDriver.cpp` — https://github.com/mozilla-firefox/firefox
- [K1] WebKit: `Source/WebCore/html/parser/HTMLParserScheduler.cpp`; `dom/Node.h`, `ContainerNode.h`; `xml/parser/XMLDocumentParserLibxml2.cpp` — https://github.com/WebKit/WebKit
- [S1] Servo: html5ever's `README.md` — https://github.com/servo/html5ever ; `components/script/dom/servoparser/async_html.rs`, `mod.rs`, `xml.rs`; `components/script/dom/node/node.rs` — https://github.com/servo/servo
- [L1] Ladybird: `Libraries/LibWeb/TreeNode.h`, `DOM/Node.h`, `DOM/CharacterData.h`, `HTML/Parser/`; `Libraries/LibXML/Parser/Parser.cpp` — https://github.com/LadybirdBrowser/ladybird
- This repository: [S1 results](../research/s1-results.md) (bytes per live node); [S3 results](../research/s3-results.md) §6 and §9 (link columns; rooting one array).
