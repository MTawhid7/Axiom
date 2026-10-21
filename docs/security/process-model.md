# Process Model

| | |
|---|---|
| **Status** | **Binding** since ADR-0004's acceptance (2026-10-17). The owner accepted SI1–SI7 as staged (SI3 limited to L1 and non-sensitive L2; out-of-process iframes required for L3) and B1–B7 as initial baselines. Spike S4 measurements are folded in ([S4 results](../research/s4-results.md)). **Refined at Step 5's Gate C** (2026-10-20), with the owner's approval: S5's and S7's metadata rules, ImageIO's place, and B1's scope. **The final B1–B7** at Step 6's Gate B (2026-10-20), with [ADR-0019](../adr/0019-performance-methodology-and-budgets.md) (change record below). **Amended by the architecture review's Round 1** (the owner's dispositions, 2026-10-21; [the review](../review/README.md) §5): IR10 for RV1-16 (CP1); the GPU/compositor process, real processes from M1, renderers without IOKit and B1's split, for RV1-02, RV1-07 and RV1-09 (CP2); the GPU process's rules and its call before its profile, from S10 (2026-10-21); the signing identity for RV1-17 (CP5); B3's content tier and B6 for RV1-05 (CP6) |
| **Question** | Into which processes is Axiom split? What may each one do, how are sites assigned to processes, which rules govern IPC, and how much memory may each process use on the 8 GB M1? |
| **Feeds** | ADR-0004; the architecture proposal (Step 4): event loop, pipeline, embedder API; Step 5 subsystems; Step 6 budgets and roadmap |
| **Builds on** | [Threat model](threat-model.md); [macOS confinement note](../research/macos-process-confinement.md); [ADR-0001](../adr/0001-scope-and-target.md); [ADR-0002](../adr/0002-implementation-language.md); [ADR-0003](../adr/0003-gc-handle-and-rooting-pattern.md); [S2 results](../research/s2-results.md) |
| **Last updated** | 2026-10-21: Round 1's accepted findings (change record); S10's measurements (§3 note 1, §7's B1); the GPU process's rules from S10, approved (§4.1, §10) |

Everything here is **planned**; nothing is implemented. Claims about other browsers are sourced (§11). Budgets are **targets**, to be validated (§7).

## Change record

The model is binding, so it changes only with the owner's approval. Changes since it became binding with ADR-0004 (2026-10-17):

| Date | Change | Evidence | The owner's approval |
|---|---|---|---|
| 2026-10-20 | **Renderers, system fonts** (§4.1): read-only access also covers FontServices' `Resources` directory and its `Reserved` subtree, where PingFang, CoreText's fallback for Chinese, lives, with the metadata of each component of their path, symbolic links included. Without them CoreText silently resolves a stub font with no file | [S5 results](../research/s5-results.md) §3, by removal tests (informal for these rules) | Step 5's Gate C: "Process Model: Approved. Record the S5 and S7 sandbox metadata rules in the process model change record." |
| 2026-10-20 | **Renderers and the network process, the executable's directory** (§4.1): the metadata of the helper's own executable directory, one directory with no listing and no contents. Without it, `CFBundleGetMainBundle` fails inside the profile: in renderers ImageIO crashes, and CoreText does when it draws colour emoji; in the network process `SecPolicyCreateSSL` returns NULL, and verification fails closed. The kernel logs no denial for it | [S5 results](../research/s5-results.md) §7 (rule R1); [S7 results](../research/s7-results.md) §3.1, checked by removal | As above |
| 2026-10-20 | **The network process, `/var`** (§4.1): the metadata of the `/var` symlink, which DNS-SD's client needs to reach `/var/run/mDNSResponder`. The kernel logs no denial for it | [S7 results](../research/s7-results.md) §3.1, §5, checked by removal | As above |
| 2026-10-20 | **ImageIO in the media utility process** (§3, note 3; §4.2): ImageIO decodes AVIF there rather than in a renderer, because in a renderer it needs VideoToolbox's decoder service, a Mach service §4.1 denies renderers | [S5 results](../research/s5-results.md) §7; [images and media](../architecture/images-and-media.md) IM3 | Step 5's Gate C: "IM3's fallback (AVIF in the media utility process) is approved." |
| 2026-10-20 | **B1's scope** (§7): ≤ 60 MB now covers the core browser process state, and each visible window's tile surfaces get a budget of their own, by their pixel dimensions, which Step 6 sets. One window's compositor state had left about 4 MB of the old B1 for everything else | [S6 results](../research/s6-results.md) §7 (S6-Q5) | After Step 5's Gate C ([OQ-23](../open-questions.md)): "Approved. In Step 6, budget visible window tile memory separately based on surface pixel dimensions (e.g. 25.2 MB for this Retina window), and maintain a 60 MB budget for the core browser process state." |
| 2026-10-20 | **The final B1–B7** (§7), set by Step 6 as ADR-0004's D6 required: B1's tiles at ≤ 6.2 bytes per window pixel beside the core's 60 MB; B3's 50 MB median kept, with a ceiling for every corpus page, half of Chrome Canary 158's measured cost capped by tier; B6 bounded for sessions of document and content pages; B2, B4, B5 and B7 unchanged. The initial targets (2026-10-17) were B1 ≤ 60 MB in all, B3 ≤ 50 MB as a median, and B6 ≤ 1 GB for 10 tabs on about 6 sites, with B2, B4, B5 and B7 as now | [Performance budgets](../performance.md) §4 (PF6–PF12); [S6 results](../research/s6-results.md) §7; [S8 results](../research/s8-results.md) | Step 6's Gate B: "ADR-0019 (Performance Methodology and Budgets): Accepted as proposed, including the B1-B7 process model amendment (B1 tile scaling formula and B3 per-page memory ceilings)." |

| 2026-10-21 | **IR10, failure semantics** (§6.2): every executable aborts on a panic, so a helper's panic is a death IR10 handles, and the browser's validators are panic-free | The architecture review's [RV1-16](../review/round-1/02-findings.md#rv1-16-failure-semantics-panics-and-overflow) | Round 1's dispositions (2026-10-21), RV1-16: "Enforce panic = "abort" per binary; enforce checked arithmetic / overflow checks in parsers and IPC validators." |
| 2026-10-21 | **The GPU/compositor process** (§1; §3 and its note 1; §4.1; §4.2; §9; §10): renderer → GPU/compositor process → browser, from M2. The GPU process validates and rasterizes display lists into IOSurfaces it owns; the browser holds the layer tree, scrolls and animates layers, and routes input. At M1 the compositor runs as a browser thread behind the same interfaces. Web-supplied GPU work stays out of the compositor's process until M9's plan decides | The architecture review's [RV1-02](../review/round-1/02-findings.md#rv1-02-compositing-stays-in-the-unsandboxed-browser-through-l3); spike S10 measures the hop | Round 1's dispositions (2026-10-21), RV1-02: "Formally design the sandboxed GPU/compositor process topology now (Renderer → GPU Process → Browser CALayer tree)." "Build the separate GPU/compositor process at Milestone 2; split Budget B1 accordingly." "Ensure Level 3 (L3) security certification at Milestone 6 is performed on the final multi-process topology." |
| 2026-10-21 | **Real processes from M1** (§9): sandboxed renderer and network processes at L0; single-process mode only for automated tests, fuzzing and Miri | [RV1-07](../review/round-1/02-findings.md#rv1-07-milestone-1-in-one-process) | RV1-07: "Run real, sandboxed renderer and network processes from Milestone 1 using the profiles validated in Spikes S4, S5, and S7." "Retain single-process mode strictly for rapid automated testing, fuzzing, and Miri." |
| 2026-10-21 | **Renderers without IOKit** (§4.1, §10): pixels only in Mach memory entries; an explicit `iokit-open` denial in every renderer profile; every IOSurface owned by the compositor | [RV1-09](../review/round-1/02-findings.md#rv1-09-iosurfaces-in-renderers) | RV1-09: "Renderers write pixels into shared memory entries; only the compositor owns IOSurfaces; renderer profiles explicitly deny iokit-open." |
| 2026-10-21 | **The GPU/compositor process's rules, from S10** (§4.1): ten rules, each checked by removal: the browser's rendezvous; the metadata of the executable's directory, of `/var` and of the user's cache directory; IOKit's `AGXDeviceUserClient` and `IOSurfaceRootUserClient`; read access to the GPU's driver bundle and the shader compiler's identity; Metal's compiler service; Metal's shader caches, read and write; `hw.memsize`. **Its one call before its profile** (§10; ADR-0004 D2): it resolves the user's cache directory first. **The removal tests run among M2's checks** (TS14's V1), since five rules fail silently | [S10 results](../research/s10-results.md) §6, §8 | The owner, 2026-10-21: "GPU Process Sandbox Rules: Confirmed. The 10 rules identified in docs/research/s10-results.md §6.1 are adopted into docs/security/process-model.md §4.1. Incorporate the removal tests into Milestone 2's validation suite to catch silent framework failures." "GPU Process Cache Warm-Up: Approved. The GPU process may resolve its cache directory via confstr(_CS_DARWIN_USER_CACHE_DIR) immediately before applying its sandbox profile. This avoids widening the sandbox profile with opendirectoryd/dirhelper and avoids the 450 ms shader recompilation penalty on launch/restart. Update ADR-0004 D2 to document this single exception." |
| 2026-10-21 | **B1 split** (§7): B1a, the browser's core at ≤ 60 MB; B1b, the GPU/compositor process at ≤ 15 MB (hypothesis) plus the tiles' 6.2 bytes per window pixel | RV1-02; S6's measurements | RV1-02: "split Budget B1 accordingly" |
| 2026-10-21 | **The signing identity** (§10): a stable local identity, if S15 passes; ad hoc as the fallback | [RV1-17](../review/round-1/02-findings.md#rv1-17-the-signing-identity); Apple's TN3127 | RV1-17: "Validate local self-signed codesigning persistence via Spike S15." |
| 2026-10-21 | **B3's content tier and B6** (§7): the content pages gated on a normalized workload (to the 50th item) by a slope per 100 items and a plateau after a collection, 150 MB a stretch target; B6 derived again at M6's plan with out-of-process frames' renderers counted | [RV1-05](../review/round-1/02-findings.md#rv1-05-the-content-tiers-150-mb-cap) | RV1-05: "Replace the flat static 150 MB cap with a normalized workload (standardized 50-item scroll depth)." "Gate on memory growth slope per 100 items scrolled plus post-GC idle plateau recovery; retain 150 MB as a stretch target. Enforce zero-copy resource sharing." |
---

## 1. Summary

- **Five process types:**
  - a trusted **browser** process;
  - **renderers** that interpret web content, each locked to one site from security level L2;
  - one **network** process that holds cookies, the cache and web storage;
  - a **GPU/compositor** process, from M2: it validates every renderer's display lists, rasterizes tiles into IOSurfaces and hands the browser layer contents (Round 1, RV1-02);
  - **utility** processes for isolated jobs such as media decoding.
- **Confinement on macOS.** Every helper is a separate executable that applies its own deny-by-default Seatbelt profile before it touches input (option O1 in the [macOS note](../research/macos-process-confinement.md)). The browser process is unsandboxed but hardened, and it never parses web content. From M2 it consumes no renderer display lists either: it holds the CoreAnimation layer tree, scrolls and animates layers, and routes input (§3, note 1).
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
| **Browser** | 1 | Z1 (trusted) | Windows and browser UI (AppKit). Navigation and the frame tree, which it holds as the source of truth. Launching processes and assigning site locks. Permissions and tracking of user activation. Profile data other than web data (history, bookmarks, settings, permissions). Downloads, which it writes and marks as quarantined. **The presenter:** each window's CoreAnimation layer tree, with the GPU process's IOSurfaces as contents; scrolling and compositor animation on those layers; input routing by hit-test regions (note 1). At M1 the compositor also runs here, as a thread behind its own interface | Messages from children (safe Rust), the GPU process's layer commits among them; address-bar text. **No web content, and from M2 no display lists** |
| **Renderer** | Many | Z3 (untrusted) | Everything that interprets web content: HTML and CSS parsing, the DOM, style, layout, JavaScript (one heap per agent cluster, ADR-0003), image decoding, font parsing and shaping, and building display lists | All web content |
| **Network** | 1 | Z2 | HTTP(S), TLS, DNS; the HTTP cache; the cookie jar; web storage (`localStorage`, IndexedDB, Cache API); ORB-style response blocking; CORS and initiator checks against the requesting renderer's lock | All network bytes |
| **GPU/compositor** | 1, from M2 | Z2 | The Metal device: validates every renderer's display lists (PL9), holds their property trees and resources, rasterizes damaged tiles into IOSurfaces it owns, and sends the browser layer commits (note 1). **No web-supplied GPU work:** WebGL and WebGPU (M9) run where M9's plan places them | Display lists and resources from every renderer |
| **Utility** | On demand | Z2 or Z3 | Isolated jobs, first of all media decoding with system codecs. One instance per kind of job | Media streams and similar |

**Design notes.**

1. **The GPU/compositor process composites, from M2** (Round 1, RV1-02; the owner, 2026-10-21: "Formally design the sandboxed GPU/compositor process topology now (Renderer → GPU Process → Browser CALayer tree)." "Build the separate GPU/compositor process at Milestone 2; split Budget B1 accordingly." "Ensure Level 3 (L3) security certification at Milestone 6 is performed on the final multi-process topology.").
   - **The topology: renderer → GPU/compositor process → browser.** A renderer sends its display-list updates and resources (PL14, PL8) only to the GPU process. The GPU process validates them (PL9's checks, unchanged in content), holds the property trees, and rasterizes damaged tiles with Axiom's pipelines into IOSurfaces it owns (GR2–GR4). It sends the browser, per window, **layer commits:** the layer tree derived from property-tree nodes (GR2), each tile's IOSurface as a Mach port, the scroll and animation nodes those layers need (PL15), and the frames' hit-test regions (PL16).
   - **The browser** holds the CoreAnimation layer tree and sets the IOSurfaces as layer contents, a public interface (GR14). **It scrolls and animates layers itself,** so input moves a layer with no hop to another process (IN1, IN4), and it sends the new offsets to the GPU process, which rasterizes the tiles they expose (IV8), and to the renderer. It validates each layer commit as untrusted input (IR1): IDs it issued, counts, finite numbers, IOSurface sizes and formats, child frames against its frame tree (FT1), and each frame's hit-test regions within that frame's bounds. The browser keeps the display links and offers frames (EL19, EL21).
   - **Why** ([RV1-02](../review/round-1/02-findings.md#rv1-02-compositing-stays-in-the-unsandboxed-browser-through-l3), with its sources): until this change the browser consumed display lists until a GPU process arrived at M9, three milestones after L3's gate. A compromised renderer chooses their parameters, so a validator's logic bug, or a driver fault reached through them, would run with the owner's authority, and a GPU fault would end every window. Safari has drawn web content in its GPU process with UI-side compositing on macOS since 2023; Chrome composites in its GPU process (Viz); Ladybird moved its compositor into a process of its own in 2026.
   - **Where scrolling and animation run** (RV1-02's open choice): **in the browser, on layers,** which is what UI-side compositing means in WebKit (background). Input arrives in the browser, so moving the layer there costs no hop; rasterizing newly exposed tiles takes the hop, which the prepainted row hides (PF20). Rejected: scrolling in the GPU process, which would put two hops on every scroll frame (graphics §9).
   - **Staged in the implementation, not the topology.** At M1 the compositor's code runs as a browser thread behind the same interfaces (IP2); M2 moves it into the GPU process. So L3's checks at M6 run on the final topology, as the owner required.
   - **Web-supplied GPU work** (WebGL, WebGPU) never reaches the browser. It arrives at M9, and whether it shares the compositor's process or gets its own is M9's plan's decision. **Recommended: its own,** because the compositor's process holds every site's rendered pixels, which a shader-compiler exploit there would expose (inference; SG10).
   - **Measured by spike S10** (2026-10-21, [S10 results](../research/s10-results.md)): the browser applied each layer commit 48.8 µs at the median after the GPU process sent it, within [performance](../performance.md) PF13's 1 ms; validation cost about 9.4 ns an item; the GPU process sampled renderer memory entries without copies (RV1-09). Its rules in §4.1 and its one call before its profile (§10) are S10's, approved by the owner on 2026-10-21 (change record).
2. **Web storage lives with the cookie jar, in the network process,** as in WebKit, whose Networking process also hosts storage. That keeps renderer-supplied structured data out of the browser process. Step 4 may split storage into its own process if crash isolation or memory requires it.
3. **Images and fonts are decoded in the renderer,** with no separate decoder process per image (contrast Ladybird [10]).
   - A compromised decoder there yields only that site's data (from L2).
   - Safe Rust decoders are preferred. A system decoder (ImageIO) or CoreText runs only inside a renderer, the GPU process or, for ImageIO's AVIF, the media utility process (change record, 2026-10-20).
   - Images that the browser itself shows, such as favicons, are decoded with safe Rust decoders or in a renderer.
4. **Where glyphs are rasterized** (renderer or GPU process) is a Step 5 decision (I1, I2). S4-Q5 tests whether CoreText works inside a renderer's sandbox.

## 4. What each process may do

### 4.1 OS capabilities

"Rules from S4" means the exact sandbox rules are set by evidence from spike S4, starting from denial.

| Capability | Browser | Renderer | Network | GPU/compositor (from M2) | Utility (media) |
|---|---|---|---|---|---|
| User files | Yes, on user intent | **No** | **No** | **No** | **No** |
| Own data directory | Profile (settings, history) | None | Web data: cookies, cache, storage | Metal's shader caches only: `com.apple.metal` under the user's cache directory, read and write (S10) | None |
| Network sockets | Not used | **No** | Outbound TCP and UDP; the system DNS and certificate-trust services | **No** | **No** |
| Window server, AppKit | Yes | **No** | **No** | **No** | **No** |
| GPU (Metal, IOKit) | At M1 only, while the compositor runs in it. From M2, no Metal device: IOSurfaces only as layer contents | **No.** Pixels go only into Mach memory entries, and the profile denies `iokit-open` with an explicit rule (RV1-09) | **No** | Yes: Metal, through two IOKit user clients only, the GPU's (`AGXDeviceUserClient` on the M1) and IOSurface's (`IOSurfaceRootUserClient`); read access to the GPU's Metal driver bundle and the shader compiler's identity (`agxa_compiler_hash`); the sysctl `hw.memsize`. It owns every IOSurface the compositor uses (S10) | Hardware decode only |
| System fonts | Yes | Read-only: `/System/Library/Fonts`, FontServices' `Resources` directory and `Reserved` subtree, and ICU's data in `/usr/share/icu` (S4-Q5; S5, change record) | **No** | **No:** glyphs arrive rasterized (TX12) | **No** |
| Metadata of paths outside its own files | Yes | The executable's directory (S5, change record) | The executable's directory and `/var` (S7, change record) | The executable's directory, `/var` and the user's cache directory (S10) | Rules from S4; ImageIO needs the executable's directory (S5) |
| Pasteboard; camera, microphone, location, screen | Yes, mediated and prompted | **No** | **No** | **No** | **No** |
| Launch processes | Yes, its own helpers only | **No** | **No** | **No** | **No** |
| Executable memory (JIT) | **No** | **No**. Only the JIT renderer variant, under the JIT policy in ADR-0004 | **No** | **No** | **No** |
| Keychain | Yes, if secrets are stored | **No** | **No** | **No** | **No** |
| Mach service lookups | Unrestricted (not sandboxed) | The browser's rendezvous service only, plus logging | DNS, certificate trust, logging (rules from S4) | The browser's rendezvous service, and Metal's shader compiler, the XPC service `com.apple.MTLCompilerService` (S10); nothing else | Media services (rules from S4) |

### 4.2 Rule of 2 placement

| Work | Untrusted input | Memory-unsafe code | High privilege | Where it may run |
|---|---|---|---|---|
| Parsing web content and running script (safe Rust) | Yes | No (yes once a JIT exists) | — | Renderer |
| System image and font parsers (ImageIO, CoreText) | Yes | Yes | **Must not** | Renderer (or GPU process); ImageIO's AVIF in the media utility process (change record) |
| HTTP, TLS and DNS parsing | Yes | No if our Rust; yes if Apple's TLS stack | Medium | Network process: sandboxed either way |
| Decoding IPC in the browser | Yes | **No** (ADR-0002 rule 5) | Yes | Browser (two of three) |
| Consuming and rasterizing display lists | Yes | Yes: Apple's Metal driver and frameworks, reached with renderer-chosen parameters | **Must not** | The GPU/compositor process, from M2; the browser at M1 only (L0) (RV1-02). The old row's premise, "the Metal driver receives only our commands", bounded the commands, not their parameters |
| Applying the GPU process's layer commits | Yes | No | Yes | Browser (two of three): bounded layer data, decoded and validated in safe Rust |
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
| IR10 | **When a child dies,** the browser discards everything that child reported and recovers from its own state: a crash page in the tab, and a reload on request. **As amended in Round 1** (RV1-16): every executable aborts on a panic (ADR-0002's failure semantics), so a helper's panic is a death handled here, and a panic on one agent's thread ends the whole renderer, never one agent alone. The browser's validators can't panic (IP11); a panic elsewhere in the browser ends it. In single-process test mode, a panic on any thread ends the process: no recovery hides a defect |

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

**The final budgets** (Step 6's Gate B, 2026-10-20, [ADR-0019](../adr/0019-performance-methodology-and-budgets.md); the initial targets of 2026-10-17 are in the change record), measured as each process's physical footprint, under the conditions of the [performance budgets](../performance.md)' PF1–PF4. A page's cost includes the GPU and compositor memory allocated for it, charged to its renderer (PF4):

| # | Budget | Target | Basis |
|---|---|---|---|
| B1 | **Split in Round 1** (RV1-02; the owner: "split Budget B1 accordingly") | **B1a, the browser,** idle, one window: **≤ 60 MB** for the core browser process state. From M2 it holds no Metal device and no tiles, only the layer tree.<br>**B1b, the GPU/compositor process,** idle, one window: **≤ 15 MB** for its device, pipelines and display lists (**measured** by S10 at 13.4 MB with S6's page kept, [S10 results](../research/s10-results.md) §7; **kept by the owner,** 2026-10-21: "Budget B1b (15 MB): Approved. Keep the 15 MB baseline ceiling. Milestone 2 will measure display-list memory density per item and introduce per-window scaling if complex DOM trees exceed the margin."), **plus each visible window's tiles at ≤ 6.2 bytes per window pixel** while idle (25.4 MB at 2,560 × 1,600; 32.1 MB at 2,880 × 1,800), ≤ 7 bytes during updates, the spares freed within 1 s.<br>**At M1,** while the compositor runs in the browser, the browser holds both | S2's 20 MB, plus browser UI and profile. S6 measured the device and pipelines at 0.5 MB, one window's display lists at 6.3 MB, and its tiles at 6.14 bytes a pixel ([S6 results](../research/s6-results.md) §7); PF6. S10 measured which process macOS charges the tiles to: the GPU process that creates them, not the browser that holds them as layer contents |
| B2 | Renderer, sandboxed and ready, empty document | ≤ 10 MB | S4 measured a sandboxed helper at 1.14 MiB, and 4.05 MiB once CoreText has drawn text. Engine state and an empty heap come on top; PF7 |
| B3 | Renderer, typical page | **≤ 50 MB as the median over the documents and content tiers' corpus pages, and no corpus page above its ceiling:** half of Chrome Canary 158's measured cost, capped at 100 MB (documents), 150 MB (content), 300 MB (media and maps) and 300 MB (applications); the ceilings are listed in the [performance budgets](../performance.md) §4.1. **As amended in Round 1** (RV1-05): the content tier's pages are gated on a normalized workload by a slope and a plateau, with 150 MB as a stretch target | Chrome's 42 MB median, for comparable content in 2018 [3]; S8's census of the corpus ([S8 results](../research/s8-results.md)); PF8 |
| B4 | Network process, idle | ≤ 25 MB | Excludes the disk cache's mapped files. Chrome's took 13–16 MB on the automated corpus pages (S8); PF9 |
| B5 | Renderer soft limit on 8 GB | 16 processes | 16 × B3 = 800 MB at typical pages. A soft limit, as in Chromium [2]; PF10 |
| B6 | Whole browser, typical session | **≤ 1 GB for 10 tabs of document and content pages on about 6 sites;** sessions with media or application pages are bounded by each page's ceiling, B5 and discarding under memory pressure | B1a + B1b + B4 + about 7 renderers at B3, plus headroom; PF11 |
| B7 | Per-renderer cap | 2 GiB, a quarter of physical memory | Above it, the renderer is terminated and the tab shows an error (threat model SG11); PF12 |

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
| L0 | The browser, with **sandboxed renderer and network processes from M1** (Round 1, RV1-07), under S4's, S5's and S7's profiles; the compositor runs as a browser thread behind its interface. **Single-process mode exists strictly for automated tests, fuzzing and Miri** (PM6) | Earliest milestones: local test pages and public documents; never credentials |
| L1 | L0's processes hardened (audit tokens, V1's and V2's L1 parts, crash containment), and **the GPU/compositor process** (M2, RV1-02): the browser consumes no display lists | Browsing without logging in |
| L2 | L1 plus site locks and SI1–SI7, partitioned third-party state, ORB | Logins to accounts that aren't sensitive |
| L3 | **Out-of-process iframes in place.** The gate's checks V1–V9 pass. A JIT renderer variant may exist under the JIT policy | Sensitive accounts |

**Adopted in Round 1** (RV1-07; the owner, 2026-10-21: "Run real, sandboxed renderer and network processes from Milestone 1 using the profiles validated in Spikes S4, S5, and S7." "Retain single-process mode strictly for rapid automated testing, fuzzing, and Miri."). This section's earlier recommendation, that the renderer become a separate process no later than the first milestone loading pages beyond public documents, is overtaken: M1 already loads public documents live, through real processes. The reasons stand:
- Retrofitting a process boundary is what made Firefox's Electrolysis and Fission take years (decision map §2, item 3).
- Under PM6, the boundary already exists as message interfaces, so splitting the process is mostly mechanical.
- Process-specific defects (shared memory, crash recovery, silent sandbox denials, per-process memory) then surface during M1, not all at once at M2 ([RV1-07](../review/round-1/02-findings.md#rv1-07-milestone-1-in-one-process)).
- The single-process mode remains strictly for automated tests, fuzzing and Miri.

## 10. How this maps onto macOS

From the [macOS confinement note](../research/macos-process-confinement.md):

- **Executables.**
  - The browser executable has the hardened runtime and no sandbox. Later it gains the hardened runtime's resource-access entitlements for the camera and microphone. By TN3125's categories these are unrestricted (inference).
  - One helper executable per privilege set: `renderer`, `renderer-jit` (later), `network`, `gpu` (from M2) and `utility`. Each is signed ad hoc with the hardened runtime and its own entitlements. **As amended in Round 1** (RV1-17): with a stable local signing identity instead, if spike S15 passes, so that privacy decisions and Keychain approvals persist across rebuilds (ADR-0004 D2). **S15 passed** (2026-10-21; [S15 results](../research/s15-results.md)) for S4's probes, the JIT entitlements and the Keychain. Only `renderer-jit` carries `allow-jit`, together with `single-jit`.
  - Whether helpers must be `.app` bundles is settled by S4.
- **Profiles.** The browser compiles each profile once, from the repository, and hands the bytecode to the helper, which applies it at the top of `main` (S4: 17 µs). **The one exception** (ADR-0004 D2, after S10): the GPU/compositor process first resolves the user's cache directory (`confstr(_CS_DARWIN_USER_CACHE_DIR)`), so that Metal can use its shader caches inside the profile. Every profile except `renderer-jit`'s contains an explicit `(deny dynamic-code-generation)`, because default deny doesn't cover it (S4).
- **Graphics.** Renderers write pixels only into Mach memory entries and never touch IOKit (RV1-09). The GPU/compositor process renders tiles into IOSurfaces it owns and sends them to the browser as Mach ports, which the browser sets as layer contents (RV1-02). Neither renderers nor the GPU process connect to the window server.
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
