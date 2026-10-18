# ADR-0012: Graphics, images and media

| | |
|---|---|
| **Status** | **Proposed** (2026-10-18). **Acceptance pre-approved** by the owner at Step 5's Gate B, effective once S6-Q1, S6-Q3 and S6-Q5 pass and S5-Q5 validates ImageIO inside the renderer's sandbox, as ADR-0008 waited for S3. **S5-Q5 reported** on 2026-10-18 ([results](../research/s5-results.md)): its condition is not met, because AVIF needs a rule the process model forbids renderers. The ADR stays Proposed until Gate C, with an amendment to IM3 recommended |
| **Date** | 2026-10-18 |
| **Deciders** | Project owner (only the owner can move an ADR to Accepted) |
| **Related** | [Graphics](../architecture/graphics.md) (GR1–GR14); [images and media](../architecture/images-and-media.md) (IM1–IM13); [rendering pipeline](../architecture/rendering-pipeline.md) PL6–PL9; [invalidation](../architecture/invalidation.md) IV7–IV12; [ADR-0007](0007-frame-tree-pipeline-and-input.md); [ADR-0010](0010-dependency-and-supply-chain-policy.md); [Step 5 tracker](../research/step-5-subsystems.md) §10.1–§10.2 (spikes S5, S6); decision-map items I2 and I4 |

## Decision history

| Date | Event |
|---|---|
| 2026-10-18 | Proposed at Step 5, CP9 |
| 2026-10-18 | **Owner's Gate B decision:** "Pre-approved; will move to Accepted upon S6-Q1, S6-Q3, and S6-Q5 passing, and S5-Q5 validating ImageIO inside the renderer sandbox." Spike S6 is "Authorized for questions S6-Q1 through S6-Q5 under spikes/s6/". "The performance thresholds are confirmed: typical update ≤ 4 ms GPU / ≤ 2 ms compositor thread; scrolling/transforms ≤ 1 ms." S5-Q5 is authorized with S5 |
| 2026-10-18 | **S5-Q5 reported** (Step 5, CP10; [results](../research/s5-results.md) §7). ImageIO crashes under S4's renderer profile on every format until it may read the executable's directory's metadata (R1). AVIF then needs five more rules, and one of them (R4) opens VideoToolbox's decoder service, which the [process model](../security/process-model.md) §4.1 denies renderers. **The condition is not met.** Recommended for Gate C: amend IM3 to decode AVIF in the media utility process through IM9's interface. **IM2's JPEG choice:** the safe decoder stays (1.1–1.8× ImageIO's time, and deterministic, where ImageIO's JPEG pixels varied). The ADR stays Proposed until Gate C; S6's conditions are pending in any case |

## Question

What draws the display list, and how is it presented? How are images decoded, within budgets and the Rule of 2? Where does media run?

## Requirements and constraints

- **PL6–PL9, AC4:** per-frame display lists of validated primitives; the browser composites and never decodes.
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

[Graphics](../architecture/graphics.md) and [images and media](../architecture/images-and-media.md), §3–§4, and their Sources. Observations: this Mac's GPU and display; SDK 27's QuartzCore, Metal, IOSurface and ImageIO headers.

## Decision

**Recommended: accept GR1–GR14 and IM1–IM13, once S6-Q1, S6-Q3 and S6-Q5 pass, and S5-Q5 shows ImageIO working under the renderer's profile.** S6-Q4 decides whether GR5's amendment goes to the owner. In summary:
1. **Metal directly, through `platform`;** no portability layer in the browser (GR1).
2. **The window server composites retained CoreAnimation layers.** Axiom draws damaged tiles into its own IOSurfaces, with layers for scrollers, animated subtrees, child frames and video (GR2, GR7, GR8).
3. **Our own pipelines,** compiled at first use and kept in a binary archive; tile-memory aware (GR3, GR4).
4. **Paths are rasterized in the renderer** until S6 reports. GPU path filling would need **an amendment adding path items to PL7,** which is a separate decision for the owner (GR5, SC1).
5. **Display P3 at 8 bits by default;** HDR later, per layer (GR6).
6. **Compositor animations:** the transform family and opacity, then `filter`, `backdrop-filter` and scroll-driven timelines (GR9).
7. **Canvas 2D in the renderer.** WebGL and WebGPU only in a GPU process. Video in layers of its own (GR10, GR14).
8. **Images decode in the renderer,** into shared memory; the browser decodes nothing (IM1).
9. **Safe Rust decoders.** ImageIO handles AVIF under DP10's exception, which this ADR accepts (IM2, IM3).
10. **Budgets:** incremental decoding, decoding at display size, discarding, bounded animation, limits; color management with `moxcms` (IM4–IM10).
11. **Media in a sandboxed utility process,** with VideoToolbox and AudioToolbox (IM11–IM13).

## Rationale

- Retained layers and damage-limited drawing are the measured way to spend little power on macOS. That matters on a fanless MacBook Air (charter G4).
- Safe decoders remove the class of bug behind the 2023 libwebp exploit for most formats. ImageIO, kept for AVIF, gets macOS's security updates.

## Assumptions

- CoreAnimation honours IOSurface contents and their color tagging as the SDK header suggests (S6-Q5).
- The safe decoders are fast enough on this Mac (S5-Q5).

## Risks and unresolved issues

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| The compositor misses its frame budget | Low | High | S6-Q1 before any compositor code; few pipelines; depth-tested overdraw |
| ImageIO won't run under the renderer's profile | Medium | Medium | S5-Q5; otherwise AVIF waits, or gets its own helper (IM9) |
| `jpeg-decoder` stays in maintenance mode | Medium | Low | A JPEG decoder of our own (IM2) |
| Tiles exceed B1 | Medium | Medium | Display ports, purgeable surfaces, release on hide (GR12); S6-Q5 |

## Conditions for revisiting

- A GPU process arrives (GR14): rasterization moves, and the interfaces stay.
- S6 shows that GPU path filling pays: GR5's amendment.
- A safe AV1 decoder meets ADR-0010: ImageIO's exception ends.

## Consequences

- `platform` gains Metal, QuartzCore, IOSurface and ImageIO calls, in allowlisted modules.
- The [register](../technology/dependency-register.md) records `png`, `jpeg-decoder`, `gif`, `image-webp`, `jxl`, `moxcms` and ImageIO.
- CP15 paints into GR3's primitives. CP14 marks the GR9 animations for the compositor.
