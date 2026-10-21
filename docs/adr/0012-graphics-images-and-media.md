# ADR-0012: Graphics, images and media

| | |
|---|---|
| **Status** | **Accepted** (2026-10-20), by the owner at Step 5's Gate C, with IM3's fallback: AVIF decoded in the media utility process. Of the Gate B conditions, S6-Q1 and S6-Q3 pass at the median, in two formal runs; S6-Q5's colour condition is met, by the owner's screenshot, and its memory condition is not met as measured, so B1 is revised as the owner approved; S5-Q5's was not met ([S6 results](../research/s6-results.md); [S5 results](../research/s5-results.md) §7). GR5's amendment is deferred to the graphics milestone. **GR1, GR2, GR10–GR14 and IM1 amended by the architecture review's Round 1** (2026-10-21): the GPU/compositor process from M2; no IOSurfaces in renderers (RV1-02, RV1-09); IM2's JPEG decoder decided at M1's plan (RV1-22) |
| **Date** | 2026-10-18 |
| **Deciders** | Project owner (only the owner can move an ADR to Accepted) |
| **Related** | [Graphics](../architecture/graphics.md) (GR1–GR14); [images and media](../architecture/images-and-media.md) (IM1–IM13); [rendering pipeline](../architecture/rendering-pipeline.md) PL6–PL9; [invalidation](../architecture/invalidation.md) IV7–IV12; [ADR-0007](0007-frame-tree-pipeline-and-input.md); [ADR-0010](0010-dependency-and-supply-chain-policy.md); [Step 5 tracker](../research/step-5-subsystems.md) §10.1–§10.2 (spikes S5, S6); decision-map items I2 and I4 |

## Decision history

| Date | Event |
|---|---|
| 2026-10-18 | Proposed at Step 5, CP9 |
| 2026-10-18 | **Owner's Gate B decision:** "Pre-approved; will move to Accepted upon S6-Q1, S6-Q3, and S6-Q5 passing, and S5-Q5 validating ImageIO inside the renderer sandbox." Spike S6 is "Authorized for questions S6-Q1 through S6-Q5 under spikes/s6/". "The performance thresholds are confirmed: typical update ≤ 4 ms GPU / ≤ 2 ms compositor thread; scrolling/transforms ≤ 1 ms." S5-Q5 is authorized with S5 |
| 2026-10-18 | **S5-Q5 reported** (Step 5, CP10; [results](../research/s5-results.md) §7). ImageIO crashes under S4's renderer profile on every format until it may read the executable's directory's metadata (R1). AVIF then needs five more rules, and one of them (R4) opens VideoToolbox's decoder service, which the [process model](../security/process-model.md) §4.1 denies renderers. **The condition is not met.** Recommended for Gate C: amend IM3 to decode AVIF in the media utility process through IM9's interface. **IM2's JPEG choice:** the safe decoder stays (1.1–1.8× ImageIO's time, and deterministic, where ImageIO's JPEG pixels varied). The ADR stays Proposed until Gate C; S6's conditions are pending in any case |
| 2026-10-20 | **S6 reported** ([S6 results](../research/s6-results.md)), from the owner's formal run of 2026-10-19 at `45bff95`.<br>• **S6-Q1 passes at the median:** a typical update took 0.17–0.28 ms on the GPU and 0.67–1.84 ms on the compositor thread. In run 1, which began on a busy Mac, 192 of 600 updates exceeded 2 ms.<br>• **S6-Q3 passes at the median:** moving layers took 0.19–0.40 ms a frame; 2–7 of 1,085 scrolling frames per run exceeded 1 ms.<br>• **S6-Q5:** the surfaces carry their colour tags, and whether CoreAnimation honours them is the owner's visual check. **Its memory condition is not met as measured:** one window's compositor state, trimmed, leaves about 4 MB of B1 for the rest of the browser, so B1 needs revising in Step 6.<br>• **S6-Q4:** GPU path filling pays in time and memory, not in quality, so GR5's amendment goes to the owner, recommended for the graphics milestone with a GPU filler of analytic quality. GR5 stands meanwhile |
| 2026-10-20 | **Accepted** by the owner at Step 5's Gate C: "ADR-0012 (Graphics, Images, and Media): Accepted. The formal S6 benchmark run is complete (target/s6-results/20261019-201443/summary.txt) and easily meets all frame time thresholds. IM3's fallback (AVIF in the media utility process) is approved." **IM3 is amended accordingly:** ImageIO decodes AVIF in the media utility process, through IM9's interface (decision 9). The [process model](../security/process-model.md)'s change record notes ImageIO's place |
| 2026-10-20 | **S6's second formal run, and the colour check** ([S6 results](../research/s6-results.md)), run by the owner at `b4948ce`. It reproduced the first: S6-Q1 and S6-Q3 pass at the median again, with the tails again in the session's first runs; S6-Q4's pixels and S6-Q5's footprints match. **S6-Q5's colour condition is met:** in the owner's screenshot of the pattern, the P3-tagged red is (255, 0, 0) in Display P3 and the sRGB-tagged red (234, 51, 35), exactly sRGB's red converted, so CoreAnimation honours both tags; the untagged surface is not converted, so every surface is tagged (GR6). **The owner's decisions** on S6's open items: on B1, "Approved. In Step 6, budget visible window tile memory separately based on surface pixel dimensions (e.g. 25.2 MB for this Retina window), and maintain a 60 MB budget for the core browser process state." On GR5, "Approved. Keep path rasterization in the renderer for now to preserve analytic edge quality. Defer the GR5 display list amendment to the graphics milestone." On the rasterizer, "Approved. Adopt vello_cpu as the primary Rust rasterizer for web fonts and paths, subject to crate dependency grading at the text milestone." |
| 2026-10-21 | **Amendment accepted** in the owner's dispositions of the architecture review's Round 1 ([the review](../review/README.md) §5.1), RV1-02: "Formally design the sandboxed GPU/compositor process topology now (Renderer → GPU Process → Browser CALayer tree)." "Build the separate GPU/compositor process at Milestone 2; split Budget B1 accordingly." "Ensure Level 3 (L3) security certification at Milestone 6 is performed on the final multi-process topology." And RV1-09: "Renderers write pixels into shared memory entries; only the compositor owns IOSurfaces; renderer profiles explicitly deny iokit-open." GR1, GR2 and GR10–GR14 are amended in [graphics](../architecture/graphics.md), and IM1 in [images and media](../architecture/images-and-media.md): renderer → GPU/compositor process → browser from M2; canvas and images in memory entries; WebGL and WebGPU at M9, placed by M9's plan, never in the browser. Decisions 7, 8 and 12 record it |
| 2026-10-21 | **Amendment accepted** in the owner's dispositions of the architecture review's Round 1 ([the review](../review/README.md) §5.1), RV1-22: "RV1-19 to RV1-22 (Timing Side-Channels, Rustls ECH, Passkeys, Product Gaps): Accepted. Correct stale facts and schedule resolutions for Milestones 5–6." IM2: `jpeg-decoder`, in maintenance mode, is graded under DP8 or replaced by a decoder of Axiom's own at M1's plan |
| 2026-10-21 | **Spike S10's results** ([S10 results](../research/s10-results.md)): the GPU process's layer commit reached the browser's CoreAnimation commit in 48.8 µs at the median (p99 118.7 µs), so GR14's one ordered stream stands and graphics' alternative G stays rejected (§3). Renderer pixels are sampled from Mach memory entries without copies, the pages staying charged to the renderer, with more GPU time when images are minified (PL8, §4); validation costs about 9.4 ns an item (PL9, §5); the tiles are charged to the GPU process that creates them (§7) |
| 2026-10-21 | **Amendment accepted** in the owner's dispositions of the architecture review's Round 2 ([the review](../review/README.md) §5.5): "All 15 findings (RV2-01 through RV2-15) are Accepted as proposed." RV2-05: GR12 adds a protocol for reusing tile surfaces across processes (the browser deletes each port after lookup; the GPU process checks `IOSurfaceIsInUse`; a bounded pool allocated ahead; tested at M2 with a window) and the GPU process's accounting by page; GR8 refers to it ([graphics](../architecture/graphics.md)) |

## Question

What draws the display list, and how is it presented? How are images decoded, within budgets and the Rule of 2? Where does media run?

## Requirements and constraints

- **PL6–PL9, AC4:** per-frame display lists of validated primitives; the browser composites and never decodes. (**As amended in Round 1,** RV1-02: from M2 the GPU/compositor process composites; decision 12.)
- **IV7, IV8:** redraw only damage; scroll and animate from property trees.
- **AC3:** idle costs nothing.
- **Process model §3:** web-supplied GPU work and media decoding outside the browser. B1: the browser at ≤ 60 MB idle with one window.
- **S2:** the Command Line Tools have no offline Metal compiler.
- **ADR-0010:** pure-Rust decoders at `ub-risk-2` or better; Apple's parsers only in sandboxed helpers, under DP10's exception.

## Alternatives considered

- **Graphics:**
  - wgpu in the compositor;
  - one `CAMetalLayer` redrawn whole;
  - Skia;
  - Vello's compute renderer;
  - the proposal: Metal through `platform` into retained CoreAnimation layers.
- **Images:**
  - ImageIO for everything;
  - the C libraries other engines still use;
  - a decoder process per image;
  - the proposal: safe Rust decoders, with ImageIO for AVIF only.

## Historical context and modern developments

- **Firefox** measured large power savings on macOS from drawing into IOSurface-backed CoreAnimation tiles (2019). Chromium and WebKit also present through layer trees.
- **Chrome's Skia Graphite** reaches Metal through Dawn, in a sandboxed GPU process.
- **Rust decoders:** Chromium decodes PNG and JPEG XL in Rust, and Firefox decodes JPEG XL in Rust. Both still use C for JPEG, WebP and AV1.
- **Media:** every engine decodes it outside the content process.

Sourced in [graphics](../architecture/graphics.md) §3 and [images and media](../architecture/images-and-media.md) §3.

## Analysis

| | Proposal | Main alternative |
|---|---|---|
| Presentation | Retained layers: damage-limited tiles, scrolling and animation without redraw | One `CAMetalLayer`: simpler, with no partial presentation |
| GPU interface | Metal through `platform`: a small `unsafe` surface in the browser | wgpu: portability Axiom doesn't need, and a large `unsafe` tree |
| Decoders | Safe Rust for PNG, JPEG, GIF, WebP, JPEG XL, BMP, ICO; ImageIO for AVIF | ImageIO for all: Apple's C parsers on every image |

## Evidence and sources

[Graphics](../architecture/graphics.md) and [images and media](../architecture/images-and-media.md), §3–§4, and their Sources. Observations: this Mac's GPU and display; SDK 27's QuartzCore, Metal, IOSurface and ImageIO headers. Measurements: [S5 results](../research/s5-results.md) §7 (ImageIO in the renderer's profile; the safe decoders) and [S6 results](../research/s6-results.md) (compositing, damage, layers, paths, memory and colour).

## Decision

**Accepted: GR1–GR14 and IM1–IM13** (2026-10-20), with IM3's fallback: AVIF in the media utility process. S6-Q4 put GR5's amendment to the owner ([S6 results](../research/s6-results.md) §8), who deferred it to the graphics milestone; GR5 stands as written until then. In summary:
1. **Metal directly, through `platform`;** no portability layer in the browser (GR1), nor in the GPU/compositor process.
2. **The window server composites retained CoreAnimation layers.** Axiom draws damaged tiles into its own IOSurfaces, with layers for scrollers, animated subtrees, child frames and video (GR2, GR7, GR8).
3. **Our own pipelines,** compiled at first use and kept in a binary archive; tile-memory aware (GR3, GR4).
4. **Paths are rasterized in the renderer.** S6-Q4 found GPU filling in the compositor faster and smaller, but coarser at the edges. It would need **an amendment adding path items to PL7,** a separate decision for the owner (GR5, SC1).
5. **Display P3 at 8 bits by default;** HDR later, per layer (GR6).
6. **Compositor animations:** the transform family and opacity, then `filter`, `backdrop-filter` and scroll-driven timelines (GR9).
7. **Canvas 2D in the renderer.** WebGL and WebGPU only in a GPU process. Video in layers of its own (GR10, GR14). **As amended in Round 1** (RV1-09): canvas 2D draws into a Mach memory entry; WebGL and WebGPU arrive at M9, in a process M9's plan places.
8. **Images decode in the renderer,** into shared memory; the browser decodes nothing (IM1). **As amended in Round 1** (RV1-09): into Mach memory entries only, sampled in place by the compositor; renderers have no IOKit.
9. **Safe Rust decoders.** ImageIO handles AVIF under DP10's exception, which this ADR accepts, **in the media utility process,** as the owner amended IM3 at Gate C (IM2, IM3, IM9).
10. **Budgets:** incremental decoding, decoding at display size, discarding, bounded animation, limits; color management with `moxcms` (IM4–IM10).
11. **Media in a sandboxed utility process,** with VideoToolbox and AudioToolbox (IM11–IM13).
12. **The GPU/compositor process from M2,** added in Round 1 (RV1-02): renderers send display lists only to it; it validates them, rasterizes tiles into IOSurfaces it owns, and sends the browser layer commits; the browser holds the layer tree, scrolls and animates layers, and routes input (GR14).

## Rationale

- Retained layers and damage-limited drawing are the measured way to spend little power on macOS. That matters on a fanless MacBook Air (charter G4).
- Safe decoders remove the class of bug behind the 2023 libwebp exploit for most formats. ImageIO, kept for AVIF, gets macOS's security updates.

## Assumptions

- CoreAnimation honours IOSurface contents and their color tagging as the SDK header suggests. **S6-Q5 confirmed it** in the owner's screenshot: tagged surfaces are converted exactly, and untagged ones are not converted at all, so every surface is tagged (GR6).
- The safe decoders are fast enough on this Mac. S5-Q5: PNG, GIF and WebP decode faster than ImageIO, and JPEG in 1.1–1.8× its time.

## Risks and unresolved issues

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| The compositor misses its frame budget | Low (S6-Q1 and S6-Q3 pass at the median) | High | Few pipelines; depth-tested overdraw; damage-limited tiles. S6's slowest frames allocated surfaces, so surfaces are allocated ahead of need ([S6 results](../research/s6-results.md) §8) |
| ImageIO won't run under the renderer's profile | Occurred (S5-Q5) | Medium | IM3's fallback, approved at Gate C: AVIF in the media utility process, through IM9's interface |
| `jpeg-decoder` stays in maintenance mode | Medium | Low | A JPEG decoder of our own (IM2). **Round 1** (RV1-22): decided at M1's plan, by DP8's grade or Axiom's own |
| Tiles exceed B1 | Occurred (S6-Q5) | Medium | B1 revised, as the owner approved: a visible window's tiles are budgeted by their pixels in Step 6, and the core browser keeps 60 MB; display ports, purgeable surfaces and release on hide (GR12) |

## Conditions for revisiting

- The GPU/compositor process arrives at M2 (GR14, as amended in Round 1): validation and rasterization move, and the interfaces stay. S10's measurements of the hop could reopen GR14's routing (graphics §9, alternative G). **They don't:** the hop took 48.8 µs at the median ([S10 results](../research/s10-results.md) §3.5).
- The graphics milestone, to which the owner deferred GR5's amendment: if a GPU filler of analytic quality is shown and the owner accepts the amendment, path items enter PL7 (GR5, SC1).
- A safe AV1 decoder meets ADR-0010: ImageIO's exception ends.

## Consequences

- `platform` gains Metal, QuartzCore, IOSurface and ImageIO calls, in allowlisted modules.
- The [register](../technology/dependency-register.md) records `png`, `jpeg-decoder`, `gif`, `image-webp`, `jxl`, `moxcms` and ImageIO, decided with this ADR; ImageIO runs in the media utility process.
- CP15 paints into GR3's primitives. CP14 marks the GR9 animations for the compositor.
- The [process model](../security/process-model.md) records ImageIO in the media utility process (its change record, 2026-10-20).
- Step 6 revises B1 from S6-Q5's breakdown; the process model is binding, so B1 is the owner's ([S6 results](../research/s6-results.md) §7, §8).
