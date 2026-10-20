# ADR-0016: Style, Layout and Paint

| | |
|---|---|
| **Status** | **Accepted** (2026-10-20), by the owner at Step 5's Gate C, as proposed, with the amendment to [ADR-0007](0007-frame-tree-pipeline-and-input.md) that ST25 and ST31 need, and `:visited` by partitioned history (ST10) |
| **Date** | 2026-10-19 |
| **Deciders** | Project owner (only the owner can move an ADR to Accepted) |
| **Related** | [Style](../architecture/style.md) (ST1–ST36); [layout and paint](../architecture/layout-and-paint.md) (LY1–LY32); [ADR-0007](0007-frame-tree-pipeline-and-input.md) (the ST25 and ST31 amendment); [ADR-0011](0011-text-fonts-and-unicode.md); [ADR-0012](0012-graphics-images-and-media.md) (GR5, GR9); [ADR-0015](0015-html-parsing-and-the-dom.md); [ADR-0010](0010-dependency-and-supply-chain-policy.md); decision-map item P6 |

## Decision history

| Date | Event |
|---|---|
| 2026-10-19 | Proposed at Step 5, CP18, with the amendment to ADR-0007 (ST25, ST31) and the `:visited` policy (ST10) for the owner |
| 2026-10-20 | **Accepted** by the owner at Step 5's Gate C: "ADR-0016 (Style, Layout, and Paint): Accepted as proposed." In the same decisions: "ADR-0007 Amendment (ST25, ST31): Accepted. Nested lifecycle phases for size container queries are adopted." And: ":visited Policy (ST10): Accepted. Partitioned history by top-level site is approved." **S6 reported the same day** ([S6 results](../research/s6-results.md) §6, §8): GPU path filling pays in time and memory, not in quality, so GR5's amendment, which would change LY23, is put to the owner; GR5 and LY23 stand as written meanwhile |

## Question

How is CSS parsed and the cascade computed, with its caches and invalidation? How are boxes built, laid out into fragments, painted into display lists with property trees, and hit-tested? What is the layout unit?

## Requirements and constraints

- **DP1 (a), (b):** parsing CSS, and style, layout and paint, are Axiom's own.
- **PL1–PL7, PL10, PL14–PL16:** computed styles immutable and shared in a side table; immutable fragments with their constraints; per-frame display lists of validated primitives with property trees; style and layout on the event loop's thread for now.
- **IV1–IV14:** a compiler-checked lifecycle with two entry points; pending, selector-driven invalidation; reuse at each stage; compositor-driven scrolling and animation; incremental equals from scratch.
- **GR5, GR9:** paths as renderer images, with GR5's amendment deferred to the graphics milestone; GR9's list of compositor animations.
- **TX6–TX20, IM5, IM9, AX10:** text's interface; images through IM9; system preferences as media features.
- **J3:** no dynamic code generation outside `renderer-jit`.

## Alternatives considered

- **Stylo,** or `cssparser` and `selectors`, under or instead of our own cascade.
- **No rule tree,** only a cache keyed by a hash of what matched (Blink, WebKit).
- **A selector JIT** (WebKit).
- **Floating-point layout units,** or Firefox's 1/60 of a pixel.
- **Taffy** for flexbox and grid (Servo, for grid).
- **Style and layout on a thread of their own** over copied render state (Ladybird).
- **`:visited` with restricted styles** over global history.
- **The proposal:** our own parser, matcher and cascade with shared styles and evictable caches; LayoutNG's contract in fixed point; paint into PL7's primitives.

## Historical context and modern developments

- **Stylo** shares styles through its rule tree and a style sharing cache of 32 candidates per depth; Blink and WebKit cache cascades by a hash of what matched. Blink's invalidation sets and its interleaving of style with layout for size containers are the precedent for IV3 and ST25.
- **Ladybird moved style to Rust** on 2026-08-13, as a memory-bounded incremental computation with evictable acceleration state and a cache-free exact matcher, and layout to Rust on 2026-07-28.
- **Blink, WebKit and Ladybird lay out in 1/64 of a pixel;** Firefox in 1/60. LayoutNG's fragments are immutable, cached by constraint space, and painted and hit-tested by traversal.

Sourced in [style](../architecture/style.md) §3 and [layout and paint](../architecture/layout-and-paint.md) §3.

## Analysis

| | Proposal | Alternatives |
|---|---|---|
| Strings | The CSS parser is generic over PL11's 8- and 16-bit units | Stylo and `cssparser` parse UTF-8, so every sheet would be converted |
| Memory | Grouped, shared styles; caches budgeted and evictable | Every engine shares styles; Ladybird shows the value of evictable caches |
| Correctness | A cache-free path and IV14 check every shortcut; fixed point makes comparisons exact | Floating point makes incremental and full results differ in their last bits |
| Security | No selector JIT; `:visited` by partitioned history | A JIT breaks J3; global history in renderers leaks to a compromised one |

## Evidence and sources

[Style](../architecture/style.md) §2–§4 and [layout and paint](../architecture/layout-and-paint.md) §2–§4, with their sources: the W3C CSS reports, the HTML, Compatibility and Quirks Mode standards, SVG 2's snapshot of 2026-10-06, and Blink, Firefox, Stylo, WebKit, Servo, Ladybird, Taffy and HarfRust at their latest commits.

## Decision

**Accepted: ST1–ST36 and LY1–LY32** (2026-10-20), with the amendment to ADR-0007 and ST10's `:visited` policy. In summary:
1. **CSS parsing** is Axiom's own, generic over PL11's code units, with one property table generating parsers, computed-style groups, impact classes and the CSSOM's accessors (ST1–ST6).
2. **Matching** right to left over the DOM's lent columns, with rule buckets, an ancestor Bloom filter and `:has()` invalidation; **no selector JIT** (ST7–ST11).
3. **The cascade** of Cascade 5 and 6, custom properties with the required expansion limit, and immutable, grouped computed styles shared through a rule tree, a sharing cache and an evictable cascade cache, with a cache-free path as the oracle (ST12–ST22).
4. **Container queries resolved during layout,** in nested lifecycles, under **the amendment to ADR-0007** the owner accepted (IV1's nested lifecycles; IV2's examples gain `getComputedStyle` reads that need layout) (ST25, ST31).
5. **`:visited` by partitioned history** (Selectors 4, Appendix C), as the owner approved: the browser keeps the history and each renderer receives only its own partitions; never matching until that history service exists (ST10).
6. **Animations** on Web Animations' model, compositor animations exactly as GR9 lists them; the CSSOM as a view over immutable contents (ST27–ST31).
7. **The layout unit: 1/64 of a CSS pixel in an `i32`,** saturating (LY1); LayoutNG's contract for every algorithm; Axiom's own formatting contexts, **with tables marked approximate** (LY6–LY15).
8. **Scrolling** through scroll nodes that carry snap areas, sticky constraints and compositor-moved scrollbar thumbs, within PL6 and PL7 (LY16–LY18).
9. **Paint** in CSS 2.1's order over fragments, with property trees and display items reused by identity; paths as images per GR5; form controls by Axiom's own theme; hit testing in the renderer (LY19–LY28).

## Rationale

- DP1 makes all of it core, and PL11 rules out the UTF-8 crates.
- The three mature engines share computed styles and cache cascades by what matched; Axiom adds Ladybird's two memory principles, which fit an 8 GB machine.
- Container queries can't be implemented under IV1 as written; nested lifecycles keep its compiler-checked order, as Blink's interleaving keeps its own.
- Fixed point makes IV14's comparison exact and matches three engines' arithmetic.
- Partitioned history keeps `:visited` useful without global history in any renderer.

## Assumptions

- Stylo's measured cache sizes, and Blink's budgets, are sound starting points (Step 6 tunes them on the corpus).
- ±33.5 million CSS pixels is enough range for real pages; beyond it, results saturate as in Blink and Ladybird.
- GR5 holds until the graphics milestone, to which the owner deferred its amendment (2026-10-20). S6-Q4 found GPU filling faster and smaller but coarser at the edges ([S6 results](../research/s6-results.md) §6); if the amendment is accepted then, path items enter PL7, and LY23 changes only where paths are filled.

## Risks and unresolved issues

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Nested container-query lifecycles add complexity to IV1's tokens | Medium | Medium | Confined to size containers, whose containment rules out cycles; tested by IV14's rebuild comparison |
| Table layout differs from other engines on real pages | Medium | Medium | Approximate by design, following the engines where WPT shows they agree (LY10) |
| Style or layout limit frame production on large pages | Medium | Medium | Parallel style and layout over lent arenas (ST33, LY30), built when profiling shows it (PL10) |
| The owner rejects the ADR-0007 amendment | Retired at Gate C | — | The owner accepted it (2026-10-20) |

## Conditions for revisiting

- GR5's amendment is accepted at the graphics milestone, to which the owner deferred it ([S6 results](../research/s6-results.md) §8): path items enter PL7, and LY23's path images change.
- Profiling shows style or layout limiting frames: build ST33's and LY30's parallel paths.
- Ladybird's dependency-program style engine proves its memory and incrementality over a longer period: revisit ST14–ST17 at the implementation milestone.

## Consequences

- [Graphics](../architecture/graphics.md) receives property trees with scroll nodes carrying snap areas, sticky constraints and scrollbar thumbs, and compositor animation descriptions (LY17, LY18, ST28).
- [Text](../architecture/text-and-fonts.md) receives font descriptors and `@font-face` rules, and its shaped runs carry HarfRust's unsafe-to-break flags (ST26, LY8).
- The browser holds visited history as partitioned triples, and sends each renderer its own partitions (ST10).
- The browser sends each frame its visible rectangle in the top-level viewport with its viewport (LY28).
