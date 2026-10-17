# Architecture Overview v0

| | |
|---|---|
| **Status** | **Proposed** (Step 4, CP11): the architecture proposal v0. Its decisions are put to the owner as ADRs 0005–0009 at Gate B; the proposal as a whole is approved at the end of Phase Zero (Step 6) |
| **Question** | How do the parts fit together: processes × threads × pipeline; the crate structure and its `unsafe` map; what happens end to end in common scenarios; and how does the system grow from L0 to L3 without a rewrite? |
| **Answers** | Step 4 question Q16 ([tracker](../research/step-4-architecture-proposal.md) §3), and the check of the architecture criteria AC1–AC10 (tracker §5) |
| **Builds on** | Step 3: [threat model](../security/threat-model.md), [process model](../security/process-model.md), [ADR-0004](../adr/0004-process-model-and-sandbox.md). Step 4: [macOS runtime note](../research/macos-runtime-facilities.md), [event loop and threading](event-loop-and-threading.md), [IPC](ipc.md), [frame tree and navigation](frame-tree-and-navigation.md), [rendering pipeline](rendering-pipeline.md), [invalidation](invalidation.md), [input and accessibility](input-and-accessibility.md), [embedder API](embedder-api.md), [script and heap](script-and-heap.md) |
| **Last updated** | 2026-10-17 |

Everything here is **planned**; nothing is implemented. This document adds no new rules; it assembles the rules of the documents above (EL, IP, FT, PL, IV, IN, EM, SH), which it cites.

## 1. The system on one page

```
 Browser process (trusted, hardened, unsandboxed)
 ┌────────────────────────────────────────────────────────────────────────────┐
 │ main thread (AppKit)     core thread               compositor thread      │
 │ shell UI · NSEvents ──▶  frame tree · navigation   display links · frames │
 │ IME mirror · AX cache    session history · policy  validate · composite   │
 │                          process manager · profile hit test · scroll      │
 │            ◀── embedder API (IDL messages) ──▶     ◀── frame offers ──▶   │
 └──────────────────────┬──────────────────────────────────┬──────────────────┘
          Mach IPC: browser-created channels (capabilities), audit-token pinned
 ┌──────────────────────┴───────────────────┐   ┌──────────┴───────────────────┐
 │ Renderer (sandboxed, site-locked from L2) │   │ Network process (sandboxed)  │
 │ control thread                            │   │ I/O thread: HTTP, TLS,       │
 │ event-loop thread per agent:              │◀─▶│ loader endpoints, ORB, CORS  │
 │   heap · DOM · style · layout · paint     │   │ storage thread: cookies,     │
 │   → display list per frame                │   │ web storage, cache index     │
 │ helper pool: decode, glyph raster         │   └──────────────────────────────┘
 └───────────────────────────────────────────┘    later: GPU process, utility
```

| Layer | Holds | Rules |
|---|---|---|
| **Shell** (browser main thread) | Windows, tabs, menus, prompts; mirrors that answer AppKit's synchronous IME and accessibility queries | EM1, EM6, IN7, IN9 |
| **Core** (browser core thread) | The authoritative frame tree, navigation, session history, process launch and site locks, policy, profile data | FT1–FT10, EM7 |
| **Compositor** (browser compositor thread) | Display links, frame offers, validation, compositing, hit testing, scrolling and compositor animation | EL19–EL22, PL9, IV8, IN1 |
| **Renderers** | One thread and heap per agent; the pipeline from DOM to display list per frame | EL15, SH1, PL1–PL7 |
| **Network process** | Fetching, caching, cookies and storage behind endpoints bound to each document's facts | FT11, EL8 |
| **IPC** | Mach channels created only by the browser; IDL-generated safe decoders; `Untrusted<T>` validation | IP1–IP22 |

## 2. Crate structure and the `unsafe` map

A proposal for the implementation milestones. The final names and splits are set when code is authorized. Engine crates go under `crates/`, as the workspace manifest already anticipates.

| Crate | Purpose | Process | `unsafe` |
|---|---|---|---|
| `platform` | All FFI: Mach IPC, kqueue and run loops, sandbox entry and profile compilation, `posix_spawn`, shared memory and IOSurfaces, QoS, wait-on-address, CoreText, ImageIO, Metal | All | **Yes**, in listed modules (ADR-0002 §15.3) |
| `gc` | Pattern B heap: typed arenas, handles, scopes, the root stack, the collector | Renderer | **Only `src/raw/`**, if C1 needs it (SH7, S3) |
| `jit` | Later, only if J1's evidence calls for it | `renderer-jit` | **Yes**, under J1–J7 |
| `shell` | The AppKit shell, through objc2 (EM11) | Browser | **Only its `platform` module** |
| `ipc`, `idl` | Channels, the in-process transport, `Untrusted<T>`; the build-time generator | All | No |
| `event-loop` | Thread loops, the HTML event loop scheduler, the deterministic test scheduler | All | No |
| `core`, `frame-tree`, `embedder-api` | The browser's authority: records, navigation, policy, process manager; the API to the shell | Browser | No |
| `compositor`, `display-list` | Validation and compositing; the display-list format shared with renderers | Browser (later GPU) | No (Metal via `platform`) |
| `html`, `dom`, `css`, `style`, `layout`, `paint`, `text`, `image` | The pipeline, parsers and decoders | Renderer | No (ADR-0002 rule 5) |
| `js`, `bindings` | The JavaScript engine (parser, bytecode, interpreter, builtins); generated WebIDL glue | Renderer | No |
| `net`, `storage` | HTTP, cache, cookies, CORS and ORB; web storage | Network | No (the TLS choice is Step 5's) |
| `headless` | The headless embedder for tests and WPT (EM9) | Test | No |
| Executables | `axiom` (shell and browser), `axiom-renderer`, `axiom-network`; later `axiom-renderer-jit`, `axiom-gpu`, `axiom-utility` | — | — |

**Dependency rules:**
- engine crates never depend on `shell` (EM3);
- renderer crates depend on browser crates only through the protocol crates (`ipc`, `display-list`);
- `net` and `storage` never depend on renderer crates;
- only `platform`, `gc`'s raw module, `jit` and the shell's `platform` module may contain `unsafe` (AC9).

## 3. Five scenarios, end to end

**1. Opening a URL in a new tab** (FT7, FT10, IP4, EL19–EL21, PL9):
1. The shell sends a load command through the embedder API. The core creates the tab's records and a navigation, and the network process fetches it on the browser's behalf.
2. At the response, the core computes the origin, the site and the agent cluster key, and chooses a renderer. If it must launch one, the helper receives its profile bytecode through its one inherited descriptor and applies it, then looks up its rendezvous and says hello. The core pins its audit token.
3. The core commits the document, sending its ID, origin, policy container, frame-tree skeleton, loader and storage endpoints, and the response's body stream.
4. The renderer starts an event-loop thread with a new heap, parses as the stream arrives, and subscribes for frames.
5. At the next vsync the compositor offers a frame. The renderer runs style, layout and paint, and sends its display-list update. The compositor validates it, composites, and presents.
6. The acknowledgement makes the commit authoritative. Session history updates, and the shell learns the URL and title.

**2. Typing into a text field** (IN6, IN7, IV3–IV6):
1. `keyDown` arrives on the main thread and goes straight to the compositor's input sink, then to the focused frame's event loop, recorded by the core.
2. The renderer dispatches the key events, edits the DOM, and invalidates just the text control's style and layout. It then updates the browser's IME mirror and accessibility cache.
3. During a composition, the IME's synchronous queries read the mirror on the main thread, and its edits travel to the renderer as messages.
4. At the next offered frame, an incremental display-list update repaints the control.

**3. Scrolling a page that contains a cross-site iframe, at L3** (IN1, IN3, IN4, IV8, IV9):
1. A trackpad scroll begins. The compositor hit-tests the composited frames and latches the gesture to the scroll node under the pointer, in the child frame, whose document is in another renderer.
2. No listener there may block, so the compositor scrolls that node at once. At its limit, the remaining distance chains to the parent's scroll node, still in the compositor.
3. Both renderers learn the new offsets asynchronously and fire scroll events. Display ports are refreshed as the user moves, and momentum stays latched until it ends.

**4. Script that forces layout** (IV1, IV2, IV5):
1. A task modifies the DOM, so style invalidations are pending, then reads `offsetWidth`.
2. The synchronous entry point runs style, box updates and layout for that document only, up to `LayoutClean`, and returns the value. A cross-agent parent isn't consulted: the frame uses its last viewport.
3. The next rendering update reuses the fresh fragments.

**5. A renderer crashes** (FT13, IP6, IR10):
1. The browser learns of it from `EVFILT_PROC` and from dead-name notifications on its channels.
2. The core marks that renderer's documents gone. The tab shows a crash page, a subframe shows a placeholder, and every other replica in the affected groups is updated.
3. The compositor drops that renderer's display lists and resources, and the network process cancels its loaders. The shell hears of the crash, and a reload launches a new renderer.

## 4. Criteria check

| # | Criterion | Evidence | Verdict |
|---|---|---|---|
| AC1 | No rewrite from L0 to L3 | Every agent interaction goes through the browser even in one process (FT6). Frames subscribe, paint and receive input individually (EL20, FT12, IN1). No synchronous flush crosses an agent (IV2). The same IPC interfaces run in-process (IP2). L3 changes only process selection (FT10; §5 below) | **Met** |
| AC2 | The browser is the authority | One authoritative frame tree from the browser's own facts (FT1–FT3). Navigation is browser-driven (FT7). Focus, capture and activation are browser records (IN5–IN8). `Untrusted<T>` is validated against them (IP10) | **Met** |
| AC3 | Idle costs nothing | One wait per thread, no polling (EL1–EL3). Display links pause when no one needs frames (EL19). Idle documents unsubscribe (IV12). Timers carry leeway, and background timers are throttled (EL18) | **Met** by design; measured when built |
| AC4 | The browser never decodes images or parses web fonts | Display lists carry validated primitives only: glyph images rasterized in the renderer, decoded pixels, references (PL7–PL9) | **Met** |
| AC5 | One single-threaded heap per agent; nothing GC-managed crosses a thread | One heap per agent on its thread (SH1, EL15). Only copies, transfers and shared buffers between agents (SH2–SH3). Style and layout stay on the event loop's thread (PL10) | **Met as written.** Gate B decides whether to allow read-only lending of DOM arenas during a frozen rendering stage, for parallel style and layout |
| AC6 | A deterministic, testable core | Thread loops behind an interface, with a virtual clock (EL24). In-memory channels (IP2). The headless embedder (EM9). Incremental-versus-rebuild oracles (IV14) | **Met** |
| AC7 | Memory is designed | Each structure has an owner and a lifetime (PL13, SH14, EL §4.10). Bounded queues (IP16). Budgets measured in Step 6 | **Met** by design; budgets in Step 6 |
| AC8 | The complete model fits | Every capability has a home without architectural change: the table below | **Met** |
| AC9 | `unsafe` stays where ADR-0002 allows it | `platform`, `gc`'s raw module (only if C1 needs it), `jit` (later), and the shell's `platform` module (§2). Decoders and generators are safe Rust (IP8) | **Met** |
| AC10 | Every decision traces to evidence | Each document cites its precedent and specifications, and labels inference. The decisions that sources can't settle are left to measurement: C1 by S3, IP20's priority boost at the first implementation milestone, C4's order on the corpus | **Met** |

**The complete model (AC8):**

| Capability | Where it lives |
|---|---|
| Dedicated, shared and service workers | Their own threads and heaps; dedicated in the owner's process, shared and service in a renderer locked to their site (EL §4.6, SH1) |
| `SharedArrayBuffer` and Wasm threads | Shared memory outside heaps, in the cluster's one process; `Atomics.wait` on wait-on-address (SH3) |
| WebGL and WebGPU | The GPU process, which takes over rasterizing and presenting; canvas contents are display-list resources (PL7, PL §5; ADR-0004 D1) |
| Media | Decoding in a utility process; video frames as resources; real-time audio on `os_workgroup` threads (PL7, EL §4.2) |
| Accessibility | A browser-side cache stitched across frames (IN9) |
| IME | A mirror on the main thread, kept current by the focused frame (IN7) |
| Back-forward cache | Frozen documents in their renderers, restored through the same commit (FT9) |
| Out-of-process iframes (L3) | Process selection only (FT10) |

## 5. Staging from L0 to L3

| Level | Built | Unchanged from the level before |
|---|---|---|
| **L0** | One process. Every thread role, channel, record and replica exists; helpers are threads; channels are in-memory or Mach within the process | — |
| **L1** | Renderers (one per tab) and the network process become sandboxed processes with browser-compiled profiles. Launch pins audit tokens | Every interface, record and protocol |
| **L2** | Site locks; separate agents and event loops for cross-site frames in their embedder's process (SI3); partitioning and ORB at the endpoints. A GPU process with the first web-supplied GPU work | Every interface |
| **L3** | Cross-site frames in renderers locked to their own sites; V1–V9 pass | Every interface: only process selection changes |

## 6. Decisions for Gate B

From the documents above. ADR-0005 to ADR-0009 record them, Proposed:

| ADR | Decides |
|---|---|
| 0005 | IPC: Mach transport, capabilities, IDL and generated decoders, `Untrusted<T>`, bounds (IP1–IP22) |
| 0006 | Event loop and threading (EL1–EL24) |
| 0007 | Frame tree, rendering pipeline, invalidation, and input across frames (FT, PL, IV, IN) |
| 0008 | Script, agents and the heap (SH1–SH14), accepted after spike S3 |
| 0009 | Engine/embedder API and the shell (EM1–EM12, OQ-11) |

Open owner questions raised along the way:
- authorizing spike S3, with its optional parts and its 10% threshold;
- amending AC5 for read-only lending during frozen rendering stages (PL10);
- confirming the pure-Rust shell (EM11).
