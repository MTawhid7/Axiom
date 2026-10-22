# Review Round 4: The Areas No Round Had Reviewed

| | |
|---|---|
| **Status** | **Round 4, delivered** (2026-10-22), for the owner's dispositions |
| **Question** | What holds, and what is wrong, in the areas Rounds 1 and 2 left out ([Round 2's limits](../README.md#8-round-2s-coverage-and-limits)), which the owner put in scope: "Everything unreviewed" (2026-10-22)? |
| **Covers** | The [requirements](../../requirements.md)' trace (RQ1–RQ66); the methods of spikes S1–S4; the [decision map](../../decision-map.md)'s couplings; [invalidation](../../architecture/invalidation.md) (IV1–IV14); [HTML and the DOM](../../architecture/html-and-dom.md) (HD1–HD33); the [JavaScript engine](../../architecture/javascript-engine.md)'s details, with its [bindings](../../architecture/bindings.md) and [script and heap](../../architecture/script-and-heap.md); [networking](../../architecture/networking.md) beyond its keys (NW1–NW18); dated facts older than six months |
| **Summary** | [Findings](06-findings.md): RV4-01, RV4-03, RV4-08, RV4-13, RV4-14, RV4-16, RV4-17 · [Actions](07-actions.md) |
| **Last updated** | 2026-10-22 |

Each section says what was read and how, what holds, and what became a finding. Evidence is labelled as the [research conventions](../../research/README.md) require.

---

## 1. The requirements' trace

**Read:** all 66 rows of `docs/requirements.md`, each against the rules, milestone and test it names. **Checked mechanically** (observed, 2026-10-22, a script kept in `scratch/review-r4/scripts/`): every row has all four cells; the 116 IDs cited in the rule and milestone columns all exist in the documents, the corpus's CO rules included.

**What holds:** the trace is complete and every link resolves; each charter goal and ADR decision has a requirement, and each requirement a test and a milestone.

**What is stale** (part of [RV4-14](06-findings.md#rv4-14-stale-statements-and-couplings-the-decision-map-lacks)):

| Requirement | It says | Since | Should say |
|---|---|---|---|
| RQ30 | ThreadSanitizer "when its runtime works (S9-Q2)" | S12 found a route (RV1-11; MA7) | ThreadSanitizer by S12's route, from M1 |
| RQ40 | The XML parser at M4 | RV4-01, if accepted | XML documents at M1 |
| RQ54 | B2 measured at M2 | Renderers are processes from M1 (RV1-07); MA6 measures B2 | M1 |
| RQ56 | B4 measured at M2 | The network process is a process from M1; MA6 measures B4 | M1 |
| RQ61 | Hangs and IPC limits at M2 | PF19's watchdog over the network process and IP16's containment are tested at M1 (RV2-04, RV2-13; MA7) | M1 for hangs and IPC; M2 for the GPU process's |

**One gap:** RQ5 ("Efficiency with measurable budgets") is traced to "Every gate" only ([RV4-06](06-findings.md#rv4-06-efficiency-is-checked-only-at-gates)).

## 2. The methods of S1–S4, and the spikes' builds

**Read:** S1–S4's results in full (method, threats, reproduction), their trackers' method sections, and the ADRs that rest on them (0002–0005).

| Spike | Its method | Its conditions | Its threats section | Does its ADR follow? |
|---|---|---|---|---|
| **S1** (GC patterns) | Generated inputs, verified op by op against a reference interpreter; ABBA alternation; warm-ups; three process runs; medians of medians | AC power, thermal state checked; **load uncontrolled,** 3–4, about 8 in one run, which slowed one pattern by up to 50% in that run | Seven threats, including the unrealistic root set and the uncontrolled load | **Yes.** ADR-0003 chose Pattern B "strongly" for builtins that call back into script, an architectural reason; S1's timing differences don't carry the choice. S3 later corrected what S1's root push measured (S3 §3.2) |
| **S2** (Rust and AppKit) | Builds three times; five smoke runs; five idle runs sampled by `top` and `ps` | AC power; load not recorded | Five, including one cold-start observation | **Yes,** for ADR-0002's toolchain and DP18's features: S2-Q6's 6.0 s against 29 s is a large effect |
| **S3** (interpreter, root stacks) | Deterministic counts; ABBA rotation; three runs; profiles; an addendum that re-ran everything | AC power, Low Power Mode off; **a battery run discarded** after it showed Low Power Mode's 1.64× slowdown; the addendum at load 3.1–5.4 | Ratios relied on, within a run | **Yes:** ADR-0008's `SliceStack` and JE6's `match` loop rest on ratios measured within each run, which the addendum reproduced |
| **S4** (sandbox, IPC) | 30 launches per configuration; five IPC batches; probes as single observations | AC power; macOS 26B5112a recorded; "memory was under pressure from other applications" | Six, including one machine and one OS build | **Yes:** ADR-0004 D2's bytecode profiles rest on 2.11 ms against 8 ms, and 1.14 MiB against 3.94 MiB; ADR-0005's Mach channels on S4's IPC series |

**What holds:** every spike verified its outputs before timing them, alternated variants within runs, reported medians with ranges, and wrote down its threats; the four ADRs rest on effects far larger than the noise the conditions could add (inference). PF1's conditions, made a rule at Step 6, came from these runs' lessons, Low Power Mode's above all.

**What became a finding:**
- **Builds** ([RV4-13](06-findings.md#rv4-13-spikes-measured-on-an-earlier-macos-build)): S4–S7 recorded macOS 26B5112a; S10, S12 and S16 26B118; S1–S3, S8, S9 and S15 "macOS 27.2" without a build (observed, the results' method sections). TS16's re-checks for S5, S6 and S7 haven't run on 26B118.
- **No re-run is needed** for S1–S4's decisions; S4's probes ran again under S15 (2026-10-21).

## 3. The decision map's couplings

**Read:** the map in full, against every ADR's decision history as Rounds 1 and 2 amended them.

**What holds:** §2's eight couplings still describe the architecture: JavaScript as the hub, language and memory model as one decision, the data model capping incremental work, deferral only behind owned interfaces, strings and accessibility as data-model properties, the machine as a requirement.

**What is stale:** §2 item 3's plan to "split processes later", which RV1-07 replaced with real processes from M1; §4 item 1's "single-process debugging mode", which RV1-07 narrowed to tests, fuzzing and Miri.

**Couplings this round found that the map doesn't show** ([RV4-14](06-findings.md#rv4-14-stale-statements-and-couplings-the-decision-map-lacks)):

| Coupling | What ties them | Found as |
|---|---|---|
| **P5 (HTML and the DOM) ↔ X1, X2 (standards, testing)** | The XML parser's milestone decides whether M1's CSS target can run | RV4-01 |
| **X4 (dependencies) ↔ I1, I4 (text, images)** | DP8's bar decides whether HarfRust, ICU4X's segmenter and a JPEG decoder can be used | RV4-02 |
| **P1 (event loop) ↔ F3 (process model)** | Hang detection, synchronous requests and the helpers' watchdogs | RV4-03 |
| **I2 (graphics) ↔ X3 (budgets)** | Memory one process maps and another allocated, and who is charged for it | RV4-04 |

## 4. Invalidation

**Read:** IV1–IV14 and §8, against the pipeline (PL), style's invalidation sets (ST19), layout's reuse, M1's hover and resize criteria (MA6) and IV14's self-oracle (MA8).

**What holds:** the compiler-checked lifecycle (IV1, with ST25's nested lifecycles); Blink's invalidation sets (ST19); stage reuse checked against a rebuild (IV14), which MA8 makes a gate criterion; compositor-driven scrolling within display ports (IV8, PF20).

**What became findings:**
- **IV13** sends a child frame's rectangle "to the browser in the parent's display-list update", which from M2 goes to the GPU process instead ([RV4-16](06-findings.md#rv4-16-child-frames-geometry-after-the-gpu-process)).
- **§8's questions:** the first (scroll-blocking listeners, display ports, starvation) was settled by PF19 and PF20 at Step 6, and the third (damage tracking) by I2's damage-limited tiles at Step 5; the second, which animations may run on the compositor beyond `transform` and `opacity`, was to be settled at Step 5 and wasn't, so it needs a milestone: M4, with CSS animations (part of RV4-14).

## 5. HTML and the DOM

**Read:** HD1–HD33 and §1–§10, against TX3's encodings and sniffing, the node arena (SH10, S3's layouts), the mutation hooks, MA1's row and the roadmap.

**What holds:**
- one tokenizer and tree builder, decoding straight into 8- and 16-bit strings (HD1–HD5), with chunk-independence tested (HD33, MA8);
- the node arena of `u32` columns whose slots never move (HD11), measured in S3's node layouts;
- one mutation path with fixed hooks (HD15);
- forms whose submission is a navigation request to the browser (HD26), so a renderer can't send a request the browser didn't see;
- XML without external entities, with bounded expansion (HD9);
- XSLT deferred (OQ-22).

**What became findings:**
- **The XML parser's staging** ([RV4-01](06-findings.md#rv4-01-m1s-css-gate-needs-an-xml-parser-m1-doesnt-build)), the round's Critical finding.
- **Text entry and the preload scanner** ([RV4-08](06-findings.md#rv4-08-m1s-scope-against-value)): the first document milestone's "forms without autofill", with input's "mirrors and caches exist" at L0, implies typing into pages at M1, which no criterion needs; the preload scanner is staged a milestone after it first pays.
- **§10's third question** ("the parser's time budget and the read-ahead bound", "Step 6, on the corpus") was answered by PF23 (4 ms interactive, 30 ms during the load, 64 KiB ahead) and still reads as open (RV4-14).

## 6. The JavaScript engine's details

**Read:** JE1–JE32, the bindings (WB1–WB22) and script and heap (SH), against S1's and S3's results and their dated facts.

**What holds:**
- a parser of Axiom's own over 8- and 16-bit source (JE1); bytecode with a `match` loop (JE6), which S3 measured fastest on stable Rust;
- `RegExp` interpreted, never compiled, with a linear-time fallback (JE20), and `Intl` on ICU4X (JE21);
- Temporal's and BigInt's arithmetic as replaceable details, graded before M3 and M4 (RV1-08; JE22, JE13);
- one heap for JavaScript and the DOM, with `SliceStack` and no `unsafe` in `gc` (ADR-0008, S3).

**Dated facts, checked again** (observed and sourced, 2026-10-22):
- **Explicit tail calls are still incomplete:** `nightly-2026-10-21` (rustc 1.101.0-nightly of 2026-10-20), the toolchain S12 installed, still warns that "the feature `explicit_tail_calls` is incomplete", so JE6's condition for adopting `become` ("once stable and no longer flagged incomplete") isn't met.
- **`temporal_rs`'s only grade is at 0.0.6** (Google, Chromium), against 0.2.6 now ([reuse and scope](03-reuse-and-scope.md#4-dated-facts-on-the-crates-axiom-would-use) §4), as JE22's grading before M3 already allows for.

**No finding.** The engine's first milestone is M3, and nothing here blocks M1.

## 7. Networking beyond its keys

**Read:** NW1–NW18, §4's build-or-reuse table and §8's staging, against S7's results and the advisories of 2026.

**What holds:**
- protocols as state machines driven by one I/O thread (NW1), which S7-Q4 measured;
- one URL parser in every process (NW2), with `url` as the differential reference;
- rustls with the system verifying certificates (NW6, S7-Q2), so `rustls-webpki`'s verifier advisories (RV4-15) touch only the spikes;
- Fetch's policy enforced from the browser's records (NW12), with HSTS's preload list compiled in and refreshed by rebuilds;
- decompression bounded by ratio and size (NW13), which covers the class of `ruzstd`'s 2026 advisory, a decompression bomb, beyond its fix;
- cookies only in the network process, with third-party cookies blocked (NW11).

**What became findings:**
- **HTTP/2 at M1** ([RV4-08](06-findings.md#rv4-08-m1s-scope-against-value)).
- **§8's L1 row,** "The network process sandboxed, with S7's rules", since RV1-07 a property of M1 (RV4-14).
- **NW10's cache is "bounded in size"** with no size, and no budget covers disk ([RV4-06](06-findings.md#rv4-06-efficiency-is-checked-only-at-gates), PF27).
- **The cache and the cookie jar arrive at M2,** on a storage engine the roadmap chooses at M3 ([RV4-17](06-findings.md#rv4-17-m2s-cache-and-cookie-jar-wait-on-a-storage-engine-chosen-at-m3)).

## 8. Dated facts older than six months

Sources a decision rests on, checked again where the decision is near (observed, 2026-10-22):

| Fact | Where it is used | Checked | Result |
|---|---|---|---|
| Crates' versions, maintenance, advisories and grades | The register; TX, IM, NW §4 | crates.io, RustSec, the six audit sets ([reuse and scope](03-reuse-and-scope.md#4-dated-facts-on-the-crates-axiom-would-use) §4) | One open advisory, in the spikes' lock (RV4-15); four dormant candidates; old grades, re-graded by DP8 at adoption |
| WebKit's and Chromium's IOSurface, lock and hang designs | GR12, GR14, SW10, PF19 | Their sources at 2026-10-22's revisions ([verification](02-verification.md#3-sources)) | Matches the designs, with RV4-03, RV4-07 and RV4-10's additions |
| XNU's IPC and accounting | IP5, PL8, PF4 | `xnu-12377.121.6` (2025-10-16), **macOS 26's kernel:** macOS 27's isn't published, so these are the latest available, and S16 measured the behaviour on this Mac's kernel | RV4-04, RV4-05 |
| Explicit tail calls | JE6 | `nightly-2026-10-21` | Still incomplete (§6) |
| Character encodings' use | TX3 | W3Techs, dated 2026-10-22 (secondary) | UTF-8 99.1% (RV4-08) |
| The CSS tests' media types | MA4 | `wptserve` at `0f26972422` | XHTML is XML (RV4-01) |

**Not checked again:** the standards' snapshot dates, which MA3 pins afresh at M1's start; the corpus's terms and robots files, which CO3 reads again before each census.
