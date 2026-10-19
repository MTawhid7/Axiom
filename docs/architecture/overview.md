# Architecture Overview

| | |
|---|---|
| **Status** | **Proposed: v1** (Step 5, CP18, 2026-10-19), refining v0 (Step 4, CP11). The owner accepted ADRs 0005–0007 and 0009 at Step 4's Gate B (2026-10-17) and ADR-0008 at its Gate C (2026-10-18); at Step 5's Gate B, ADRs 0010 and 0014 (2026-10-18), then ADR-0013 once spike S7 met its condition (2026-10-19). ADRs 0011, 0012 and 0015–0017 await Step 5's Gate C. The proposal as a whole is approved at the end of Phase Zero (Step 6) |
| **Question** | How do the parts fit together: processes × threads × pipeline; the crate structure, its `unsafe` map and its dependencies; what happens end to end in common scenarios; and how does the system grow from L0 to L3 without a rewrite? |
| **Answers** | Step 4 question Q16 ([Step 4 tracker](../research/step-4-architecture-proposal.md) §3) and the check of AC1–AC10; Step 5's CP18 ([Step 5 tracker](../research/step-5-subsystems.md) §6.1): the crates refined, a summary of the [register](../technology/dependency-register.md), scenarios traced through the subsystems' rules, and the check of SC1–SC10 |
| **Builds on** | Step 3: [threat model](../security/threat-model.md), [process model](../security/process-model.md), [ADR-0004](../adr/0004-process-model-and-sandbox.md). Step 4: [event loop and threading](event-loop-and-threading.md), [IPC](ipc.md), [frame tree and navigation](frame-tree-and-navigation.md), [rendering pipeline](rendering-pipeline.md), [invalidation](invalidation.md), [input and accessibility](input-and-accessibility.md), [embedder API](embedder-api.md), [script and heap](script-and-heap.md). Step 5: the [dependency policy](../technology/dependency-policy.md), [text and fonts](text-and-fonts.md), [graphics](graphics.md), [images and media](images-and-media.md), [networking](networking.md), [storage and workers](storage-and-workers.md), [accessibility](accessibility.md), [HTML and the DOM](html-and-dom.md), [style](style.md), [layout and paint](layout-and-paint.md), [the JavaScript engine](javascript-engine.md), [WebIDL bindings](bindings.md) |
| **Last updated** | 2026-10-19: v1, Step 5's subsystems assembled (CP18) |

Everything here is **planned**; nothing is implemented. This document adds no new rules; it assembles the rules of the documents above, which it cites: Step 4's EL, IP, FT, PL, IV, IN, EM and SH, and Step 5's DP, TX, GR, IM, NW, SW, AX, HD, ST, LY, JE and WB.

## 1. The system on one page

```
 Browser process (trusted, hardened, unsandboxed)
 ┌────────────────────────────────────────────────────────────────────────────┐
 │ main thread (AppKit)     core thread               compositor thread      │
 │ shell UI · NSEvents ──▶  frame tree · navigation   display links · frames │
 │ IME mirror · AX cache    session & visited history validate · Metal tiles │
 │                          process manager · policy  hit test · scroll      │
 │            ◀── embedder API (IDL messages) ──▶     ◀── frame offers ──▶   │
 └──────────────────────┬──────────────────────────────────┬──────────────────┘
          Mach IPC: browser-created channels (capabilities), audit-token pinned
 ┌──────────────────────┴───────────────────┐   ┌──────────┴───────────────────┐
 │ Renderer (sandboxed, site-locked from L2) │   │ Network process (sandboxed)  │
 │ event-loop thread per agent:              │◀─▶│ I/O thread: rustls, HTTP/1-3,│
 │   JS heap + DOM · parse · style · layout  │   │ quinn-proto, DNS-SD, ORB     │
 │   · paint → display list per frame        │   │ worker pool: SecTrust, disk  │
 │ helper pool: decode, glyph & path raster, │   │ storage thread: cookies,     │
 │   compile scripts and stylesheets         │   │ cache index, redb stores     │
 │ CoreText (system fonts) · safe decoders   │   └──────────────────────────────┘
 └───────────────────────────────────────────┘   later: GPU, media utility (AVIF)
```

| Layer | Holds | Rules |
|---|---|---|
| **Shell** (browser main thread) | Windows, tabs, menus, prompts; the mirrors that answer AppKit's synchronous IME and accessibility queries; the accessibility bridge | EM1, EM6, IN7, IN9, AX4–AX8 |
| **Core** (browser core thread) | The authoritative frame tree, navigation, session and visited history, process launch and site locks, policy, profile data | FT1–FT10, EM7, ST10 |
| **Compositor** (browser compositor thread) | Display links, frame offers, validation, Metal tiles in CoreAnimation layers, hit testing, scrolling, snapping, scrollbars and compositor animation | EL19–EL22, PL9, IV8, IN1, GR1–GR9, LY17, LY18 |
| **Renderers** | One thread and heap per agent; the HTML parser, the DOM, the JavaScript engine and its bindings, style, layout and paint to a display list per frame; text, image decoding and rasterization on the helper pool | EL15, SH1, PL1–PL7, HD, ST, LY, JE, WB, TX, IM1–IM10 |
| **Network process** | URLs, HTTP/1.1–3, TLS, DNS, cache, cookies and storage behind endpoints bound to each document's facts | FT11, EL8, NW1–NW18, SW1–SW13 |
| **IPC** | Mach channels created only by the browser; IDL-generated safe decoders; `Untrusted<T>` validation | IP1–IP22 |

## 2. Crate structure and the `unsafe` map

A proposal for the implementation milestones; final names and splits are set when code is authorized. Engine crates go under `crates/`, as the workspace manifest anticipates. Dependencies are the [register](../technology/dependency-register.md)'s (§3 below).

| Crate | Purpose | Process | Dependencies | `unsafe` |
|---|---|---|---|---|
| `platform` | All FFI: Mach IPC, kqueue and run loops, sandbox entry and profile compilation, `posix_spawn`, shared memory and IOSurfaces, QoS, wait-on-address; CoreText and CoreGraphics; Security's trust evaluation, DNS-SD, SystemConfiguration; Metal and CoreAnimation; ImageIO, where Gate C places it | All | `objc2` and its framework crates, `block2`, `dispatch2` | **Yes**, in listed modules (ADR-0002 §15.3), under stated invariants |
| `gc` | Pattern B heap: typed arenas, handles, scopes, `SliceStack`, the collector | Renderer | — | **None** (SH7) |
| `js` | The JavaScript engine: parser, compiled scripts, bytecode, interpreter, objects, strings, builtins, `RegExp`, `Intl`, Temporal (JE1–JE32) | Renderer | ICU4X (TX2) | No |
| `webidl`, `bindings` | The WebIDL compiler (a build tool) and its generated glue (WB1–WB23) | Build; renderer | — (webref's IDL as data) | No: generated code is safe (WB2) |
| `html`, `xml`, `dom` | The parsers and the DOM (HD1–HD33) | Renderer | — | No (rule 5) |
| `css`, `style` | The CSS parser, matching, the cascade and its caches (ST1–ST36) | Renderer | — | No |
| `layout`, `paint` | Boxes, fragments, property trees, display items, hit testing (LY1–LY32) | Renderer | A path rasterizer (GR5) | No |
| `text` | Encodings, segmentation and bidi, font matching, web fonts, shaping, glyph atlases (TX1–TX21) | Renderer | ICU4X, `unicode-bidi`, HarfRust, `read-fonts`, `skrifa`, `brotli-decompressor`, a glyph rasterizer | No |
| `image` | Image decoding behind IM9's interface, colour management (IM1–IM10) | Renderer | `png`, `gif`, `image-webp`, `jxl`, `jpeg-decoder`, `moxcms` | No |
| `jit` | Later, only if J1's evidence calls for it | `renderer-jit` | — | **Yes**, under J1–J7 |
| `shell` | The AppKit shell through objc2 (EM11), with the accessibility bridge (AX8) | Browser | `objc2` crates | **Only its `platform` module** |
| `ipc`, `idl` | Channels, the in-process transport, `Untrusted<T>`; the IPC generator | All; build | — | No |
| `event-loop` | Thread loops, the HTML event loop, the deterministic test scheduler | All | — | No |
| `core`, `frame-tree`, `embedder-api` | The browser's authority: records, navigation, history, policy, process manager; the API to the shell | Browser | — | No |
| `compositor`, `display-list` | Validation, layers, tiles and Metal pipelines; the display-list format shared with renderers | Browser (later GPU) | — | No (Metal via `platform`) |
| `net` | URLs, HTTP/1.1, HTTP/2, HTTP/3, WebSockets, cache, cookies, Fetch's policy, CORS and ORB (NW1–NW18) | Network | rustls with `aws-lc-rs`, `quinn-proto`, `miniz_oxide`, `brotli-decompressor`, `ruzstd` | No in Axiom's code; `aws-lc-sys` is C and assembly under DP10's exception |
| `storage` | Web storage, IndexedDB, the Cache API, quotas (SW1–SW13) | Network | `redb` (SQLite as the fallback) | No |
| `headless` | The headless embedder for tests and WPT (EM9) | Test | — | No |
| Executables | `axiom` (shell and browser), `axiom-renderer`, `axiom-network`; later `axiom-renderer-jit`, `axiom-gpu`, `axiom-utility` | — | — | — |

**Dependency rules:**
- engine crates never depend on `shell` (EM3);
- renderer crates depend on browser crates only through the protocol crates (`ipc`, `display-list`);
- `net` and `storage` never depend on renderer crates;
- only `platform`, `jit` and the shell's `platform` module may contain `unsafe` (AC9); a dependency's own `unsafe` and C are recorded in the register with their grades (DP8–DP10).

## 3. The dependency register, summarized

From the [register](../technology/dependency-register.md), as of 2026-10-19:

| Class (DP5) | Entries | Status |
|---|---|---|
| **Required platform facilities** | libSystem and libsandbox; AppKit, Foundation, QuartzCore, Metal, IOSurface, and AppKit's accessibility with HIServices' text markers (browser); CoreText and CoreGraphics (renderer); Security's trust evaluation, DNS-SD and SystemConfiguration (network process); ImageIO for AVIF, and VideoToolbox, AudioToolbox and CoreAudio (the media utility process) | Decided by ADRs 0004–0009, 0013 and 0014; CoreText and ImageIO with ADRs 0011 and 0012, at Gate C |
| **Foundational dependencies** | `objc2` and its framework crates; ICU4X; rustls | `objc2` decided (ADR-0009), rustls decided (ADR-0013); ICU4X with ADR-0011 |
| **Replaceable implementation details** | Text: HarfRust, `read-fonts`, `skrifa`, `unicode-bidi`, `brotli-decompressor`, a glyph rasterizer (`zeno` or `vello_cpu`). Graphics: a path rasterizer (S6). Images: `png` (with `flate2`), `gif`, `image-webp`, `jxl`, `jpeg-decoder`, `moxcms`. Network: `aws-lc-rs`, `quinn-proto`, `miniz_oxide`, `ruzstd`. Storage: `redb`, with SQLite as the fallback | Network and storage decided (ADR-0013); the rest with ADRs 0011 and 0012 |
| **Considered and not admitted** | `encoding_rs`, `zune-jpeg`, `httparse` (grades below DP8's bar); Network framework for TLS (S7-Q2); wgpu in the browser (GR1); AccessKit (AX8) | Recorded with reasons |

**Part 2 (CP13–CP17) adds no runtime dependency.** It names test references (`html5ever`, `xml5ever`, `regress`, `temporal_rs`, `num-bigint`, and Taffy as a candidate) and data (webref's IDL, html5lib-tests, css-parsing-tests, test262), each recorded in the register when a test or build first uses it (DP2). **Open risk:** `smallvec`, which HarfRust and ICU4X require, carries a published `ub-risk-3` grade on an old version; Axiom grades the version it adopts before admitting it (DP8).

## 4. Seven scenarios, end to end

**1. Opening a URL in a new tab** (FT7, FT10, NW, HD, ST, LY, JE, GR):
1. The shell sends a load command through the embedder API. The core creates the tab's records and a navigation. The network process resolves the host through DNS-SD (NW7), connects in the navigation's partition (NW8), runs TLS through rustls and has the worker pool verify the chain with Security (NW6), and fetches over HTTP/2 or HTTP/3 (NW4, NW5).
2. At the response, the core computes the origin, the site and the agent cluster key, and chooses or launches a renderer, which applies its browser-compiled profile (process model §8).
3. The core commits the document, sending its ID, origin, policy container, frame-tree skeleton, loader and storage endpoints, and the body's stream (FT11, NW14).
4. The renderer starts an event-loop thread with a new heap. It sniffs and decodes the bytes straight into the DOM's strings (HD3) and parses in budgeted tasks (HD5), while the preload scanner fetches stylesheets, scripts and images (HD6). Stylesheets parse on the helper pool (ST33); scripts compile there into heap-free compiled scripts (JE3) and run on the event loop's thread (JE6).
5. At the next vsync the compositor offers a frame. The renderer runs style (ST12–ST17), builds boxes (LY3) and lays them out, shaping text through HarfRust (LY8, TX9), then pre-paints and paints (LY19–LY22), with glyphs rasterized into atlases on the helper pool (TX12, TX19). It sends the display-list update.
6. The compositor validates it (PL9), draws only damaged tiles with its Metal pipelines (GR3, GR7), commits the CoreAnimation transaction, and acknowledges. Session history updates, and the shell learns the URL and title.

**2. Typing into a text field** (IN6, IN7, HD28, ST19, LY8, LY21):
1. `keyDown` arrives on the main thread and goes straight to the compositor's input sink, then to the focused frame's event loop.
2. The renderer fires `beforeinput`, edits the DOM, and fires `input` (HD28). HD15's hooks record invalidation sets for the selectors the edit can affect (ST19), mark the control's inline content (LY4), and queue the IME mirror's and the accessibility cache's updates (IN7, AX3).
3. At the next offered frame, style restyles only the marked elements and stops where inherited values didn't change (ST20); layout re-collects only that inline formatting context, and reshapes only the words that changed (LY8, TX10); paint reuses every other item (LY21); the compositor redraws only the damaged tiles (GR7).

**3. Scrolling a page that contains a cross-site iframe, at L3** (IN1, IN3, IN4, IV8, LY17, LY18, GR8):
1. A trackpad scroll begins. The compositor hit-tests the composited frames and latches the gesture to the scroll node under the pointer, in the child frame, whose document is in another renderer.
2. No listener there may block (LY26), so the compositor moves that node's content layer at once, without redrawing it (GR8), and moves its scrollbar thumb from the same offset (LY18). At its limit, the remaining distance chains to the parent's scroll node. When the gesture ends, the compositor snaps to the snap area its scroll node carries (LY17).
3. Both renderers learn the new offsets asynchronously and fire scroll events; display ports are refreshed as the user moves.

**4. Script that forces layout** (WB10, HD15, ST19, ST31, LY6, LY27):
1. A task calls `element.classList.add()` through a generated binding (WB10). The mutation's hook records pending invalidation sets (HD15, ST19).
2. The script reads `offsetWidth`. IV2's synchronous entry point runs style for the marked elements, box updates and layout for that document only, reusing every fragment whose constraint space is unchanged (LY6, LY27). A cross-agent parent isn't consulted. Reading `getComputedStyle(element).width` would take the same path, since that value is a used value (ST31).
3. The next rendering update reuses the fresh fragments.

**5. A renderer crashes** (FT13, IP6, IR10):
1. The browser learns of it from `EVFILT_PROC` and from dead-name notifications on its channels.
2. The core marks that renderer's documents gone. The tab shows a crash page, a subframe shows a placeholder, and every other replica in the affected groups is updated.
3. The compositor drops that renderer's display lists and resources, and the network process cancels its loaders. A reload launches a new renderer.

**6. A web font and an image arrive late** (IV10, TX7, TX20, ST26, IM4, IM5, LY5):
1. The image's `width` and `height` attributes gave its box an aspect ratio before any bytes (LY5). As bytes stream in, the safe decoder reports dimensions, then rows (IM4, IM9), decoding at the displayed size (IM5); paint repaints only the image's items.
2. A text run needs a face from `@font-face` (ST26). The font is fetched through the loader, decompressed and validated by Axiom's own code, never CoreText (TX7). During `font-display`'s block period the text is laid out with a fallback's metrics; when the font arrives, only the runs that used the fallback are reshaped and relaid (TX20).

**7. A click on a button with a listener** (IN1, IN2, LY25, HD18, HD21, WB17, JE6):
1. The compositor hit-tests its regions and routes the click to the frame under the pointer (IN1).
2. The renderer's hit test walks fragments in reverse painting order to the button (LY25), and the event becomes a task (HD21).
3. Dispatch builds the event path and the listener snapshot as two heap arrays (HD18) and calls each listener through the generated callback path (WB16, WB17); the interpreter runs it (JE6). Its DOM changes invalidate as in scenario 2, and the next frame shows them.

## 5. Criteria checks

### 5.1 Step 4's architecture criteria

| # | Criterion | Evidence | Verdict |
|---|---|---|---|
| AC1 | No rewrite from L0 to L3 | Every agent interaction goes through the browser even in one process (FT6). Frames subscribe, paint and receive input individually (EL20, FT12, IN1). No synchronous flush crosses an agent (IV2). Remote window proxies already shape cross-agent script (WB20). L3 changes only process selection (FT10) | **Met** |
| AC2 | The browser is the authority | One authoritative frame tree from the browser's own facts (FT1–FT3). Navigation, focus, capture, activation, visited history and Fetch's policy are browser records (FT7, IN5–IN8, ST10, NW12) | **Met** |
| AC3 | Idle costs nothing | One wait per thread (EL1–EL3), demonstrated for the network process by S7-Q4. Display links pause (EL19); idle documents unsubscribe (IV12); no Metal work without damage (GR7); style, layout and animations idle without polling (ST35, LY30) | **Met** by design; measured when built |
| AC4 | The browser never decodes images or parses web fonts | Display lists carry validated primitives only (PL7–PL9); glyphs are rasterized in the renderer (TX12, TX14), images decoded there or in the media process (IM1, IM3) | **Met** |
| AC5 | One single-threaded heap per agent; DOM arenas lent read-only only for a frozen stage | SH1, EL15; parallel style and layout use that lending only (ST33, LY30) | **Met** |
| AC6 | A deterministic, testable core | EL24's virtual clock; the headless embedder (EM9, GR11); IV14's oracles, extended by ST22's cache-free path and LY29's exact comparison | **Met** |
| AC7 | Memory is designed | Each subsystem names its owners and estimates (TX21, GR12, IM6, NW16, HD32, ST34, LY31, JE28) | **Met** by design; budgets in Step 6 |
| AC8 | The complete model fits | The table below, and each Step 5 document's complete-model section | **Met** |
| AC9 | `unsafe` stays where ADR-0002 allows it | §2's map; spikes S5 and S7 kept theirs in `platform` modules under stated invariants; the register records each dependency's `unsafe` and grade | **Met**, with `smallvec`'s grade open (§3) |
| AC10 | Every decision traces to evidence | Each document cites dated sources and labels inference; spikes S5 and S7 settled what sources couldn't, and S6 settles GR5 | **Met** |

**The complete model (AC8):**

| Capability | Where it lives |
|---|---|
| Dedicated, shared and service workers | Their own threads and heaps; dedicated in the owner's process, shared and service in a renderer locked to their site (EL §4.6, SH1, SW rules) |
| `SharedArrayBuffer` and Wasm threads | Shared memory outside heaps, in the cluster's one process (SH3, JE23) |
| WebAssembly | OQ-3, with the engine's evidence recorded (JE31) |
| WebGL and WebGPU | The GPU process (GR10, ADR-0004 D1) |
| Media | A utility process; video frames as layers (IM11–IM13, GR10) |
| Accessibility and IME | A browser-side cache stitched across frames; a main-thread mirror (AX1–AX12, IN7) |
| Back-forward cache | Frozen documents restored through the same commit (FT9) |
| Out-of-process iframes (L3) | Process selection only (FT10), with remote window proxies (WB20) |
| A JIT | `renderer-jit` only, on J1's evidence, fed by the interpreter's feedback (JE30) |

### 5.2 Step 5's subsystem criteria

| # | Criterion | Evidence | Verdict |
|---|---|---|---|
| SC1 | Step 4's interfaces hold; any change is an amendment for the owner | Two amendments are proposed and recorded in their ADRs: ADR-0008 (HD11, node slots never move) and ADR-0007 (ST25 and ST31, nested lifecycles for size containers, and IV2's examples). Refinements for the binding process model are noted for the owner, not edited (S5's and S7's metadata rules). GR5 would amend PL7 only if S6 shows GPU path filling pays. Nothing changed silently | **Met** |
| SC2 | Untrusted input is parsed only where the Rule of 2 allows | Every parser of web content is safe Rust in a renderer or the network process (HD1, HD9, ST1, JE1, TX7, IM2, NW3); Apple's parsers run only in sandboxed helpers: CoreText on system fonts in renderers, Security in the network process, ImageIO in the media utility process as S5-Q5 recommends | **Met**; ImageIO's placement is decided at Gate C |
| SC3 | `unsafe` stays where ADR-0002 allows it, dependencies included | §2 and the register | **Met**, with `smallvec` open |
| SC4 | Build or reuse follows the charter | Each document's §4; the core of DP1 is Axiom's own; reused infrastructure is justified and classed in the register | **Met** |
| SC5 | Standards first, and testable | Each document names dated specifications and its suites; approximate behaviour is kept apart: `execCommand` (HD28), automatic table layout (LY10), `Error.prototype.stack` and `Date.parse`'s legacy formats (JE25, JE22) | **Met** |
| SC6 | Efficiency is designed | Owners, estimates, threads and idle behaviour in every document (AC3, AC7 above) | **Met** by design; budgets in Step 6 |
| SC7 | Incremental by design | Each pipeline document states its caches, invalidation and oracle (HD33, ST19–ST22, LY29, TX20, IM6) | **Met** |
| SC8 | Deterministic and headless | EL24, EM9 and GR11; system fonts, system settings and the environment injected in tests (TX21, ST36, LY32) | **Met** |
| SC9 | The complete model fits | Each document's complete-model section; §5.1's table | **Met** |
| SC10 | Every decision traces to evidence | Dated primary sources in every document; spikes S5 and S7 measured; S6 pending | **Met** |

## 6. Staging from L0 to L3

| Level | Built | Unchanged from the level before |
|---|---|---|
| **L0** | One process. Every thread role, channel, record and replica exists; helpers are threads; channels are in-memory or Mach within the process | — |
| **L1** | Renderers (one per tab) and the network process become sandboxed processes with browser-compiled profiles. Launch pins audit tokens | Every interface, record and protocol |
| **L2** | Site locks; separate agents and event loops for cross-site frames in their embedder's process (SI3), reached through remote window proxies (WB20); partitioning and ORB at the endpoints. A GPU process with the first web-supplied GPU work | Every interface |
| **L3** | Cross-site frames in renderers locked to their own sites; V1–V9 pass | Every interface: only process selection changes |

## 7. Decisions at Step 4's gates

From the documents above, recorded in ADR-0005 to ADR-0009. **The owner, 2026-10-17:** ADRs 0005, 0006, 0007 and 0009 accepted; ADR-0008 pre-approved, then **accepted at Gate C (2026-10-18)** once S3's addendum was measured, with C1 met by the safe `SliceStack`. Also decided: spike S3's authorization with its 10% threshold; AC5 amended for read-only lending (OQ-19); the pure-Rust shell (EM11).

## 8. Decisions for Step 5's Gate C

Each with a recommendation; the [tracker](../research/step-5-subsystems.md)'s Gate C section gives the rationale in full.

| # | Decision | Recommendation |
|---|---|---|
| 1 | [ADR-0015](../adr/0015-html-parsing-and-the-dom.md), HTML parsing and the DOM | Accept |
| 2 | The amendment to [ADR-0008](../adr/0008-script-agents-and-heap.md): SH10's compaction skips the node arena (HD11) | Accept |
| 3 | [ADR-0016](../adr/0016-style-layout-and-paint.md), style, layout and paint | Accept |
| 4 | The amendment to [ADR-0007](../adr/0007-frame-tree-pipeline-and-input.md): nested lifecycles for size containers (IV1), and IV2's examples (ST25, ST31) | Accept |
| 5 | `:visited` by partitioned history (ST10) | Accept |
| 6 | [ADR-0017](../adr/0017-javascript-engine-and-bindings.md), the JavaScript engine and bindings | Accept |
| 7 | XSLT ([OQ-22](../open-questions.md)) | Not planned until a use requires it |
| 8 | [ADR-0011](../adr/0011-text-fonts-and-unicode.md): the visual check of S5-Q1's and S5-Q2's sheets, then TX12 | Check the sheets; CoreText for system fonts |
| 9 | [ADR-0012](../adr/0012-graphics-images-and-media.md): S5-Q5 missed its condition; S6's formal run waits for the owner | AVIF in the media utility process (IM3's fallback); run S6 formally |
| 10 | Verification in the network process's worker pool (NW6) | Confirm |
| 11 | The process model's refinements from S5 and S7 (metadata rules) | Note them in the process model's change record |
| 12 | Merging `step-5` into `main` | After the decisions above, once S6's results are recorded or deferred by the owner |
