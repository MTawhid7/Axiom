# Performance Methodology and Budgets

| | |
|---|---|
| **Status** | **Binding** since [ADR-0019](adr/0019-performance-methodology-and-budgets.md) was accepted (2026-10-20, Step 6's Gate B), with the final B1–B7 amending the [process model](security/process-model.md), as ADR-0004's D6 requires. Written at Step 6, CP6. **Every corpus page's ceiling** set from S8's measurements of Chrome, the eight measured by hand included (§4.1). **Amended by the architecture review's Round 1** (the owner's dispositions, 2026-10-21; [the review](review/README.md) §5): PF4, PF6 and PF13 for the GPU/compositor process and zero-copy resources (RV1-02, RV1-09; CP2); PF18 for RV1-18 (CP5); §4.1, PF11 and PF15 for the content tier's gate, the collector and the slot width (RV1-04–RV1-06; CP6). **Amended by Round 4** (the owner's dispositions, 2026-10-22; [the review](review/README.md) §5.9): PF19 and PF21 for RV4-03 (CP3); PF4 for RV4-04 (CP4) |
| **Question** | How is performance measured on this Mac, and what are the budgets: memory, time, energy and idle, and build? What are the limits and deadlines the designs left open, and how is each budget enforced? |
| **Answers** | Step 6 question Q4 ([tracker](research/step-6-standards-budgets-roadmap.md) §3); decision-map item X3; the tracker's handed-over questions 11–24 (§2.2) |
| **Feeds** | The process model's B1–B7 (CP7); the [testing strategy](testing.md)'s benchmarks (TS13); the roadmap (CP8); Milestone 1 (CP9) |
| **Builds on** | The process model §7 and its change record; [ADR-0002](adr/0002-implementation-language.md)'s T1–T4; spikes S1–S8, their methods and results; [graphics](architecture/graphics.md) GR12; the [event loop](architecture/event-loop-and-threading.md)'s EL16 and §4.7; the open questions of the [IPC](architecture/ipc.md), [invalidation](architecture/invalidation.md), [input](architecture/input-and-accessibility.md), [frame tree](architecture/frame-tree-and-navigation.md), [HTML](architecture/html-and-dom.md), [style](architecture/style.md), [layout](architecture/layout-and-paint.md), [JavaScript](architecture/javascript-engine.md), [bindings](architecture/bindings.md), [storage](architecture/storage-and-workers.md) and [images](architecture/images-and-media.md) documents; [script and heap](architecture/script-and-heap.md) SH10 |
| **Last updated** | 2026-10-22: Round 4's accepted findings |

Rules are numbered PF1–PF24. Every value is labelled with its evidence:
- **measured,** by a spike, named;
- **sourced,** from another engine's code or a specification, dated;
- **hypothesis,** naming the milestone that will measure it, as readiness criterion RC2 allows.

MB means 10⁶ bytes, as B1 and the spikes' results use it; MiB means 2²⁰ bytes.

## Summary

- **One method for every measurement** (PF1–PF5): S1–S8's conditions, made rules. Each budget becomes a benchmark or test with its threshold, checked at every milestone gate.
- **The final memory budgets,** accepted by the owner at Gate B (PF6–PF12):
  - **B1:** the core browser at 60 MB, plus the visible windows' tiles at 6.2 bytes per pixel (S6 measured 6.14), so 25.4 MB for a 2,560 × 1,600 window and 32.1 MB for a full-screen one here; **split in Round 1** (RV1-02) into B1a, the browser's 60 MB, and B1b, the GPU/compositor process's 15 MB (hypothesis) plus the tiles, from M2;
  - **B2:** a ready, empty renderer at 10 MB;
  - **B3:** a typical page at 50 MB, as the median of the documents and content tiers, with **a ceiling for every corpus page**, from S8's measurements of Chrome: 70–300 MB, at 12–50% of Chrome's cost;
  - **B4:** the network process at 25 MB;
  - **B5:** 16 renderers;
  - **B6:** a typical session at 1 GB;
  - **B7:** a renderer's cap at 2 GiB.

  A page's cost includes the GPU and compositor memory allocated for it, charged to its renderer: since Round 2, accounted by page in the GPU process, which macOS charges for it (PF4, RV2-05).
- **Time** (PF13–PF15):
  - S6's frame budgets;
  - no long task of the engine's own (none over 50 ms, the Long Tasks threshold);
  - the collector's pauses: 4 ms while the page is interacting, 50 ms in idle time.
- **Energy and idle** (PF16, PF17): no wakeups at all while a static page is idle; timers in hidden pages throttled as Chrome does; energy impact compared with Safari on the same page.
- **Build** (PF18): T1–T4, with the generated bindings counted.
- **Limits and deadlines** (PF19–PF24), each with its evidence:
  - hang deadlines and starvation bounds;
  - scrolling deadlines and display ports;
  - IPC's queues and timeouts;
  - the parser's time slices;
  - caches and strings;
  - storage quotas;
  - images' limits.

## 1. Binding inputs

| Input | Consequence here |
|---|---|
| **ADR-0004, D6:** B1–B7 are initial targets, set finally by Step 6; **the process model's change record** (2026-10-20): the owner approved B1's revision, "budget visible window tile memory separately based on surface pixel dimensions (e.g. 25.2 MB for this Retina window), and maintain a 60 MB budget for the core browser process state" | PF6–PF12 |
| **Charter G4:** efficiency is a primary goal, "with measurable budgets on the target machine"; **§7:** a fanless machine throttles, so benchmarks control for it | PF1, and every budget here |
| **ADR-0002:** the build thresholds T1–T4, each with a target and a level that triggers a revisit | PF18 |
| **The designs' open questions** left to Step 6: hang deadlines and starvation bounds (event loop 1, EL16, §4.7); listeners that may block scrolling, display ports and imprecise hit tests (invalidation 1, input 1); IPC's timeouts and queues, and whose budget pays for surfaces (IPC 3 and 4); the back-forward cache and `beforeunload` (frame tree 2 and 4); the parser's time budget and read-ahead (HTML 3); style's caches and the user-agent stylesheet (style 3, 4); strings' thresholds (JavaScript 3); the bindings' size (bindings 1); quotas (storage 2); image limits (images 4); parallel style and layout (style 6, layout 4) | PF19–PF24 |
| **The [testing strategy](testing.md):** budgets are enforced by benchmarks under S1–S8's conditions (TS13) | PF5 |

## 2. Evidence

- **Spikes on this Mac:**
  - S1 measured the stop-the-world collector: about 4 ms at 130,000 live nodes, 15 ms at 333,000 objects (OQ-13).
  - S2 measured an idle window with Metal at 20–21 MB, and S4 a sandboxed helper at 1.14 MiB (4.05 MiB once CoreText has drawn).
  - S6 measured compositing ([S6 results](research/s6-results.md)):
    - typical updates within 2 ms on the compositor and 4 ms on the GPU, at the median;
    - layer-only scrolling and animation within 1 ms;
    - a frame that draws a newly exposed row of tiles in 1.3–2.2 ms at the median, and at most 8.6 ms;
    - one window's tiles: 96 surfaces of 256 KiB, 25.2 MB.
  - S8 measured the corpus in Chrome Canary 158 ([S8 results](research/s8-results.md)).
- **Other engines' values,** read in their sources on 2026-10-20:
  - **Hangs:**
    - Chromium declares a desktop renderer hung after 15 s, with a 1 s ping timeout, "going any further with the modal dialog UI would be disruptive" [C1];
    - WebKit's responsiveness timeout is 3 s [K1];
    - Firefox stops a slow script after 10 s [F1].
  - **Hidden pages:** Chrome throttles their timers to once per second, and chained timers in pages hidden more than 5 minutes to once per minute [C2].
  - **Scrolling:** Firefox waits at most 400 ms for a page's listener before scrolling, and its display port extends 3.5 viewport heights vertically and 1.5 widths horizontally on desktop [F1].
  - **Parsing:** Firefox's HTML parser stays off the event loop for 3 ms at a time in interactive mode, and 30 ms in its load-time mode [F1].
  - **Images:** Firefox re-decodes an animation's frames once they exceed 20 MB, and caps its surface cache at a quarter of memory [F1].
  - **Strings:** V8's minimum length for a rope or a slice is 13 [V1].
- **Specifications:**
  - the Long Tasks draft: a long task is one "whose duration exceeds 50ms", so that input can be answered within 100 ms [L1];
  - the Storage Standard: `localStorage` and `sessionStorage` quotas of 5 × 2²⁰ bytes [W1].
- **Browsers' quotas** (MDN, last modified 2026-01-05, secondary) [M1]:
  - **Chromium:** 60% of the disk per origin;
  - **Firefox:** the smaller of 10% of the disk and 10 GiB per site, in best-effort mode;
  - **Safari:** about 60% per origin, and 80% overall.
- **This Mac** (observed 2026-10-20):
  - **memory:** 8 GB, with 49–71% free during S8's runs;
  - **disk:** a 245 GB disk with 24 GB available (`df -H /`);
  - **Mach's limits** (SDK headers): a port's queue defaults to 5 messages, `MACH_PORT_QLIMIT_MAX` is 1,024, and messages are reliable up to 256 KiB (`MACH_MSG_SIZE_RELIABLE`).

## 3. Method

| # | Rule |
|---|---|
| PF1 | **Conditions for a formal measurement,** S1–S8's practice made a rule:<br>• **build and power:** a release build; AC power with Low Power Mode off, checked with `pmset`;<br>• **load:** a load average below 3.0 at the start of each run, waiting up to 10 minutes, with the load average and memory pressure recorded before and after each run;<br>• **repetition:** a 60 s cool-down before each run; three runs, reporting the median with the range, and the 95th percentile for per-frame series;<br>• **the window and display:** a visible window of 1,280 × 800 points, with visibility checked at every sample, since a hidden page isn't composited and takes no input (S8, §2 of its results); the display mode and the appearance recorded;<br>• **the system:** the macOS build recorded.<br>**Evidence:** S6's second run began at a load average of 14.35, and its slow frames gathered in the first runs ([S6 results](research/s6-results.md)). S8 lost two runs to a hidden window |
| PF2 | **Tools,** all shipped with macOS or the toolchain:<br>• `footprint`: a process's physical footprint, and for several processes the total with shared memory counted once;<br>• `vmmap`: attribution;<br>• `top -c d`: CPU time, idle wakeups and energy impact, sampled once a second;<br>• `proc_pid_rusage`: in Rust harnesses;<br>• `samply`: profiles;<br>• Metal's GPU timestamps;<br>• `/usr/bin/time -l`: builds.<br>Instruments waits for Xcode (OQ-7). **A counter that reads zero for every process while they use CPU has failed,** and that sample is left out and reported (S8) |
| PF3 | **Reference engines** are measured with S8's tools on the same pages: automated where the corpus allows it, by hand otherwise (CO3, CO6). Their figures are context, frozen at the date measured, never targets to copy |
| PF4 | **What a page costs:** every process that serves it. That is its renderers (out-of-process frames included), plus **the GPU and compositor memory allocated on its behalf, charged to its renderer:** atlases, image textures, WebGL and WebGPU allocations, its share of tiles beyond the window's. **As amended in Round 1** (RV1-02, RV1-09): from M2 that memory lives in the GPU/compositor process, and a renderer's image counts once, since the compositor samples its memory entry in place where Metal allows (PL8). This answers the IPC design's open question 4. **As amended in Round 2** (RV2-05): S10 measured that macOS charges memory to the process that allocates it, with no API to move the charge of memory a renderer allocated: a renderer's memory entries stay on its ledger, while tiles, atlases and any blitted textures are the GPU process's ([S10 results](research/s10-results.md) §4, §7). So **the GPU process accounts by page:** it tags each allocation with the renderer and frame it serves, and reports the totals to the browser on request, through its control interface; a page's cost is its renderers' footprints plus that share. `IOSurfaceSetOwnershipIdentity`, which moves a surface's charge to another task's ledger, is tested at M2 as the alternative ([macOS runtime note](research/macos-runtime-facilities.md) §6.1). Costs are taken after loading, after the interaction, and after 30 s without input. **Budgets apply to the last, the steady state, and to the peak**.<br>**As amended in Round 4** (RV4-04): **the accounting by page counts what is mapped:** the resident pages of the renderer memory the compositor maps, read with `mach_vm_region` as spike S16 read them, so memory a dead renderer left behind shows in the totals instead of being charged to no process ([S16 results](research/s16-results.md) §3) |
| PF5 | **Enforcement.** Every budget below is a benchmark or test with that threshold (TS13), run under PF1 at each milestone gate that touches it. A missed budget fails the gate, unless the owner accepts a revision, which this document then records with its date and evidence |

## 4. Memory: the final B1–B7

| # | Budget | Value | Evidence |
|---|---|---|---|
| PF6 | **B1,** the browser process, idle, with one window; **split in Round 1** (RV1-02) into **B1a, the browser,** and **B1b, the GPU/compositor process** (process model §7) | **The core browser ≤ 60 MB, excluding tiles** (B1a; from M2 without the Metal device). **Each visible window's tiles ≤ 6.2 bytes per window pixel** while idle, and ≤ 7 bytes during updates, the spares freed within 1 s (B1b from M2, with ≤ 15 MB for the GPU process's device, pipelines and display lists: **measured** by S10 at 13.4 MB with S6's page kept, a margin that depends on the display lists' representation, [S10 results](research/s10-results.md) §7; **kept by the owner,** 2026-10-21: "Budget B1b (15 MB): Approved. Keep the 15 MB baseline ceiling. Milestone 2 will measure display-list memory density per item and introduce per-window scaling if complex DOM trees exceed the margin."). So 25.4 MB for a 2,560 × 1,600 window, against the 25.2 MB S6 measured, and 32.1 MB for a full-screen window in this Mac's display mode (2,880 × 1,800) | **Measured,** S6: 96 surfaces of 256 KiB, 25,165,824 bytes, for 4,096,000 pixels, is 6.14 bytes a pixel, 1.5 times the window's own surface, because tiles overhang its edges and frames have their own layers. With spares, 28.3 MB (6.9 bytes). The owner approved the split (2026-10-20). The core's 60 MB is unchanged: **hypothesis,** measured at the shell milestone |
| PF7 | **B2,** a renderer, sandboxed and ready, with an empty document | **≤ 10 MB** (unchanged) | **Hypothesis,** measured at the first renderer milestone. S4's helper took 1.14 MiB, and 4.05 MiB once CoreText had drawn; engine state and an empty heap come on top |
| PF8 | **B3,** a renderer with a typical page | **≤ 50 MB, as the median over the documents and content tiers' corpus pages** (unchanged), and **no corpus page above its ceiling** in §4.1 | **Hypothesis,** measured at each tier's milestone. §4.1 derives the ceilings |
| PF9 | **B4,** the network process, idle | **≤ 25 MB** (unchanged) | Chrome's network process took 13–16 MB on the corpus pages (S8, measured). **Hypothesis** for Axiom, measured at the network milestone |
| PF10 | **B5,** the soft limit on renderers | **16** (unchanged) | 16 × B3 = 800 MB; Chromium's limit is soft too ([process model](security/process-model.md) §7) |
| PF11 | **B6,** the whole browser in a typical session | **≤ 1 GB for 10 tabs of document and content pages on about 6 sites** (unchanged). Sessions with media or application pages are bounded instead by each page's ceiling, B5 and memory-pressure discarding | B1 + B4 + seven renderers at B3, plus the GPU process and headroom; from M2, B1a + B1b, since B1 is split (Round 1, RV1-02). **Hypothesis,** measured at the content milestone. **Round 1** (RV1-05): at L3 every cross-site frame is a renderer of its own, at least B2's 10 MB before content, so B6 is derived again at M6's plan with each corpus page's cross-site frames counted. B5's soft limit counts frames' renderers but never merges sites (SI6): memory pressure discards background tabs instead |
| PF12 | **B7,** the cap on one renderer | **2 GiB,** a quarter of physical memory. Past it, the renderer is ended and the tab shows an error | **Measured,** S8: the largest single Chrome renderer on the automated pages held about 0.5 GB (VS Code). On the pages measured by hand, Chrome's renderers together reached 1.3 GB (Figma, loaded) and 1.5 GB (Reddit, after scrolling), across five processes each; their single largest wasn't recorded. So the cap stops runaway pages without touching the corpus, by inference, with the least margin on Figma and Reddit. Unchanged |

### 4.1 Ceilings for the corpus pages

**The rule:** a page's steady-state cost in Axiom (PF4) is at most **half of Chrome Canary 158's** for the same page, as S8 measured it on 2026-10-20, and at most **its tier's cap**:
- documents 100 MB;
- content 150 MB, **a stretch target since Round 1** (RV1-05): the content pages are gated by a slope and a plateau on a normalized workload (below);
- media and maps 300 MB;
- applications 300 MB.

Values are rounded down to 5 MB. They are fixed numbers: they don't move with Chrome's later versions (PF3).

**Why half of Chrome:**
- **Charter G4:** efficiency is a primary goal.
- **S8 shows costs in Chrome that Axiom's design avoids** (inference):
  - a document for each SVG image, 150 on Wikipedia's article ([S8 results](research/s8-results.md) §4);
  - GPU processes holding several hundred MB on GPU pages (§5 of the results).

  Axiom rasterizes an SVG image once and keeps its source (to be designed at the images milestone), and charges GPU memory to the page (PF4).
- **The JavaScript heap is the risk.** VS Code's was 66.9 MB in V8 (S8). Axiom's heap layout may hold the same objects in more memory than V8's compressed pointers do (hypothesis, measured at the applications milestone). **As amended in Round 1** (RV1-06): measured before M3, by spike S11, which compares 32-bit heap slots with 64-bit NaN-boxed ones on heaps shaped like the corpus's ([the JavaScript engine](architecture/javascript-engine.md) JE9).

**Why the caps:**
- B3's median of 50 MB over the documents and content tiers needs typical pages far below their ceilings.
- The caps bound the outliers, so B6's 1 GB session holds.

| Page | Tier | Chrome's steady-state cost above about:blank (S8, measured) | Axiom's ceiling | Share of Chrome's cost | Measured at (CP8 places it) |
|---|---|---|---|---|---|
| Wikipedia | Documents | 319 MB | **100 MB** (the tier's cap) | 31% | The documents milestone |
| MDN | Documents | 145 MB | **70 MB** | 48% | The documents milestone, then script for its examples |
| Google Search | Content | 582 MB, signed in, by hand | **150 MB**, a stretch target; gated by slope and plateau (Round 1) | 26% | The content milestone |
| Instagram | Content | 578 MB, signed in, by hand | **150 MB**, a stretch target; gated by slope and plateau (Round 1) | 26% | The content milestone |
| Reddit | Content | 1,260 MB, signed in, by hand | **150 MB**, a stretch target; gated by slope and plateau (Round 1) | 12% | The content milestone |
| YouTube | Media | 908 MB, signed in, by hand | **300 MB** (the tier's cap) | 33% | The media milestone |
| Twitch | Media | 401 MB, signed in, by hand | **200 MB** | 50% | The media milestone |
| Google Maps | Media and maps | 627 MB | **300 MB** (the tier's cap) | 48% | The milestone that brings WebGL (M9) |
| Discord | Applications | 427 MB, signed in, by hand | **210 MB** | 49% | Its milestone, signed in from L2 with a test account |
| Three.js | Applications | 470 MB | **235 MB** | 50% | The milestone that brings WebGPU |
| Figma | Applications | 1,111 MB, signed in, by hand | **300 MB** (the tier's cap) | 27% | Its milestone, signed in from L2 with a test account |
| VS Code for the Web | Applications | 457 MB | **225 MB** | 49% | The applications milestone |
| Prism | Applications | 494 MB, signed in, by hand | **245 MB** | 50% | Its milestone, signed in from L2 with a test account |

**Where the figures come from:**
- **The five automated pages:** logged out, in Chrome Canary 158.0.8112.0 ([S8 results](research/s8-results.md) §5).
- **The eight pages measured by hand:** signed in, by the owner, in Chrome Canary 158.0.8113.0, the next build (§7 of the results).

  Their baseline is a fresh Canary on about:blank, measured after the session, so their costs include the dedicated profile's own state.

**Every ceiling is a hypothesis** (RC2), measured at the milestone named, under PF1. A miss goes back to the owner as a revision (PF5).

**The caps bind on seven pages** (for the content tier, superseded in Round 1 by the gate below; the paragraph records Gate C's reasoning). On the content pages they bind hardest: signed-in feeds cost as much as applications in Chrome, so their ceilings are 12–26% of Chrome's cost, against about half elsewhere. ADR-0019 names that case among its conditions for revisiting ("S8's session by hand shows a corpus page's Chrome cost to differ from its assumed tier cap"), so it went to the owner at Gate C. **The owner kept the cap** (2026-10-20): "Keep the 150 MB ceiling as the baseline hypothesis for review." So the ceilings above stand as the accepted rule gives them, and the architecture review examines the cap ([OQ-25](open-questions.md)).

**The content tier, as amended in Round 1** (RV1-05; the owner: "Replace the flat static 150 MB cap with a normalized workload (standardized 50-item scroll depth)." "Gate on memory growth slope per 100 items scrolled plus post-GC idle plateau recovery; retain 150 MB as a stretch target. Enforce zero-copy resource sharing."). The finding: Reddit's runs in Chrome spread from 495 to 1,631 MB with the depth of the owner's scrolling, a feed keeps whatever its own script holds, and a flat cap at an uncontrolled depth measures the workload more than the engine ([RV1-05](review/round-1/02-findings.md#rv1-05-the-content-tiers-150-mb-cap)). So **Google Search, Instagram and Reddit are gated on a normalized workload, by a slope and a plateau:**
- **The workload:** load the page; scroll to its 50th item (a result or a post); wait 30 s without input. By the owner's hand for the pages measured by hand (CO6), with the items reached recorded.
- **The slope:** memory growth per 100 items scrolled, from the cost after load to the cost at the 50th item, **at most half of Chrome's slope** on the same workload. Chrome's slope comes from the census addendum the owner authorized before M4: memory-infra traces on the normalized feeds ([corpus](corpus.md) §10), which also split Chrome's cost into its heap, DOM, images, compositor and GPU, for a budget per component.
- **The plateau:** after a collection and 30 s without input, the page's cost **changes by less than 2% over a further 30 s, and is no higher than at the 50th item:** garbage from scrolling is reclaimed, and nothing grows while the page is idle.
- **150 MB stays as a stretch target** at the 50th item, reported at each gate, not a pass condition.
- **Zero-copy resource sharing is a rule** (PL8, RV1-09): a decoded image is held once, in the renderer's memory entry, which the compositor samples in place.

The documents, media and applications tiers keep their ceilings as above.

**Safari's figures are context,** and don't change the ceilings. They were measured logged out, by hand, with the owner's advanced tasks, and the ceilings come to 16–54% of them ([S8 results](research/s8-results.md) §7.4).

## 5. Time

| # | Budget | Value | Evidence |
|---|---|---|---|
| PF13 | **Frames,** at 60 Hz (16.7 ms) | Compositor ≤ 2 ms and GPU ≤ 4 ms for a typical update; ≤ 1 ms for frames that only move layers; **from M2** (Round 1, RV1-02), the hop from the GPU process's layer commit to the browser's CoreAnimation commit ≤ 1 ms at the median (**measured,** S10: 48.8 µs at the median, 118.7 µs at the 99th percentile, [S10 results](research/s10-results.md) §3), and frames that only move layers stay in the browser; the main thread's rendering update (style, layout, paint) for a typical update ≤ 8 ms, leaving the rest of the frame to script | **Measured,** S6-Q1 and S6-Q3 for the compositor. The main thread's 8 ms is a **hypothesis,** measured at the layout milestone |
| PF14 | **No long tasks of the engine's own** | No task, rendering update or pause of the engine's own over 50 ms while a page is interactive. Work that could exceed it is split, or done in idle time | **Sourced:** the Long Tasks threshold, chosen so that input gets an answer within 100 ms [L1] |
| PF15 | **The collector's pauses** | **≤ 4 ms** while the page is interacting (input, scrolling or animation within the last second); **≤ 50 ms** in idle time. A corpus page that breaks either is the trigger for SH10's incremental marking, then its nursery. **As amended in Round 1** (RV1-04; the owner: "Fix the GC end-state before Milestone 3 via an amendment to ADR-0008: generational nursery, allocation-driven incremental marking, parallel marking in pauses, and non-moving tenured arenas with selective evacuation." "Evaluate pause times on a 1M-object heap in Spike S11."): the collector's end-state is fixed before M3 instead of waiting for a trigger (SH10), and spike S11 measures pauses on a heap of a million objects against these bounds | **Measured:** S1's stop-the-world pauses, 4 ms at 130,000 live nodes. S8's largest DOM had 45,815 nodes in Chrome (Wikipedia), about a third of that, though a page's heap holds its script's objects too (inference). The idle bound is PF14's |

## 6. Energy and idle

| # | Budget | Value | Evidence |
|---|---|---|---|
| PF16 | **No wakeups while idle** | A loaded static page with no timers or animations: **no idle wakeups** in its renderer over 30 s, and none in the browser or network processes beyond what the system's own events cause. Pages with their own timers: no wakeups beyond what they schedule, coalesced with the system's timer leeway | **Measured** possible: S6's compositor and S7's network process had no idle wakeups in most windows (AC3, PM8). Chrome's browser process used 0.45–0.9 s of CPU in every 30 s window of S8, whatever the page (measured) |
| PF17 | **Hidden pages and energy impact** | Hidden pages: no rendering; timers once a second; chained timers once a minute after 5 minutes hidden and 30 s silent; video paused unless audible. Pages that animate or play: **energy impact no higher than Safari's** on the same page | **Sourced:** Chrome's throttling [C2]. Safari's figures for ten pages, logged out, come from S8's corrected session by hand, under the owner's advanced tasks ([S8 results](research/s8-results.md) §7.4); **hypothesis** until the media milestone measures both with the same interaction |

## 7. Build

| # | Budget | Value | Evidence |
|---|---|---|---|
| PF18 | **T1–T4,** with the generated code counted | T1 incremental rebuild 5 s (revisit above 30 s); T2 clean build 5 min (20 min); T3 compiler memory 3 GiB (5 GiB); T4 one `rustc` 1.5 GiB (3 GiB). **The bindings generator's output counts in all four,** which settles the bindings' open question 1. **Round 1** (RV1-18): spike S13 measures generated bindings before M3, split by interface group, and style's generated code is measured at M1's third checkpoint | ADR-0002, accepted 2026-10-17 |

## 8. Limits and deadlines

| # | Area | Value | Evidence |
|---|---|---|---|
| PF19 | **Hangs and starvation** (event loop §4.7, EL16) | The browser pings each renderer's control thread once a second. **First deadline, 5 s** without acknowledging input: a non-modal offer to stop the page. **Second deadline, 30 s,** unless the owner chose to wait: the renderer is ended and the tab shows the hang page. **Starvation bounds:** continuous input can delay a rendering update by at most two frames (33 ms); rendering and input can delay ordinary tasks by at most 50 ms; idle tasks only in idle periods, with `requestIdleCallback`'s timeouts honoured. **As amended in Round 2** (RV2-04): **every helper is watched too,** the network process from M1 and the GPU/compositor process from M2: the browser pings its control thread once a second, and a helper that doesn't answer for **5 s** is ended and recovered as its death would be (IR10). A hung helper stalls every tab it serves, so there is no offer to wait.<br>**As amended in Round 4** (RV4-03): **hangs are detected by task age as well as by input.** Every watched thread records when its current task began, one atomic store per task: each event loop; the network process's I/O and storage threads; from M2, the GPU process's threads. The control thread's reply to each ping carries the age of the oldest running task. **A renderer:** at 5 s of task age, the offer to stop the page, in its tab, shown when the tab is visible, and at once if another agent waits on a Web Lock it holds; at 30 s, ended as above. **A helper:** a thread whose task passes 5 s ends and recovers the helper (IR10), as an unanswered ping does. So a loop that never yields is caught with no input pending, and so is a stalled storage thread whose control thread still answers | **Sourced:** the engines' 3 s, 10 s and 15 s [K1, F1, C1]; the 50 ms of PF14. The 5 s and 30 s are a **hypothesis,** tuned at the event loop milestone |
| PF20 | **Scrolling** (invalidation 1, input 1) | A listener that may block scrolling has **400 ms** to call `preventDefault()`, after which the scroll proceeds. An imprecise hit test waits at most one frame for the main thread's answer, then routes the event there asynchronously. **The display port** prepaints **one row of tiles** beyond each edge of the viewport, within B1's tile budget, not Firefox's 3.5 viewport heights, because drawing a newly exposed row costs 1.3–2.2 ms at the median (S6-Q3) | **Sourced:** Firefox's 400 ms and 3.5 [F1]. **Measured:** S6-Q3's row cost. **Hypothesis** for the row margin, measured at the scrolling milestone |
| PF21 | **IPC** (IPC 3) | Receive ports that carry input or display lists take **1,024** queued messages (the maximum); control endpoints take **16**; every other endpoint takes the default, **5**. **The browser's backlog** for a child whose queue is full is at most 1,024 messages or 8 MiB, past which the child is treated as hung (PF19). **As amended in Round 2** (RV2-13): the network and GPU/compositor processes keep the same bound for each renderer they serve, past which they report it to the browser as hung; a child's own backlog of messages to a full queue is bounded the same way, past which it stops producing (IP16). **Synchronous requests** (cookies, storage, synchronous XHR) have no timeout of their own. **As corrected in Round 4** (RV4-03): their answer is bounded by PF19's task-age deadline over the thread that answers them, the network process's storage or I/O thread, and by the caller's own task age; the control-thread ping alone watched neither. A message beyond **256 KiB** goes out of line (IP14) | **Observed:** Mach's limits in the SDK headers (§2) |
| PF22 | **Navigation** (frame tree 2 and 4) | **The back-forward cache** holds at most **2 documents per tab**, within B6, and is emptied first under memory pressure. **`beforeunload`** is dispatched to every frame's renderer at once, and each has **1 s** to answer; one that doesn't is treated as not asking, so a page can't hold navigation hostage | Firefox sizes its cache from available memory [F2]; the 2 and the 1 s are a **hypothesis,** measured at the navigation milestone |
| PF23 | **The engine's own costs** (HTML 3, style 3 and 4, JavaScript 3, bindings 1, style 6, layout 4) | **The parser** yields after **4 ms** while the page is interactive and **30 ms** during the initial load, and reads ahead at most **64 KiB** of input speculatively. **Style's caches:** sizes set at the style milestone, within B3. **The user-agent stylesheet** is parsed at build time if parsing it at launch takes more than 1 ms. **Strings:** ropes and slices from length **13**, as V8. **Parallel style and layout** only if profiling on the corpus shows the main thread over PF13's 8 ms | **Sourced:** Firefox's 3 ms and 30 ms [F1]; V8's 13 [V1]. The rest is **hypothesis,** settled at each subsystem's milestone |
| PF24 | **Storage and images** (storage 2, images 4) | **Quotas:** `localStorage` and `sessionStorage` at 5 MiB, as the Storage Standard sets them [W1]. Every other bucket per site: the smaller of 10% of the disk, 10 GiB and half the free space at the time of writing. All sites together: half the free space. Eviction is least recently used, by site, sparing persistent buckets. **As amended in Round 2** (RV2-12): a bucket belongs to a storage key (SW1), so the quota per site is the total for all that site's keys, and each key's buckets count against it. **Images:** at most 32,767 pixels on a side and 2²⁸ pixels in all; decoded frames of one animation at most 20 MB in memory at a time, others re-decoded on demand; a decoded image's memory counts against its renderer's budget (PF4). An image past a limit is a decode error (IM10) | **Sourced:** the Storage Standard; Firefox's 10% and 10 GiB, and its 20 MB for animations [M1, F1]. **Observed:** this Mac's disk has 24 GB free (§2), so free space, not disk size, bounds quotas. The image dimensions are a **hypothesis,** checked at the image milestone |

## 9. Open questions

1. **The content tier's cap** (§4.1): it binds at 12–26% of Chrome's cost on the three content pages, signed in. ADR-0019's condition for revisiting was met, and the owner kept the cap at Gate C as the baseline hypothesis for review ([OQ-25](open-questions.md)). **Answered in Round 1** (RV1-05): the content pages are gated by a slope and a plateau on a normalized workload, with 150 MB a stretch target (§4.1); Chrome's slope comes from the census addendum, before M4.
2. **Safari's figures:** measured with the corrected tool for the ten pages that work logged out (S8 results §7.4), so PF17 has Safari's energy impact for those pages. They were taken under the owner's advanced tasks, so the media milestone measures Safari again beside Axiom, with the same interaction.
3. **Values marked hypothesis** are measured at the milestones named. A miss returns to the owner as a revision (PF5).

## Sources

All read on 2026-10-20. Dates in brackets are the last change to each file, where known.

- [C1] Chromium, `components/input/input_constants.h` (`kHungRendererDelay`, `kHungRendererPingTimeout`) — https://github.com/chromium/chromium/blob/main/components/input/input_constants.h
- [C2] Chrome for Developers, "Heavy throttling of chained JS timers beginning in Chrome 88" [2021-01-18] — https://developer.chrome.com/blog/timer-throttling-in-chrome-88
- [K1] WebKit, `Source/WebKit/UIProcess/ResponsivenessTimer.h` (`defaultResponsivenessTimeout = 3_s`) — https://github.com/WebKit/WebKit/blob/main/Source/WebKit/UIProcess/ResponsivenessTimer.h
- [F1] Firefox, `modules/libpref/init/StaticPrefList.yaml`: `dom.max_script_run_time`, `apz.content_response_timeout`, `apz.y_stationary_size_multiplier`, `apz.x_stationary_size_multiplier`, `content.sink.interactive_parse_time`, `content.sink.perf_parse_time`, `image.animated.decode-on-demand.threshold-kb`, `image.mem.surfacecache.size_factor` — https://github.com/mozilla-firefox/firefox/blob/main/modules/libpref/init/StaticPrefList.yaml
- [F2] Firefox, `modules/libpref/init/all.js`, `browser.sessionhistory.max_total_viewers` — https://github.com/mozilla-firefox/firefox/blob/main/modules/libpref/init/all.js
- [V1] V8, `src/objects/string.h` (`ConsString::kMinLength`, `SlicedString::kMinLength`) — https://github.com/v8/v8/blob/main/src/objects/string.h
- [L1] W3C, *Long Tasks API*, Editor's Draft, 18 October 2026, §2 — https://w3c.github.io/longtasks/
- [W1] WHATWG, *Storage Standard*, last updated 2026-10-19, §4.1 — https://storage.spec.whatwg.org/
- [M1] MDN, "Storage quotas and eviction criteria" [2026-01-05] (secondary) — https://developer.mozilla.org/en-US/docs/Web/API/Storage_API/Storage_quotas_and_eviction_criteria
