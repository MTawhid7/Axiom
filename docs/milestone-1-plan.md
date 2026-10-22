# Milestone 1: The Implementation Plan

| | |
|---|---|
| **Status** | **Draft, for the owner** (2026-10-22). Produced after the review cycle ended, as the owner directed: "Produce the implementation plan (docs/milestone-1-plan.md)" ([the review](review/README.md) §5.9, item 12). It becomes binding when the owner adopts it as Milestone 1's first checkpoint, CP1 (MS10), with the authorization MA10 lists. **No engine code exists** until then |
| **Question** | How is Milestone 1 built: in which checkpoints, each with what scope, tests, efficiency checks, dependencies and exit criterion; which decisions its start needs; and where the owner's gates fall? |
| **Builds on** | [Milestone 1](milestone-1.md) (MA1–MA12), which this plan refines and doesn't change; the [roadmap](roadmap.md)'s MS7 and MS10; the [performance budgets](performance.md) (PF1–PF30, PF5's three tiers); the [testing strategy](testing.md); the [dependency policy](technology/dependency-policy.md) and [register](technology/dependency-register.md); the architecture review's [Round 4 actions](review/round-4/07-actions.md) §4, whose outline this plan expands |
| **Last updated** | 2026-10-22 |

**How to read it.** §1 says what the owner authorizes to start; §2 settles the decisions MA12 leaves to the plan, each with a recommendation; §3 is the checkpoints, M1-CP2 to M1-CP13, one per item of MA11's outline; §4 is the efficiency section MS10 asks of every plan; §5 lists the risks with the checkpoint that retires each; §6 the owner's gates. Rule IDs are the binding documents'. Where this plan says more than they do, it is **inference,** a proposal for the owner. Checkpoint numbers here are Milestone 1's own, written M1-CPn, so that they don't collide with the review's.

---

## 1. What starting Milestone 1 authorizes (MA10)

The owner's authorization of MA10's six items, as amended through Round 4:

| Item | What it covers |
|---|---|
| **The branch and this plan** | `milestone-1` from `main`; this document adopted as M1-CP1 |
| **The skeleton** | MA1's crates under `crates/`; the executables `axiom`, `axiom-renderer`, `axiom-network` and `axiom-headless`, signed by the stable local identity the owner creates (S15), each launched under its profile from S4, S5 and S7; RV1-16's release profiles (`panic = "abort"`) and lints |
| **The dependencies,** each in the register before use (DP6, DP8) | **Foundational:** `objc2` and its framework crates with `block2`; ICU4X's segmentation, properties and normalization; rustls. **Replaceable:** HarfRust, `read-fonts`, `skrifa`, `unicode-bidi`, `vello_cpu` (graded first), `png`, `gif`, `image-webp`, **JPEG's decoder** (§2), `moxcms`, `aws-lc-rs` (C, under DP10's exception), `miniz_oxide`, `brotli-decompressor`, `ruzstd`, **`smallvec`** (graded; admissible at `ub-risk-3` under DP8 as amended) and **`encoding_rs`** (`sandboxed`, under DP8's conditions). **Test-only:** html5ever and xml5ever (differential references), `loom`, `libfuzzer-sys`, wptrunner and WPT's tools in a virtual environment under `target/` |
| **The test data,** pinned (MA3) | WPT at one revision (§2); html5lib-tests at `c777c408`; Unicode 17's conformance files; the corpus copies, dated, never committed (CO12) |
| **The tools** | `cargo-deny` and `cargo-vet` (DP22); cargo-fuzz 0.13.2 under `target/`; `nightly-2026-10-21` for Miri and ThreadSanitizer (S12) |
| **Live loads** | Wikipedia and MDN, within CO3 and CO4 |

## 2. The decisions MA12 leaves to this plan

Each with a recommendation, for the owner to confirm at M1-CP1.

| # | Decision | Recommendation | Settled at |
|---|---|---|---|
| 1 | **Xcode** (OQ-7) | **Not for M1.** No design needs it (GR14); `samply` and command buffers' timestamps measure. Revisit if M2's GPU process needs Metal's debugger | M1-CP1 |
| 2 | **WPT's revision** (MA3) | **`0f26972422`** (2026-10-18), which wpt.fyi aligned and the standards calibrated, unless a newer aligned revision is wanted; MA4's subset and floors are recomputed there either way | M1-CP1 |
| 3 | **Unicode's version** | **17.0, through ICU4X 2.3** (SB1, SB2): one version throughout | M1-CP1 |
| 4 | **The nightly** | **`nightly-2026-10-21`,** which S12 validated for ThreadSanitizer with its own runtime | M1-CP1 |
| 5 | **Hosted runners** (TS17) | **This Mac only:** the five checks already run here before every commit | M1-CP1 |
| 6 | **The signing identity** (RV1-17) | **Created by the owner** in the login keychain at M1's start; TCC's persistence checked by hand at M1-CP10 | M1-CP1 |
| 7 | **QoS overrides on one-way messages** (OQ-20) | **Measured** with the first Mach channels: a one-way message with an override to a thread waiting in `kevent64` | M1-CP2 |
| 8 | **`smallvec`'s grade** | **Graded** at the version HarfRust and ICU4X lock (1.16.3 compiles 43 `unsafe` blocks, 11 commented); at `ub-risk-3` it is admitted in the renderer under DP8's conditions, its audit naming each block's invariant and a fuzz target reaching them | M1-CP5, before shaping |
| 9 | **`vello_cpu`'s grade** | **Graded:** its root forbids `unsafe`, so the grade rests on its dependencies | M1-CP5 |
| 10 | **`encoding_rs`'s DP8 conditions** | **Met before use:** the audit of each `unsafe` block across its tree, `simdutf8` included (143 compiled blocks in 2026-10-21's census), and a libFuzzer target under AddressSanitizer through TX3's interface | M1-CP5 |
| 11 | **JPEG's decoder** (IM2) | **`zune-jpeg` without its `x86` and `neon` features,** where its root forbids `unsafe`, if it matches `jpeg-decoder`'s pixels and isn't slower on the corpus's images; `jpeg-decoder` with `platform_independent` otherwise. Both have no `unsafe` in those configurations; `zune-jpeg` is maintained, `jpeg-decoder` dormant since 2025-06-21 | M1-CP8 |
| 12 | **Surfaces allocated ahead** (GR12) | **A pool sized from S6's slowest frames,** a hypothesis measured at the paint checkpoint | M1-CP7 |
| 13 | **AWS-LC's jitter entropy** | **Keep AWS-LC's default** unless its 29–43 ms first-use cost misses PF25's helper start-up, measured in the network process | M1-CP9 |
| 14 | **The user-agent string** (SB13) | **Safari-compatible with an Axiom token,** from the Safari shipping at M1's start | M1-CP9, before live loads |
| 15 | **The objc2 classes** (EM11; OQ-11) | **Validated** for the shell's text input and accessibility; the Swift fallback stays behind the same API | M1-CP10 |

## 3. The checkpoints

Each checkpoint leaves the workspace clean and working, with the five checks green, the suites it adds passing, and the tracker updated (MS10). **From M1-CP3, every checkpoint also runs PF5's ratchet** against `perf/baseline.toml`. A checkpoint's **done when** is its exit criterion.

### M1-CP2. The skeleton, the processes and IPC (MA11, item 1)

- **Scope:** the workspace of MA1's crates, with `tools/unsafe-policy.txt` entries for each `platform` module; the first register entries (`objc2` and its framework crates); a window, a Metal clear and a CoreAnimation layer; the browser launching `axiom-renderer` and `axiom-network` under browser-compiled profiles (ADR-0004 D2), each with a Mach channel and a memory entry. **IPC from the start:** handles owned or destroyed (IP9, IP11; RV4-05); returned messages kept whole or destroyed (IP16); in-memory channels that model Mach's limits (IP2; RV4-11); the rendezvous pinning the PID and its version, one hello per child (IP4; RV4-12); PF19's watchdog with task ages (RV4-03).
- **Tests:** S4's denial probes in each helper; each renderer profile denies `iokit-open` (MA7); the IDL's generated decoders under their first fuzz target; Miri on `platform`; OQ-20's measurement.
- **Efficiency:** tier 1, the sizes of IPC headers and handle lists.
- **Done when:** a window shows a Metal-cleared layer; both helpers run under their profiles, answer pings with their task ages, and pass the probes.

### M1-CP3. The harness end to end, and the parsers (MA11, item 2)

- **Scope:** the headless embedder's offscreen texture and PNG output (EM9); HTTP/1.1 to wptrunner's server on the loopback (NW3), enough for the harness; wptrunner's `axiom` product with the script-free executor (TS2); URLs (NW2) and `file:` loading; the HTML tokenizer and tree builder (HD1–HD8) with their runners (TS4); **the XML parser for documents** (HD9; RV4-01); the node arena and the DOM (HD10–HD16) with the DOM dump (RV1-15); **the efficiency harness** (PF5; RV4-06): `rusage_info_v6` read at each process's exit, `perf/baseline.toml` and the ratchet.
- **Tests:** a trivial reference test through wptrunner end to end; html5lib's tokenizer tests at 100% and the tree-construction runner reporting; a sample of `css/CSS2`'s XHTML tests parsing; fuzz targets for the HTML tokenizer and tree builder (against html5ever), the XML parser (against xml5ever) and URLs; HD33's chunking test.
- **Efficiency:** tier 1, the node's size (HD10's 36 bytes); tier 2, the first baseline: parsing the two corpus copies (instructions, peak footprint).
- **Done when:** wptrunner runs a reference test through `axiom-headless`; html5lib's tokenizer suite passes; the ratchet runs in the checkpoint routine.

### M1-CP4. Style (MA11, item 3)

- **Scope:** CSS syntax, selectors, the cascade and computed values (style's first-stage row); style sharing and invalidation sets (ST19); the computed-style dump; style's generated property code, with T1–T4 measured (RV1-18).
- **Tests:** fuzz targets for CSS syntax, selectors, values and media queries; the computed-style dumps of the corpus copies, reviewed; IV14's self-oracle on a hover.
- **Efficiency:** tier 1, the computed style's size and the sharing cache's hits on fixtures; tier 2, styling the corpus copies; T1–T4 (PF18).
- **Done when:** both corpus copies style without error, and T1–T4 hold with the generated code.

### M1-CP5. Text (MA11, item 4)

- **Scope:** the font database through CoreText (TX6); shaping with HarfRust; line breaking and bidi through ICU4X; the atlas; the Rust path for WPT's test fonts (TX21); **the encodings through `encoding_rs`** behind TX3's interface (RV4-08). Before adopting them: `smallvec`'s and `vello_cpu`'s grades, and `encoding_rs`'s DP8 conditions (§2, items 8–10).
- **Tests:** Unicode's line-breaking and bidi files at 100% (MA4); S5's shaping comparison against CoreText as a regression test; the encodings' fuzz target under AddressSanitizer.
- **Efficiency:** tier 2, shaping and breaking the corpus copies' text.
- **Done when:** the corpus copies' text shapes and breaks; the conformance files pass; every `ub-risk-3` crate adopted has its per-block audit and its fuzz target.

### M1-CP6. Layout (MA11, item 5)

- **Scope:** block and inline layout, floats and positioning, then tables (approximate, LY10); the box and fragment dumps.
- **Tests:** the first `css/CSS2` reference tests counted, with expectation files (SB9); layout's limits for hostile documents under fuzzing (layout §7).
- **Efficiency:** tier 1, the fragment's size; tier 2, laying out the corpus copies.
- **Done when:** both corpus copies lay out with ≥ 99% of their visible text placed (MA5's measure).

### M1-CP7. Paint and the compositor (MA11, item 6)

- **Scope:** paint, display lists and the display-list dump; the compositor thread's tiles, damage and scrolling layers (GR2–GR8); **release at a renderer's death** (PL8, IR10; RV4-04); a tile committed once its commands are scheduled (GR12; RV4-10); the pool of surfaces allocated ahead (§2, item 12).
- **Tests:** MA7's renderer-death test (nothing mapped 1 s later); the display-list decoder's fuzz target; S6's frame measurements as benchmarks (PF13).
- **Efficiency:** tier 2, frames for the corpus copies' first paint and a scroll; the window's tiles against B1's 6.2 bytes a pixel.
- **Done when:** the corpus copies paint and scroll within PF13 at the median, and a renderer's death leaves nothing mapped.

### M1-CP8. Images (MA11, item 7)

- **Scope:** PNG, JPEG (§2, item 11), GIF and WebP behind IM9's interface, decoded incrementally and at display size, with colour management through `moxcms`.
- **Tests:** each decoder's wrapper under its fuzz target; S5-Q5's pixel comparisons as regression tests.
- **Efficiency:** tier 2, decoding the corpus copies' images (instructions, peak footprint).
- **Done when:** the corpus copies' images decode, and the JPEG decision is recorded in the register.

### M1-CP9. Networking (MA11, item 8)

- **Scope:** HTTP/1.1 over TLS, rustls with `aws-lc-rs` and the system verifying certificates (NW6), DNS-SD (NW7), the in-memory cache, mixed content, HSTS with its preload list compiled in, and Referrer-Policy (NW12); content decoding with NW13's limits; the network log. HTTP/2 waits for M2 (RV4-08).
- **Tests:** MA7's network-process tests (killed, stalled, and a storage thread stalled while the control thread answers); the containment test (a stalled renderer); fuzz targets for HTTP/1.1 framing and content decoding; **S7-Q2's comparison per byte** of rustls and Network framework, measured again on a direct network path, since the re-run on 26B118 couldn't reproduce it ([S7 results](research/s7-results.md) §12).
- **Efficiency:** PF26's CPU per page load begins: the corpus copies over the loopback, then live (§2, item 14 first).
- **Done when:** both pages load live within CO3 and CO4, and the network process recovers from each induced failure.

### M1-CP10. The shell (MA11, item 9)

- **Scope:** the address field, back and forward, history; the objc2 classes for the shell's text input and accessibility (EM11). No text entry into pages before M4 (RV4-08); forms submit as navigations, and MA7's password rule holds.
- **Tests:** the shell's own text input and its accessibility class validated; MA7's test that a form with a non-empty password field isn't submitted below L2; TCC's persistence across a rebuild, by hand (§2, item 6).
- **Efficiency:** tier 2, start-up (PF25): warm launch to the first window drawn.
- **Done when:** the owner can navigate both pages from the address field, with back and forward.

### M1-CP11. Reference tests at scale (MA11, item 10)

- **Scope:** the `css/CSS2` selection through wptrunner, with expectations (SB8, SB9), against MA4's target.
- **Tests:** MA4's subset and floors recomputed at the pinned revision by `tools/wpt-script-deps.py --needs` and checked by `tools/wpt-recount.sh` (SB11); the run reports every directory against its floor.
- **Done when:** the subset's two counts agree, and the pass rate is reported against **≥ 90.0% with the floors** (RV4-09), with every failure attributed to a capability or a defect.

### M1-CP12. Fuzzing, determinism and budgets (MA11, item 11)

- **Scope:** every fuzz target's campaign, at least an hour under AddressSanitizer (MA7); MA8's determinism tests; **the stress configuration** in real processes (TS8, MA8; RV4-11); every budget of MA6, PF25–PF30 included, measured under PF1.
- **Done when:** no open fuzzing finding; determinism and stress runs equal the plain runs; MA6's report has a value for every row.

### M1-CP13. The corpus checks and the gate (MA11, item 12)

- **Scope:** MA5's checks on the dated copies and live; MA9's gate report, every criterion's measured value beside its target, with the raw records under `target/`.
- **Done when:** the owner holds the gate report (§6).

## 4. Efficiency (MS10, as amended in Round 4)

| Checkpoint | What it adds to the cost | The tier that catches a regression | The budget it draws on |
|---|---|---|---|
| M1-CP2 | Three processes and their channels | Tier 1: IPC headers' sizes; from M1-CP3, tier 2: each helper's start-up and footprint | B2, B4, PF25 |
| M1-CP3 | The parsers and the DOM | Tier 1: the node's size; tier 2: parsing the copies | B3's pages |
| M1-CP4 | Style and its caches | Tier 1: computed styles' size; tier 2: styling | B3; PF23's caches |
| M1-CP5 | Fonts, shaping, the atlas | Tier 2: shaping and breaking | B3 |
| M1-CP6 | Fragments | Tier 1: the fragment's size; tier 2: layout | B3 |
| M1-CP7 | Tiles and display lists | Tier 2: frames, the window's tiles | B1, PF13 |
| M1-CP8 | Decoded images | Tier 2: decoding | B3, IM10 |
| M1-CP9 | TLS and the cache | Tier 2: CPU per page load | B4, PF26 |
| M1-CP10 | The shell | Tier 2: start-up | B1, PF25 |
| M1-CP12, CP13 | — | Tier 3: PF1's formal run of every budget | MA6, PF25–PF30 |

## 5. Risks, and where each is retired

| Risk | Retired at |
|---|---|
| The milestone is large: the whole static pipeline | Every checkpoint leaves the workspace working; the gate counts only M1's scope |
| A helper's profile silently denies something M1's code needs | M1-CP2's probes; each later checkpoint's tests run under the real profiles |
| `smallvec` or `encoding_rs` fails Axiom's audit | M1-CP5, before adoption: an upstream fix, or the owner's decision |
| The XML parser's scope grows beyond documents | M1-CP3: documents only; `DOMParser` and XHR at M3, SVG documents at M4 |
| The CSS target is missed | M1-CP11 attributes every failure; a miss with its reason goes to the owner (SB11) |
| Efficiency drifts within the budgets | The ratchet from M1-CP3 |
| Wikipedia's chrome needs flexbox and grid | MA5 judges the article; the chrome is M4's |

## 6. The owner's gates

1. **M1-CP1:** this plan adopted, with §2's decisions, and MA10's authorization.
2. **Each dependency's adoption,** with its register entry, as the checkpoints in §3 reach it, unless the owner approves them all at M1-CP1.
3. **Any amendment** to an ADR or a binding rule that M1 finds necessary (MS12).
4. **The gate:** MA9's report at M1-CP13; then the merge of `milestone-1` into `main`, `--no-ff`, when the owner says so.
