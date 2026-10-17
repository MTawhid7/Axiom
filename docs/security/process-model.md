# Process Model

| | |
|---|---|
| **Status** | **Binding** since ADR-0004's acceptance (2026-10-17). The owner accepted SI1–SI7 as staged (SI3 limited to L1 and non-sensitive L2; out-of-process iframes required for L3) and B1–B7 as initial baselines. Spike S4 measurements are folded in ([S4 results](../research/s4-results.md)) |
| **Question** | Into which processes is Axiom split? What may each one do, how are sites assigned to processes, which rules govern IPC, and how much memory may each process use on the 8 GB M1? |
| **Feeds** | ADR-0004; the architecture proposal (Step 4): event loop, pipeline, embedder API; Step 5 subsystems; Step 6 budgets and roadmap |
| **Builds on** | [Threat model](threat-model.md); [macOS confinement note](../research/macos-process-confinement.md); [ADR-0001](../adr/0001-scope-and-target.md); [ADR-0002](../adr/0002-implementation-language.md); [ADR-0003](../adr/0003-gc-handle-and-rooting-pattern.md); [S2 results](../research/s2-results.md) |
| **Last updated** | 2026-10-17 |

Everything here is **planned**; nothing is implemented. Claims about other browsers are sourced (§11). Budgets are **targets**, to be validated (§7).

---

## 1. Summary

- **Five process types:**
  - a trusted **browser** process;
  - **renderers** that interpret web content, each locked to one site from security level L2;
  - one **network** process that holds cookies, the cache and web storage;
  - a **GPU** process, once web content can send work to the GPU;
  - **utility** processes for isolated jobs such as media decoding.
- **Confinement on macOS.** Every helper is a separate executable that applies its own deny-by-default Seatbelt profile before it touches input (option O1 in the [macOS note](../research/macos-process-confinement.md)). The browser process is unsandboxed but hardened, and it never parses web content.
- **Cross-site iframes may stay in their embedder's process only at L1 and non-sensitive L2,** under three compensating rules: third-party state partitioned by top-level site, ORB-style response blocking, and no sensitive site ever rendered inside another site's process. **Out-of-process iframes are mandatory before L3** (§5; the owner's decision, 2026-10-17).
- **IPC rules IR1–IR10** make every message from a less privileged process untrusted input. Decoding is safe Rust, decisions come from the receiver's own records, and any violation kills the sender (§6).
- **Initial memory targets:** about 1 GB for a typical session of 10 tabs, with a soft limit of 16 renderers on 8 GB, same-site process reuse, and tab discarding under memory pressure. These targets are not measurements (§7).

## 2. Principles

| # | Principle | Source |
|---|---|---|
| PM1 | **Least privilege per process type.** Each process gets only the OS capabilities its job needs | Threat model §7 |
| PM2 | **Rule of 2.** No component combines untrusted input, memory-unsafe code and high privilege. For Axiom, memory-unsafe code means `unsafe` Rust and Apple's C, C++ and Objective-C libraries | Chromium [4]; threat model TA5 |
| PM3 | **The browser is the policy authority.** Children are untrusted, and policy facts come from the browser's own records | Threat model TB4 |
| PM4 | **Capabilities, not ambient authority.** A child receives handles (channel endpoints, descriptors, IOSurface ports) for exactly what it uses, and can't name global services | macOS note §5 |
| PM5 | **Fail closed.** A protocol violation kills the sender. A crash leads to a clean restart, never to carrying on with partial trust | Chromium [2] |
| PM6 | **Design for the split now; ship it in stages.** Every interface between future processes is a message protocol from the first line of code, and it also works in-process for tests | [Decision map](../decision-map.md) §2, item 3 |
| PM7 | **Memory is a budget.** Process count and per-process footprint are designed, measured and enforced | Charter G4; ADR-0001 consequence 7 |
| PM8 | **Idle costs nothing.** No process runs timers or polls while nothing changes, and background renderers are throttled | [S2 results](../research/s2-results.md) §5 |

## 3. Process types

| Process | Instances | Trust zone | Its job | Untrusted input it handles |
|---|---|---|---|---|
| **Browser** | 1 | Z1 (trusted) | Windows and browser UI (AppKit). Navigation and the frame tree, which it holds as the source of truth. Launching processes and assigning site locks. Permissions and tracking of user activation. Profile data other than web data (history, bookmarks, settings, permissions). Downloads, which it writes and marks as quarantined. **The compositor and presenter** until a GPU process exists | Messages from children (safe Rust); address-bar text. **No web content** |
| **Renderer** | Many | Z3 (untrusted) | Everything that interprets web content: HTML and CSS parsing, the DOM, style, layout, JavaScript (one heap per agent cluster, ADR-0003), image decoding, font parsing and shaping, and building display lists | All web content |
| **Network** | 1 | Z2 | HTTP(S), TLS, DNS; the HTTP cache; the cookie jar; web storage (`localStorage`, IndexedDB, Cache API); ORB-style response blocking; CORS and initiator checks against the requesting renderer's lock | All network bytes |
| **GPU** | 0 or 1 | Z2 | The Metal device: rasterizes and composites display lists; runs WebGL and WebGPU; hands frames to the browser as IOSurfaces | Display lists; web-supplied shaders and commands |
| **Utility** | On demand | Z2 or Z3 | Isolated jobs, first of all media decoding with system codecs. One instance per kind of job | Media streams and similar |

**Design notes.**

1. **The browser composites until a GPU process exists.**
   - At first, the browser consumes renderer display lists that contain only simple primitives: geometry, colours, glyph masks or glyph runs from system fonts, and decoded pixels in shared memory.
   - The browser never decodes images or parses fonts. Decoding validated primitives in safe Rust keeps it within the Rule of 2 (PM2).
   - The GPU process becomes necessary with the first feature that sends **web-supplied** work to the GPU: WebGL, WebGPU or hardware video. That work reaches Apple's shader compiler and drivers, which must not run in the browser (threat model TA5).
   - The display-list interface is designed so that moving compositing out of the browser doesn't change renderers (decision map §2, item 5).
   - This is an **inference** from the threat model and S2's measured cost of Metal: about 20 MB for an idle device.
2. **Web storage lives with the cookie jar, in the network process,** as in WebKit, whose Networking process also hosts storage. That keeps renderer-supplied structured data out of the browser process. Step 4 may split storage into its own process if crash isolation or memory requires it.
3. **Images and fonts are decoded in the renderer,** with no separate decoder process per image (contrast Ladybird [10]).
   - A compromised decoder there yields only that site's data (from L2).
   - Safe Rust decoders are preferred. A system decoder (ImageIO) or CoreText runs only inside a renderer or the GPU process.
   - Images that the browser itself shows, such as favicons, are decoded with safe Rust decoders or in a renderer.
4. **Where glyphs are rasterized** (renderer or GPU process) is a Step 5 decision (I1, I2). S4-Q5 tests whether CoreText works inside a renderer's sandbox.

## 4. What each process may do

### 4.1 OS capabilities

"Rules from S4" means the exact sandbox rules are set by evidence from spike S4, starting from denial.

| Capability | Browser | Renderer | Network | GPU | Utility (media) |
|---|---|---|---|---|---|
| User files | Yes, on user intent | **No** | **No** | **No** | **No** |
| Own data directory | Profile (settings, history) | None | Web data: cookies, cache, storage | Shader cache, if needed | None |
| Network sockets | Not used | **No** | Outbound TCP and UDP; the system DNS and certificate-trust services | **No** | **No** |
| Window server, AppKit | Yes | **No** | **No** | **No** | **No** |
| GPU (Metal, IOKit) | Yes, until a GPU process exists | IOSurface allocation only, if needed (rules from S4) | **No** | Yes | Hardware decode only |
| System fonts | Yes | Read-only: `/System/Library/Fonts` and ICU's data in `/usr/share/icu` (S4-Q5) | **No** | Only if it rasterizes text | **No** |
| Pasteboard; camera, microphone, location, screen | Yes, mediated and prompted | **No** | **No** | **No** | **No** |
| Launch processes | Yes, its own helpers only | **No** | **No** | **No** | **No** |
| Executable memory (JIT) | **No** | **No**. Only the JIT renderer variant, under the JIT policy in ADR-0004 | **No** | **No** | **No** |
| Keychain | Yes, if secrets are stored | **No** | **No** | **No** | **No** |
| Mach service lookups | Unrestricted (not sandboxed) | The browser's rendezvous service only, plus logging | DNS, certificate trust, logging (rules from S4) | Metal services (rules from S4) | Media services (rules from S4) |

### 4.2 Rule of 2 placement

| Work | Untrusted input | Memory-unsafe code | High privilege | Where it may run |
|---|---|---|---|---|
| Parsing web content and running script (safe Rust) | Yes | No (yes once a JIT exists) | — | Renderer |
| System image and font parsers (ImageIO, CoreText) | Yes | Yes | **Must not** | Renderer (or GPU process) |
| HTTP, TLS and DNS parsing | Yes | No if our Rust; yes if Apple's TLS stack | Medium | Network process: sandboxed either way |
| Decoding IPC in the browser | Yes | **No** (ADR-0002 rule 5) | Yes | Browser (two of three) |
| Consuming display lists in the browser | Yes | No; the Metal driver receives only our commands | Yes | Browser, until web-supplied GPU work exists |
| Compiling WebGL and WebGPU shaders | Yes | Yes (Apple's compiler) | **Must not** | GPU process |
| Accessibility trees from renderers | Yes | No | Yes | Browser (two of three) |

## 5. Site isolation policy

**Definitions.**
- A **site** is a scheme plus a registrable domain (eTLD+1), computed with the Public Suffix List. Chromium and WebKit both define it this way [2][9].
- A **site lock** binds a renderer to one site. It is assigned before any content loads and never changes, as in Chromium [2].

| # | Rule | From level |
|---|---|---|
| SI1 | Every renderer is locked to one site before it receives content. The lock never changes | L2 |
| SI2 | A top-level document of site X runs only in a renderer locked to X. Cross-site navigations switch processes, and the browser alone decides when | L2 |
| SI3 | **Cross-site iframes, at L1 and non-sensitive L2 only** (the owner's decision, 2026-10-17), render in the embedder's process, under three compensating rules: (a) **partitioned third-party state:** a cross-site frame's cookies and storage are keyed by the top-level site and the frame's site, so it gets no unpartitioned credentials; third-party cookies are blocked by default; (b) **response blocking:** the network process applies ORB, so cross-site no-CORS HTML, JSON and XML never reach a renderer [7]; (c) **no sensitive site inside another site's process:** as a cross-site subframe, a sensitive site (SI5) is either loaded out of process, once that exists, or replaced by a placeholder that opens it in its own tab | L2 |
| SI4 | **Out-of-process iframes are mandatory before L3** (the owner's decision, 2026-10-17). From the first line of code, the browser owns the frame tree, each frame produces its own display list for the compositor, and the browser routes input to frames, so that moving to out-of-process iframes needs no architectural rewrite. These are constraints on Step 4 | Design: Step 4. Required: L3 |
| SI5 | **Sensitive sites** are the owner's explicit list plus heuristics that Chromium uses on memory-constrained devices: sites where the user entered a password, logged in through OAuth, or met a COOP header [2]. A sensitive site always gets a dedicated process, runs without a JIT (ADR-0004), and is never embedded in another site's process | L2 |
| SI6 | **Process limits never merge sites.** Above the soft limit (§7, B5), same-site processes are reused, but different sites never share a renderer. Memory pressure is handled by discarding background tabs. Chromium's desktop limit is likewise soft [2] | L2 |
| SI7 | **Workers and agent clusters.** A dedicated worker runs in its document's process. Shared and service workers run in a renderer locked to their site. A cross-origin isolated browsing context group (COOP plus COEP) gets its own processes, because `SharedArrayBuffer` can't leave its agent cluster [6]. Input to OQ-5 | L2 |

**Why SI3 is acceptable for now** (inference).
- **Full out-of-process iframes are expensive.** Every process must hold a representation of the whole frame tree [9]. Chromium needed proxies and replicated frame state [3]. WebKit was still at "step 2" of its plan in January 2025 [9], and in July 2025 it added a shared process for cross-site iframes "for performance reasons" [9].
- **Partitioning limits what an iframe exposes.** With third-party state partitioned, a cross-site iframe in the embedder's process carries only data that its site associates with the embedder's top-level site. A Spectre read or a compromised renderer there gains little that the embedder couldn't already obtain through that partition.
- **The residual exposure** is the embedded site's public content, unpartitioned state the user explicitly grants (a Storage Access API grant must then require a separate process, or be refused), and timing side channels.
- **Rule (c) covers the most valuable targets** during prototyping. SI3 is not enough for sensitive use: L3 requires out-of-process iframes (threat model §8, check V9).

**Memory evidence.** Chrome with Site Isolation, measured on desktop in 2018 [3]:
- the median number of renderer processes rose from 4.4 to 6.2, with 6.0 unique sites open;
- private memory per renderer fell from 87.2 MB to 42.3 MB;
- total browser memory rose by 9–13%.

**Inference.** Isolating every site costs about a tenth more memory, not a multiple, because the same work is divided among more, smaller processes. That makes full isolation of top-level sites affordable on 8 GB.

## 6. IPC

### 6.1 Requirements for the transport

The transport itself (Mach messages, or Unix-domain socket pairs) is chosen in Step 4 from S4-Q3's measurements.

| # | Requirement |
|---|---|
| R1 | Asynchronous, two-way channels of discrete messages |
| R2 | Handle passing: channel endpoints, shared memory, file descriptors, IOSurfaces |
| R3 | Each peer's identity known from the moment it is launched (its process ID or audit token), never taken from message contents |
| R4 | Bulk data in shared memory, mapped read-only in the receiver where the OS allows |
| R5 | Bounded: caps on message size and queue length, and backpressure |
| R6 | The same API in-process, for tests, fuzzing and a single-process debugging mode |

### 6.2 Trust rules

| # | Rule |
|---|---|
| IR1 | **Every message from a less privileged process is untrusted input.** Decoders are generated safe Rust: no `unsafe` (ADR-0002 rule 5), and a generator that emits none (rule 7). Every length and count is bounded |
| IR2 | **Decisions come from the receiver's own records:** the sender's process type, site lock and granted capabilities. A field that restates a known fact, such as an origin, is checked against the record. A mismatch counts as a violation, like Chromium's "jail" checks [2] |
| IR3 | **Any violation terminates the sender** and is logged. Violations include a message that won't decode, an out-of-range value, a message unexpected in the current state, and a failed lock check. There is no recovery path |
| IR4 | **Interfaces are capabilities.** A renderer holds endpoints only for its own frames, for a network loader and a cookie and storage accessor that are each bound to its site lock, and for its compositor surface. It can't address other processes or global services |
| IR5 | **The browser never blocks on a less privileged process.** It makes no synchronous calls to a child, puts timeouts on replies, and kills an unresponsive child |
| IR6 | **Synchronous calls from a child to the browser** exist only where the web platform requires synchronous semantics, such as reading `document.cookie`. They are bounded and can't re-enter |
| IR7 | **Shared memory has one writer.** The reader copies what it interprets before validating it, so it never reads the same bytes twice. Shared memory holds bounds-checked offsets, never pointers |
| IR8 | **One build only.** The handshake carries a build identifier, and a mismatch refuses the connection. There is no compatibility logic between versions |
| IR9 | **Only the browser launches processes,** and only its own bundle's helper executables |
| IR10 | **When a child dies,** the browser discards everything that child reported and recovers from its own state: a crash page in the tab, and a reload on request |

**Unsafe boundary.** Receiving a Mach message or a socket message with attached handles needs FFI, which lives in `platform`. The platform layer hands the safe decoder plain bytes and a typed list of handles, so no IPC decoding code needs `unsafe` (inference).

## 7. Memory budgets and process limits

**Data available today:**

| Measure | Value | Source |
|---|---|---|
| Footprint of a minimal process (`/bin/sleep`) | 1.03 MB | Observed (macOS note §9) |
| An idle AppKit window with a Metal device | 20–21 MB; peak while drawing 35.6 MB | [S2 results](../research/s2-results.md) §5 |
| Chrome 69 with Site Isolation, per renderer, median and 99th percentile private memory (Windows desktop, 2018) | 42.3 MB; 509.7 MB | Paper [3] |
| Chrome 69 with Site Isolation, total browser memory, median | 685 MB | Paper [3], Figure 3 |
| Unique sites open, median and 99th percentile | 6.0; 41.9 | Paper [3] |

**Initial targets**, measured as each process's physical footprint, as in S2. They are validated by S4-Q3 and set finally in Step 6 (performance budgets, X3):

| # | Budget | Target | Basis |
|---|---|---|---|
| B1 | Browser process, idle, one window | ≤ 60 MB | S2's 20 MB, plus browser UI, profile and compositor |
| B2 | Renderer, sandboxed and ready, empty document | ≤ 10 MB | S4 measured a sandboxed helper at 1.14 MiB, and 4.05 MiB once CoreText has drawn text. Engine state and an empty heap come on top |
| B3 | Renderer, typical page (median) | ≤ 50 MB | Chrome's 42 MB median, for comparable content in 2018 |
| B4 | Network process, idle | ≤ 25 MB | Excludes the disk cache's mapped files |
| B5 | Renderer soft limit on 8 GB | 16 processes | 16 × B3 = 800 MB at typical pages. A soft limit, as in Chromium [2] |
| B6 | Whole browser, typical session (10 tabs, about 6 sites) | ≤ 1 GB | B1 + B4 + about 7 renderers at B3, plus GPU and headroom |
| B7 | Per-renderer cap | 2 GiB, a quarter of physical memory; to be set in Step 6 | Above it, the renderer is terminated and the tab shows an error (threat model SG11) |

**Responding to memory pressure.** The browser listens for macOS memory-pressure events (macOS note §2.7):
- **At warning:** it drops the spare renderer, trims caches, and discards the renderers of background tabs, oldest first. A discarded tab keeps its URL and state, and reloads when shown.
- **At critical:** it discards every background renderer.

Sensitive sites are not exempt from discarding. Discarding only ends a process; it never merges sites (SI6).

## 8. Lifecycle

- **Launch.** The browser spawns a helper with `POSIX_SPAWN_CLOEXEC_DEFAULT`, a minimal environment, and only its IPC bootstrap handle and a log descriptor. The helper applies its profile at once, from bytecode the browser compiled, then connects. The browser knows the helper from launch (R3). S4 measured about 2 ms from launch to ready for a bare helper.
- **A spare renderer.** To hide launch latency, the browser keeps one pre-launched, sandboxed but not yet locked renderer, and locks it on first use. Chrome's spare process masks start-up cost this way [3]. It costs B2, and is dropped under memory pressure.
- **Crash.** Affected tabs show a crash page and reload on request. After repeated crashes from the same site, automatic reloads stop.
- **Hang.** If a renderer stops answering input, the browser offers to end it. The browser itself never waits (IR5).
- **Background work.** Background renderers run at background QoS with throttled timers, and an idle tab costs nothing (PM8).

## 9. Staging by security level

The levels are defined in the [threat model](threat-model.md) §8.

| Level | Processes | Notes |
|---|---|---|
| L0 | Browser. The renderer and network work may run inside it, through the message interfaces (PM6) | Earliest milestones: local test pages and public documents; never credentials |
| L1 | Browser, renderers (one per tab, not locked), network process. All helpers sandboxed (O1); the browser composites | Browsing without logging in |
| L2 | L1 plus site locks and SI1–SI7, partitioned third-party state, ORB. A GPU process arrives with the first web-supplied GPU work | Logins to accounts that aren't sensitive |
| L3 | **Out-of-process iframes in place.** The gate's checks V1–V9 pass. A JIT renderer variant may exist under the JIT policy | Sensitive accounts |

**Recommendation** (inference). The renderer becomes a separate process no later than the first milestone that loads pages from the network beyond public documents.
- Retrofitting a process boundary is what made Firefox's Electrolysis and Fission take years (decision map §2, item 3).
- Under PM6, the boundary already exists as message interfaces, so splitting the process is mostly mechanical.
- The single-process mode remains for tests, fuzzing and debugging.

## 10. How this maps onto macOS

From the [macOS confinement note](../research/macos-process-confinement.md):

- **Executables.**
  - The browser executable has the hardened runtime and no sandbox. Later it gains the hardened runtime's resource-access entitlements for the camera and microphone. By TN3125's categories these are unrestricted (inference).
  - One helper executable per privilege set: `renderer`, `renderer-jit` (later), `network`, `gpu` (later) and `utility`. Each is signed ad hoc with the hardened runtime and its own entitlements. Only `renderer-jit` carries `allow-jit`, together with `single-jit`.
  - Whether helpers must be `.app` bundles is settled by S4.
- **Profiles.** The browser compiles each profile once, from the repository, and hands the bytecode to the helper, which applies it at the top of `main` (S4: 17 µs). Every profile except `renderer-jit`'s contains an explicit `(deny dynamic-code-generation)`, because default deny doesn't cover it (S4).
- **Graphics.** Frames cross processes as IOSurfaces sent as Mach ports. Renderers have no window-server connection.
- **The fallback** if S4 shows that custom profiles no longer apply is App Sandbox per helper (option O2), with the weaker guarantees stated in the note.

## 11. Sources

All web sources were checked on 2026-10-17. Local evidence is in the [macOS note](../research/macos-process-confinement.md) §9.

1. [Threat model](threat-model.md) and [macOS confinement note](../research/macos-process-confinement.md) (this step).
2. Chromium, "Process Model and Site Isolation": the site is "scheme plus eTLD+1"; locks are "always assigned before any content is loaded" and remain constant; jail and citadel checks; a "soft" process limit on desktop based on available memory; partial isolation on Android for sites where users enter passwords, use OAuth, or receive COOP headers. https://chromium.googlesource.com/chromium/src/+/HEAD/docs/process_model_and_site_isolation.md
3. C. Reis, A. Moshchuk, N. Oskov, "Site Isolation: Process Separation for Web Sites within the Browser", USENIX Security 2019, §4.1 and §5.3: process counts, per-renderer and total memory, the spare process, proxies for out-of-process iframes. https://www.usenix.org/conference/usenixsecurity19/presentation/reis
4. Chromium, "The Rule Of 2". https://chromium.googlesource.com/chromium/src/+/HEAD/docs/security/rule-of-2.md
5. [S2 results](../research/s2-results.md): idle and peak footprint of a window with a Metal device.
6. WHATWG HTML §2.7.3–2.7.4 and §8.1.2: `SharedArrayBuffer` can't leave its agent cluster; cross-origin isolation. https://html.spec.whatwg.org/multipage/
7. A. van Kesteren, "Opaque Response Blocking (ORB, aka CORB++)". https://github.com/annevk/orb
8. Chromium, "Site Isolation" (chromium.org): 10–13% overhead on desktop (Chrome 67); 3–5% on Android when isolating sites users log into (Chrome 77). https://www.chromium.org/Home/chromium-security/site-isolation/
9. WebKit, "Site Isolation" (docs.webkit.org): the site definition; "the entire representation of the FrameTree" in every process; "step 2" as of January 2025. WebKit bug 296260: a shared process for cross-site iframes "for performance reasons", committed 2025-07-22. https://docs.webkit.org/Deep%20Dive/SiteIsolation.html ; https://bugs.webkit.org/show_bug.cgi?id=296260
10. Ladybird, "Process Architecture": a fresh ImageDecoder process for each image. https://github.com/LadybirdBrowser/ladybird/blob/master/Documentation/ProcessArchitecture.md
