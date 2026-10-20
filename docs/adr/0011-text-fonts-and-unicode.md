# ADR-0011: Text, fonts and Unicode

| | |
|---|---|
| **Status** | **Accepted** (2026-10-20), by the owner at Step 5's Gate C, after the visual check of S5's sheets that S5-Q1's and S5-Q2's criteria include. The owner had pre-approved it at Gate B, effective once S5-Q1 and S5-Q3 passed, with TX12 settled by S5-Q2 ([S5 results](../research/s5-results.md)). **TX12: CoreText rasterizes system fonts** |
| **Date** | 2026-10-18 |
| **Deciders** | Project owner (only the owner can move an ADR to Accepted) |
| **Related** | [Text, fonts and Unicode](../architecture/text-and-fonts.md) (the full design, TX1–TX21); [rendering pipeline](../architecture/rendering-pipeline.md) PL7, PL8, PL11; [S4 results](../research/s4-results.md); [ADR-0010](0010-dependency-and-supply-chain-policy.md); [Step 5 tracker](../research/step-5-subsystems.md) §10.1 (spike S5); decision-map items I1 and P4 |

## Decision history

| Date | Event |
|---|---|
| 2026-10-18 | Proposed at Step 5, CP9 |
| 2026-10-18 | **Owner's Gate B decision:** "Pre-approved; will move to Accepted upon S5-Q1 and S5-Q3 passing, with TX12 settled by S5-Q2." Spike S5 is "Authorized for questions S5-Q1 through S5-Q5 under spikes/s5/, including the listed candidate crates and OFL/WPT test web fonts" |
| 2026-10-18 | **S5 reported** (Step 5, CP10; [results](../research/s5-results.md)). **S5-Q3 passes** on all four conditions. **S5-Q1:** no shaping error in 432 runs (no glyph differences), but its criterion includes the owner's visual check of the rendered pairs, outstanding. **S5-Q2:** reported, with the recommendation that CoreText rasterizes system fonts (TX12); TX12 follows once the owner has compared the sheets. S5 also refines TX6 (PingFang's directory; a file-backed check), confirms TX10, and answers TX13's question. The ADR stays Proposed until the owner's checks |
| 2026-10-20 | **Accepted** by the owner at Step 5's Gate C: "ADR-0011 (Text, Fonts, and Unicode): Accepted. I have visually inspected the S5 sheets and verified the Hebrew niqqud and Apple Color Emoji differences. CoreText is approved for system font rasterization under TX12." With that check, S5-Q1 passes, and S5-Q2 settles TX12: CoreText for system fonts. **S6-Q4 reported the same day** ([S6 results](../research/s6-results.md) §6) the evidence S5 left for the renderer's one Rust rasterizer: `vello_cpu` filled paths 2.1–6.8× faster than `zeno`, at the same quality. It is recommended for web fonts and paths, subject to grading its tree when it is adopted (DP8, DP15) |
| 2026-10-20 | **`vello_cpu` adopted** by the owner, after Gate C: "Renderer Rasterizer: Approved. Adopt vello_cpu as the primary Rust rasterizer for web fonts and paths, subject to crate dependency grading at the text milestone." TX12 and the [register](../technology/dependency-register.md) record it |

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

**Accepted: TX1–TX21** (2026-10-20), with S5-Q1 and S5-Q3 passed, and TX12 settled by S5-Q2: CoreText rasterizes system fonts. In summary:
1. **ICU4X is the one Unicode source,** with compiled data, for text, CSS, URLs and JavaScript (TX1, TX2).
2. **Encodings are Axiom's own, in safe Rust** (TX3). `encoding_rs` is graded `ub-risk-3`.
3. **Web content sees system fonts and web fonts,** never user-installed fonts (TX5).
4. **CoreText's database and fallback cascades serve system fonts** (TX6).
5. **Web fonts never reach CoreText.** Our own containers, `read-fonts`, HarfRust and a Rust rasterizer handle them (TX7, TX8).
6. **HarfRust shapes every run** (TX9–TX11).
7. **Glyphs are rasterized in the renderer, into atlases** (TX12–TX15). **CoreText rasterizes system fonts,** as S5-Q2 recommended and the owner approved; web fonts use a Rust rasterizer. A GPU process would receive outlines, never fonts.
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
| HarfRust differs from CoreText visibly on some scripts | Low (S5-Q1: the same glyphs in 432 runs; the owner checked the pairs) | Medium | HarfRust passes most of HarfBuzz's suite; report upstream |
| A Rust rasterizer looks unlike macOS text | Medium | Low | CoreText rasterizes system fonts (TX12). Web fonts differ mainly in smoothing, which TX13 emulates at the text milestone |
| On-demand system fonts unreadable in the sandbox | Medium | Low | TX5 treats them as absent until a reviewed rule exists |

## Conditions for revisiting

- HarfRust stops tracking HarfBuzz, or falls behind on a script the corpus needs.
- A newer `encoding_rs` earns a published grade of `ub-risk-2` or better, and the encodings show measurable costs.

## Consequences

- The [register](../technology/dependency-register.md) records ICU4X (foundational), `unicode-bidi`, HarfRust, Fontations and `brotli-decompressor`.
- CP15's inline layout consumes shaped runs, break opportunities and bidi levels (§6 of the design).
- CP6 resolved IDNA in the browser: only canonical ASCII URLs from other processes (NW2).
