# Images and Media

| | |
|---|---|
| **Status** | **Binding** since [ADR-0012](../adr/0012-graphics-images-and-media.md)'s acceptance (2026-10-20, Step 5's Gate C). Written at Step 5, CP5. **IM3 amended at Gate C:** ImageIO decodes AVIF in the media utility process, as the owner approved after S5-Q5 found that in the renderer it needs a rule the process model forbids renderers ([results](../research/s5-results.md) §7). IM2's safe JPEG decoder stays. **Amended by the architecture review's Round 1** (the owner's dispositions, 2026-10-21; [the review](../review/README.md) §5): IM1, decoded pixels in memory entries only (RV1-09; CP2); IM2, JPEG's decoder decided at M1's plan (RV1-22; CP5). **Amended by Round 4** (the owner's dispositions, 2026-10-22; [the review](../review/README.md) §5.9): IM2 and §10 for RV4-02 (CP2) |
| **Question** | Which image formats are decoded, by safe Rust or by ImageIO inside the renderer's sandbox? How do decoding, animation and color profiles stay within memory budgets? Where do media decoding and audio output run? |
| **Answers** | Step 5 question Q5 ([tracker](../research/step-5-subsystems.md) §3); decision-map item I4 |
| **Feeds** | ADR-0012 (CP9); spike S5 (tracker §10.1, S5-Q5); CP6 (streamed bodies); CP15 (`<img>`, `<picture>`, SVG as an image); a later media milestone |
| **Builds on** | [Rendering pipeline](rendering-pipeline.md) PL7, PL8, PL10, PL13; [invalidation](invalidation.md) IV10–IV12; the [process model](../security/process-model.md) §3 and §7; [graphics](graphics.md) GR6, GR10, GR12; the [dependency policy](../technology/dependency-policy.md) DP8–DP10, DP21 |
| **Last updated** | 2026-10-22: Round 4's accepted findings |

Everything here is **planned**; nothing is implemented. Rules are numbered IM1–IM13. Precedent is sourced (§3); the design is inference unless marked otherwise.

## Summary

- **Images are decoded in the renderer, on its helper pool, into shared memory** (IM1). The browser never decodes one, not even a favicon.
- **Safe Rust decodes every format that has a decoder meeting the dependency policy** (IM2):
  - `png`, `gif` and `image-webp` all forbid `unsafe`;
  - `jxl` is Rust; Chromium and Firefox decode JPEG XL with it;
  - BMP and ICO get small decoders of our own.

  JPEG uses a safe Rust decoder, either `jpeg-decoder` without its `unsafe` paths or one of our own. `zune-jpeg` is graded `ub-risk-3`, so the policy bars it. WebP in safe Rust matters: the 2023 libwebp vulnerability was exploited in the wild.
- **AVIF has no decoder in safe Rust that meets the policy** (IM3). It goes to ImageIO, under the dependency policy's exception for memory-unsafe code, **in the media utility process.** S5-Q5 found that in the renderer ImageIO needs VideoToolbox's decoder service, which the process model denies renderers, and the owner approved IM3's fallback at Gate C.
- **Decoding stays within budgets** (IM4–IM7):
  - images decode incrementally, and at the size they are displayed;
  - pixels of images that aren't visible are discarded and decoded again on demand, as Firefox does;
  - animations decode frame by frame, and only while visible;
  - hostile dimensions are refused.
- **Color profiles are applied during decoding,** into the window's working space (IM8), by a Rust color-management library without its SIMD `unsafe`.
- **Media lives in a sandboxed utility process** (IM10–IM12):
  - demuxers in safe Rust;
  - VideoToolbox and AudioToolbox for the codecs the hardware decodes;
  - frames delivered to the compositor as IOSurfaces, and audio output from the same process.

  Firefox, Chromium and WebKit all decode media outside their content processes.

## 1. Binding inputs

| Input | Consequence here |
|---|---|
| **AC4, PL7–PL8:** the browser never decodes images; display lists carry decoded pixels in Mach memory entries (IOSurfaces only in the compositor, as amended in Round 1, RV1-09), as resources of their renderer, counted against its budget | IM1 |
| **Process model §3, note 3:** images are decoded in the renderer, with no decoder process per image, by safe Rust decoders by preference; ImageIO only inside a renderer or the GPU process; images the browser shows, such as favicons, are decoded by safe Rust decoders or in a renderer | IM1–IM3 |
| **Process model §3:** media decoding with system codecs belongs to a utility process | IM10 |
| **PL10:** image decoding runs on the helper pool | IM1 |
| **IV10–IV12:** an arriving image repaints only, if its size was known, and relayouts otherwise; hidden content does no rendering work; frames only when offered and needed | IM4, IM7 |
| **GR6, GR10, GR12:** one working color space per window; video frames as IOSurfaces with layers of their own; layers released when hidden | IM8, IM11 |
| **DP8–DP10, DP21:** grades of `ub-risk-3` or worse are barred; a reused decoder of untrusted input must be pure Rust at `ub-risk-2` or better across its tree; C, C++ or Apple's parsers only in sandboxed helpers, with no pure-Rust alternative, and with an ADR | IM2, IM3 |
| **Charter §6:** DRM-protected media is a non-goal | IM12 |

## 2. Specifications

- **Image formats:** PNG (with APNG), JPEG, GIF, WebP, AVIF, JPEG XL, BMP and ICO.
- **Color:** ICC profiles, versions 2 and 4; coding-independent code points (CICP), as PNG's `cICP` chunk and AVIF carry them; CSS Color 4 for untagged images.
- **HTML's image processing:** `img`, `picture`, `srcset`, `decoding`, `loading`; owned by CP13 and CP15.
- **Media:** HTML's media elements, Media Source Extensions, Web Audio, and the codecs and containers the target corpus needs. Encrypted Media Extensions are out of scope (charter §6).

## 3. Precedent

Read on 2026-10-18; dates in brackets are the last commits to each file, from GitHub's API.

| Engine | Images and media |
|---|---|
| **Chromium** | **PNG** decodes through Skia's Rust decoder, `SkPngRustDecoder` (`png_image_decoder.cc` [2025-09-12]) [C1]. **JPEG XL** decodes through the Rust `jxl` crate, version 0.7, and its feature `kJXLImageFormat` is enabled by default in builds that include the decoder (`jxl_image_decoder.h`; `features.cc`) [C2]. **AVIF** goes through `crabbyavif` (`avif_image_decoder.cc` [2026-10-15]) [C3]. **JPEG and WebP** still use the C libraries libjpeg-turbo and libwebp [C4]. It also vendors the `moxcms` color-management crate, with default features off, the `image` crate's BMP and ICO decoders, and `symphonia`'s audio decoders [C5]. **The audio service** "Runs in a separate process on Windows, Mac (sandboxed) and Linux (unsandboxed)" (`services/audio/README.md`) [C6] |
| **Firefox** | **JPEG XL** decodes through the Rust `jxl` crate, 0.7.4 (`image/rust/jxl`), and `image.jxl.enabled` is `true` in builds that include it [F1]. **AVIF** uses the Rust container parser `mp4parse`, with the C decoders dav1d and aom (`nsAVIFDecoder.h`) [F2]. **Color management** uses `qcms`, "Firefox's library for transforming image data between ICC profiles", in Rust [F3]. **Memory:** `image.downscale-during-decode.enabled` is `true`. `image.mem.discardable` is `true`: it "Discards inactive image frames and re-decodes them on demand from compressed data". On memory pressure the surface cache discards, but "we never discard visible images" (`StaticPrefList.yaml` [2026-10-16]) [F4]. **Media** decodes in its RDD process ("Remote Data Decoder"), which has its own macOS sandbox policy (`SandboxPolicyRDD.h`) [F5] |
| **WebKit** | On Apple platforms, decodes through ImageIO: `ImageDecoderCG.cpp` creates `CGImageSource`s [W2]. Its GPU process hosts media playback and audio (`RemoteMediaPlayerProxy`, `RemoteAudioDestinationManager`) [W1] |
| **Ladybird** | Decodes images out of process: "Image decoding and network connections are done out of process to be more robust against malicious content" [L1] |

**The case for safe decoders.** The 2023 libwebp heap overflow was exploited in the wild ([decision map](../decision-map.md), source S7; [language evaluation](../technology/language-evaluation.md) §12).

**This Mac's ImageIO** (SDK 27 headers, observed): `kCGImagePropertyAVISDictionary` (AVIF image sequences) and `kCGImagePropertyHEIFDictionary` from macOS 13; `kCGImagePropertyWebPDictionary` from macOS 11.

**Observations** (inference):
- **Two engines are moving decoders to Rust,** format by format. Chromium has moved PNG, JPEG XL and AVIF's container, and Firefox JPEG XL and AVIF's container. JPEG, WebP and AV1 decoding are still C in both.
- **Every engine keeps media out of the content process.**

## 4. Build or reuse

Grades are Google's published audits (google/supply-chain [2026-09-17]), for the version named; versions adopted are audited again (DP15). `unsafe` policies were read in each crate's `lib.rs` [A1].

| Format or need | Candidates | Decision |
|---|---|---|
| **PNG, APNG** | **`png`** 0.18.1: `forbid(unsafe_code)`; `ub-risk-0` at 0.18.0-rc. Its tree includes `crc32fast` and `simd-adler32`, both `ub-risk-2`. Chromium's decoder (§3) | **`png`** |
| **JPEG** | **`zune-jpeg`** 0.5.15: graded **`ub-risk-3`** at 0.4.19, so DP8 bars it. **`jpeg-decoder`** 0.3.2: `deny(unsafe_code)`, which becomes `forbid` with its `platform_independent` feature, though its README says it is "in maintenance mode". **Our own.** **ImageIO.** **libjpeg-turbo,** C | **Safe Rust:** `jpeg-decoder` in `platform_independent` mode, or a decoder of our own if S5-Q5 finds it too slow or its maintenance lapses. ImageIO is only S5-Q5's comparison point: DP10's exception needs a missing safe alternative |
| **GIF** | **`gif`** 0.14.2: `forbid(unsafe_code)`; `ub-risk-1` at 0.12.1 | **`gif`** |
| **WebP** | **`image-webp`** 0.2.4: `forbid(unsafe_code)`, not graded. **libwebp,** C, the library of the 2023 exploit | **`image-webp`** |
| **AVIF** | **ImageIO,** macOS 13 and later, a platform facility, patched by macOS updates. **dav1d or aom,** C, as Firefox and Chromium use. **`rav1d`** 1.1.0, a Rust port of dav1d with an optional assembly feature and no published grade | **ImageIO in the renderer,** under DP10's exception (IM3) |
| **JPEG XL** | **`jxl`** 0.7.4 (jxl-rs): `deny(unsafe_code)` at its root; `ub-risk-1` at 0.1.1. Chromium and Firefox decode with it (§3) | **`jxl`**, staged |
| **BMP, ICO** | **The `image` crate's codecs,** with `image` itself `ub-risk-2` at 0.24.6 and a broad tree; **our own** | **Our own:** small formats, kept in the project (DP4) |
| **Color management** | **`moxcms`** 0.9.1: `forbid(unsafe_code)` unless a SIMD feature is enabled; vendored by Chromium with default features off. **`qcms`** 0.3.0 (Firefox's): contains `unsafe`, last released 2024-01. **ColorSync,** Apple's | **`moxcms`,** with no SIMD feature (IM8) |

## 5. Design

### 5.1 Where decoding runs

| # | Rule |
|---|---|
| IM1 | **Images are decoded in the renderer, on its helper pool** (PL10), from bytes streamed by the document's loader (FT11), **into Mach memory entries only** (**as amended in Round 1,** RV1-09: renderers never touch IOKit, and the compositor owns every IOSurface). The results are registered as the renderer's resources (PL8). **The browser never decodes an image** (AC4). Images that the browser's own interface shows, such as favicons, are decoded in a renderer and arrive as validated pixels, because the decoders' trees are graded `ub-risk-2`, below the `ub-risk-1` DP9 asks of the browser |
| IM2 | **Safe Rust decodes every format that has a decoder meeting DP10** (§4): `png`, `gif`, `image-webp` and `jxl`; a safe JPEG decoder; and BMP and ICO decoders of our own. Each sits behind Axiom's decoder interface (IM9), so it can be replaced. **S5-Q5 checked it** (2026-10-18): in the renderer's sandbox, `png`, `gif` and `image-webp` decode faster than ImageIO and use less memory. `jpeg-decoder` takes 1.1–1.8× ImageIO's time, 5–10 ms per megapixel, with the same memory, and gives the same pixels every time, where ImageIO's JPEG pixels varied between decodes. **The safe JPEG decoder stays** ([S5 results](../research/s5-results.md) §7). **Round 1** (RV1-22): `jpeg-decoder` is in maintenance mode, so the decoder is decided at M1's plan: `jpeg-decoder` graded under DP8, or a decoder of Axiom's own, which adds scope (RV1-08).<br>**As amended in Round 4** (RV4-02): under DP8 as amended, M1's plan chooses between **`zune-jpeg` 0.5.15** without its `x86` and `neon` features, where its root forbids `unsafe` (observed), maintained and released 2026-10-19, and **`jpeg-decoder` 0.3.2** with `platform_independent`, which forbids it too, unreleased since 2025-06-21; both are measured on the corpus's images. A decoder of Axiom's own is the fallback only (DP4, as amended) |
| IM3 | **AVIF goes to ImageIO, in the media utility process** (amended at Gate C, as below; written at CP5 as "in the renderer"). No AV1 decoder in pure Rust meets DP10 (§4), so ImageIO is admitted under DP10's exception: it runs in a sandboxed helper, no pure-Rust alternative meets the requirement, and ADR-0012 is the ADR that accepts it. ImageIO is a platform facility (DP21), and macOS updates patch it, where a vendored dav1d would wait for a rebuild (threat model §9). **S4 never tested ImageIO in a sandbox,** so S5 gains S5-Q5, which checks it under the renderer profile and finds the rules it needs. If ImageIO can't work there, AVIF waits for a decoder that meets DP10, or for a separate decoding helper with its own profile (IM9). **S5-Q5's result** (2026-10-18): ImageIO crashes under the renderer profile on every format until it may read its executable's directory's metadata (R1, which CoreText needs too). AVIF then needs five more rules, and one of them opens VideoToolbox's decoder service, `VTDecoderXPCService`, which launchd starts for the decoding process: a Mach service the [process model](../security/process-model.md) §4.1 denies renderers. **Amended at Gate C** (2026-10-20), by the owner: "IM3's fallback (AVIF in the media utility process) is approved." AVIF is decoded in the media utility process (IM11), whose profile admits media services and hardware decoding, through IM9's interface, which callers can't tell apart ([S5 results](../research/s5-results.md) §7, §9). The [process model](../security/process-model.md) records ImageIO's place in its change record |

### 5.2 Budgets

| # | Rule |
|---|---|
| IM4 | **Decoding is incremental.** Decoders take bytes as they arrive and produce rows or frames as they can: interlaced PNG, progressive JPEG, partial GIF. So an image paints before it is complete, and repaints as it fills in (IV10) |
| IM5 | **Images decode at the size they are displayed,** when that is smaller than their intrinsic size, as Firefox does (§3). JPEG scales during its inverse transform; other formats downscale band by band as they decode. A larger display size, after a zoom for example, decodes again from the compressed bytes |
| IM6 | **Decoded pixels are discardable.** The compressed bytes are kept, and the decoded pixels of images that aren't visible are discarded and decoded again on demand, as Firefox's `image.mem.discardable` does (§3). Visible images are never discarded. Under memory pressure, the renderer discards everything else first ([process model](../security/process-model.md) §7). Decoded images count against the renderer's budget (B3: a typical page in ≤ 50 MB) |
| IM7 | **Animated images** (APNG, GIF, animated WebP, AVIF sequences, JPEG XL) decode frame by frame, keeping a small window of decoded frames. They advance on the renderer's frame clock only while visible (EL20, IV11). Very short frame delays are clamped, as browsers do; the values are pinned, with sources, at implementation |

### 5.3 Color

| # | Rule |
|---|---|
| IM8 | **Color is managed during decoding.** An image's embedded ICC profile (versions 2 and 4) or CICP tags convert its pixels into the window's working space (GR6). An untagged image is sRGB (CSS Color 4). Profiles are untrusted input, so the color-management library falls under DP10: `moxcms` with no SIMD feature, so that it forbids `unsafe`. Converted pixels are cached with the decoded image (IM6) |

### 5.4 The decoder interface and limits

| # | Rule |
|---|---|
| IM9 | **One decoder interface, shaped like a message boundary.** Bytes go in; metadata, rows and frames come out; errors are values. Callers can't tell where a decoder runs, so a decoder can move into its own helper process without changing them, as Ladybird decodes out of process (§3). The candidates for moving are ImageIO's formats, if S5-Q5 or the threat model ever ask for more isolation than the renderer's. **AVIF has moved,** to the media utility process (IM3, amended at Gate C) |
| IM10 | **Hostile inputs are bounded.** Each image has limits on its dimensions, pixel count, frame count and decoded memory. An image past a limit is a decode error and shows as a broken image, never a crash. Decoders are fuzzed, and decoded results are compared against reference decoders (Step 6, X2) |

### 5.5 Media

| # | Rule |
|---|---|
| IM11 | **Media is decoded in a sandboxed utility process,** one kind of job per process (process model §3), like Firefox's RDD process, Chromium's sandboxed audio service and WebKit's GPU-process media (§3).<br>• **Demuxers** (MP4, WebM, Ogg) parse untrusted input, so DP10 applies: Axiom's own, or reused Rust at `ub-risk-2` or better. Candidates, chosen at the media milestone: `mp4parse` (Mozilla, MPL-2.0) and `symphonia` (MPL-2.0; vendored by Chromium).<br>• **Codecs:** VideoToolbox and AudioToolbox, platform facilities, decode what this Mac's hardware and Apple's codecs cover, inside the sandboxed process (DP21). Other codecs need a Rust decoder that meets DP10, or DP10's exception with an ADR.<br>• **Output:** video frames go to the compositor as IOSurfaces, referenced by the renderer's display list as resources (PL7, GR10). Audio is output by the same process through CoreAudio, on real-time threads ([event loop and threading](event-loop-and-threading.md) §4.2) |
| IM12 | **Media's interfaces don't depend on where it runs.** A media element in the renderer controls playback through messages; Media Source Extensions stream bytes from the renderer to the media process; Web Audio's graph runs in the media process. Encrypted Media Extensions are not built (charter §6). The media process's sandbox profile is set by S4's method at the media milestone |
| IM13 | **Memory and idle.** The media process pools frames and bounds how far it decodes ahead. A hidden video stops decoding (IV11), and a paused one holds only its current frame. Audio threads are real-time only while sound plays |

## 6. Interfaces with neighbouring subsystems

| With | Interface |
|---|---|
| **Networking** (CP6) | Streamed response bodies (FT11), with content sniffing for images by the MIME Sniffing standard |
| **HTML and layout** (CP13, CP15) | Intrinsic size and density as soon as headers decode; `decoding` and `loading`; frames for painting; SVG images rendered by the engine itself in a restricted mode, with no script and no subresource loads |
| **Graphics** (CP4) | Decoded pixels and video IOSurfaces as resources, converted to the working color space (GR6, GR10) |
| **JavaScript** (CP16) | `createImageBitmap`, `ImageDecoder` (WebCodecs) later, through the same decoder interface |

## 7. Security

- **Rule of 2:** every image and media parser runs in a sandboxed process: the renderer, or the media utility process.
- **Memory safety by default:** safe Rust decodes every format with an admissible decoder. ImageIO (AVIF) and Apple's media frameworks are the recorded exceptions, each in a sandboxed helper and accepted by an ADR (DP10).
- **The browser decodes nothing** (IM1, AC4).
- **Hostile dimensions and decompression bombs are bounded** (IM10).

## 8. Staging and the complete model

| Stage | What exists |
|---|---|
| First image milestone | PNG, JPEG, GIF, WebP; incremental decoding; decoding at display size; discarding; color management |
| Later | AVIF through ImageIO in the media utility process (IM3); JPEG XL; animated formats in full |
| Media milestone | The media utility process: demuxers, VideoToolbox and AudioToolbox, video layers, audio output, Media Source Extensions |
| L0–L3 | No change: decoding stays in renderers and media helpers at every level |

**The complete model** (SC9): WebCodecs and `ImageDecoder` reuse the decoder interface (IM9); video and audio capture (camera and microphone) are mediated by the browser (SG5) and live with media; hardware decoding is VideoToolbox's.

## 9. Alternatives considered

| | Alternative | Assessment |
|---|---|---|
| A | **ImageIO for every format,** as WebKit does | Fast, and patched by macOS updates. But it would put Apple's C parsers on every image, where safe decoders exist for most formats. Kept only for AVIF (IM3) |
| B | **The C libraries Chromium and Firefox still use** (libjpeg-turbo, libwebp, dav1d) | Mature and fast, but memory-unsafe parsers of hostile input, and libwebp's 2023 exploit is the case in point. Rejected while safe options exist |
| C | **A decoder process per image,** as Ladybird does | It isolates decoders from the renderer, at a cost in memory and latency, while the renderer is already a site-locked sandbox. The process model chose the renderer (§1). IM9 keeps the option open |
| D | **Decoding in the browser** | It breaks AC4 and the Rule of 2. Rejected |

## 10. Open questions

| # | Question | Where it is settled |
|---|---|---|
| 1 | Whether ImageIO decodes AVIF under the renderer's profile, with which rules, and how its speed and memory compare with the safe decoders | S5-Q5. **Answered** (2026-10-18): not without a rule the process model forbids renderers; AVIF goes to the media utility process (IM3, amended at Gate C) |
| 2 | `jpeg-decoder`, or a JPEG decoder of our own | S5-Q5's measurements (the safe decoder stays), then the first image milestone. **Round 4** (RV4-02): `zune-jpeg` without SIMD or `jpeg-decoder`, at M1's plan |
| 3 | The demuxers and software codecs for media | The media milestone |
| 4 | The limits of IM10 | The first image milestone, with Step 6's budgets |
| 5 | Whether one media utility process decodes AVIF for every site, or each site lock gets its own instance from L2 (IM3, IM11). A shared instance would expose images decoded for other sites if ImageIO were compromised there (inference) | The media milestone, with the threat model |

## Sources

All read on 2026-10-18. Dates in brackets are the last commits to each file, from GitHub's API.

- [A1] The crates' `lib.rs` and READMEs, and crates.io's API: `png` (image-rs/image-png), `jpeg-decoder` (image-rs/jpeg-decoder), `gif` (image-rs/image-gif), `image-webp` (image-rs/image-webp), `jxl` (libjxl/jxl-rs), `moxcms` (awxkee/moxcms), `qcms` (FirefoxGraphics/qcms), `rav1d` (memorysafety/rav1d), `zune-jpeg`; Google's audits — https://github.com/google/supply-chain/blob/main/audits.toml
- [C1] Chromium, `third_party/blink/renderer/platform/image-decoders/png/png_image_decoder.cc` — https://github.com/chromium/chromium/blob/main/third_party/blink/renderer/platform/image-decoders/png/png_image_decoder.cc
- [C2] Chromium, `image-decoders/jxl/jxl_image_decoder.h`, and `third_party/blink/common/features.cc` — https://github.com/chromium/chromium/tree/main/third_party/blink/renderer/platform/image-decoders/jxl
- [C3] Chromium, `image-decoders/avif/avif_image_decoder.cc` — https://github.com/chromium/chromium/blob/main/third_party/blink/renderer/platform/image-decoders/avif/avif_image_decoder.cc
- [C4] Chromium, `image-decoders/jpeg/jpeg_image_decoder.cc` and `image-decoders/webp/webp_image_decoder.h` — https://github.com/chromium/chromium/tree/main/third_party/blink/renderer/platform/image-decoders
- [C5] Chromium, `third_party/rust/chromium_crates_io/Cargo.toml` — https://github.com/chromium/chromium/blob/main/third_party/rust/chromium_crates_io/Cargo.toml
- [C6] Chromium, `services/audio/README.md` — https://github.com/chromium/chromium/blob/main/services/audio/README.md
- [F1] Firefox, `image/rust/jxl/Cargo.toml`, `image/decoders/nsJXLDecoder.h` — https://github.com/mozilla-firefox/firefox/tree/main/image
- [F2] Firefox, `image/decoders/nsAVIFDecoder.h` — https://github.com/mozilla-firefox/firefox/blob/main/image/decoders/nsAVIFDecoder.h
- [F3] Firefox, `gfx/qcms/README.md` — https://github.com/mozilla-firefox/firefox/tree/main/gfx/qcms
- [F4] Firefox, `modules/libpref/init/StaticPrefList.yaml` — https://github.com/mozilla-firefox/firefox/blob/main/modules/libpref/init/StaticPrefList.yaml
- [F5] Firefox, `dom/media/ipc` (`RDDProcessHost`) and `security/sandbox/mac/SandboxPolicyRDD.h` — https://github.com/mozilla-firefox/firefox/tree/main/dom/media/ipc
- [W1] WebKit, `Source/WebKit/GPUProcess/media` — https://github.com/WebKit/WebKit/tree/main/Source/WebKit/GPUProcess/media
- [W2] WebKit, `Source/WebCore/platform/graphics/cg/ImageDecoderCG.cpp` — https://github.com/WebKit/WebKit/blob/main/Source/WebCore/platform/graphics/cg/ImageDecoderCG.cpp
- [L1] Ladybird, `README.md` — https://github.com/LadybirdBrowser/ladybird
- Local: the SDK's `ImageIO.framework/Headers/CGImageProperties.h`
