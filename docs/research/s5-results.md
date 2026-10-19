# S5 Results: The Text Stack and ImageIO in the Renderer's Sandbox

| | |
|---|---|
| **Status** | Complete (2026-10-18). **The owner's visual checks of S5-Q1 and S5-Q2 were made at Gate C** (2026-10-20): "I have visually inspected the S5 sheets and verified the Hebrew niqqud and Apple Color Emoji differences. CoreText is approved for system font rasterization under TX12." So S5-Q1 passes, and TX12 is settled; ADR-0011 is accepted. S5-Q5's recommendation, AVIF in the media utility process, was approved at the same gate |
| **Question** | Does HarfRust shape like CoreText on this Mac's system fonts (S5-Q1)? How do Rust rasterizers' glyphs compare with CoreText's, and at what cost (S5-Q2)? Do valid and malformed web fonts go through a Rust-only path inside S4's renderer profile, with no new rule (S5-Q3)? What does each text stack add to a sandboxed helper's footprint and executable, and how fast is shaping (S5-Q4)? Does ImageIO decode AVIF under the renderer profile, and how do the safe image decoders compare with ImageIO (S5-Q5)? |
| **Feeds** | [ADR-0011](../adr/0011-text-fonts-and-unicode.md) (its acceptance conditions: S5-Q1, S5-Q3, and TX12 from S5-Q2); [ADR-0012](../adr/0012-graphics-images-and-media.md) (its S5-Q5 condition); [text and fonts](../architecture/text-and-fonts.md) TX6, TX10, TX12, TX13; [images and media](../architecture/images-and-media.md) IM2, IM3; the [dependency register](../technology/dependency-register.md); a refinement of the [process model](../security/process-model.md) §4.1 for the owner |
| **Plan** | [Step 5 tracker](step-5-subsystems.md) §10.1, authorized at Gate B (2026-10-18) |
| **Code** | `spikes/s5/rust-text` (`s5-rust-text`), `spikes/s5/text` (`s5-text`) and `spikes/s5/image` (`s5-image`), on branch `step-5`. Formal run at commit `bad07a8`, reproduced at `f20e7f4` (§2) |
| **Last updated** | 2026-10-20: the owner's checks at Gate C, in the status above |

**Evidence labels** follow the [research conventions](README.md):
- **Measured** facts come from S5's binaries and scripts, in the formal run named in §2 unless stated otherwise.
- **Observed** facts were read on this Mac: font tables, crash reports, the denial log, processes. Each names its file or command.
- **Sourced** facts link to their source, listed at the end.
- **Inference** marks an explanation drawn from them; **hypothesis** marks an untested claim, with the step that would test it.

---

## Summary

| Question | Answer |
|---|---|
| **S5-Q1** Does HarfRust shape like CoreText? | **No shaping error found; the owner's visual check is outstanding.** Over 432 runs (22 lines covering every script S5-Q1 names, and emoji, at 16 and 32 pt, in the 16 fonts CoreText's fallback chose), HarfRust chose **the same glyphs in every run**, with no errors. 365 runs are identical. 38 put the same glyphs at the same positions and attribute an adjustment differently. 15 put the same glyphs at the same positions but end the run elsewhere: CoreText's private emoji spacing, and vocalized Arabic's marks. **14 place marks differently:** Hebrew niqqud in Lucida Grande, a font with no mark-positioning data at all, so each engine uses its own fallback rules (§3) |
| **S5-Q2** How do Rust rasterizers compare? | **Report, with a recommendation for TX12: CoreText rasterizes system fonts.** At 20 px and above, and at every size on a 2× display, `zeno` and `vello_cpu` match CoreText's unsmoothed masks to 0.56–1.06/255 on average. At 12–16 px on a 1× display they differ by 4.3–5.7/255, because CoreText's ink sits 0.10–0.18 px higher. macOS's font smoothing adds 16–52% ink, and CoreText draws Apple Color Emoji larger and lower than the font's data says. CoreText is also the fastest: 1.83 µs a glyph at 16 px, against 2.72 for `zeno` and 4.57 for `vello_cpu` (§4) |
| **S5-Q3** Do web fonts go through the Rust path safely? | **Passes.** Under S4's renderer profile, unchanged: all 322 WPT fonts with an expectation behave as expected (288 render, 34 are rejected, for the reasons their names give), with outcomes and render hashes identical to an unsandboxed run. **4,000 malformed variants** of 8 fonts, each in its own sandboxed process: 2,709 render, 1,291 are rejected, **no crash, no hang**. The web-font executable links no Apple framework, and drew **no sandbox denial** in 4,322 sandboxed processes (§5) |
| **S5-Q4** Footprint, size and speed? | **B2 stays achievable.** A sandboxed helper's footprint after its first text (one Latin line) is 2.52 MiB with the Rust path, 3.03 MiB with HarfRust and CoreText rasterization, and 3.08 MiB with CoreText alone, against 1.77 MiB doing nothing. After the whole corpus: 6.25, 8.86 and 8.97 MiB. ICU4X's segmentation data adds nothing measurable to the footprint. Stripped, HarfRust adds 0.95 MiB to an executable, `skrifa` with `zeno` 0.93 MiB, and ICU4X's line breaker with its dictionary and LSTM data 389 KiB. **HarfRust shapes in 566 ns a glyph,** CoreText the same runs in 1,227 ns, and in 3,036 ns with its line layout and fallback (§6) |
| **S5-Q5** ImageIO in the renderer; the safe decoders? | **ImageIO decodes AVIF in the renderer's sandbox only with six rules,** and one of them (R4) opens VideoToolbox's decoder service, which the process model denies renderers. Every format first needs R1, without which ImageIO crashes. **So the condition is not met as the process model stands, and IM3's fallback applies:** AVIF decoded outside the renderer, recommended in the media utility process (§7, §9). **IM2's JPEG choice:** the safe `jpeg-decoder` is 1.1–1.8× slower than ImageIO in the sandbox, and gives the same pixels every time; ImageIO's JPEG pixels varied between decodes. The safe PNG, GIF and WebP decoders are faster than ImageIO, and use less memory |

**Main findings**, beyond the pass criteria:
1. **CoreText silently substitutes a stub for PingFang under S4's text profile.** PingFang, CoreText's fallback for Chinese, lives in FontServices' `Reserved` directory, which the profile can't read. CoreText then resolves a 2,416-byte font with PingFang's name and no file, which maps every Han character to glyph 4, and HarfRust shaping the stub agrees with CoreText. **A font check by name, or by shaping agreement, misses it; a check that the font is file-backed catches it** (TX6).
2. **ImageIO crashes in any renderer-like profile unless it can read the executable's directory's metadata.** The crash reports show it asking whether the application opted in to out-of-process decoding, which reads the main bundle's Info dictionary; inside the profile the bundle is NULL. **CoreText hits it too,** when it draws Apple Color Emoji, whose bitmaps it decodes through ImageIO.
3. **CoreText applies a font's normal track at the text's size; HarfRust 0.14 applies `trak` differently.** HarfRust uses `trak` only in fonts that also have `STAT`, assumes 12 pt when given no size, and applies a font's only track whatever its value. TX10's design, Axiom applying the normal track after shaping, is what matches CoreText.
4. **ImageIO's JPEG output is not deterministic.** For the large images, its two ways of decoding give different pixels, and once it gave the other way's; in the sandbox, where the hardware JPEG decoder is denied, it gives a third result, and with the AVIF rules the results varied between processes. `jpeg-decoder` gave the same pixels for each file in every process.
5. **Everything deterministic reproduced byte for byte** across the two formal runs, at two commits.

---

## 1. What was built

S5 is three crates, where §10.1 planned one (`s5-text`): this split keeps the web-font executable free of Apple frameworks, so S5-Q3's last condition can be checked by `otool -L`.

| Crate | What it does | `unsafe` |
|---|---|---|
| `s5-rust-text` (library, and the `s5-webfont` executable) | The Rust-only path of TX7, TX8 and TX12. **WOFF 1.0 [S1] and WOFF2 [S2] decoding of our own,** following Google's reference decoder [R1] where the specification leaves a check implicit: WOFF2's `glyf`, `loca` and `hmtx` transforms, its collections, and Brotli [S3] through `brotli-decompressor`. **Validation** with `read-fonts` and `skrifa`, within limits (30 MiB, 128 tables). **Shaping** with HarfRust, with AAT tracking applied by Axiom after shaping (TX10). **Rasterization:** `skrifa`'s outlines filled by `zeno` or `vello_cpu`; `sbix`, `CBDT` and `EBDT` colour bitmaps through `png`. A deterministic mutator writes malformed variants of a font. 3,864 lines, 27 tests | **None:** `#![forbid(unsafe_code)]`. `s5-webfont` links only `libSystem`, `libsandbox` and `libbsm` (`otool -L`, recorded in each run's `environment.txt`) |
| `s5-text` | CoreText against the Rust path on this Mac's system fonts: shaping (S5-Q1), rasterization (S5-Q2), and footprint, speed and executable size (S5-Q4). Six size probes, each linking one component. S5's text profile, `profiles/text.sb`. 3,030 lines, 8 tests | 42 blocks and 2 `unsafe impl`s, all in `src/platform/`, under invariants C1–C6 (allowlisted) |
| `s5-image` | ImageIO under S4's renderer profile, against the safe decoders `png`, `jpeg-decoder` (in its `platform_independent` mode, which forbids `unsafe`), `gif` and `image-webp` (S5-Q5). S5's ImageIO rules, `profiles/imageio.sb` (R1) and `profiles/avif.sb` (R2–R6). 759 lines, 5 tests | 6 blocks, in `src/platform/`, under invariants I1–I4 (allowlisted) |

**Inputs:**
- **Text:** `spikes/s5/corpus/corpus.txt`, 22 lines: Latin in Helvetica, SF and Times New Roman; Vietnamese, Greek, Cyrillic; Arabic, with and without harakat; Hebrew, with and without niqqud; Devanagari (with conjuncts), Bengali, Tamil; Thai, Khmer, Myanmar; Chinese (simplified and traditional), Japanese, Korean; and emoji sequences (a ZWJ family, a skin tone, flags, a keycap, a VS16 heart, the rainbow flag, a ZWJ astronaut with a skin tone, a technologist). Each line names a base font, and CoreText's fallback chooses the rest.
- **Web fonts:** 327 files from web-platform-tests at commit `850b36a` (2026-10-18), listed in `spikes/s5/corpus/webfonts.txt` and fetched and hash-checked by `tools/s5-fetch.sh` [W1]. 322 carry an expectation: 288 must render and 34 must be rejected, from the WOFF2 suite's index for its test fonts, and "render" for the real fonts under `fonts/`.
- **Images:** made by `tools/s5-images.sh` from pictures that ship with macOS (a 512² photograph, and a desktop picture scaled to 3000²) and a generated 2048² pattern of gradients, edges and noise. Each is written as PNG, JPEG (quality 85), GIF and AVIF by `sips`, and as lossless WebP by `image-webp`'s encoder. Nothing is downloaded.

**Scripts:** `tools/s5-fetch.sh`, `tools/s5-images.sh`, and `tools/s5-measure.sh`, which builds release binaries, signs copies ad hoc with the hardened runtime, and runs everything below (§11).

## 2. Method and environment

- **Machine:** MacBook Air M1 (`MacBookAir10,1`, 8 GB), macOS 27.2 (26B5112a), on AC power with Low Power Mode off (`pmset`); rustc 1.94.1; Command Line Tools only. Every binary records SDK 27.0 and `flags=0x10002(adhoc,runtime)`.
- **Formal run:** `tools/s5-measure.sh --crash-checks` at commit `bad07a8`, from a clean tree: `target/s5-results/20261018-204642/`. **Reproduction:** the same script at `f20e7f4`: `target/s5-results/20261018-203606/`.
  - Every deterministic output is byte-identical between the two: shaping verdicts and glyph detail, mask comparisons, WPT outcomes and render hashes, mutant outcomes, pixel comparisons and the sheets (measured, `diff` and `cmp`).
  - `bad07a8` added the Latin-only footprint configurations (§6), fixed the decoders' peak memory to the first decode's (at `f20e7f4` it also counted the first image, held while later repeats decoded), and added the JPEG determinism check. So the reproduction's image peaks are not used.
- **Timing:** three timed runs, each after a 60 s cool-down, as S1 and S3 did. Each figure is the median of the three runs' medians. **The Mac was in normal use, not idle:** load averages were 2.4–3.3 at the start.
- **Footprint:** the physical footprint (`ri_phys_footprint` and `ri_lifetime_max_phys_footprint`, from `proc_pid_rusage`), as S4 read it, of a sandboxed helper that has done its work and waits. §10.1 named `/usr/bin/time -l`, whose "peak memory footprint" is the second counter; S4's tooling reads both directly.
- **Shaping comparison (S5-Q1).** For each CoreText run, merged with visually adjacent runs of the same font and direction, HarfRust shapes the same UTF-16 range with the rest of the line as context, in the same font file and variation, and is compared glyph by glyph. Positions are anchored at CoreText's first glyph, with a tolerance of 0.01 pt. The verdicts:
  - **same:** the same glyphs, positions and advances;
  - **split:** the same glyphs at the same positions and the same total advance, with an adjustment attributed to a different glyph;
  - **advance:** the same glyphs at the same positions, but the run's total advance differs;
  - **positions:** the same glyphs, with some position different;
  - **glyphs:** different glyphs; **error:** HarfRust failed.
- **Rasterization comparison (S5-Q2).** 199 outline glyphs and 9 colour glyphs, the first distinct glyphs of each line, at 12–32 px at 1× and 2×. The reference is CoreText's 8-bit mask with smoothing off. Each comparison reports the mean absolute difference (0–255), the share of pixels differing by more than 32, and the ink ratio. Timing draws each glyph 100 times: CoreText into one reused context, and the Rust path from outline extraction to filled mask.
- **Images (S5-Q5).** Every decode ends in premultiplied sRGB RGBA in Axiom's own buffer. ImageIO is used two ways: decoding when the image is created, then drawing it ("decode, then draw"); or caching nothing, so drawing decodes ("draw decodes"). Each decode runs in a fresh process, in three phases: cold; after a warm-up decode of a 16² image of the same format ("warm"); and warm inside the profile a renderer would use for that decoder ("sandboxed"). Peak memory is the first decode's, its output included, from the warm phase: in the sandboxed phase, compiling the profile in the process raises the high-water mark first, which hides small decodes.

## 3. S5-Q1: shaping

**Verdicts, unsandboxed** (measured, `q1-shape-none.txt`; identical under S5's text profile):

| Line | Font CoreText chose | Runs that differ (of the line's runs at both sizes) | Largest difference |
|---|---|---|---|
| Hebrew with niqqud | Lucida Grande | **14 positions** | Marks move up to 0.98 pt at 16 pt, 1.95 pt at 32 pt |
| Emoji sequences | Apple Color Emoji | 9 advance (16 pt only) | 21 pt per emoji in CoreText, 16 pt (the font's advance) in HarfRust |
| Arabic with harakat | Geeza Pro | 6 advance | HarfRust's run is 0.58–1.16 pt wider at 16 pt, 1.16–2.31 pt at 32 pt |
| Myanmar | Noto Sans Myanmar | 22 split | — |
| Arabic | Geeza Pro | 8 split | — |
| Devanagari | Kohinoor Devanagari | 4 split | — |
| Latin (Helvetica), Latin (Times New Roman) | Helvetica, Times New Roman | 2 split each (kerning) | — |
| The other 14 lines, including all CJK, Thai, Khmer, Tamil, Bengali and SF | — | **None** | — |

Totals over 432 runs: 365 same, 38 split, 15 advance, 14 positions, **0 glyphs, 0 errors**.

**The differences, one by one:**
- **Hebrew niqqud.** Lucida Grande's face has no GPOS, GDEF, `kerx` or `ankr` table (observed, its table directory in `/System/Library/Fonts/LucidaGrande.ttc`), so it carries no mark-positioning data. Each engine therefore places the niqqud by its own fallback rules: HarfRust by HarfBuzz's fallback mark positioning, CoreText by its own. **Inference:** this is a difference "both engines' rules allow", which the criterion reports rather than fails, unless the owner's visual check finds a mark misplaced.
- **Emoji spacing.** At 16 pt, CoreText advances each Apple Color Emoji glyph 21 pt, where the font's `hmtx` says one em; at 32 pt the engines agree (measured). The glyphs are the same.
  - **CoreText's advances at other sizes** (observed with a probe of `CTLine` and `CTFontGetAdvancesForGlyphs`, outside the formal runs): 12 pt at 9 pt, 16 at 12, 19 at 14, 21 at 16, 23 at 20 and 25 at 24, and the font's own advance from 26 pt.
  - **The font's `trak` table doesn't explain it.** It has one track, −1, of up to 46 units of 800 (0.92 pt at 16 pt): the same shape, a fifth of the size.
  - **Inference:** a rule CoreText applies to this font, outside the font's data. §4 finds the same in the glyphs' size and placement.
- **Vocalized Arabic.** Every glyph is at the same position. CoreText gives each haraka a negative advance and its base a correspondingly larger one, where HarfRust gives the haraka a positive advance; the runs' totals differ by about one mark's width (measured, `q1-shape-none-detail.txt`). In a line, text after the run would shift by that much.
- **Splits** keep every glyph where it was. In Myanmar, CoreText gives marks small advances where HarfRust gives them none, and HarfRust merges a base and its marks into one cluster where CoreText keeps one index per character.

**CoreText under the profiles** (measured, `q1-sandbox-compare.txt`). Under S5's text profile, CoreText's font and glyphs match unsandboxed in all 432 runs. **Under S4's text profile, 4 runs differ:** the two Chinese lines at both sizes. There CoreText resolves `PingFangSC-Regular` and `PingFangTC-Regular` with **no file and 2,416 bytes of tables**, where unsandboxed it reads the 61 MB `PingFangUI.ttc` in `/System/Library/PrivateFrameworks/FontServices.framework/Resources/Reserved/`. Unsandboxed, every run's font in the corpus is file-backed (386 runs in `/System/Library/Fonts`, 42 in `Supplemental`, 4 in FontServices' `Reserved`). The stub maps every Han character to glyph 4, and **HarfRust shaping the stub reports "same"** (measured): agreement between the engines doesn't detect it.

**S5's text profile** is S4's plus three rules, each found by removing it and comparing glyphs and pixels with an unsandboxed run (S4's method):
- `file-read-metadata` of the executable's directory (§7, R1): without it CoreText crashes drawing colour emoji (measured: exit 139 in `q1-crash-check-s4-text.txt`; the crash report's frames are ImageIO's opt-in check, §7);
- `file-read*` of FontServices' `Resources` directory and `Reserved` subtree, and `file-read-metadata` of each component of their path, symbolic links included: without any one of them the stub comes back (informal removal tests).

All three reach only Apple's sealed system volume and one directory's metadata.

**Tracking (TX10)** (measured, `q1-shape-harfrust-tracking.txt`; sourced and observed facts as named):
- **CoreText applies a font's normal track at the text's size.** SF's `trak` table has tracks −1, 0 and +1, and its normal track is −40 units at 16 pt and +26 at 32 pt (observed, `/System/Library/Fonts/SFNS.ttf`, unitsPerEm 2048). With Axiom applying that track after shaping, the SF line matches CoreText exactly at both sizes, 119 glyphs. Apple's specification names track 0 "normal" but leaves the choice of track to "the user or application" [A1].
- **HarfRust 0.14.0 applies `trak` itself, differently** [H1]: only when the font also has a `STAT` table (`src/shaper_font.rs`), at 12 pt when given no size (`src/aat/trak.rs`, "CoreText fallback"), and, in a font with one track, that track whatever its value.
- **PingFang shows the difference.** It has `STAT` and a single track, +1 ("loose"), of 20 units at 16 pt and 9 at 32 pt (observed). CoreText applies none of it. HarfRust applies it: with `--tracking harfrust`, the Chinese lines drift 13.44 pt over 43 characters at 16 pt and 12.10 pt at 32 pt, and only those 4 runs change.

## 4. S5-Q2: rasterization

**Masks against CoreText's, smoothing off** (measured, `q2-raster-none.txt`; mean absolute difference over 199 glyphs, 0–255, with the share of pixels more than 32 apart):

| Size | `zeno` | `vello_cpu` |
|---|---|---|
| 12 px, 1× | 5.66 (5.79%) | 5.57 (5.73%) |
| 14 px, 1× | 4.63 (4.97%) | 4.56 (4.99%) |
| 16 px, 1× | 4.44 (4.11%) | 4.34 (4.19%) |
| 20–32 px, 1× | 0.93–1.06 (0.31–0.39%) | 0.59–0.71 (0.25–0.34%) |
| 12–32 pt, 2× | 0.66–1.04 (0.29–0.39%) | 0.56–0.72 (0.26–0.34%) |

The ink ratios are 0.96–0.99 at 12–16 px and within 0.005 of 1 above. Under S5's text profile every comparison row is identical to unsandboxed (measured, `q2-sandbox-diff.txt`).

**Where the small sizes differ.**
- **The differences are concentrated in some fonts:** at 12–16 px at 1×, the mean for `zeno` is 6.5–7.8 in Helvetica, Times New Roman, SF, Apple SD Gothic Neo and Hiragino Sans, 4.9–5.0 in Lucida Grande and Kohinoor, 2.3–2.8 in Thonburi, Tamil, Khmer and Myanmar, and 0.6 in Geeza Pro.
- **CoreText's ink sits higher.** Its ink centroid is 0.18 px above `zeno`'s at 12 px and 0.11 px at 14–16 px, and within 0.005 px at 20 px and above and at every 2× size (measured, the "centroid" lines).
- **It is not light hinting.** `skrifa`'s light hinting moves the masks further from CoreText's: 10.50 at 16 px and 7.06 at 32 px.
- **Inference:** CoreText adjusts small glyphs on 1× displays in a way neither Rust rasterizer reproduces. The sheets show it at the tops of stems and bowls.

**Font smoothing.** CoreText's smoothing adds ink: 1.33–1.52× at 1× (most at 12–20 px) and 1.16–1.44× at 2×, the same in a gray and an RGB context, so there is no subpixel colour (measured). Neither `AppleFontSmoothing` nor `CGFontRenderingFontSmoothingDisabled` is set on this Mac (observed, `defaults read -g`), so the system default applies to applications.

**Colour emoji.** The Rust path draws Apple Color Emoji's `sbix` bitmaps at the font's size and offset. CoreText draws them larger below 24 pt (the Rust path has 62–66% of CoreText's ink at 12–16 pt, 83% at 20 pt) and lower by an eighth of an em at every size (3 px at 24 pt, 4 px at 32 pt, at 1×) (measured, the "colour" lines). With S5-Q1's wider spacing, **CoreText's emoji follow rules of its own,** not the font's data.

**Cost per glyph** (measured, `q2-timing.txt`; median of three runs, range in brackets):

| Size | CoreText | `zeno` | `vello_cpu` |
|---|---|---|---|
| 16 px | 1,827 ns (1,824–1,905) | 2,717 ns (2,685–2,761) | 4,568 ns (4,542–4,624) |
| 32 px | 2,004 ns (2,001–2,016) | 3,228 ns (3,207–3,248) | 5,420 ns (5,414–5,436) |

**For TX12** (inference, §9): CoreText gives the platform's look, including its small-size adjustments, its smoothing and its emoji, and is the fastest. A Rust rasterizer would have to reproduce all three for system fonts to look native.

## 5. S5-Q3: web fonts

**WPT fonts** (measured, `q3-wpt-renderer.txt`, `q3-wpt-none.txt`). One process per file, under S4's renderer profile, unchanged:
- **all 322 behave as expected:** 288 render (decode, validate, shape and rasterize), and 34 are rejected;
- every rejection is for the reason the test's name gives: 33 by the container decoder and 1 at sniffing. Examples: extraneous data, misplaced metadata or private blocks, overlapping blocks, a bad `UIntBase128`, unpaired `glyf` and `loca` in a collection, a wrong `loca` length, a Brotli stream of the wrong length, a composite glyph without a bounding box, and reserved `hmtx` transform flags;
- the outcomes and render hashes are **identical** to an unsandboxed run.

**Mutants** (measured, `q3-mutants.txt`). 500 variants of each of 8 seed fonts. The seeds cover WOFF2 with transformed tables (WPT's `valid-005`, Noto Naskh Arabic, Noto Sans Mongolian, Hasubi Mono), WOFF 1.0 (Scheherazade), TrueType (Noto Sans Devanagari), CFF (a Noto Sans CJK subset) and colour bitmaps (a Noto Color Emoji subset). Each variant was checked in its own sandboxed process with a 10 s watchdog.
- About 60% of the mutations land inside the font's tables: a re-encoded WOFF table, a rewrapped WOFF2 stream, or a changed sfnt table. The rest change the file as a whole: random bytes, truncation, deleted or duplicated chunks, header fields.
- **Result: 2,709 render, 1,291 are rejected, 0 crashes, 0 hangs.**

| Strategy | Variants | Render |
|---|---|---|
| WOFF2 stream rewrapped | 1,224 | 1,108 |
| sfnt table changed | 890 | 812 |
| WOFF table re-encoded | 297 | 274 |
| Random bytes | 348 | 139 |
| Header field | 314 | 167 |
| Duplicated chunk | 301 | 90 |
| Deleted chunk | 289 | 94 |
| Truncated | 337 | 25 |

The variants that render ran corrupted tables and glyph data through the validator, HarfRust and both rasterizers.

**The profile needed no new rule.** The kernel logged **no sandbox denial for `s5-webfont`** in the run's 4,322 sandboxed processes (measured, `sandbox-denials.txt`): the path never attempts anything the renderer profile refuses. **No CoreText call can receive web-font bytes:** the executable links no Apple framework. S5-Q3 passes on all four conditions.

## 6. S5-Q4: footprint, size and speed

**Footprint of a sandboxed helper after its work** (measured, `q4-footprint.txt`; median of five helpers, each sandboxed with S5's text profile compiled by the broker):

| Configuration | First text: the Latin line, 119 glyphs | The corpus: 22 lines, 1,475 glyphs |
|---|---|---|
| Nothing (the helper enters the profile and waits) | 1.77 MiB | — |
| CoreText shapes and draws | 3.08 MiB | 8.97 MiB |
| HarfRust shapes, CoreText draws | 3.03 MiB | 8.86 MiB |
| HarfRust shapes, `zeno` draws (the Rust path) | 2.52 MiB | 6.25 MiB |
| The Rust path, plus ICU4X's line and word segmentation of every line | — | 6.25 MiB |

The five helpers of each configuration agree within 0.24 MiB. The corpus configurations keep a 0.6 MiB glyph atlas and, on the Rust paths, 16 mapped font files, whose pages are clean and so mostly outside the footprint (inference).

**For B2** (a renderer, sandboxed and ready with an empty document, at most 10 MB; process model §7): **every configuration leaves it achievable.** An empty document draws no text. A renderer's first text adds 0.75 MiB (Rust path) to 1.31 MiB (CoreText) to this helper's 1.77 MiB. The rest of B2 is the engine's own state.

**Executable size** (measured, `q4-sizes.txt`; release builds, then stripped; each probe against a 336,928-byte base that reads a file):

| Component | Adds (stripped) |
|---|---|
| HarfRust, shaping one run | 0.95 MiB |
| `skrifa` and `zeno`, one glyph | 0.93 MiB |
| ICU4X's line breaker, rule-based only | 65 KiB |
| ICU4X's line breaker with its dictionaries and LSTM models ("auto") | 389 KiB |
| ICU4X's line and word breakers, grapheme and sentence segmenters, and the Script property | 2.38 MiB |

`s5-webfont`, the whole web-font path including S4's sandbox code, is 2.61 MiB stripped.

**Shaping speed** (measured, `q4-speed.txt`; 100 passes over the corpus, 147,500 glyphs; median of three runs):

| Engine | Per glyph |
|---|---|
| CoreText's line layout, with font fallback | 3,036 ns (3,015–3,048) |
| CoreText on the same runs, each in its resolved font | 1,227 ns (1,225–1,236) |
| HarfRust on those runs, with prepared shapers | **566 ns** (566–602) |

All three produced the same number of glyphs.

## 7. S5-Q5: images

**ImageIO under S4's renderer profile** (measured, `q5-sandbox.txt`, `q5-removal.txt`; crash reports archived in `crash-reports/`).
- **Unchanged, the profile crashes ImageIO on every format** (SIGSEGV, exit 139).
- The crash reports' frames: `__CFCheckCFInfoPACSignature` ← `CFBundleGetInfoDictionary` ← `ApplicationOptIn()` ← `__IIOIsOOPEnabled_block_invoke`, at address 0x8 (observed). **Inference:** ImageIO asks whether the application opted in to out-of-process decoding, reading the main bundle's Info dictionary. Without metadata access to the executable's directory, `CFBundleGetMainBundle` returns NULL inside the profile.

**The rules S5 found** (S4's method: a crash or a logged denial, then the pixels compared with an unsandboxed decode):

| Rule | SBPL | What it opens |
|---|---|---|
| **R1** | `(allow file-read-metadata (literal (param "EXECUTABLE_DIR")))` | The metadata of one directory: no listing, no contents |
| **R2** | `(allow file-read* (literal "/System/Library/Video/Plug-Ins"))` | The list of the system's video plug-ins |
| **R3** | `(allow file-read* (subpath "/System/Library/Video/Plug-Ins/AV1DecoderSW.bundle"))` | One system bundle, the software AV1 decoder |
| **R4** | `(allow mach-lookup (global-name "com.apple.coremedia.videodecoder"))` | VideoToolbox's decoder service, `VTDecoderXPCService.xpc` (observed, its `Info.plist`) |
| **R5** | `(allow iokit-open-user-client (iokit-user-client-class "IOSurfaceRootUserClient"))` | The IOSurface driver's interface: allocating surfaces |
| **R6** | `(allow iokit-open-user-client (iokit-user-client-class "IOSurfaceAcceleratorClient"))` | The interface of the hardware scaler and colour converter |

**What each rule does** (measured, both runs):
- **With R1,** PNG, GIF and WebP decode with pixels identical to unsandboxed.
- **JPEG decodes too, by another path.** Its pixels differ slightly from unsandboxed ImageIO's for the two large images, with means within 0.1. ImageIO tries to open `AppleJPEGDriverUserClient`, by its name the hardware JPEG decoder's driver, and is denied (62 denials logged in the run).
- **AVIF comes back all zero, with no error.**
- **With R1–R6, AVIF is identical to unsandboxed.**
- **Removing each rule in turn:**
  - without R1, every format crashes;
  - without R2, R3, R4 or R5, AVIF comes back all zero;
  - without R6, AVIF decodes with slightly different pixels (means within 0.4).
- **During an AVIF decode, launchd starts a `VTDecoderXPCService` process for the decoding process, sandboxed or not, and it exits after the decode** (observed with `pgrep`, outside the formal runs).

**The review against the process model** (§4.1, binding since ADR-0004). For renderers, the model allows Mach lookups of "the browser's rendezvous service only, plus logging", and "IOSurface allocation only, if needed":
- **R1:** not covered by the model; low exposure, one directory's metadata. CoreText needs it as well (§3).
- **R2, R3:** read-only access to the sealed system volume; low exposure, and needed only for AVIF.
- **R4: contradicts the model.** It lets a renderer start a system decoder service and send it untrusted AV1 data. The model reserves media services for the media utility process.
- **R5:** within "IOSurface allocation, if needed".
- **R6:** beyond allocation, but optional: AVIF decodes without it, with slightly different pixels.

**So ImageIO doesn't decode AVIF under the renderer profile without a rule the process model forbids.** S5-Q5's condition is not met. §9 takes IM3's fallback.

**ImageIO's JPEG output varies** (measured: every decode in `q5-determinism.txt`, `q5-sandbox.txt` and `q5-decode.txt`, which record a hash of the pixels; the two large images):
- **Unsandboxed, its two modes give different pixels:** decoding at creation gives one result, and letting drawing decode gives another, each the same in all six timed decodes per image. **Once, decoding at creation gave the drawing mode's pixels:** 1 of 17 decodes of the pattern image (the landscape image: 17 of 17 alike). **Inference:** two decoders, the one used at creation probably the hardware decoder that the sandbox denies.
- **With R1, both modes give a third result,** the same in all 17 decodes of each image.
- **With R1–R6, the results varied:** the landscape image gave two results (10 and 2 of 12 decodes), and the pattern image one more (1 of 11).
- **`jpeg-decoder` gave one result for each image in every process,** 31 per image, sandboxed or not.

**Pixels against ImageIO's** (measured, `q5-compare.txt`, unsandboxed):
- PNG, GIF and WebP are **identical**.
- JPEG differs by a mean of 0.07–1.23/255 (the largest single difference 96; 0.02–1.1% of values more than 8 apart). JPEG decoders are allowed to differ in their inverse transform and chroma upsampling, so both results are valid decodes (inference).

**Time, sandboxed** (measured, `q5-decode.txt`; milliseconds, median of three runs' medians of five decodes):

| Image | Format | Rust | ImageIO, decode then draw | ImageIO, draw decodes |
|---|---|---|---|---|
| Photo, 0.26 MP | PNG | **2.70** | 5.72 | 5.04 |
| | JPEG | 2.73 | 1.72 | **1.54** |
| | GIF | **2.21** | 3.28 | 3.26 |
| | WebP | **3.71** | 10.95 | 5.62 |
| | AVIF | — | 16.71 | 8.85 |
| Pattern, 4.2 MP | PNG | **39.2** | 61.1 | 56.8 |
| | JPEG | 39.8 | 25.1 | **23.5** |
| | GIF | **30.6** | 41.2 | 41.3 |
| | WebP | **56.4** | 168.6 | 86.2 |
| | AVIF | — | 179.6 | 93.3 |
| Landscape, 9.0 MP | PNG | **74.7** | 113.6 | 106.2 |
| | JPEG | 46.7 | 41.3 | **27.3** |
| | GIF | **57.4** | 80.8 | 80.5 |
| | WebP | **123.1** | 325.2 | 167.6 |
| | AVIF | — | 138.5 | 76.5 |

**Peak memory of one decode, output included** (measured, warm phase; MiB; the RGBA output alone is 1.0, 16.0 and 34.3 MiB):

| Image | PNG: Rust / ImageIO | JPEG | GIF | WebP | AVIF: ImageIO |
|---|---|---|---|---|---|
| Photo | 2.1 / 2.9 | 2.3 / 2.2 | 2.3 / 3.4 | 2.0 / 3.5 | 3.7 |
| Pattern | 28.2 / 38.7 | 34.4 / 34.5 | 36.1 / 50.0 | 32.1 / 39.1 | 50.4 |
| Landscape | 60.4 / 78.5 | 74.5 / 70.2 | 77.4 / 105.2 | 68.8 / 77.3 | 107.8 |

The ImageIO figure is the lower of its two modes.

**For IM2's JPEG choice** (inference, §9): `jpeg-decoder` costs 1.1–1.8× ImageIO's time in the sandbox: 5.2 ms per megapixel on the landscape image, 9.5 on the noisy pattern and 10.4 on the small photograph. It uses about the same memory and is deterministic. The other safe decoders beat ImageIO on time and memory.

## 8. Dependencies found

Recorded in the [register](../technology/dependency-register.md); trees from `cargo tree`, `unsafe` counts from the crates' sources (lines that use the keyword, outside comments), grades from Google's and Chromium's audit files [G1, G2] (all observed or sourced 2026-10-18):
- **The text crates forbid `unsafe` themselves:** HarfRust 0.14.0, `read-fonts` 0.45.1 and `skrifa` 0.48.0. Their tree now also holds `yoke`, `once_cell`, `foldhash`, `stable_deref_trait` and `zerofrom`, beside `font-types`, `bytemuck` and `smallvec`.
  - **The published grades are for older versions.** `read-fonts` is graded only to 0.27.3 and `skrifa` to 0.29.0 (`ub-risk-0`); `bytemuck` to 1.22.0 (`ub-risk-2`, with one delta at `ub-risk-3`); `yoke` to 0.8.0 and `foldhash` to 0.1.5 (`ub-risk-2`).
  - `once_cell` has only `safe-to-run` audits; `smallvec` 1.11.0 is still `ub-risk-3`. HarfRust itself is not graded.
- **`png` 0.18.1 depends on `flate2` unconditionally,** for its encoder (observed, its `Cargo.toml` and `src/encoder.rs`). The register had set `flate2` aside as unneeded. Google and Chromium grade `flate2` 1.1.0 `ub-risk-2`; this tree has 1.1.10.
- **`zeno` 0.3.3** has no dependencies and one `unsafe` operation, an unchecked float-to-integer conversion that is unsound for coordinates out of range; `s5-rust-text` checks every coordinate first. **`vello_cpu` 0.3.0** forbids `unsafe` itself, but its tree holds `fearless_simd` (111 such lines), `hashbrown` (450), `bytemuck` (323) and `peniko` (36). Neither crate is graded.
- **`brotli-decompressor` 6.1.0 and its allocator crates** forbid `unsafe` unless their `unsafe` or `ffi-api` features are on; S5 enables neither. Ungraded at these versions.
- **`jpeg-decoder` 0.3.2** forbids `unsafe` in `platform_independent` mode; `gif`, `image-webp`, `fdeflate`, `miniz_oxide` and `png` forbid it outright. `gif` is graded `ub-risk-1` at 0.12.1, `fdeflate` `ub-risk-0` at exactly 0.3.7; `image-webp` and `jpeg-decoder` are not graded.

## 9. Implications for Axiom

1. **TX12: recommend CoreText for system fonts, and a Rust rasterizer for web fonts.**
   - CoreText gives the platform's look at small sizes, its font smoothing and its emoji, which a Rust rasterizer matches only above 20 px at 1×, and it is the fastest (§4).
   - The cost is about 0.5 MiB more footprint for a first text, and 2.6 MiB on a page as multilingual as the corpus (§6).
   - **For web fonts,** `zeno` and `vello_cpu` match CoreText equally well. `zeno` is 1.7× faster with no dependencies, but has one unsound conversion that Axiom must guard, as S5 does, or fix upstream. `vello_cpu` brings a larger, ungraded tree. **Recommended: choose with S6-Q4's path rasterizer,** so the renderer has one.
   - TX12 is settled once the owner has looked at the sheets.
2. **TX6 needs two additions.** The renderer's profile must make FontServices' `Reserved` directory readable, where PingFang lives (S5's two rules). And **the check that a resolved font is the one requested must require a file-backed font:** a font CoreText resolves without a file is treated as missing. A name check, or agreement between CoreText and HarfRust, didn't catch the stub (§3).
3. **TX10's design is confirmed:** Axiom hides `trak` from HarfRust and applies the normal track at the text's size after shaping (§3).
4. **TX13:** smoothing adds 16–52% ink, most at small sizes on a 1× display. Web fonts drawn by the Rust rasterizer without dilation will look lighter beside smoothed system fonts. **Recommended:** emulate smoothing for web fonts when the system's setting is on, calibrated against CoreText with S5's comparison, at the text milestone.
5. **IM2: keep the safe JPEG decoder** (§7). It costs a few milliseconds per megapixel, is deterministic, and keeps Apple's C decoder off every JPEG, as IM2 intends. `jpeg-decoder` is in maintenance mode (register), and `zune-jpeg` is barred at `ub-risk-3`, so a JPEG decoder of Axiom's own remains the long-term option.
6. **IM3: AVIF moves out of the renderer.** ImageIO needs R4 for AVIF, which the process model forbids renderers (§7).
   - **Recommended:** decode AVIF in the media utility process (IM11), whose column of the model already allows media services and hardware decoding, through IM9's decoder interface. Callers can't tell where a decoder runs, so this changes no interface.
   - The alternative is to wait for a safe AV1 decoder that meets DP10.
   - **For the owner at Gate C:** ADR-0012's condition "S5-Q5 validating ImageIO inside the renderer sandbox" is not met, so it stays Proposed. Accepting it with this amendment to IM3 is one decision.
   - **Inference:** it doesn't undermine an input part 2 depends on. Layout gets an image's size and pixels through IM9's interface wherever the decoding runs.
7. **The process model's renderer row (§4.1) needs refining:** "System fonts: `/System/Library/Fonts` and ICU's data" should add FontServices' `Reserved` directory and the metadata of the executable's directory (R1). The model is binding, so this is for the owner.
8. **ADR-0011:** S5-Q3 passes. S5-Q1 shows no shaping error but its criterion includes the owner's visual check, and TX12 follows S5-Q2's report, which includes the owner's comparison. So the ADR stays Proposed until those checks, then moves to Accepted under the owner's pre-approval. **The owner made the checks and accepted ADR-0011 at Gate C** (2026-10-20), with CoreText for system fonts.

## 10. Threats to validity

1. **One machine, one OS build.** macOS 27.2 (26B5112a) on an M1. CoreText's small-size adjustments, emoji metrics and sandbox needs could change with any update; so could ImageIO's.
2. **A busy Mac.** The formal runs were made with the Mac in normal use (load averages 2.4–3.3), not idle. The timings agree within a few percent across runs and between the two formal runs, but absolute figures on an idle machine would be lower.
3. **The corpus is small:** 22 lines, one or two sentences each, written for this spike rather than taken from a standard test set. HarfRust's fidelity on other text, fonts and features (vertical text, OpenType features set by CSS, variable fonts at other axes) isn't tested here.
4. **The verdicts compare HarfRust with CoreText, not with the fonts' designers' intent.** Where both engines fall back (the niqqud), neither is the reference; the owner's eye is.
5. **The rasterization reference is CoreText with smoothing off,** at the sizes listed, on glyphs chosen by line. Shapes rare in the corpus aren't covered.
6. **Footprints are of a stand-in helper,** without an engine. They show what the text stack adds, not a real renderer's total.
7. **The mutator is S5's own.** Its strategies are reasonable but not a coverage-guided fuzzer, so "no crash" means none in 4,000 variants, not none at all. Continuous fuzzing of the web-font path belongs to its milestone.
8. **Profile rules were confirmed by removal for ImageIO's six rules in the formal runs, but for S5's text profile informally.** The text profile's effect (glyphs and pixels identical to unsandboxed) is formal.
9. **The `VTDecoderXPCService` observation was made once, by hand,** outside the formal runs.

## 11. Reproducing these results

```sh
cargo test -p s5-rust-text -p s5-text -p s5-image   # 40 tests
tools/s5-fetch.sh                                   # WPT's fonts into target/s5/webfonts, hash-checked
tools/s5-images.sh                                  # test images into target/s5/images
tools/s5-measure.sh --crash-checks                  # everything; target/s5-results/<timestamp>/
```

| Run | Directory |
|---|---|
| Formal run (commit `bad07a8`, clean tree) | `target/s5-results/20261018-204642/`: `summary.txt`, `environment.txt`, the `q1-` to `q5-` files named above, `sheets-q1/` and `sheets-q2/`, `crash-reports/` (6), `profiles/`, `sandbox-denials.txt` |
| Reproduction (commit `f20e7f4`, clean tree) | `target/s5-results/20261018-203606/` |

`--crash-checks` runs what is expected to crash: ImageIO under S4's unchanged renderer profile and under each removal, and CoreText drawing emoji under S4's text profile. The script moves the resulting crash reports from `~/Library/Logs/DiagnosticReports` into the results.

## Sources

All checked 2026-10-18.
- [W1] web-platform-tests, commit `850b36a56cfc15b9117a9d222d32a63104662f49` (2026-10-18): `css/WOFF2/support/` and `fonts/` — https://github.com/web-platform-tests/wpt/tree/850b36a56cfc15b9117a9d222d32a63104662f49
- [S1] W3C, *WOFF File Format 1.0*, Recommendation (2012-12-13) — https://www.w3.org/TR/WOFF/
- [S2] W3C, *WOFF File Format 2.0*, Recommendation (updated 2024-08-08) — https://www.w3.org/TR/WOFF2/
- [S3] IETF, RFC 7932, *Brotli Compressed Data Format* (2016-07) — https://www.rfc-editor.org/rfc/rfc7932
- [R1] Google, `woff2`, the reference decoder `src/woff2_dec.cc`, at `fb9c337` (2026-05-08) — https://github.com/google/woff2
- [A1] Apple, *TrueType Reference Manual*, "The 'trak' table" (no date on the page) — https://developer.apple.com/fonts/TrueType-Reference-Manual/RM06/Chap6trak.html
- [H1] HarfRust 0.14.0, `src/shaper_font.rs`, `src/planner.rs` and `src/aat/trak.rs`, read in the crate as published to crates.io — https://github.com/harfbuzz/harfrust
- [G1] Google, `rust-crate-audits`, `audits.toml`, at `508eb72` (2026-09-17) — https://github.com/google/rust-crate-audits/blob/main/audits.toml
- [G2] Chromium, `third_party/rust/chromium_crates_io/supply-chain/audits.toml`, at `12e6ddd` (2025-03-22) — https://github.com/chromium/chromium/blob/main/third_party/rust/chromium_crates_io/supply-chain/audits.toml
- This repository: `spikes/s5/`, `tools/s5-fetch.sh`, `tools/s5-images.sh`, `tools/s5-measure.sh`; the [Step 5 tracker](step-5-subsystems.md) §10.1; [S4 results](s4-results.md) for the sandbox method and tooling.
