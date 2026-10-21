# Text, Fonts and Unicode

| | |
|---|---|
| **Status** | **Binding** since [ADR-0011](../adr/0011-text-fonts-and-unicode.md)'s acceptance (2026-10-20, Step 5's Gate C), after the owner's visual check of S5's sheets. Written at Step 5, CP3. **Spike S5** ([results](../research/s5-results.md)) settled TX12: CoreText rasterizes system fonts. TX6, TX10, TX12 and TX13 carry its findings. **Amended by the architecture review's Round 1** (the owner's dispositions, 2026-10-21; [the review](../review/README.md) §5): TX3 and §4, the encodings behind Axiom's interface and `encoding_rs` 0.8.42 graded (RV1-08; CP4) |
| **Question** | How does the engine handle Unicode: properties, segmentation, line breaking, bidi, normalization, encodings? How are fonts found, matched and fallen back to, how are web fonts loaded safely, how is text shaped, and where and how are glyphs rasterized? |
| **Answers** | Step 5 questions Q2 and Q3 ([tracker](../research/step-5-subsystems.md) §3); decision-map item I1, and the text part of P4; the tracker's handed-over questions 2 and 3 (§2.2) |
| **Feeds** | ADR-0011 (CP9); spike S5 (tracker §10); CP4 (the glyph atlas); CP6 (IDNA); CP13 (decoders, sniffing); CP14 (font properties, `@font-face`); CP15 (inline layout); CP16 (`Intl`, `RegExp`, identifiers) |
| **Builds on** | [Rendering pipeline](rendering-pipeline.md) PL7, PL8, PL10, PL11; [invalidation](invalidation.md) IV10; the [process model](../security/process-model.md) §3–§4; [S4 results](../research/s4-results.md) (CoreText in a sandbox); the [dependency policy](../technology/dependency-policy.md) |
| **Last updated** | 2026-10-21: Round 1's accepted findings |

Everything here is **planned**; nothing is implemented. Rules are numbered TX1–TX21. Precedent is sourced (§3); the design is inference unless marked otherwise.

## Summary

- **One Unicode source for the whole engine: ICU4X,** with its data compiled in (TX1). It serves segmentation, properties, normalization and case mapping for text, CSS, URLs and JavaScript alike.
  - Firefox's line and word breaking already run on it, and Chromium vendors it.
  - The SDK exposes no public interface to the system ICU's segmentation, bidi or normalization (observed).
- **Encodings are Axiom's own, in safe Rust** (TX3). The Encoding Standard specifies every decoder. The established crate, `encoding_rs`, is graded `ub-risk-3` by Google, which the dependency policy doesn't admit.
- **Web content sees system fonts and web fonts only** (TX5). User-installed fonts stay hidden: they are a fingerprinting vector, and reading them would mean reading the user's files from a renderer.
- **Fonts are found and fallen back to through CoreText's database** (TX6), as Chromium does on macOS. **Every font is shaped by HarfRust** (TX9), a Rust port of HarfBuzz with Apple's AAT tables. Chromium and Firefox both shape with HarfBuzz on macOS.
- **Web fonts never reach CoreText** (TX7, TX8). They are decoded, parsed, shaped and rasterized only by memory-safe code, so no C font parser ever sees untrusted font data. CoreText reads only Apple's own system fonts.
- **Glyphs are rasterized in the renderer, into an atlas** (TX12–TX14), as PL7 requires, so the browser never sees a font. For system fonts the rasterizer is CoreText or a Rust rasterizer; **spike S5 decides.** If a GPU process later rasterizes paths, renderers send glyph outlines as paths, so fonts still never leave the renderer (TX15).
- **Line breaking, word and grapheme segmentation come from ICU4X,** and bidi from `unicode-bidi` over ICU4X's data (TX16, TX17). Hyphenation is staged, behind an interface (TX18).

## 1. Binding inputs

| Input | Consequence here |
|---|---|
| **PL7, PL8, AC4:** display lists carry text as glyph images from an atlas the renderer rasterized, never font data; the browser never parses a font | TX12–TX15 |
| **PL10:** glyph rasterization runs on the renderer's helper pool | TX19 |
| **PL11:** strings are 8-bit Latin-1 or 16-bit UTF-16; CoreText receives UTF-16 | TX4, TX9 |
| **S4-Q5:** CoreText renders correctly inside the renderer's sandbox, with system fonts and ICU's data readable. Failures are silent. User-installed fonts and on-demand system font assets need further, reviewed rules | TX5, TX6 |
| **Process model §3, note 3, and §4.1:** fonts are parsed in the renderer; the renderer may read `/System/Library/Fonts` and `/usr/share/icu` | TX5–TX8 |
| **IV10:** a font that arrives re-shapes only the text that uses it, under its `font-display` | TX20 |
| **Dependency policy:** DP1–DP2 (Unicode data, shaping, font parsing and rasterization are infrastructure that may be reused); DP8 (`ub-risk-3` and worse not admitted); DP10 (parsers of untrusted input); DP21 (CoreText only in the renderer, never on the browser's side for web fonts) | §4 |
| **ADR-0001:** web content in all languages, in stages | TX1, TX9, TX16 |

## 2. Specifications

What Axiom implements, through the components below. Versions are pinned when code is written.
- **Unicode:**
  - the Bidirectional Algorithm (UAX #9);
  - line breaking (UAX #14);
  - normalization (UAX #15);
  - text segmentation (UAX #29);
  - identifiers (UAX #31, for JavaScript);
  - vertical text layout (UAX #50);
  - emoji (UTS #51);
  - IDNA (UTS #46, for URLs).

  ICU4X 2.3.0 documents UAX #29 at version 17.0.0, and UAX #14 at 15.1.0, or 17.0.0 through its `*_17_*` constructors [U1].
- **The WHATWG Encoding Standard.**
- **CSS:**
  - Fonts Level 4: font matching, `@font-face`, `font-display`;
  - Text Level 3: white space, line breaking, `word-break`, `overflow-wrap`, hyphenation;
  - Writing Modes Level 4.
- **Font formats:** OpenType; WOFF 1.0 and WOFF2; Apple's AAT tables (`morx`, `kerx`, `trak`).

## 3. Precedent

Read on 2026-10-18; dates in brackets are the last commits to each file, from GitHub's API.

| Engine | Text on macOS |
|---|---|
| **Chromium** | **Shapes with HarfBuzz,** reading font tables through Skia's CoreText typeface (`harfbuzz_face.cc` [2026-10-14]) [C1]. **Falls back per character with CoreText:** cascade lists from `CTFontCopyDefaultCascadeListForLanguages`, resolved by `CTFontCreateForString` (`font_cache_mac.mm` [2026-10-14]) [C2]. **Rasterizes with CoreText** through Skia's port (`SkScalerContext_mac_ct.cpp` [2026-06-12]) [C3].<br>Since Chrome 133, web fonts go to the Rust library Skrifa on Linux, Android and ChromeOS. On macOS and Windows Skrifa is only "a fallback, in cases where the system does not support a font format". Every web font passes through the OpenType Sanitizer (OTS) first, inside a sandbox. FreeType left Blink in Chrome 145 [C4].<br>**It vendors** HarfRust 0.14, ICU4X 2.2 (`icu_capi`, `icu_properties`, `icu_normalizer`, `icu_casemap`), `read-fonts` and `skrifa` (`chromium_crates_io/Cargo.toml` [2026-10-08]) [C5]. Its `HarfRustShaping` runtime feature has status `test`, so it is not shipped (`runtime_enabled_features.json5` [2026-10-16]) [C6] |
| **Firefox** | **Shapes with HarfBuzz.** CoreText shapes only fonts that need AAT layout, and only when `gfx.font_rendering.coretext.enabled` is set. That preference is `false` by default (`gfxMacFont.cpp` [2026-09-21]; `StaticPrefList.yaml` [2026-10-16]) [F1].<br>**Rasterizes with CoreText** in WebRender's glyph rasterizer, which detects whether macOS applies font smoothing ("dilation") (`font.rs` [2026-07-19]) [F2].<br>**Line, word, grapheme and sentence breaking use ICU4X's segmenters** (`LineBreaker.cpp` [2026-07-30], `Segmenter.cpp` [2026-06-23]) [F3].<br>**Web fonts** go through OTS. Its WOFF2 decoder, which is C++, runs under RLBox (`gfxUserFontSet.cpp` [2026-10-11]; `gfx/ots`) [F4].<br>**Encodings:** `encoding_rs`, "used in Gecko starting with Firefox 56" [E1] |
| **WebKit** | Shapes with CoreText (`CTRun` glyphs and advances in `ComplexTextControllerCoreText.mm` [2026-10-13]) [W1]. Hyphenates with CoreFoundation's dictionaries (`HyphenationCF.cpp` [2026-10-13]) [W2]. It has an `allowUserInstalledFonts` setting. Its engineers call every case where user-installed fonts leak to the web "a potential fingerprinting vector" (WebKit bug 190195, 2018) [W3] |
| **Servo** | HarfBuzz through `harfbuzz-sys`, with CoreText on macOS; `fontsan`, its OTS binding; ICU4X's properties; `read-fonts` and `skrifa` (`components/fonts/Cargo.toml` [2026-09-20]) [S1] |
| **Ladybird** | HarfBuzz, ICU, Skia and WOFF2, from its dependency manifest (see the [dependency policy](../technology/dependency-policy.md) §2) |

**Observations** (inference):
- **On macOS, two of three engines shape with HarfBuzz rather than CoreText,** and all three rasterize with CoreText. Shaping with one engine on every font gives consistent behaviour and full control of OpenType features. Rasterizing with CoreText matches the platform's look.
- **Every engine protects its C font parsers from web fonts** with OTS, and Firefox sandboxes its C++ WOFF2 decoder as well. A stack that parses web fonts only in memory-safe code needs that protection less. Even so, Chrome still runs OTS in front of Skrifa.
- **Rust is entering every engine's text stack:** ICU4X in Firefox, Chromium and Servo; Fontations in Chromium and Servo; HarfRust in Chromium's tests.

**This Mac** (observed 2026-10-18, macOS 27.2, build 26B5112a):
- **System fonts:** `/System/Library/Fonts` holds 369 font files with 786 faces. Of those faces:
  - 242 have an AAT `morx` table, and 165 of them have no OpenType `GSUB`. Among those are Apple Color Emoji, Devanagari MT, and Arabic fonts such as Al Nile, Baghdad and DecoType Naskh;
  - 58 are variable (`fvar`), and 9 use CFF2;
  - 2 carry `sbix` colour bitmaps (Apple Color Emoji);
  - none has `COLR`, `CBDT` or `SVG `.

  These come from a read-only scan of the files' table directories.
- **On-demand font assets:** `/System/Library/AssetsV2/com_apple_MobileAsset_Font8` holds 60 more files with 268 faces.
- **User-installed fonts:** 1 file in `/Library/Fonts`, and 5 in `~/Library/Fonts`.
- **ICU's data** is `/usr/share/icu/icudt78l.dat`. The SDK's `usr/include/unicode` ships `uchar.h`, `uidna.h`, `uregex.h` and `ustring.h`, but not `ubrk.h`, `ubidi.h` or `unorm2.h`: the system ICU's segmentation, bidi and normalization have no public headers.
- **CoreText's headers** provide `CTFontCreateForString` (macOS 10.5), `CTFontCopyDefaultCascadeListForLanguages` (10.8), `CTFontManagerCreateFontDescriptorFromData` (10.7) and `CTFontCopyTable` (10.5).

## 4. Build or reuse

Grades are Google's published audits, unless stated otherwise (google/supply-chain [2026-09-17] and Chromium's [2025-03-22]; [A1]). Each grade applies to the version audited, so the versions Axiom adopts need their own audits (DP15).

| Need | Candidates | Decision |
|---|---|---|
| **Unicode properties, segmentation, normalization, case mapping** | **ICU4X 2.3:** pure Rust, licensed Unicode-3.0. Grades (2.0 betas): segmenter `ub-risk-1`, properties `ub-risk-0`, normalizer and provider `ub-risk-1`. Its data crates `zerovec` 0.11.1, `zerotrie`, `yoke` and `icu_collections` are `ub-risk-2`, so the tree is `ub-risk-2`. **The system's ICU:** C++; its segmentation, bidi and normalization headers aren't in the SDK. **Tables of our own:** a project in itself (DP4) | **ICU4X** (TX1), a foundational dependency, since text, CSS, URLs and JavaScript all use it |
| **Bidi** | **`unicode-bidi` 0.3.18** (Servo): `ub-risk-1`. It runs Unicode's `BidiTest.txt` and `BidiCharacterTest.txt` as tests, with version 15.0 of those files (observed [S2]). ICU4X can supply its character data | **`unicode-bidi` over ICU4X's data** (TX17) |
| **Encodings** | **`encoding_rs`:** graded **`ub-risk-3`** at 0.8.33, which DP8 doesn't admit. Its 2026 releases rewrote the ASCII paths "to reduce `unsafe`" [E1], but no newer grade is published. **Axiom's own audit of 0.8.42** (Round 1, 2026-10-21): still `ub-risk-3` on this Mac with SIMD off, since 121 of its 143 compiled `unsafe` blocks have no safety comment, uninitialized memory is treated as initialized behind an `asm!` barrier, and `simdutf8`'s ungraded NEON code is unconditional on aarch64 ([audit](../research/encoding-rs-audit.md)). **Our own:** the Encoding Standard specifies every decoder and encoder, with published index tables | **Our own, in safe Rust** (TX3), behind Axiom's interface (DP2). `encoding_rs` serves as a reference to test against |
| **Font database, matching data, fallback** | **CoreText,** a required platform facility, in the renderer. Our own index would duplicate it and miss macOS's fallback cascades | **CoreText** (TX6), for system fonts only |
| **Web-font containers** (WOFF, WOFF2) | A WOFF2 crate (`woff2` 0.3.0, last released 2022) [A2]; Google's C++ decoder, which Firefox sandboxes with RLBox; **our own,** over a reused Brotli decompressor (`brotli-decompressor` 6.1.0) [A2] | **Our own containers,** in safe Rust, over a Brotli decompressor that passes DP10 (TX7) |
| **Font parsing** | **`read-fonts`** (Fontations): `ub-risk-0` at 0.27.3, but its tree includes `bytemuck`, `ub-risk-2` at 1.22.0. Chrome sends web fonts to Skrifa, which is built on it [C4]. **OTS,** C++. **CoreText,** C | **`read-fonts`,** for every font (TX8) |
| **Shaping** | **HarfRust 0.14:** "a Rust port of HarfBuzz", matching HarfBuzz 14.5.1. It "for the most part passes the HarfBuzz test and fuzzing suites", is "completely safe" apart from `bytemuck`, and is "less than 25% slower than HarfBuzz on most common fonts". It has no Graphite [H1]. It implements AAT (`morx`, `kerx`, `trak`; observed in its source, `harfrust/src/aat`). **HarfBuzz,** C++, as Servo binds it. **CoreText,** as WebKit uses it. **Our own:** shaping engines for every script, a project in itself (DP4) | **HarfRust** (TX9), behind Axiom's shaping interface |
| **Rasterization** | **CoreText** (Chromium, Firefox); **a Rust rasterizer** over `skrifa`'s outlines: candidates such as `zeno`, `vello_cpu` and `tiny-skia` [A2] | **Spike S5 decides** for system fonts (TX12). Web fonts use the Rust path (TX7) |
| **Hyphenation** | CoreFoundation's dictionaries (WebKit [W2]); Liang's patterns in Rust, as Firefox's `mapped_hyph` does | **Open,** staged (TX18) |

**A dependency risk: `smallvec`.** HarfRust and ICU4X's segmenter both require it [A2]. Google graded `smallvec` 1.11.0 at `ub-risk-3`. Later versions carry only `safe-to-deploy` audits, from Google, Mozilla and the Bytecode Alliance, with no soundness grade [A1]. The newest version is 1.16.3, and 2.0 is in beta [A2]. So Axiom grades the version it adopts, at the milestone that adds these crates. If that version is still `ub-risk-3`, DP8 bars it, and the fix is upstream: a better-graded `smallvec`, or none.

## 5. Design

### 5.1 Unicode and encodings

| # | Rule |
|---|---|
| TX1 | **ICU4X is the engine's one source of Unicode data and algorithms:** character properties (script, bidi class, line-break class, East Asian width, vertical orientation, emoji, identifier classes), segmentation, normalization and case mapping. It serves text, CSS (case-insensitive matching, `text-transform`), URLs (IDNA, CP6) and JavaScript (`Intl`, `RegExp` property escapes, identifiers, `normalize`; CP16), so every subsystem agrees on one Unicode version. Its data is compiled into the executables, so nothing is loaded or parsed at run time. That includes the data for the scripts whose line breaks depend on context: Khmer, Lao, Myanmar and Thai. For them ICU4X offers an LSTM model, "smaller than the full dictionary but more expensive during segmentation", or a dictionary, which is faster but larger [U2]. Firefox asks for ICU4X's automatic choice (§3). Its size is measured (S5-Q4) |
| TX2 | **ICU4X sits behind Axiom's own `unicode` interface,** in Axiom's types. ICU4X itself is classed as a foundational dependency (DP5), because `Intl` will use its formatters directly (CP16). The interface keeps segmentation and properties replaceable |
| TX3 | **Encodings are Axiom's own, in safe Rust.** The Encoding Standard's decoders and encoders, for every encoding it defines, use its algorithms and WHATWG's index tables (licensed BSD-3-Clause). Rule 5 applies: no `unsafe`. UTF-8 uses the standard library's validation. ASCII fast paths are written so the compiler can vectorize them. The oracle is WPT's `encoding` directory, with `encoding_rs` as a differential reference. HTML's sniffing algorithm is CP13's.<br>**As amended in Round 1** (RV1-08; the owner: "Reclassify WHATWG text encodings … as replaceable details behind clean interfaces (DP2)."): the encodings sit behind an interface Axiom owns, so a graded crate could replace them inside `text`. **Axiom's audit graded `encoding_rs` 0.8.42 `ub-risk-3`** with SIMD off on this Mac (2026-10-21; [audit](../research/encoding-rs-audit.md)), which DP8 doesn't admit, so the implementation stays Axiom's own. Differential fuzzing against `encoding_rs` (TS7) is what pays down the decoders' logic risk |
| TX4 | **No text is converted for these components.** DOM and JavaScript strings are Latin-1 or UTF-16 (PL11). ICU4X's segmenters take both, as Firefox calls them (§3). HarfRust takes code points with any cluster index (`push(codepoint, cluster)`; observed in its `buffer.rs`), so clusters map straight back to UTF-16 offsets. CoreText receives UTF-16 |

### 5.2 Fonts

| # | Rule |
|---|---|
| TX5 | **Web content sees two kinds of font: those that ship with macOS, and web fonts.** User-installed fonts (`/Library/Fonts`, `~/Library/Fonts`) are not exposed. They identify the user, the fingerprinting concern WebKit's setting addresses (§3), and reading them would mean reading user files from a renderer (SG1). macOS's on-demand font assets are system fonts. Making them readable needs a sandbox rule reviewed against S4's method, and until that review they are treated as absent. The browser's own interface draws with AppKit, which can see every font |
| TX6 | **The system font database, matching data and fallback come from CoreText, in the renderer.** The renderer enumerates system families and faces through CoreText. **CSS Fonts' matching algorithm is Axiom's,** over that list, by family, weight, width and style. **Per-character fallback** uses CoreText's language-aware cascade lists (`CTFontCopyDefaultCascadeListForLanguages`), resolved by `CTFontCreateForString`, as Chromium does. Language matters for Han characters, whose glyphs differ between Chinese and Japanese. **The font data itself** is read from each system font's file, mapped read-only, so its pages are shared between renderers, or through `CTFontCopyTable` when a font has no file. Because CoreText fails silently, every resolved font is checked to be the one requested (S4). **S5 (2026-10-18) refines this:** CoreText's fallback for Chinese, PingFang, lives in FontServices' `Reserved` directory, which the renderer's profile must make readable (two rules, in S5's text profile). Without them CoreText resolves a 2,416-byte stub with PingFang's name and no file, and HarfRust shaping it agrees with CoreText. So **the check requires a file-backed font:** a font CoreText resolves without a file is treated as missing ([S5 results](../research/s5-results.md) §3) |
| TX7 | **Web fonts never reach CoreText.** A web font is fetched through the document's loader (FT11), then decoded from WOFF or WOFF2 by Axiom's own container code. WOFF uses zlib, WOFF2 uses Brotli and its glyph transforms, and both are safe Rust over a decompressor admitted by DP10. **It is then validated before use:** its table directory and required tables parse with `read-fonts`, and it stays within limits on size, glyph count and table sizes, set at implementation. Shaping (TX9) and rasterization (TX12) use Rust code only. **So no C font parser sees untrusted font data.** Chrome on macOS and Firefox both rasterize sanitized web fonts with CoreText (inference from §3). A web font is registered only for its own document, and freed with it |
| TX8 | **Every font is parsed by `read-fonts`:** system fonts for shaping, and web fonts for everything. Because the font parser is the same for both kinds, shaping behaves the same on both |

### 5.3 Shaping

| # | Rule |
|---|---|
| TX9 | **HarfRust shapes every run,** system fonts and web fonts alike, including Apple's AAT-only fonts through its `morx`, `kerx` and `trak` support. That is 165 of this Mac's 786 system faces (§3). Chromium and Firefox shape with HarfBuzz on macOS; WebKit's choice of CoreText is rejected, because it would put Apple's C shaping on web fonts, against TX7. **Shaping input:** a run of one font, script, language, direction and feature set, from inline layout (CP15), with OpenType features from CSS (`font-feature-settings`, `font-variant-*`, `font-variation-settings`). **Shaping output:** glyph IDs, clusters, advances and offsets, in font units. HarfRust shapes at units per em [H1], so results scale with size. **Graphite fonts are not supported** (an approximation, recorded) |
| TX10 | **Shaped words are cached.** Text is split at spaces and shaped a word at a time where shaping can't cross the boundary, as Firefox and Blink cache shaped words. The cache key is the font, its variation coordinates, the features, script, language, direction and the text. Size-dependent adjustments, such as AAT tracking, are applied after shaping, as Firefox applies them (§3). **S5 confirmed this (2026-10-18):** CoreText applies a font's normal track (track 0) at the text's size, while HarfRust 0.14 applies `trak` only in fonts that have `STAT`, at 12 pt when given no size, and applies a font's only track whatever its value. So the shaper doesn't see `trak`, and Axiom applies the normal track itself ([S5 results](../research/s5-results.md) §3). The cache is bounded per document, and trimmed under memory pressure ([process model](../security/process-model.md) §7) |
| TX11 | **Shaping sits behind Axiom's interface.** Layout asks for runs; the interface hides HarfRust, so another shaper could replace it without changing layout. S5-Q1 measures HarfRust against CoreText on this Mac's fonts |

### 5.4 Rasterization and the display list

| # | Rule |
|---|---|
| TX12 | **Glyphs are rasterized in the renderer.** Web fonts always use the Rust rasterizer: `skrifa`'s outlines, filled by a Rust rasterizer chosen in S5. **For system fonts, S5 decides between CoreText and the same Rust rasterizer.** CoreText matches the platform's look, as Chromium and Firefox rasterize with it. The Rust rasterizer gives one code path and output that doesn't change with macOS updates (SC8). If S5 isn't authorized, CoreText is the default for system fonts. **S5-Q2's report recommends CoreText for system fonts** (2026-10-18): it alone gives the platform's look at 12–16 px on 1× displays, its font smoothing and its emoji metrics, and it is the fastest (1.8 µs a glyph at 16 px, against 2.7 for `zeno` and 4.6 for `vello_cpu`). For web fonts, `zeno` and `vello_cpu` match CoreText equally well above 20 px; the choice is made with S6-Q4's path rasterizer, so the renderer has one.<br>**Settled at Gate C** (2026-10-20): the owner compared S5's sheets, and "CoreText is approved for system font rasterization under TX12." **S6-Q4** then measured the web-font rasterizer's other use, paths: `vello_cpu` filled them 2.1–6.8× faster than `zeno`, at the same quality. **The owner adopted `vello_cpu`** (2026-10-20) "as the primary Rust rasterizer for web fonts and paths, subject to crate dependency grading at the text milestone" (DP8, DP15) ([S5 results](../research/s5-results.md) §4, §9; [S6 results](../research/s6-results.md) §6, §8) |
| TX13 | **The atlas holds coverage masks and colour bitmaps.** Monochrome glyphs are 8-bit coverage. Colour glyphs, such as Apple Color Emoji's `sbix` bitmaps, or later `COLR` and `SVG ` web fonts, are premultiplied RGBA in a separate atlas. Glyphs are keyed by font, size, variation coordinates and a quantized horizontal subpixel phase. Whether to emulate macOS's font smoothing, the "dilation" WebRender detects (§3), is a quality question for S5-Q2. **S5-Q2 measured it:** smoothing adds 16–52% ink, most at small sizes on 1× displays, and is the same in gray and RGB contexts. Recommended: emulate it for web fonts when the system's setting is on, calibrated against CoreText, at the text milestone ([S5 results](../research/s5-results.md) §4) |
| TX14 | **The display list stays as PL7 defines it:** text items give atlas entries and positions, never font data or glyph IDs. Atlases are renderer resources (PL8), shared with the compositor, counted against the renderer's budget, and evicted least-recently-used. **No amendment to PL7 is needed** (SC1) |
| TX15 | **Fonts never leave the renderer, even with a GPU process.** If CP4 or S6 shows that GPU rasterization of paths pays, a renderer sends glyph outlines as validated path primitives instead of masks (S6-Q4: it pays in time and memory, not in quality; the owner deferred GR5's amendment to the graphics milestone). The GPU process then fills paths, and never parses a font. That answers the handed-over question 3: rasterization stays in the renderer, and a GPU process would receive paths, never fonts (PL8, AC4) |

### 5.5 Segmentation, bidi and hyphenation

| # | Rule |
|---|---|
| TX16 | **ICU4X segments text:** line-break opportunities follow UAX #14, with CSS's `line-break` and `word-break` options, which its line segmenter supports [U1]. Word, grapheme-cluster and sentence boundaries follow UAX #29. Firefox's breaking runs on the same segmenters (§3). Layout applies `white-space`, `overflow-wrap` and `word-break: break-all` over these opportunities (CP15) |
| TX17 | **Bidi follows UAX #9, through `unicode-bidi` over ICU4X's character data,** so the Unicode version is one. Paragraph levels come from the DOM's `dir` and CSS's `direction` and `unicode-bidi`. Layout reorders runs by level (CP15) |
| TX18 | **Hyphenation sits behind an interface,** which returns the hyphenation opportunities in a word for a language. `hyphens: manual` (soft hyphens) needs no dictionary. `hyphens: auto` waits for the milestone that implements it, which chooses between CoreFoundation's hyphenation in the renderer, as WebKit uses (§3), and Liang's patterns in safe Rust, as Firefox uses, with dictionaries whose licenses DP17 allows |

### 5.6 Threads, memory and determinism

| # | Rule |
|---|---|
| TX19 | **Threads:** segmentation, bidi and shaping run on the event loop's thread, inside layout (PL10). Rasterizing a glyph runs on the helper pool, and writes into an atlas page the compositor reads only after the display-list update that references it (PL8, PL14) |
| TX20 | **Late fonts invalidate only their text** (IV10): when a web font arrives or fails, the runs that used its fallback are re-shaped and re-laid out, under `font-display`'s periods |
| TX21 | **Memory has owners** (PL13):<br>• system fonts are mapped files, shared between processes;<br>• web fonts belong to their document;<br>• the shaped-word cache and the atlases are bounded per renderer;<br>• ICU4X's data is part of the executable, shared.<br>Estimates for budgets: an 8-bit atlas page of 1,024 × 1,024 is 1 MiB, and an RGBA page is 4 MiB, the second allocated only when colour glyphs appear. S5-Q4 measures the stack's footprint against B2 (10 MB for a ready renderer). **Tests stay deterministic** (SC8): reference tests compare two renderings in one build, and pixel tests use web fonts, such as WPT's test fonts, which take the Rust path and so don't change with macOS |

## 6. Interfaces with neighbouring subsystems

| With | Interface |
|---|---|
| **Layout** (CP15) | Itemization into runs (script, bidi level, font, language); shaped runs with clusters and advances; break opportunities; hyphenation; font metrics (ascent, descent, line gap, x-height, underline position) |
| **Style** (CP14) | Font descriptors from computed `font-*` values; `@font-face` rules and their loading state |
| **Paint and the compositor** (CP4, CP15) | Glyph atlas entries and positions in display items (PL7); atlas pages as resources (PL8) |
| **HTML** (CP13) | The Encoding Standard's decoders, for the parser's sniffing and decoding |
| **URLs** (CP6) | IDNA's mapping and normalization data from ICU4X. In the browser, DP9 asks `ub-risk-1` for code that handles data from renderers, while ICU4X's data crates are graded `ub-risk-2`. CP6 must resolve how the browser processes IDNA |
| **JavaScript** (CP16) | Unicode properties, case mapping and normalization; ICU4X's formatters for `Intl` |

## 7. Security

- **Rule of 2.** Untrusted text and web fonts are processed only in the renderer, a sandboxed process, and only by safe Rust (TX7–TX9). CoreText, which is C, sees Apple's own system fonts only (TX6, TX12). The browser never parses a font (AC4, TX14–TX15).
- **`unsafe`.** Axiom's own text code has none: encodings, WOFF containers and validation are parsers under rule 5. CoreText's calls live in the renderer's `platform` module (DP21). The reused crates are graded under DP8–DP10. Every tree named here is `ub-risk-2` or better, except for `smallvec`'s grade (§4).
- **Resource exhaustion.** Web fonts are bounded in size, glyph count and table sizes (TX7). Shaping and rasterization time count against the renderer's hang deadlines (EL §4.7).

## 8. Staging and the complete model

| Stage | What exists |
|---|---|
| First text milestone | System fonts through CoreText's database; HarfRust shaping; ICU4X line breaking and bidi; the atlas; the encodings |
| Web fonts | WOFF and WOFF2 through the Rust path (TX7) |
| Later | Vertical text (Writing Modes, UAX #50; 92 system faces carry `vmtx`); `hyphens: auto`; colour web fonts (`COLR`, then `SVG `); `font-variant-*` completeness |
| L0–L3 | No change: text runs in the renderer at every level |

**The complete model** (SC9): every script, through ICU4X's data, HarfRust's shapers and macOS's fallback cascades; variable fonts through `read-fonts` and `skrifa`; colour fonts through the RGBA atlas; vertical text through ICU4X's vertical-orientation data and HarfRust's vertical shaping; a GPU process through glyph outlines (TX15). Graphite is the recorded exception.

## 9. Alternatives considered

| | Alternative | Assessment |
|---|---|---|
| A | **CoreText for everything,** as WebKit does | It gives native shaping and the platform's look with no dependencies. But it puts Apple's C font parsers on every web font, against TX7 and the Rule of 2's intent, and leaves OpenType feature behaviour to the OS. Rejected |
| B | **HarfBuzz itself, through bindings,** as Servo does | Mature and complete. But it is C++ parsing untrusted fonts, and a pure-Rust port exists, so DP10's exception can't be justified. Rejected |
| C | **A sanitizer (OTS) in front of CoreText for web fonts,** as Chrome does on macOS | OTS is itself C++ parsing untrusted input, and it narrows the risk without removing it. TX7 removes it. Rejected |
| D | **The system's ICU for segmentation** | Its segmentation, bidi and normalization headers are not in the SDK (§3), so it would mean undeclared interfaces, and C++ on untrusted text. Rejected |
| E | **`encoding_rs`** | Graded `ub-risk-3`. Rejected under DP8; kept as a reference (TX3) |
| F | **Exposing user-installed fonts** | It is a fingerprinting vector, and needs user-file reads from the renderer. Rejected (TX5) |

## 10. Open questions

| # | Question | Where it is settled |
|---|---|---|
| 1 | CoreText or a Rust rasterizer for system fonts; whether to emulate font smoothing | Spike S5 (TX12, TX13). **Settled** at Gate C (2026-10-20): CoreText for system fonts, after the owner compared S5's sheets; smoothing is emulated for web fonts at the text milestone (TX13) |
| 2 | The grade of the `smallvec` version adopted (§4) | The milestone that adds HarfRust or ICU4X's segmenter |
| 3 | Whether to make macOS's on-demand font assets readable from renderers, and with which reviewed rule | The first text milestone, by S4's method |
| 4 | The hyphenation backend (TX18) | The milestone that implements `hyphens: auto` |
| 5 | How the browser processes IDNA within DP9 (§6) | CP6 |
| 6 | The size of ICU4X's compiled data, by component | S5-Q4. **Answered** (2026-10-18): stripped, the rule-based line breaker adds 65 KiB, the "auto" one with its dictionaries and LSTM models 389 KiB, and line, word, grapheme and sentence segmentation with the Script property 2.38 MiB; the data adds nothing measurable to the footprint |

## Sources

All read on 2026-10-18. Dates in brackets are the last commits to each file, from GitHub's API.

- [U1] ICU4X, `icu_segmenter` 2.3.0 documentation — https://docs.rs/icu_segmenter/latest/icu_segmenter/
- [U2] ICU4X, `LineSegmenter` (2.3.0): `new_auto`, `new_lstm`, `new_dictionary` — https://docs.rs/icu_segmenter/2.3.0/icu_segmenter/struct.LineSegmenter.html
- [H1] HarfRust, README [2026-10-16] — https://github.com/harfbuzz/harfrust
- [E1] `encoding_rs`, README [2026-09-24] — https://github.com/hsivonen/encoding_rs
- [A1] Google's audits — https://github.com/google/supply-chain/blob/main/audits.toml ; Chromium's — https://github.com/chromium/chromium/blob/main/third_party/rust/chromium_crates_io/supply-chain/audits.toml
- [A2] crates.io's API, for versions, dates and dependencies: `harfrust`, `icu_segmenter`, `smallvec`, `woff2`, `brotli-decompressor`, `zeno`, `vello_cpu`, `tiny-skia` — https://crates.io/
- [C1] Chromium, `third_party/blink/renderer/platform/fonts/shaping/harfbuzz_face.cc` — https://github.com/chromium/chromium/blob/main/third_party/blink/renderer/platform/fonts/shaping/harfbuzz_face.cc
- [C2] Chromium, `third_party/blink/renderer/platform/fonts/mac/font_cache_mac.mm` — https://github.com/chromium/chromium/blob/main/third_party/blink/renderer/platform/fonts/mac/font_cache_mac.mm
- [C3] Skia, `src/ports/SkScalerContext_mac_ct.cpp` — https://github.com/google/skia/blob/main/src/ports/SkScalerContext_mac_ct.cpp
- [C4] "Memory safety for web fonts", Chrome for Developers (2025-03-19, updated 2026-04-24) — https://developer.chrome.com/blog/memory-safety-fonts
- [C5] Chromium, `third_party/rust/chromium_crates_io/Cargo.toml` — https://github.com/chromium/chromium/blob/main/third_party/rust/chromium_crates_io/Cargo.toml
- [C6] Chromium, `third_party/blink/renderer/platform/runtime_enabled_features.json5` — https://github.com/chromium/chromium/blob/main/third_party/blink/renderer/platform/runtime_enabled_features.json5
- [F1] Firefox, `gfx/thebes/gfxMacFont.cpp` and `modules/libpref/init/StaticPrefList.yaml` — https://github.com/mozilla-firefox/firefox/blob/main/gfx/thebes/gfxMacFont.cpp
- [F2] Firefox, `gfx/wr/wr_glyph_rasterizer/src/platform/macos/font.rs` — https://github.com/mozilla-firefox/firefox/blob/main/gfx/wr/wr_glyph_rasterizer/src/platform/macos/font.rs
- [F3] Firefox, `intl/lwbrk/LineBreaker.cpp` and `Segmenter.cpp` — https://github.com/mozilla-firefox/firefox/tree/main/intl/lwbrk
- [F4] Firefox, `gfx/thebes/gfxUserFontSet.cpp` and `gfx/ots` — https://github.com/mozilla-firefox/firefox/blob/main/gfx/thebes/gfxUserFontSet.cpp
- [W1] WebKit, `Source/WebCore/platform/graphics/coretext/ComplexTextControllerCoreText.mm` — https://github.com/WebKit/WebKit/blob/main/Source/WebCore/platform/graphics/coretext/ComplexTextControllerCoreText.mm
- [W2] WebKit, `Source/WebCore/platform/text/cf/HyphenationCF.cpp` — https://github.com/WebKit/WebKit/blob/main/Source/WebCore/platform/text/cf/HyphenationCF.cpp
- [W3] WebKit bug 190195, "User installed fonts are not always disabled when they should be" (2018-10-02), comments 7–10 — https://bugs.webkit.org/show_bug.cgi?id=190195
- [S1] Servo, `components/fonts/Cargo.toml` — https://github.com/servo/servo/blob/main/components/fonts/Cargo.toml
- [S2] `unicode-bidi`, `tests/conformance_tests.rs` [2026-09-10] — https://github.com/servo/unicode-bidi
- Local: the SDK's `CoreText.framework/Headers` and `usr/include/unicode`; `/usr/share/icu`; a read-only scan of the table directories under `/System/Library/Fonts` and `/System/Library/AssetsV2/com_apple_MobileAsset_Font8`
