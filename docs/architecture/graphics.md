# Graphics: Rasterization, Compositing and Presentation

| | |
|---|---|
| **Status** | **Binding** since [ADR-0012](../adr/0012-graphics-images-and-media.md)'s acceptance (2026-10-20, Step 5's Gate C). Written at Step 5, CP4. **Spike S6** ([results](../research/s6-results.md)) measured GR2–GR6, GR8 and GR12: S6-Q1 and S6-Q3 pass at the median, in two formal runs; CoreAnimation honours the colour tags; S6-Q4 put GR5's amendment to the owner, who deferred it to the graphics milestone; S6-Q5's memory condition is not met as measured, so B1 is revised as the owner approved. **Amended by the architecture review's Round 1** (the owner's dispositions, 2026-10-21; [the review](../review/README.md) §5): the GPU/compositor process from M2, and no IOSurfaces in renderers, in GR1, GR2 and GR10–GR14 (RV1-02, RV1-09; CP2) |
| **Question** | What draws the display list: the GPU interface, path and glyph rasterization, compositing of property trees and effects, color management, damage tracking and partial presentation? Which animations run on the compositor, and how does all of it move into a GPU process later? |
| **Answers** | Step 5 question Q4 ([tracker](../research/step-5-subsystems.md) §3); decision-map item I2; the tracker's handed-over questions 3–6 (§2.2); [rendering pipeline](rendering-pipeline.md) open question 3; [invalidation](invalidation.md) open questions 2 and 3 |
| **Feeds** | ADR-0012 (CP9); spike S6 (tracker §10); CP5 (images, video, color profiles); CP14 (animations); CP15 (paint) |
| **Builds on** | [Rendering pipeline](rendering-pipeline.md) PL6–PL9, PL14–PL16; [invalidation](invalidation.md) IV6–IV9, IV11, IV12; [event loop and threading](event-loop-and-threading.md) EL6, EL19–EL22; [input and accessibility](input-and-accessibility.md) IN1–IN4; [embedder API](embedder-api.md) EM9; [S2 results](../research/s2-results.md); [text and fonts](text-and-fonts.md) TX12–TX15; the [dependency policy](../technology/dependency-policy.md) |
| **Last updated** | 2026-10-21: Round 1's accepted findings; S10's measurement of the hop (GR14, alternative G). Earlier: binding; S6's results in GR2, GR5, GR6 and GR12, with the colour check and the owner's decisions on B1 and GR5 |

Everything here is **planned**; nothing is implemented. Rules are numbered GR1–GR14. Precedent is sourced (§3); the design is inference unless marked otherwise.

## Summary

- **Metal directly, through `platform`** (GR1). macOS is the only target, so a portability layer would only add a large tree of `unsafe` code to the compositor.
- **The compositor runs in a sandboxed GPU/compositor process from M2** (GR14; Round 1, RV1-02): renderer → GPU process → browser. The GPU process validates display lists and rasterizes tiles into IOSurfaces; the browser holds the CoreAnimation layer tree, scrolls and animates layers, and routes input. At M1 the compositor runs as a browser thread behind the same interface.
- **The window server composites; Axiom draws tiles** (GR2). The compositor renders with Metal into IOSurfaces of its own, attached to CoreAnimation layers:
  - tiles, redrawn only where damaged;
  - a layer for each scroller, each compositor-animated subtree, each child frame and each video.

  Firefox measured large power savings from exactly this on macOS. WebKit and Chromium also present through CoreAnimation layer trees.
- **Display items are drawn by a small, fixed set of GPU pipelines** (GR3, GR4), compiled at first use from source (the Command Line Tools have no offline compiler) and kept in a binary archive. Drawing respects the M1's tile-based GPU: few passes, few intermediate textures.
- **Paths are rasterized in the renderer, into image resources** (GR5). S6-Q4 found GPU filling in the compositor faster and smaller, but coarser at the edges, so the amendment that would add path items to PL7 is the owner's decision.
- **Color:** content is converted to the window's working space, Display P3 at 8 bits per channel by default, which matches this Mac's panel (GR6). Higher precision and HDR come later, per layer.
- **Nothing runs while nothing changes** (GR7, GR8): no CoreAnimation commit and no Metal work without damage. Scrolling moves layers, and compositor animations change layer properties without redrawing.
- **Compositor animations** (GR9) cover the transform family and opacity first, then `filter`, `backdrop-filter` and scroll-driven timelines. Chromium's compositor animates the same set.
- **Canvas, WebGL, WebGPU and video** (GR10): canvas 2D rasterizes in the renderer, into a Mach memory entry; WebGL and WebGPU arrive at M9, never in the browser; video frames arrive as IOSurfaces and get layers of their own. **Renderers never touch IOKit** (RV1-09).

## 1. Binding inputs

| Input | Consequence here |
|---|---|
| **PL6–PL9:** a display list per frame, with property trees and items of validated primitives; the browser's compositor validates, then composites, embedding child frames by token. "Step 5 designs the rasterizer behind it" | GR3–GR5 |
| **PL14–PL16:** display lists arrive as updates through a ring; scroll offsets and compositor animations live in the property trees; hit-test data rides along | GR7–GR9 |
| **IV7, IV8, IV11, IV12:** the compositor redraws only damaged regions "where the graphics backend allows"; it scrolls and animates from its property trees, with display ports; hidden content does no rendering work; frames are produced only when offered and needed | GR2, GR7, GR8 |
| **EL6, EL19–EL22:** the compositor thread owns one display link per window, paused when idle, offers frames, and never waits for a renderer | GR7, GR11 |
| **Process model §3, note 1:** the browser composites until web-supplied GPU work (WebGL, WebGPU, hardware video) requires a GPU process. S2: an idle window with a Metal device costs 20–21 MB. **As amended in Round 1** (RV1-02): the GPU/compositor process composites from M2, and the browser holds the layer tree | GR1, GR10, GR12, GR14 |
| **S2:** the Command Line Tools have no offline Metal compiler. Shaders compile from source at run time, about 0.5 s the first time, and macOS caches the result ([open questions](../open-questions.md), OQ-7) | GR4 |
| **EM9:** a headless embedder with offscreen surfaces, for WPT and reference tests | GR11 |
| **TX12–TX15:** glyphs arrive as atlas entries rasterized in the renderer. A GPU process would receive glyph outlines as paths, never fonts | GR3, GR5 |
| **Dependency policy DP9, DP21:** a crate in the browser that handles data from renderers needs `ub-risk-1`; Metal and QuartzCore are platform facilities of the browser | GR1, GR5 |

## 2. Specifications

Rendering follows CSS's painting model, which CP15 owns:
- CSS Backgrounds and Borders;
- CSS Images: gradients;
- CSS Color 4 and 5: color spaces;
- Compositing and Blending;
- Filter Effects 1 and 2;
- CSS Transforms 1 and 2;
- CSS Overflow;
- CSS Masking;
- CSS Animations and Transitions, and Scroll-driven Animations.

The compositor implements their drawing operations; it never sees CSS itself.

## 3. Precedent

Read on 2026-10-18; dates in brackets are the last commits to each file, from GitHub's API.

| Engine | Graphics on macOS |
|---|---|
| **Firefox** | **"Dramatically reduced power usage in Firefox 70 on macOS with Core Animation"** (M. Stange, 2019-10-22) [F1]:<br>• **The problem:** the compositor "redrew the entire window" on every change, because OpenGL's `flushBuffer` "gives you no way to indicate which parts of the OpenGL context have changed".<br>• **The fix:** Firefox renders into IOSurfaces assigned to CALayers, keeps its own swap chain so it knows which pixels are stale, splits each window into square tiles, and separates opaque from transparent layers.<br>• **Measured,** on a 2013 MacBook Pro with Intel graphics, net of a 3.6 W idle baseline: scrolling 16.4 → 9.4 W; a spinning square 12.8 → 2.9 W; YouTube 720p60 28.4 → 16.4 W; an idle Google Docs 7.4 → 1.6 W.<br>**Today:** "screen updates are driven from a compositing thread", which creates layers, paints them and commits. During a window resize, "the modifications to the underlying CALayers need to happen on the main thread, once compositing is done" (`NativeLayerCA.h` [2026-10-06]) [F2]. On Apple Silicon, video gets its own CoreAnimation layers by default (`gfx.core-animation.specialize-video`) [F3]. Its WebGPU binds wgpu (`gfx/wgpu_bindings`) [F4] |
| **Chromium** | **Skia Graphite** shipped "in Chrome on Apple Silicon Macs" (2025-07-08) [C1]. It reaches Metal through Dawn, Chromium's WebGPU implementation. It uses the depth test to remove overdraw from opaque draws. It consolidates "the number of rendering pipelines as much as possible", so that they compile at startup. It records on several threads. MotionMark 1.3 rose "by almost 15%" on an M3 MacBook Pro. Its future work includes GPU compute path rasterization. Presentation builds CALayer trees (`ui/accelerated_widget_mac`, for example `ca_layer_tree_coordinator.mm` [2026-10-01]) [C2]. Its compositor can animate transform, scale, rotate and translate, opacity, filter, backdrop-filter, background color, bounds, scroll offset and custom properties (`cc/trees/target_property.h`) [C3] |
| **WebKit** | The UI process hosts a remote layer tree of CoreAnimation layers, with scrolling trees on macOS (`RemoteLayerTreeDrawingAreaProxyMac.mm` [2026-10-14]) [W1]. Its GPU process receives the drawing commands that web processes record (`RemoteDisplayListRecorder` [2026-10-07]) [W2] |
| **Vello** (Linebender) | Three renderers [V1]:<br>• `vello_cpu`, "the most mature choice when predictable CPU rendering" is needed;<br>• a GPU renderer that "preprocesses paths on the CPU and uses the GPU for rasterization and compositing", "intended to become the primary renderer for production GPU use cases as it matures";<br>• a compute renderer that "remains an experimental implementation" |

**Observations** (inference):
- **All three engines present through CoreAnimation layer trees on macOS.** Firefox's measurements show why: when the window server composites retained layers, a small change costs a small redraw, and scrolling can move content without redrawing it.
- **The engines rasterize page content in a GPU process,** or, in Firefox, in the compositor's process. Axiom's compositor lived in the browser when this was written, which constrained GR5; **as amended in Round 1** (RV1-02), it moves into the sandboxed GPU/compositor process at M2, and runs as a browser thread behind its interface at M1.
- **Chromium chose an abstraction over Metal (Dawn), but in a sandboxed GPU process** that also implements WebGPU. Axiom's compositor runs in its own sandboxed GPU/compositor process from M2 (Round 1, RV1-02), and macOS is its only platform.

**Apple's guidance:**
- On tile-based deferred rendering: tile memory "saves time and energy by avoiding accessing device memory as much as possible", with "energy consumption that's significantly less than accessing device memory". Multi-pass algorithms that store intermediate results "copy intermediate data to device memory many times" [A1].
- The SDK 27 header for `CALayer` describes `contents` as "typically a CGImageRef or an IOSurfaceRef" (observed).
- `CAMetalLayer` offers `displaySyncEnabled`, `presentsWithTransaction`, `maximumDrawableCount`, `colorspace`, `wantsExtendedDynamicRangeContent` and, since macOS 26, `residencySet` (observed).
- `MTLBinaryArchive` exists since macOS 11 (observed).
- IOSurfaces have purgeable states (`kIOSurfacePurgeableVolatile`, `kIOSurfacePurgeableEmpty`) (observed in `IOSurfaceRef.h`).

**This Mac** (observed 2026-10-18):
- `system_profiler SPDisplaysDataType` reports an Apple M1 GPU with 7 cores and "Metal 4" support, and a built-in Retina LCD at 2560 × 1600.
- Apple specifies the panel as "Wide color (P3)", 400 nits [A2].

## 4. Build or reuse

| Need | Candidates | Decision |
|---|---|---|
| **GPU interface** | **Metal through `objc2-metal`,** in `platform` (a required platform facility, and objc2 is already decided). **wgpu** 30.0.1, a portability layer whose `wgpu-hal` and `naga` would bring a large tree of `unsafe` code into the browser, with no published grade found. **Dawn,** C++ | **Metal directly** (GR1). wgpu stays a candidate for WebGPU's implementation in the GPU process (GR10) |
| **Presentation** | **CoreAnimation layers** with IOSurface contents (Firefox, Chromium, WebKit); **one `CAMetalLayer`,** redrawn each frame | **CoreAnimation layers** (GR2) |
| **Drawing display items** | **Our own pipelines** over Metal, since rendering orchestration is core under DP1 (c); Skia, C++; Vello's GPU renderer, via wgpu | **Our own** (GR3) |
| **Path rasterization** | **CPU in the renderer:** `vello_cpu` 0.3.0 or `tiny-skia` 0.12.0, or our own. **GPU in the compositor:** tessellation (`lyon`), sparse strips in the manner of Vello's GPU renderer, or our own [V1] | **CPU in the renderer for now; S6 measures both** (GR5) |

## 5. Design

### 5.1 The GPU interface and presentation

| # | Rule |
|---|---|
| GR1 | **Metal, directly, through `platform`.** The compositor issues Metal commands through Axiom's `platform` crate, which wraps `objc2-metal` in Axiom's own types. Its `unsafe` sits in allowlisted modules (ADR-0002) and is audited with the module. **No portability layer:** cross-platform support is a non-goal (charter §6), and a layer such as wgpu would add a large `unsafe` tree to the browser process, against DP9. Chromium's Dawn sits in a sandboxed GPU process. **As amended in Round 1** (RV1-02): Axiom's compositor runs in a sandboxed GPU/compositor process from M2, and stays without a portability layer there too, for the same reason (DP9) |
| GR2 | **The window server composites; Axiom draws into layers.** Each window's content is a tree of CoreAnimation layers whose contents are IOSurfaces that Axiom renders with Metal: from M2 the GPU/compositor process renders them, and the browser holds the layer tree (GR14). That is Firefox's design (§3), and WebKit and Chromium also present through layer trees. **Layers come from property-tree nodes:**<br>• scroll containers, which scroll by moving their content layer, without redrawing (IV8);<br>• subtrees whose transform or opacity the compositor animates (GR9);<br>• each child frame's output (FT12), so that frames in other processes compose without copies;<br>• video (GR10).<br>**Large layers are tiled,** so that damage redraws only the tiles it touches. The tile size is chosen in S6: **256-pixel tiles** were used throughout it and met its thresholds, and no other size was measured formally. Damage-limited tiles cost a spinner 46–54× less GPU time than full redraws, and a full redraw of a 2,560 × 1,600 window missed a 60 Hz frame (S6-Q1, S6-Q2). **Opaque layers are marked opaque,** and transparent ones kept separate, as Firefox does. **Rejected:** one `CAMetalLayer` redrawn whole each frame. It is what Firefox moved away from (§3), and it can't present a partial update |
| GR3 | **Display items are drawn by Axiom's own pipelines:**<br>• rectangles and rounded rectangles, with analytic anti-aliasing in the fragment shader;<br>• simple borders;<br>• box shadows;<br>• linear, radial and conic gradients;<br>• images from renderer resources;<br>• glyph quads from atlases, coverage or colour (TX13);<br>• rectangular clips by scissor, and rounded clips by shader;<br>• opacity groups, blend modes and filters, through intermediate textures only where the content requires them.<br>Draws are batched by pipeline, and opaque draws use the depth test to remove overdraw, as Graphite does (§3). This is rendering orchestration, core under DP1 (c), so it is Axiom's own |
| GR4 | **Few pipelines, compiled once, and frugal with memory bandwidth.** A small, fixed set of pipelines is compiled from source at first use, since there is no offline compiler (S2), and stored in an `MTLBinaryArchive`, so later launches skip compilation. Rendering respects the M1's tile-based GPU (§3): passes are merged where they can be, intermediate textures avoided, and load and store actions chosen so that tile memory isn't written out needlessly |

### 5.2 Paths

| # | Rule |
|---|---|
| GR5 | **Paths are rasterized in the renderer, into image resources, until the owner decides this rule's amendment.** SVG shapes, `clip-path`, complex border styles and, later, canvas 2D are filled on the CPU by a Rust rasterizer in the renderer, the same one TX12 uses for glyphs where S5 chooses it. They are sent as image resources (PL7 as written). **If S6 shows that GPU path filling in the compositor pays,** in time, memory or energy, an amendment to PL7 goes to the owner (SC1). It would add **path items:** fills and strokes of validated paths, with bounded point counts, finite coordinates, a fill rule and stroke parameters. The path code that would then run in the browser must be Axiom's own, or a crate graded `ub-risk-1` (DP9). With a GPU process, path filling moves there with the rest of rasterization, and glyph outlines join it (TX15).<br>**S6-Q4's report** (2026-10-20; [S6 results](../research/s6-results.md) §6, §8): filling on the GPU (stencil then cover, 4× multisampled) reached a first draw 3.9–4.8× sooner than `vello_cpu`'s filling plus upload, with 31–62× less memory, but its edges differ from analytic coverage by up to 109 of 255 levels. **It pays in time and memory, which made the amendment the owner's decision.** **Deferred to the graphics milestone** by the owner (2026-10-20): "Keep path rasterization in the renderer for now to preserve analytic edge quality. Defer the GR5 display list amendment to the graphics milestone." There it would come with a GPU filler whose edges match analytic coverage (sparse strips), measured on S6-Q4's contents. Once filled, a cached path image costs the compositor about what filling again would. This rule stands as written meanwhile, with `vello_cpu` as the renderer's rasterizer (TX12) |

### 5.3 Color

| # | Rule |
|---|---|
| GR6 | **One working color space per window,** Display P3 at 8 bits per channel by default, matching this Mac's panel (§3). Colors and images are converted to it: CSS colors in any CSS Color 4 space, and images by their embedded profiles (CP5). Blending and gradient interpolation follow the CSS specifications' rules for each case. **Later, per layer:** 16-bit floating-point layers for HDR images and video, through extended-range content, enabled only where such content appears, since they double a layer's memory. How CoreAnimation reads an IOSurface's color space is checked in S6. **S6-Q5:** CoreAnimation honours the tags. In the owner's screenshot of S6's pattern, the P3-tagged red is (255, 0, 0) in Display P3 and the sRGB-tagged red (234, 51, 35), exactly sRGB's red converted. **The untagged surface is not converted:** its values are taken as the display's own, so **every surface Axiom draws is tagged** with the window's working space ([S6 results](../research/s6-results.md) §7) |

### 5.4 Updates, idle and animation

| # | Rule |
|---|---|
| GR7 | **Damage-driven drawing** (IV7). Each display-list update (PL14) yields damage: changed items, property-tree nodes and child frames. Only tiles that damage touches are redrawn. The compositor then commits the layer changes in one CoreAnimation transaction per frame. **With no damage, there is no Metal work and no commit;** with nothing subscribed, the display link pauses (EL19, AC3) |
| GR8 | **Scrolling and animation change layers, not pixels.** A scroll moves a content layer within its scroller's clip, and only newly exposed tiles inside the display port are drawn (IV8). A compositor animation sets a layer's transform or opacity each frame, from the compositor's clock (EL22), without redrawing its tiles. **S6-Q3:** moving layers took 0.19–0.40 ms a frame at the median; frames that drew a newly exposed row took 1.5–2.2 ms, and up to 8.6 ms when the surface pool had to grow |
| GR9 | **Which animations run on the compositor:**<br>• **first,** the transform family (`transform`, `translate`, `rotate`, `scale`) and `opacity`;<br>• **next,** `filter` and `backdrop-filter`, applied to layer content by the compositor's filter pipelines, and scroll-driven timelines for any of these, sampled from the compositor's scroll offsets.<br>`background-color` and `clip-path` stay on the renderer, which repaints, until profiling shows that they matter. Chromium's compositor targets cover all of them (§3). This answers invalidation open question 2 |

### 5.5 Canvas, WebGL, WebGPU and video

| # | Rule |
|---|---|
| GR10 | **Web-supplied GPU work lives only in the GPU process** (process model §3):<br>• **Canvas 2D** rasterizes in the renderer, on the CPU, into a Mach memory entry (PL7), which the compositor samples like any image resource. **As amended in Round 1** (RV1-09): renderers never allocate or map IOSurfaces.<br>• **WebGL and WebGPU** arrive at M9 and never run in the browser (ADR-0004 D1). Whether they share the compositor's process or get their own is M9's plan's decision; **recommended: their own,** since the compositor's process holds every site's pixels (process model §3, note 1). WebGPU's implementation is decided at that milestone; Firefox binds wgpu (§3), which DP10's rules would then govern there.<br>• **Video frames** arrive from the media utility process as IOSurfaces (CP5), and get CoreAnimation layers of their own, as Firefox's video layers do on Apple Silicon (§3), so the window server composes them without a copy |

### 5.6 Threads, memory, tests and the GPU process

| # | Rule |
|---|---|
| GR11 | **Threads.** **As amended in Round 1** (RV1-02): in the GPU/compositor process, a raster thread validates updates and records Metal work ([event loop](event-loop-and-threading.md) §4.2); in the browser, the compositor thread applies layer commits in one CoreAnimation transaction per frame, scrolls and animates layers, and routes input (EL6). At M1 both run in the browser. **During a live resize,** the layer changes are committed on the main thread once compositing is done, as Firefox does (§3). **The headless embedder** (EM9): the GPU process composites the same layer tree into one offscreen texture with the same pipelines, and no CoreAnimation is used. So snapshots and reference tests need no window, and their output is deterministic (SC8) |
| GR12 | **Memory** (B1: the browser at ≤ 60 MB idle with one window; **split in Round 1** into B1a, the browser, and B1b, the GPU/compositor process, which holds the device, display lists and tiles from M2: process model §7).<br>• **Accounted:** the Metal device (S2: 20–21 MB with an idle window); tiles, bounded by display ports (IV8); the textures of renderer resources (PL8).<br>• **Size:** one full layer of a 2,560 × 1,600 window is about 15.6 MiB at 8 bits per channel.<br>• **Released:** tiles of hidden windows and offscreen content are released, or marked purgeable while they may come back (IOSurface's purgeable states; IV11).<br>• **Measured** in S6 (S6-Q5; [results](../research/s6-results.md) §7): the window 14.2 MB; Metal's device and pipelines 0.5 MB; one window's display lists 6.3 MB; its 96 visible tiles 25.2 MB of surfaces, 36.5 MB at that stage. Trimmed, that leaves about 4 MB of B1 for the rest of the browser. **B1 is revised,** as the owner approved (2026-10-20): Step 6 budgets a visible window's tiles by their pixel dimensions, and the core browser process keeps 60 MB. On this Mac's scaled display a full-screen window is 2,880 × 1,800 pixels.<br>• **Allocated ahead:** S6's slowest frames allocated surfaces that CoreAnimation hadn't yet released, so spares and a pool are allocated before they are needed (S6 results §8) |
| GR13 | **Robustness.** Every display list is validated before use (PL9). Texture and buffer sizes stay within Metal's limits, and an update that would exceed them is a violation (IP11). If GPU state is lost, the compositor rebuilds it, and renderers resend their resources on request (PL8). **If the GPU/compositor process dies** (from M2), the browser relaunches it and keeps each window's last layer contents meanwhile; renderers resend their display lists and resources (IR10). Without a GPU process, a GPU fault would end every window (RV1-02). **Round 2** (RV2-04): IR10 lists this recovery beside the renderer's and the network process's, and PF19's watchdog detects a hung GPU process |
| GR14 | **The GPU/compositor process from M2** (**as amended in Round 1**, RV1-02; the owner: "Formally design the sandboxed GPU/compositor process topology now (Renderer → GPU Process → Browser CALayer tree)."). Renderers send display lists and resources only to the GPU process. It validates them (PL9), holds the property trees, rasterizes damaged tiles into IOSurfaces it owns, and sends the browser **layer commits:** the layer tree, each tile's IOSurface as a Mach port (IP1), the layers' scroll and animation nodes (PL15), and the hit-test regions (PL16). **The browser** keeps the CoreAnimation layer tree, setting those IOSurfaces as layer contents, a public interface (§3), and it scrolls and animates layers itself (GR8), so input never waits on the GPU process. The display-list format and validation stay as they are. At M1 the same code runs as a browser thread behind the same interface (process model §3, note 1). Spike S10 measured the hop: 48.8 µs at the median from the GPU process's commit to the browser's ([S10 results](../research/s10-results.md) §3). **No Xcode is needed for any of this** (OQ-7): shaders compile at run time, binary archives are a run-time interface, and frame times come from command buffers' GPU timestamps |

## 6. Interfaces with neighbouring subsystems

| With | Interface |
|---|---|
| **Paint** (CP15) | Display items and property trees (PL6, PL7); hit-test regions (PL16); path images until GR5's amendment, if any |
| **Text** (CP3) | Glyph atlases as resources; glyph quads in display items (TX13, TX14) |
| **Images and media** (CP5) | Decoded pixels in Mach memory entries, with color profiles (RV1-09); video IOSurfaces from the media process |
| **Style and animations** (CP14) | Compositor animations: keyframes and timing for the properties in GR9, in the property trees (PL15) |
| **Input** (CP8 of Step 4) | Hit testing and scrolling from the same property trees (IN1–IN4) |
| **The shell** | The window's root layer, hosted in an AppKit view (EM6) |

## 7. Security

- **The compositor draws only validated primitives, with its own code.** The pipelines and the validation are Axiom's own safe Rust, over `platform`'s Metal calls.
- **As amended in Round 1** (RV1-02): **from M2 the compositor runs in the sandboxed GPU/compositor process,** because "Metal receives only Axiom's commands" bounds the commands, not their parameters, which a compromised renderer chooses. A validator's logic bug or a driver fault is then contained in a process without the owner's authority (process model §4.2). The browser consumes only layer commits: bounded data in safe Rust.
- **No web-supplied GPU work and no font parsing reach the browser** (GR10, TX15), and **renderers have no IOKit** (RV1-09).
- **Paths would bring more renderer-supplied data into the compositor,** so GR5's amendment requires Axiom's own path code, or a crate graded `ub-risk-1` (DP9). From M2 that code would run in the GPU process, not the browser.

## 8. Staging and the complete model

| Stage | What exists |
|---|---|
| First rendering milestone (M1) | The pipelines of GR3; tiled layers with damage; scrolling layers; transform and opacity animation; P3 at 8 bits. The compositor runs as a browser thread behind the GPU process's interface |
| M2 (Round 1, RV1-02) | **The GPU/compositor process:** validation, property trees, tile rasterization and IOSurfaces move into it; the browser keeps the layer tree, scrolling, animation and input |
| Later | `filter` and `backdrop-filter` animations, and scroll-driven timelines (GR9); canvas 2D in the renderer; video layers; HDR per layer; path filling in the GPU process if GR5's amendment is accepted |
| M9 | WebGL and WebGPU, in a process M9's plan places, never the browser (GR10) |
| L0–L3 | No interface changes. Child frames' layers compose by token at every level (PL9) |

## 9. Alternatives considered

| | Alternative | Assessment |
|---|---|---|
| A | **wgpu in the browser's compositor** | It would give portability, which Axiom doesn't need, at the cost of a large `unsafe` tree in the most privileged process, the browser, where the compositor ran when this was assessed (from M2 it runs in the GPU/compositor process: Round 1, RV1-02). Rejected for the compositor; still a candidate for WebGPU in the GPU process (GR10) |
| B | **One `CAMetalLayer` per window, redrawn each frame** | Simpler, but it has no partial presentation, and scrolling must redraw. Firefox's measurements show the cost (§3). Rejected (GR2) |
| C | **Skia,** as Chromium and Ladybird use | Mature and fast, but C++, in the browser process, for orchestration that DP1 makes core. Rejected |
| D | **Vello's compute renderer** | It is experimental by its own description (§3). Rejected for now; its GPU renderer is one of S6's candidates for path filling |
| E | **CoreAnimation's own animations** (`CAAnimation`) | They would run on the window server's clock, separate from the compositor's frame times (EL22) and from the property trees. Rejected: the compositor sets layer properties itself each frame (GR8) |
| F | **Scrolling and compositor animation in the GPU process** (Round 1) | One process would hold every property tree. But input arrives in the browser, which holds the layers, so every scroll frame would cross to the GPU process and back. Rejected for GR14: the browser moves layers, and only rasterizing exposed tiles takes the hop |
| G | **Two streams, as WebKit splits them** (Round 1): the layer tree from the renderer straight to the browser, and drawing to the GPU process | Lower latency for changes to layers alone, but two streams to keep consistent per frame, and the browser decoding renderer-built layer trees. Rejected for one ordered stream through the GPU process; revisited if S10 shows the second hop costs a frame. **S10 measured it at about 50 µs** ([S10 results](../research/s10-results.md) §3.5), so G stays rejected |

## 10. Open questions

| # | Question | Where it is settled |
|---|---|---|
| 1 | Path filling: CPU in the renderer, or GPU in the compositor (GR5), and the amendment to PL7 that the second needs | Spike S6, then the owner. **S6-Q4 reported** (2026-10-20): GPU filling pays in time and memory, not in quality. **Deferred** by the owner to the graphics milestone, with a filler of analytic quality (OQ-24) |
| 2 | The tile size, and the costs of GR2–GR4 on this Mac | Spike S6. **Reported:** 256-pixel tiles met the thresholds; the costs are in [S6 results](../research/s6-results.md) §3–§5 |
| 3 | How CoreAnimation reads an IOSurface's color space (GR6) | Spike S6. **Answered:** it honours Display P3 and sRGB tags exactly, and leaves untagged surfaces unconverted |
| 4 | B1 against the compositor's measured footprint (GR12) | **Decided** by the owner (OQ-23): a visible window's tiles are budgeted by their pixels in Step 6, and the core browser keeps 60 MB |
| 5 | WebGPU's implementation, and whether WebGL and WebGPU share the compositor's process (GR10) | M9's plan |

## Sources

All read on 2026-10-18. Dates in brackets are the last commits to each file, from GitHub's API.

- [F1] M. Stange, "Dramatically reduced power usage in Firefox 70 on macOS with Core Animation", Mozilla Gfx Team Blog (2019-10-22) — https://mozillagfx.wordpress.com/2019/10/22/dramatically-reduced-power-usage-in-firefox-70-on-macos-with-core-animation/
- [F2] Firefox, `gfx/layers/NativeLayerCA.h` and `NativeLayerCA.mm` — https://github.com/mozilla-firefox/firefox/blob/main/gfx/layers/NativeLayerCA.h
- [F3] Firefox, `modules/libpref/init/StaticPrefList.yaml` (`gfx.core-animation.specialize-video`) — https://github.com/mozilla-firefox/firefox/blob/main/modules/libpref/init/StaticPrefList.yaml
- [F4] Firefox, `gfx/wgpu_bindings` — https://github.com/mozilla-firefox/firefox/tree/main/gfx/wgpu_bindings
- [C1] "Introducing Skia Graphite: Chrome's rasterization backend for the future", Google (2025-07-08) — https://blog.google/chromium/introducing-skia-graphite-chromes/
- [C2] Chromium, `ui/accelerated_widget_mac` — https://github.com/chromium/chromium/tree/main/ui/accelerated_widget_mac
- [C3] Chromium, `cc/trees/target_property.h` [2022-09-08] — https://github.com/chromium/chromium/blob/main/cc/trees/target_property.h
- [W1] WebKit, `Source/WebKit/UIProcess/RemoteLayerTree/mac` — https://github.com/WebKit/WebKit/tree/main/Source/WebKit/UIProcess/RemoteLayerTree/mac
- [W2] WebKit, `Source/WebKit/GPUProcess/graphics/RemoteDisplayListRecorder.cpp` — https://github.com/WebKit/WebKit/blob/main/Source/WebKit/GPUProcess/graphics/RemoteDisplayListRecorder.cpp
- [V1] Vello, README [2026-09-14] — https://github.com/linebender/vello
- [A1] Apple, "Tailor your apps for Apple GPUs and tile-based deferred rendering" — https://developer.apple.com/documentation/metal/tailor-your-apps-for-apple-gpus-and-tile-based-deferred-rendering
- [A2] Apple, "MacBook Air (M1, 2020) - Technical Specifications" — https://support.apple.com/en-us/111883
- crates.io's API, for `wgpu`, `vello_cpu`, `tiny-skia` and `lyon` versions and dates — https://crates.io/
- Local: the SDK's QuartzCore, Metal and IOSurface headers; `system_profiler SPDisplaysDataType`
