# ADR-0011: Text, fonts and Unicode

| | |
|---|---|
| **Status** | **Proposed** (2026-10-18), at Step 5's CP9. Recommended for acceptance once spike S5's conditions are met, as ADR-0008 waited for S3 |
| **Date** | 2026-10-18 |
| **Deciders** | Project owner (only the owner can move an ADR to Accepted) |
| **Related** | [Text, fonts and Unicode](../architecture/text-and-fonts.md) (the full design, TX1–TX21); [rendering pipeline](../architecture/rendering-pipeline.md) PL7, PL8, PL11; [S4 results](../research/s4-results.md); [ADR-0010](0010-dependency-and-supply-chain-policy.md); [Step 5 tracker](../research/step-5-subsystems.md) §10.1 (spike S5); decision-map items I1 and P4 |

## Decision history

| Date | Event |
|---|---|
| 2026-10-18 | Proposed at Step 5, CP9 |

## Question

Where does the engine's Unicode come from? How are encodings decoded, fonts found and fallen back to, web fonts loaded safely, text shaped, and glyphs rasterized?

## Requirements and constraints

- **PL7, PL8, AC4:** text reaches the display list as glyph images from an atlas the renderer rasterized; the browser never parses a font.
- **PL11:** strings are Latin-1 or UTF-16.
- **S4-Q5:** CoreText works in the renderer's sandbox, with system fonts and ICU data readable, and fails silently otherwise.
- **ADR-0010:** a reused parser of untrusted input must be pure Rust at `ub-risk-2` or better; `ub-risk-3` is barred.
- **ADR-0001:** web content in all languages, in stages.

## Alternatives considered

- **CoreText for everything,** as WebKit does.
- **HarfBuzz through bindings,** as Servo does.
- **OTS in front of CoreText for web fonts,** as Chrome does on macOS.
- **The system ICU** for segmentation.
- **`encoding_rs`** for encodings.
- **ICU4X, HarfRust, and a Rust-only path for web fonts:** the proposal.

## Historical context and modern developments

- **On macOS,** Chromium and Firefox shape with HarfBuzz and rasterize with CoreText; WebKit uses CoreText for both. Firefox's CoreText shaping is off by default.
- **ICU4X:** Firefox breaks lines and words with it, and Chromium vendors it.
- **Fonts in Rust:** Chrome moved web fonts on Linux, Android and ChromeOS to Skrifa. Chromium vendors HarfRust, which it runs in tests.

All sourced and dated in [text and fonts](../architecture/text-and-fonts.md) §3.

## Analysis

| | CoreText for all | HarfBuzz bindings | ICU4X + HarfRust + Rust web fonts |
|---|---|---|---|
| Untrusted fonts reach C | Yes | Yes (C++) | **No** |
| Platform look for system fonts | Yes | Through CoreText rasterization | Through CoreText rasterization, or a Rust rasterizer (S5) |
| One shaping engine for every font | No | Yes | Yes |
| Policy (ADR-0010) | DP10's exception for every font | DP10's exception | Pure Rust, `ub-risk-2` trees, with `smallvec`'s grade open |

## Evidence and sources

[Text and fonts](../architecture/text-and-fonts.md) §3–§4 and its Sources. Observations of this Mac: 786 system font faces, 165 of them AAT-only; the SDK's ICU headers lack segmentation, bidi and normalization.

## Decision

**Recommended: accept TX1–TX21, once S5-Q1 and S5-Q3 pass.** S5-Q2 settles TX12. If S5 isn't authorized, accept with TX12's default, CoreText for system fonts. In summary:
1. **ICU4X is the one Unicode source,** with compiled data, for text, CSS, URLs and JavaScript (TX1, TX2).
2. **Encodings are Axiom's own, in safe Rust** (TX3). `encoding_rs` is graded `ub-risk-3`.
3. **Web content sees system fonts and web fonts,** never user-installed fonts (TX5).
4. **CoreText's database and fallback cascades serve system fonts** (TX6).
5. **Web fonts never reach CoreText.** Our own containers, `read-fonts`, HarfRust and a Rust rasterizer handle them (TX7, TX8).
6. **HarfRust shapes every run** (TX9–TX11).
7. **Glyphs are rasterized in the renderer, into atlases** (TX12–TX15). For system fonts the rasterizer is CoreText or a Rust one, as S5 decides. A GPU process would receive outlines, never fonts.
8. **ICU4X segments text; `unicode-bidi` runs bidi;** hyphenation is staged (TX16–TX18).

## Rationale

- No C font parser ever sees untrusted font data, which is stricter than any engine today, at little cost.
- One Unicode version and one shaping engine give consistent behaviour across subsystems and fonts.
- PL7 holds unchanged (SC1).

## Assumptions

- HarfRust keeps tracking HarfBuzz, as it does today (it matches HarfBuzz 14.5.1).
- ICU4X's compiled data fits the renderer's budget (S5-Q4).

## Risks and unresolved issues

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| `smallvec` stays graded `ub-risk-3`, barring HarfRust and the segmenter | Medium | High | Grade the adopted version; work with upstream; recorded in the register |
| HarfRust differs from CoreText visibly on some scripts | Medium | Medium | S5-Q1; HarfRust passes most of HarfBuzz's suite; report upstream |
| A Rust rasterizer looks unlike macOS text | Medium | Low | S5-Q2; CoreText stays available for system fonts |
| On-demand system fonts unreadable in the sandbox | Medium | Low | TX5 treats them as absent until a reviewed rule exists |

## Conditions for revisiting

- HarfRust stops tracking HarfBuzz, or falls behind on a script the corpus needs.
- A newer `encoding_rs` earns a published grade of `ub-risk-2` or better, and the encodings show measurable costs.

## Consequences

- The [register](../technology/dependency-register.md) records ICU4X (foundational), `unicode-bidi`, HarfRust, Fontations and `brotli-decompressor`.
- CP15's inline layout consumes shaped runs, break opportunities and bidi levels (§6 of the design).
- CP6 resolved IDNA in the browser: only canonical ASCII URLs from other processes (NW2).
