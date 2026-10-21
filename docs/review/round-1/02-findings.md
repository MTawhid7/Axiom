# Review Round 1: Findings

| | |
|---|---|
| **Status** | **Round 1, disposed** (2026-10-21): the owner accepted every finding, in the words the [disposition log](../README.md#5-the-disposition-log) records. Where those words differ from a counter-proposal here, they govern. The findings stay as written, as the round's record |
| **Question** | Where is the Phase Zero baseline wrong, inconsistent, outdated or over-optimistic, and what should change, and when? |
| **Covers** | The baseline approved at Step 6's Gate C: the [handover](../../handover-phase-0.md), ADRs 0001–0021, the [threat model](../../security/threat-model.md) and [process model](../../security/process-model.md), the designs under `architecture/`, the [standards](../../standards.md), [testing](../../testing.md), [performance](../../performance.md), [roadmap](../../roadmap.md) and [Milestone 1](../../milestone-1.md) documents, and spikes S1–S9 |
| **Summary** | [Risk matrix](01-risk-matrix.md) |
| **Last updated** | 2026-10-21 |

**How to read it.** Each finding gives:
- its severity, its area, the milestone it blocks (whose plan can't start until the owner has decided it), and the commission's focus area where one applies;
- the claim it challenges, quoted from the baseline with its rule ID;
- the evidence, labelled as the [research conventions](../../research/README.md) require:
  - **sourced**, with a numbered source below, published and checked dates;
  - **observed**, on this Mac, naming the file or command;
  - **measured**, by a spike, named;
  - **inference**, **hypothesis** or **background**;
- the failure mode: what goes wrong if the baseline is built as written;
- a counter-proposal, and what it would cost to make the change later instead of now;
- the rules it would amend.

Severities are defined in the [review process](../README.md#3-findings-and-severities). Rule IDs belong to their documents (the handover's appendix A). Every external source was read on 2026-10-21.

---

## RV1-01. Milestone 1's CSS target and harness need script

**Critical** · Standards, Milestone 1 · Blocks M1 · Focus 5 · Challenges MA4, MA7, MA11, TS2, TS3, SB10, SB11 ([Milestone 1](../../milestone-1.md); [ADR-0021](../../adr/0021-milestone-1.md); [testing](../../testing.md); [standards](../../standards.md))

**The claim.**
- MA4 targets `css/CSS2`'s "5,848 tests" at "≥ 97%: the median of Chrome (98.2%), Firefox (97.8%) and Safari (96.4%) on those same tests", run by "wptrunner's `axiom` product, through the test-only WebDriver server".
- MA7: "No JavaScript engine is linked: the `js` and `bindings` crates aren't dependencies of any M1 executable".
- SB10: "Tests the milestone doesn't run at all count as failures".

**Evidence.**
- **Observed: 196 of the 5,848 tests contain script.** The command was `python3 -I tools/wpt-script-deps.py target/s9/wpt target/s9/MANIFEST-sparse-0f26972422.json --dir css/CSS2 --exclude css/CSS2/tables/ css/CSS2/inline-svg-`, using the [tool](../../../tools/wpt-script-deps.py) this round adds, over S9's checkout at WPT `0f26972422`, MA4's revision. The exclusions reproduce MA4's scope of 5,848.
  - **Their kind:** 193 run inline code, and 51 use `reftest-wait`, which only script removes.
  - **The bound:** if they fail, at most 5,652 pass, **96.65%**.
  - **Where they are:** they cluster in `normal-flow` (53), `box-display` (34), `text` (22) and `generated-content` (21).
- **Observed: the script matters.** Sampled, the tests mutate the document after layout:
  - `box-display/insert-inline-in-blocks-n-inlines-end-002.xht` reads `document.body.offsetHeight`, then inserts a span;
  - `normal-flow/block-in-inline-insert-002d.xht` inserts a block;
  - `box-display/delete-inline-in-blocks-beginning-001.xht` removes one.

  They test incremental relayout after mutation: the paths IV14's oracle guards, and the ones most likely to be wrong.
- **Observed: the specified harness runs script in the page.** In wptrunner at the same revision (`tools/wptrunner/wptrunner/executors/executorwebdriver.py`):
  - `WebDriverRefTestExecutor.do_test` begins with `execute_script("return [window.outerWidth - window.innerWidth, …]")` (line 1410);
  - `_screenshot` calls `protocol.testdriver.run(url, self.wait_script)` (line 1451), which runs `test-wait.js` in the page asynchronously.

  TS2 lists "execute script (including asynchronous)" among the server's commands. Without a script engine, that executor can't run one reference test.
- **Observed: a script-free precedent sits in the same directory.** `executorservo.py` launches the browser per test and reads the screenshot it writes, so it needs no script in the page.
- **Observed: a smaller inconsistency.** MA4 has the headless embedder inject "WPT's test fonts, Ahem among them, as system fonts". TX12 rasterizes system fonts with CoreText. TX21 and TS3 say pixel tests use web fonts, which "take the Rust path and so don't change with macOS".
- **Observed: MA11 builds the harness at checkpoint 10 of 12,** so the defect would surface late in the longest milestone.

**Failure mode.** The gate can't pass as written. 97% of 5,848 is 5,673 tests, and there are 5,652 script-free tests: even if every one of them passes, 21 of the script tests must pass too, and without script none can. So the milestone either stalls on a target only script can meet, or is revised at the gate under pressure. Meanwhile, the specified harness can't run at all.

**Counter-proposal.**
1. **An interim CSS target at M1** (SB11), computed the same way over the 5,652 script-free tests: the shipping median on that subset, rounded down. The full target moves to M3, when script runs, and the dynamic tests then become M3's regression guard for IV14.
2. **A script-free harness at M1.** The headless embedder takes a URL, waits natively for the load event, pending images and fonts, and writes a PNG. wptrunner's `axiom` product uses an executor modelled on `executorservo.py`. WebDriver arrives with script at M3. TS2 says so.
3. **Fonts in pixel tests at M1:** either the injected test fonts take the Rust rasterizer, as TX21 intends, or MA4 records that M1's references depend on CoreText and are rerun after macOS updates (TS16).
4. **Run the harness end to end at M1's second checkpoint,** on a trivial test, not at its tenth.
5. **Spot-check every tool-derived target against an independent count.** This is the second target found wrong by recounting, after S9's correction of `tools/wpt-pass-rates.py` (handover item 24).

**Cost now and later.** Now, a paragraph in two documents. Later, a failed gate, or a harness rebuilt at the end of M1.

**Amends.** MA4, MA7 (the headless test embedder's wording), MA11's order, TS2, TS3; ADR-0021's decision history.

---

## RV1-02. Compositing stays in the unsandboxed browser through L3

**Critical** · Security, architecture · Blocks M1 (where the compositor's code lives, and its interfaces) · Focus 1, 4 · Challenges ADR-0004 D1; the process model §3 note 1, §4.2, §7 (B1); GR10, GR12, GR14; PL8, PL9, PL15, PL16; roadmap MS5

**The claim.**
- "The browser composites until a GPU process exists" ([process model](../../security/process-model.md) §3, note 1). The GPU process "arrives with the first web-supplied GPU work" (ADR-0004 D1), which the [roadmap](../../roadmap.md) places at M9, three milestones after L3's gate (M6).
- §4.2 counts "consuming display lists in the browser" as two of the Rule of 2's three, because "the Metal driver receives only our commands".
- GR14: later, "rasterizing tiles moves into the GPU process", while "the browser keeps the CoreAnimation layer tree" and "the display-list format and validation stay as they are".

**Evidence.**
- **Sourced: Safari draws web content in its GPU process, and its UI process hosts the layers.** WebKit bug 254725, "Enable GPU Process and UI side compositing on macOS", was fixed on 2023-04-05 [W1]. A WebKit engineer, on 2024-06-17: "'GPU Process: DOM Rendering' is on by default in all recent Safari versions (Safari 17 on macOS, and even longer on iOS)" [W2].
- **Sourced: Chromium's Viz** is "a collection of subservices: compositing, gl, hit testing, and media", started as "the Viz process", to which "the browser process … serves as the privileged client" [C2]. (Background: Chrome's task manager calls it the GPU process.)
- **Sourced: Ladybird.** "The compositor also runs in its own process now", with recovery so that "a compositor crash doesn't take down the page" (newsletter, 2026-05-31) [L1]. The roadmap's §2 cites the same newsletter.
- **Observed in the design: what the browser runs at L3.** Through M6–M8 the browser process runs, with the owner's full authority (trust zone Z1):
  - it decodes and validates every renderer's display list (PL9);
  - it uploads renderer pixels (PL8);
  - it encodes Metal work from them and drives CoreAnimation.
- **Inference: the Rule-of-2 row's premise.** "Only our commands" bounds the instruction set, not its parameters. A compromised renderer (AD2) chooses the geometry, sizes, counts, clip and effect trees, texture dimensions and pixel contents. Driver and framework faults reached through valid but unusual parameters are what a GPU process contains (background). A logic error in PL9's validator then runs with the browser's privileges.
- **Inference: GR14 leaves the topology unfinished.** It moves raster and leaves validation, property trees and hit testing in the browser, so after M9 a display list has consumers in two processes. That topology is designed nowhere:
  - who receives the list first;
  - the latency budget for the hop;
  - where compositor-driven scrolling and animation run (PL15);
  - how hit-test data reaches input routing (PL16, IN1).
- **Observed: certification runs on a topology that later changes.** V1–V9 certify L3 at M6. M9 then changes the compositor's process, its endpoints and its profile, so V1, V2, V5 and V8 must run again for a surface that sat in the browser throughout M6–M8. The roadmap's MS2 accepts the re-run; it doesn't consider the exposure meanwhile.
- **Inference: availability.** A GPU hang or the loss of the Metal device in the browser process takes every window with it. Ladybird gives compositor crash isolation as a reason for its move [L1].

**Failure mode.**
1. At the sensitive-use level, a compromised renderer reaches a driver or validator bug in the most privileged process.
2. A GPU fault ends the browser.
3. The move at M9 changes interfaces that M1–M8's code assumed: a retrofit, which principle 7 forbids.

**Counter-proposal.**
1. **Design the end-state now,** in amendments to ADR-0004 and ADR-0012: renderer → **sandboxed GPU/compositor process** → browser.
   - **The GPU process:** decodes and validates display lists (PL9's checks, unchanged in content), holds the property trees, rasterizes tiles into IOSurfaces, composites, and owns every surface.
   - **The browser:** keeps windows, the CALayer tree (GR14's half), input routing and policy. It receives hit-test regions and scroll-node geometry through a shared-memory snapshot, as Viz shares hit-test data (inference), and IOSurface ports for layer contents.
2. **Decide where scrolling and compositor animation run** (PL15): either in the GPU process, with the browser routing input there; or in the browser on layer transforms, with raster remote. Measure the extra hop.
3. **Stage the implementation, not the topology.** At M1 the GPU process's code runs as a thread behind its IDL interface (IP2, IP21), and M2 splits it out with the renderer and the network process.
4. **Build the process at M2;** at the latest before M6, so that L3 is certified on the final topology.
5. **Split B1:** the browser's core, without the Metal device, and a GPU-process budget (the Metal device, about 20 MB in S2; tiles at 6.2 bytes a pixel, PF6).
6. **Spike S10** ([actions](04-round-2-actions.md#4-proposed-spikes)) measures:
   - the hop: display list to GPU process, and IOSurface to browser, at 60 Hz;
   - the cost of validation at display rate (the handover's §8 lists it as untested);
   - zero-copy resource sharing (RV1-09).

**Cost now and later.** Now, a design and two ADR amendments. Later, a topology change after L3, touching the compositor, IPC, input routing, budgets and V1–V9.

**Amends.** ADR-0004 D1; the process model §3, §4.1, §4.2, §7 (B1); GR10, GR12, GR14; PL8, PL9, PL15, PL16; IN1; the roadmap MS5 and §7; the overview §1 and §6.

---

## RV1-03. Cross-site agents before L2: the documents disagree

**High** · Architecture · Blocks M1 (the frame tree's and the bindings' shape) · Focus 1 · Challenges FT10, the frame tree §5, the overview §6, roadmap M5, bindings §8, WB19, WB20

**The claims, side by side.**

| Document | Says |
|---|---|
| [Frame tree](../../architecture/frame-tree-and-navigation.md) FT10 | "Cross-site child frames stay in their embedder's renderer, **in a separate agent and event loop**, only at L1 and non-sensitive L2 (SI3)" |
| Frame tree §5, L0 row | "The frame tree, replicas, **remote window proxies** and browser-routed operations **all exist**" |
| Frame tree §5, L2 row | "Cross-site frames **move to separate agents** and event loops, in their embedder's process" |
| [Overview](../../architecture/overview.md#6-staging-from-l0-to-l3) §6, what L2 builds | "separate agents and event loops for cross-site frames in their embedder's process (SI3), reached through remote window proxies (WB20)" |
| [Roadmap](../../roadmap.md#41-what-each-milestone-contains) M5 | "Cross-site frames **become their own agents** and event loops in their embedder's process" |
| [Bindings](../../architecture/bindings.md#8-staging-and-the-complete-model) §8 | First script milestone: "`Window` and `WindowProxy` within an agent"; remote window proxies come "Next … with L2's process separation" |
| Roadmap M3's target | "MDN in full: its live examples, **in cross-site frames**" |

**Evidence.**
- **Background:** HTML keys agent clusters by site, or by origin when origin-keyed, so documents of different sites never share an agent.
- **Observed:** the documents above. Two say "from L0/L1"; three say "from L2".

**Failure mode.** M3 and M4 need cross-site frames that run script. Without remote proxies, the only way to build them is one agent shared with the embedder. Cross-site access then becomes same-heap `WindowProxy` access, guarded by checks in software: WB19's same-site cross-origin path, stretched to cross-site, which HTML doesn't allow. M5 must then replace those paths with remote proxies and separate event loops, and change every test that assumed them. That is the retrofit FT6 and principle 7 exist to prevent.

**Counter-proposal.**
1. **One rule**, in FT10 and SH1: from M1, every cross-site frame is its own agent, with its own event loop and heap, reached only through a remote `WindowProxy`, in any process. L2 adds site locks, not agents.
2. **WB20 moves into the bindings' first script stage** (M3).
3. **Three documents are corrected to "remain separate agents":** the overview §6 (L2 row), the frame tree §5 (L2 row) and the roadmap's M5 text.
4. **A test from M3:** script in a cross-site frame that reaches its parent's DOM fails the test, in every process configuration. It is V9's check, applied from the first script milestone.

**Cost now and later.** Now, three sentences and a staging row. Later, a bindings and frame-tree retrofit at M5.

---

## RV1-04. The collector's end-state against the pause budgets

**High** · Performance, the collector · Blocks M3 (the heap and object model) · Focus 4 · Challenges SH9, SH10, EL10, EL11, EL16, JE29, PF14, PF15; ADR-0008 decision 8

**The claim.**
- SH10: "Version 0 is ADR-0003's stop-the-world mark–compact collector". Then "**incremental marking in idle periods** (EL16) … and a generational nursery". "Compaction stays a full, rarer step" ([script and heap](../../architecture/script-and-heap.md#34-collection-and-interop-c4-c5)).
- EL10: "GC heaps never cross threads".
- PF15: pauses ≤ 4 ms while interacting and ≤ 50 ms in idle time, where "a corpus page that breaks either is the trigger" for incremental marking. PF14: no engine task over 50 ms.

**Evidence.**
- **Measured (S1):** about 4 ms at 130,000 live nodes, and 15 ms at 333,000 objects: 31–45 ns per live object.
- **Inference: application heaps.** A heap of one to three million objects pauses 30–135 ms per full collection. For scale (measured, S8): VS Code's V8 heap held 66.9 MB, and Reddit's renderers about 1.5 GB in Chrome (PF12).
- **Inference: idle-only marking.** Incremental marking that runs "in idle periods" never finishes on a page that is never idle: a feed being scrolled, or an editor being typed in. Engines also advance marking from allocation: V8's incremental steps, SpiderMonkey's slices (the precedent the script-and-heap design itself cites, its §2).
- **Inference: compaction.** Incremental marking doesn't bound compaction. Rewriting every handle into a compacted arena is a pause proportional to the heap.
- **Sourced: concurrent marking is the practice.**
  - **V8** has marked concurrently since Chrome 64, cutting main-thread marking time by 60–70% [V1].
  - **JavaScriptCore's** Riptide "reduces worst-case pause times by allowing the app to run concurrently to the collector" [W6].
  - **Oilpan** "assumes heaps are not shared among threads", yet marking "can also be executed incrementally/concurrently" [V2].

  Thread-local heaps, Axiom's model, don't exclude concurrent marking. What excludes it in Axiom is storage the type system won't let another thread read while the mutator writes.
- **What holds** (inference, from the designs' evidence):
  - **One traced heap** for JavaScript and the DOM removes cycle collection between them (SH1).
  - **Microtask checkpoints and `WeakRef` clearing** follow HTML and ECMA-262 (SH4, JE16).
  - **HD11's identities** (index plus generation) make stale node references detectable outside the heap.
  - **Safety:** the `gc` crate has no `unsafe` (SH7).

**Failure mode.** The first application, or a feed scrolled long enough, breaks PF15 and PF14. The "trigger" then calls for a collector redesign under arenas, handles and an object model fixed since M3. Concurrent marking in safe Rust needs memory another thread may read while the mutator writes: atomic slots, and arenas that don't reallocate under the marker. Adding those later touches every typed column.

**Counter-proposal.**
1. **Fix the end-state before M3,** in an amendment to ADR-0008:
   - a nursery with a remembered set;
   - incremental marking driven by allocation as well as by idle time;
   - parallel marking during pauses, which EL11's frozen lending allows with atomic mark bitmaps in safe Rust;
   - a non-moving tenured space, with evacuation of selected sparse pages instead of whole-heap compaction. The node arena already never moves (HD11).
2. **Decide whether concurrent marking is excluded for good.**
   - **If not**, require from M1 arenas built of chunks that never move, and slot types that can become relaxed atomics. On ARM64 relaxed loads and stores are ordinary instructions (background), so the cost now is small; later it is every column.
   - **If so**, record it, with the evidence that incremental and parallel marking meet PF15 on the corpus.
3. **Choose the barrier semantics now:** snapshot-at-the-beginning or incremental-update, and black allocation. SH9's setters are then specified, not only placed.
4. **Spike S11** ([actions](04-round-2-actions.md#4-proposed-spikes)) measures pauses for each strategy on a synthetic million-object heap, together with RV1-06's slot width.

**Cost now and later.** Now, an amendment and a spike. Later, a collector redesign under a fixed object model.

---

## RV1-05. The content tier's 150 MB cap

**High** · Performance · Blocks M4, when the content pages are first measured · Focus 3 · Challenges PF3, PF4, PF8, PF11, the [performance](../../performance.md#41-ceilings-for-the-corpus-pages) §4.1, B3, B6, PL8; [ADR-0019](../../adr/0019-performance-methodology-and-budgets.md); OQ-25

**The claim.** "A page's steady-state cost in Axiom (PF4) is at most half of Chrome Canary 158's … and at most its tier's cap": 150 MB for Google Search, Instagram and Reddit, 12–26% of Chrome's signed-in cost. PF4 counts "its renderers (out-of-process frames included), plus the GPU and compositor memory allocated on its behalf".

**Evidence.**
- **Measured (S8, by hand): Reddit's cost, and its spread.** Chrome cost 1,260 MB at the median, with runs spread from 495 to 1,631 MB (handover §9), and its renderers together reached about 1.5 GB across five processes (PF12). The interaction was by hand, and varied between runs ([S8 results](../../research/s8-results.md) §7).
- **Inference: the cap's scale.** The cap is about a tenth of Chrome's renderer-side memory on that page, at an interaction depth nobody controlled.
- **Inference: what Axiom can't shrink.** An infinite feed keeps whatever DOM and objects its own script keeps, and no engine can drop them. Axiom controls constant factors: bytes per node, object, string and image. RV1-06 suggests its JavaScript objects may cost more than V8's, not less.
- **Observed: out-of-process frames.** PF4 charges each out-of-process frame's renderer to its page. At L3 every cross-site frame (ads, embeds, Reddit's interactive posts) is a renderer with B2's 10 MB before any content.
- **Observed: images held twice.** PL8: the browser "only uploads pixels it receives to the GPU", so a decoded image lives once in the renderer's shared memory and again as a texture, and PF4 charges both to the page.
- **Measured (S8 §5): Chrome's document costs are its caches.** On document pages Chrome's GPU process holds large caches (Wikipedia: 170 MB at the steady state), which Axiom's design avoids with one prepainted row of tiles (PF20).
- **Inference: where "half of Chrome" holds.** It is plausible for documents, where Chrome's own caches dominate, and implausible for feeds, where the page's heap does.

**Failure mode.** At M4 or M5 the gate fails for reasons outside engineering (scroll depth, the page's retention). Or it passes on a shallow interaction that no longer reflects use. Either way the cap stops carrying information.

**Counter-proposal.**
1. **A normalized workload** for each content page, a checklist for the owner's hand (CO6). For example: load; scroll ten screens, or to the 50th item; wait 30 s; record the items reached.
2. **A breakdown of Chrome's cost, by hand.** The owner records a Chrome trace with memory-infra in their own session (`chrome://tracing`; nothing automated in the page). It splits the cost into V8's heap, Blink's DOM and style, images, the compositor and the GPU, and gives a bottom-up budget per component for Axiom.
3. **Gate on what Axiom controls:**
   - a slope: megabytes per hundred items scrolled, at most half of Chrome's slope;
   - a plateau: after a collection and 30 s idle, memory returns within 10% of its level.

   150 MB stays as a stretch figure at the normalized depth.
4. **Zero-copy resource sharing as a rule** (PL8, RV1-09), so decoded images are held once.
5. **Re-derive B6** with the renderers of out-of-process frames counted, and state what B5's soft limit does when one tab's frames alone need several renderers.

**Cost now and later.** Now, a protocol, a by-hand session and a rewritten §4.1. Later, a gate whose result says nothing about the engine.

---

## RV1-06. The width of heap slots

**High** · Performance, the JavaScript engine · Blocks M3 · Focus 3, 4 · Challenges JE9, JE10, SH13, the performance §4.1 ("The JavaScript heap is the risk"), PF8

**The claim.** JE9: "A NaN-boxed 64-bit value". JE10: an object holds "a few inline slots, an optional out-of-line slot array, and an optional elements store", each slot holding a JE9 value ([the JavaScript engine](../../architecture/javascript-engine.md#53-values-objects-and-strings)).

**Evidence.**
- **Sourced:** V8's pointer compression reduced "V8 heap size up to 43%" by storing tagged fields in 32 bits (the JavaScript engine design's source [V3]).
- **Observed: references already fit in 32 bits.** Axiom's handles are `u32` (SH8), so a heap reference needs 32 bits plus its arena's kind.
- **Observed: the risk is parked late.** The performance §4.1 records it as a hypothesis "measured at the applications milestone", M8.
- **Inference: the size of the difference.** In pointer-dense heaps (arrays of objects, closures, a framework's component trees), 8-byte slots cost up to twice V8's 4-byte ones.

**Failure mode.** From M3, the object model, the interpreter's slot access, the bindings and the collector's tracing are all written against 8-byte slots, and the memory cost is first measured at M8.

**Counter-proposal.** Decide before M3, from a memory model of typical heaps:
- **Registers and frames:** values stay NaN-boxed (JE7), where arithmetic is hot.
- **Heap slots:** 32 bits, holding a small integer or a typed handle (kind bits and an index, or a kind kept per shape). Doubles are boxed, as V8 boxes them, or stored in fields the shape types as double.
- **The trade-off is measured in spike S11** (RV1-04), on synthetic heaps shaped like S8's pages.

**Cost now and later.** Now, a decision and a micro-spike. Later, a change to the object model, the interpreter, the bindings and the collector together.

---

## RV1-07. Milestone 1 in one process

**High** · Architecture · Blocks M1 · Focus 1 · Challenges MS4, IP2, IP14, IR7, IR10, FT13, MA6, the roadmap §10, the process model §9

**The claim.** "M1 is one process in which every thread role, channel, record and replica exists, with in-process channels" (MS4); "Messages are encoded and decoded in every configuration" (IP2). The [roadmap](../../roadmap.md#10-alternatives-considered) §10 rejected the network process at M1 because it "mixes two risks: the first pipeline and the first process split".

**What is strong.** IP2's rule that messages are always encoded, FT6's browser-routed operations, FT11's endpoints bound at commit and EL24's scheduler keep the interfaces honest in one process. That is the lesson of Firefox's Electrolysis and Fission, and the baseline learned it.

**Evidence: what one process doesn't test.**
- **Observed: shared memory.** IP2 makes an in-process channel "an in-memory queue". IP14's shared-memory rings, out-of-line copies and resources, and IR7's rule that a reader "copies what it interprets before validating it", have no in-process counterpart in the documents. An implementation that passes reference-counted buffers in M1 never exercises them.
- **Observed: crashes.** The documents set no panic policy (RV1-16). FT13's and IR10's recovery after a renderer dies has no counterpart at L0: a renderer defect ends the browser.
- **Measured (S4, S5, S7): silent sandbox failures.** CoreText fails silently without its files. ImageIO, CoreText's colour emoji and `SecPolicyCreateSSL` need metadata rules the kernel didn't log. Failures of this kind appear only under the profiles, at M2.
- **Observed: budgets.** MA6 can budget only the whole process (95 MB, "at L0 share one process (inference)"). B2 and B3 are first measured per renderer at M2.
- **Sourced: precedent.** Ladybird put its helpers in sandboxes before its alpha [L2]. The [process model](../../security/process-model.md#9-staging-by-security-level) §9 recommends that "the renderer becomes a separate process no later than the first milestone that loads pages from the network beyond public documents".
- **Observed: M1's size.** M1 is the largest milestone: MA11's twelve checkpoints, and ADR-0021's "High" risk that it is long.

**Failure mode.** Process-specific defects (lifetimes, shared memory, crash recovery, sandbox denials, per-process memory) surface together at M2, after the longest milestone. Meanwhile M1's code has settled on in-process shortcuts.

**Counter-proposal, two options for the owner.**
- **Minimum:**
  - in single-process mode, channels use the real `platform` shared-memory objects (Mach memory entries) and IR7's copy-then-validate;
  - a test mode ends an agent's thread loop at random points, and runs FT13's and IR10's recovery;
  - text and image tests run once in a helper process under the renderer's profile, as S4 and S5 did.
- **Recommended: M1 spawns real processes.** The renderer and the network process, and the GPU process if RV1-02 is accepted, run under the profiles S4, S5 and S7 already wrote. Single-process mode remains for tests, fuzzing and debugging (PM6, R6). M2 becomes the sandboxed engine hardened: audit tokens, V1 and V2, crash containment, the spare renderer, discarding.

**Cost now and later.** Now, part of M2's work moves into M1. Later, M2 finds the process defects M1 could have found.

---

## RV1-08. Scope against one developer

**High** · Program · Blocks M1 (encodings) and M3 (the engine's library) · Challenges DP1 (g), DP2, DP8, DP15; TX3; JE13, JE19–JE22 and the build-or-reuse table; ADR-0001's schedule risk; [ADR-0017](../../adr/0017-javascript-engine-and-bindings.md)

**The claim.**
- DP1 (g) makes the engine's builtins Axiom's own: Temporal ("our own builtins"), BigInt arithmetic, and `RegExp`.
- TX3 writes every WHATWG encoding in Axiom's own code, because `encoding_rs` was graded `ub-risk-3` at version 0.8.33.

**Evidence.**
- **Observed: the schedule risk is rated High.** ADR-0001 rates the risk that "JS engine and app-web work dominate the schedule" at High likelihood and High impact. MS11 assumes one human developer.
- **Observed: the reuse line is drawn inconsistently.**
  - `Intl`'s formatters come from ICU4X as a foundational dependency (JE21), but Temporal's calendar arithmetic over the same ICU4X data is rebuilt (JE22).
  - `temporal_rs` is shipped by V8 and used by Boa (the [JavaScript engine](../../architecture/javascript-engine.md#4-build-or-reuse) §3, sourced there), and Axiom keeps it only as a test reference.
- **Observed: `encoding_rs` was barred on an old grade.** "Its 2026 releases rewrote the ASCII paths 'to reduce `unsafe`', but no newer grade is published" ([text and fonts](../../architecture/text-and-fonts.md) §4). DP15 allows Axiom's own audits.
- **Inference: rewriting the encodings is a security trade, not only a schedule one.** The WHATWG encoders and decoders are security-relevant: GB18030, Big5, EUC-JP, ISO-2022-JP, Shift_JIS, EUC-KR and the single-byte encodings, where a decoder state error becomes an injection-class confusion. New safe code removes the memory-safety risk, but adds a logic risk that a fuzzed decoder in production (Firefox's since version 56) has already paid down.

**Counter-proposal.**
1. **Reclassify these as replaceable implementation details** behind Axiom's interfaces (DP2), subject to DP8's grades:
   - Temporal's arithmetic (`temporal_rs`);
   - BigInt arithmetic;
   - the encodings (`encoding_rs`).
2. **Keep as core** what defines the engine's architecture: the parser, bytecode, interpreter, objects and collector. Keep `RegExp` too, where JE20's reasons hold: interpretation fast enough without a JIT, and direct 8- and 16-bit matching.
3. **Before M1, grade the current release of `encoding_rs`** under DP8 (Axiom's own audit, DP15), with its SIMD feature off. Write the encodings only if it fails.
4. **For each reclassification, record what charter G1's learning goal loses and what the schedule gains,** for the owner's decision.

**Cost now and later.** Now, a decision per component. Later, years of work on specified, already-solved problems, on the schedule ADR-0001 calls the dominant risk.

---

## RV1-09. IOSurfaces in renderers

**Medium** · Security · Blocks M1 (IM1's resource path; GR10's from M3) · Challenges the process model §4.1, IM1, GR10, PL8, V1

**The claim.**
- IM1: images decode "into shared memory, or into IOSurfaces for large images".
- GR10: canvas 2D draws "into a shared IOSurface resource".
- The [process model](../../security/process-model.md#41-os-capabilities) §4.1 gives renderers "IOSurface allocation only, if needed (rules from S4)".

**Evidence.**
- **Sourced: WebKit removed the access.**
  - WebKit's WebContent profile denies `iokit-open-service`, `iokit-open-user-client` and the IOKit MIG routines under its `BlockIOKitInWebContentSandbox` state flag [W4];
  - WebKit bug 219368 (2020-11-30) is titled "GPU Process: IOSurfaces should not be mapped into the Web Content Process" [W3].
- **Sourced: Chrome kept it.** Chrome's renderer profile still allows `iokit-open` of `IOSurfaceRootUserClient` and `IOSurfaceSendRight` [C1]. So the engines differ; WebKit's is the stricter posture.
- **Inference: the cost of the access.** Mapping an IOSurface means the renderer talks to a kernel user client. The sandbox otherwise keeps IOKit out of the renderer's reach (§4.1).
- **Observed: renderers don't need GPU surfaces.** Axiom's renderers rasterize on the CPU (GR5, GR10, TX12), so they need writable memory.

**Counter-proposal.**
1. **Renderers write pixels only into Mach memory entries** (IP14). The compositor, in the GPU process if RV1-02 is accepted, wraps them without copying where Metal allows it (a buffer over the mapped, page-aligned region, then a linear texture or one blit; inference), and owns every IOSurface.
2. **Every renderer profile denies `iokit-open` with an explicit rule,** as J3 denies dynamic code generation explicitly. V1's escape suite tests it.
3. **Spike S10 measures the path** against IOSurfaces (hypothesis: one blit per changed image costs less than its decode).

The same rule removes RV1-05's doubly held images.

---

## RV1-10. The hybrid text model's gaps

**Medium** · Text · Blocks M1 (system fonts and emoji are M1's) · Focus 2 · Challenges TX8–TX14, TX21; [ADR-0011](../../adr/0011-text-fonts-and-unicode.md); S5

**The assessment: tenable.**
- **Shaping is unified:** HarfRust shapes every font (TX9).
- **At 2×, the owner's display,** `zeno` and `vello_cpu` match CoreText's unsmoothed masks to 0.56–1.06 of 255 levels; CoreText's higher ink, 0.10–0.18 px, is a 1× effect ([S5 results](../../research/s5-results.md) §4).
- **Two rasterizers mean two code paths and two calibrations,** not two atlas formats: TX13's 8-bit and RGBA pages serve both.
- **Web fonts stay away from C parsers** (TX7), which no shipping engine on macOS does ([text and fonts](../../architecture/text-and-fonts.md) §3).

**The gaps.**
1. **Metrics and ink disagree for system fonts.**
   - **Measured (S5):** CoreText draws Apple Color Emoji "larger and lower than the font's data says", and adds "CoreText's private emoji spacing". Layout takes advances from font data, through `read-fonts` and HarfRust (TX8, TX9).
   - **Sourced:** Chromium on Apple platforms has HarfBuzz compute advances itself only for fonts with a `trak` table and no `sbix` table; for every other font it takes them from Skia, which on macOS draws through CoreText [C3]. So Chrome's emoji advances are CoreText's.
   - **Inference:** Axiom's Apple Color Emoji (an `sbix` font) will be laid out in boxes smaller than CoreText's bitmaps. The result is clipped atlas entries or overlapping neighbours, on every emoji-heavy corpus page (Discord, Reddit, Instagram).
   - **The rule needed:** for each system font, the advances and metrics layout uses come from the same source as the rasterizer that draws it (CoreText's, for `sbix` faces), or the rasterized bitmap is normalized to the box the font data gives.
2. **`-webkit-font-smoothing` appears in no document.**
   - **Observed:** S8's raw records list `WebkitFontSmoothing` among the CSS properties of Google Maps, Three.js and VS Code for the Web, three of the five automated pages (`target/s8-results/20261020-121635-auto/auto-chrome-*-run1.json`, field `css_properties`).
   - **Background:** on macOS the property turns off the platform's font smoothing for the text it applies to.
   - **The consequence:** TX13's emulation must honour it on both rasterizer paths, and SB5 should record it as behaviour outside the standards that the corpus needs (the charter's non-goal on compatibility).
3. **Optical sizing.** `font-optical-sizing` and the `opsz` axis appear in no document. The system UI font is variable with an optical-size axis (background), and TX10's rule for `trak` needs a sibling for `opsz`.
4. **Calibration.** TX13's emulation of smoothing for web fonts has no acceptance metric. It needs one (S5's comparison, with the ink ratio within a stated tolerance), and a place in TS16's checks after each macOS update, since CoreText's smoothing can change with macOS.

**Also:** atlases are per renderer (TX14, TX21). At L3 each out-of-process frame has its own, which B2 and PF4 should count.

**An alternative, recorded for the owner, not proposed.** One Rust rasterizer for every font:
- system fonts read through `skrifa`, and `sbix` bitmaps through the PNG decoder;
- CoreText kept for the font database and fallback only.

At 2× its visual cost is small (S5). It would remove CoreText rasterization and its silent-failure rules from the renderer, and make every pixel test independent of macOS updates. Its price is the platform's look at 1× and the parity of smoothing, which the owner judged at Step 5's Gate C.

---

## RV1-11. Concurrency checks without ThreadSanitizer

**Medium** · Toolchain · Blocks M1's plan (MA7, MA10) · Focus 5 · Challenges TS11, MA7, MA10; S9-Q2; ADR-0021's risk row

**The claim.**
- TS11: "ThreadSanitizer for the concurrent parts: the event loop's channels, IPC, the compositor's threads, the collector's helpers."
- MA7 makes it a criterion, "or recorded as blocked".
- MA10 asks for "a pinned nightly … so that ThreadSanitizer works on macOS 27".

**Evidence.**
- **Background: what TSan can see.** Safe Rust rules out data races at compile time. ThreadSanitizer finds data races in the code it instruments. It doesn't find logic races: wrong orderings, deadlocks, lost wakeups.
- **Inference: where it matters in Axiom.** TSan's distinct value is in `unsafe` code (`platform`, later `jit`), in memory shared with the OS or with C (AWS-LC, SQLite if used), and in FFI callbacks. It doesn't treat Mach messages or kqueue events as synchronization, so hand-offs over Mach ports would need annotations that safe code can't give.
- **Measured (S9): the nightly tried crashes.** The installed nightly (1.96.0, 2026-04-08) crashes before `main` on macOS 27.2, while Apple clang 21's runtime works ([S9 results](../../research/s9-results.md#4-s9-q2-threadsanitizer)). S9 tried only that nightly, which predates macOS 27 (inference).
- **Sourced: another runtime can be linked.** rustc has `-Z external-clangrt` (pull request 121207, merged 2024-03-15). It stops rustc linking its own sanitizer runtime, so that another can be supplied [R1].
- **Sourced: the safe-Rust tools.**
  - **Miri** detects "data races and emulation of some weak memory effects", tests ranges of seeds in parallel, and "has no access to most platform-specific APIs or FFI" [R2].
  - **loom** "runs a test many times, permuting the possible concurrent executions of that test under the C11 memory model" [R3].
  - **Shuttle** implements randomized schedulers, PCT among them, and "scales to much larger test cases than Loom" [R4].
- **Observed: Axiom has a scheduler to explore.** EL24 gives a deterministic scheduler that records and replays its order.

**Counter-proposal.** TS11 becomes an ordered strategy:
1. **Design for few primitives.** State crosses threads only through a short list: the channels, EL11's frozen lending, the interrupt flag, `SharedArrayBuffer`'s atomics. Each has one implementation, in one crate.
2. **loom on each primitive, and Miri with many seeds** on every crate Miri can run.
3. **Explore schedules on EL24:** random and PCT-style orderings of event loops and channel deliveries, as Shuttle does, for logic races between agents, the browser's core and the network process.
4. **TSan for `platform` and the C dependencies,** by the first route that works:
   - a current nightly, rechecked at M1's start;
   - `-Z external-clangrt`, linking the Command Line Tools' runtime. This carries an ABI risk between LLVM versions, so it needs spike S12;
   - an aarch64 Linux virtual machine, or hosted runners (TS17), for the crates that don't depend on macOS.
5. **MA7 then requires 1–3,** and 4 where a route works, recorded either way.

**Cost now and later.** Now, an amended TS11. Later, a criterion that stays "blocked" while the concurrency it was meant to check goes untested.

---

## RV1-12. Corpus targets before their capabilities

**Medium** · Roadmap · Blocks M4 (re-placing the targets) and M7 · Challenges MS6, the [roadmap](../../roadmap.md#5-corpus-targets) §4–§5, the [corpus](../../corpus.md#2-the-sites) §2 and §10, SB6

**The claim.** MS6: "A page becomes an acceptance target at the first milestone that has every capability it needs".

**Evidence** (observed in the documents, unless marked).

| Page | Target | Needs, as the corpus records them | Arrives at |
|---|---|---|---|
| Twitch (row 7) | M7 | "chat over WebSockets" | WebSockets: M8 |
| Instagram (row 3) | M4 | "A feed of images and video built by script" | Media: M7 |
| Reddit (row 4) | M4 | "interactive posts in frames", on r/RedditGames | Games in cross-site frames may need WebGL or WebAssembly (inference): M9 |

- **Observed: the eight pages measured by hand have no recorded features.** "`chrome://histograms` wasn't saved, so they stay inference" (S8 §7.5; the corpus §10).
- **Inference:** Google Search, Instagram and Reddit likely use dedicated workers, service workers or WebSockets, which arrive at M8. That should be measured, not assumed.

**Counter-proposal.**
1. **Measure by hand.** The owner saves `chrome://histograms/Blink.UseCounter` after each of the eight pages: the corpus §10's optional step. It reads the browser's own counters, not page content.
2. **Re-place each target by MS6** against the measured features, stating which parts each target excludes, for example "Instagram's feed, without video, until M7".
3. **Move WebSockets, and dedicated workers if measured, to M4.** Both are small next to M4's scope, and they unlock several targets.

---

## RV1-13. WebAssembly and the JIT, decided late

**Medium** · Architecture · Blocks M3 · Challenges OQ-3, JE30, JE31, J1, J2, J6, SH13, the roadmap §7 (MS9)

**The claim.** OQ-3 (WebAssembly) is decided at M8's gate, for M9. J1's decision on a JIT likewise waits for M8's gate, "on the applications' measurements".

**Evidence.**
- **Observed: WebAssembly touches the heap.** JE31: WasmGC's "structs and arrays in the same collected heap, which an embedded engine with its own collector would have to share". J6: "A WebAssembly compiler counts as a JIT". S8 saw Maps and VS Code use WebAssembly, and the corpus says Figma needs it.
- **Inference: either answer touches M3's work.** An embedded engine with its own collector brings back references between heaps, the problem ADR-0001 R2 removed. An engine of Axiom's own needs WasmGC's types as arena kinds, and Wasm frames in the register-file model. Both are questions about the object model and the heap, which M3 settles.
- **Inference: Figma needs a compiler.** Compiled C++ run in a Wasm interpreter is commonly an order of magnitude slower than compiled Wasm (background; to be measured). So a Wasm compiler, which J6 treats as a JIT, is in practice required for M9's targets.
- **Observed: the JIT decision comes after the pages that need it.** J2 allows a JIT from L2 (M5). M4–M6 put script-heavy content pages (Google Search, Instagram, Reddit) on the interpreter. Their evidence starts at M3 ("The interpreter's measurements on corpus scripts start the evidence J1 asks for"), but the decision waits for M8.

**Counter-proposal.**
1. **Decide OQ-3's architecture before M3,** not its implementation: an engine of Axiom's own or an embedded one; WasmGC's objects as arena kinds; Wasm frames and the register file; the tiering policy under J6.
2. **Move J1's decision to M5's gate,** the first L2 milestone, on M3's and M4's measurements of the content pages.
3. **Spike S14:** a compute kernel, such as a C++ benchmark compiled to Wasm, run in a Rust interpreter (`wasmi`) and under Cranelift (Wasmtime) on this Mac, to size the gap.

---

## RV1-14. Site compatibility

**Medium** · Standards · Blocks M4 (it affects live corpus loads from M1) · Challenges the [standards](../../standards.md) baseline, which has no compatibility section; SB4–SB6; the charter's non-goal on compatibility

**The claim.** The charter: "behaviour outside the specs is added only when the target corpus needs it". **Observed:** no document decides:
- the `User-Agent` string or `navigator.userAgent`;
- User-Agent Client Hints (`Sec-CH-UA`, `navigator.userAgentData`);
- how per-site workarounds would be held.

A search of `docs/` finds a user agent only as the census's identification token (CO4).

**Evidence.**
- **Sourced: WebKit carries site-specific quirks,** among them per-site user-agent overrides and a quirk that adds a Chrome compatibility token for listed sites (`Quirks.cpp`, `needsCustomUserAgentOverride`) [W5].
- **Sourced: Firefox ships a webcompat system add-on,** with interventions, shims and injections (`browser/extensions/webcompat/`) [F1].
- **Sourced: Figma lists supported browsers** by name and minimum version: Chrome 120, Firefox 128 ESR, Safari 17.4, Edge 121 [FG].
- **Inference: the consequence.** An engine with an unknown user agent meets sites' unsupported-browser branches. The decision affects every live corpus check, from M1's live loads of Wikipedia and MDN onward.

**Counter-proposal: a compatibility section in the standards baseline.**
1. **A user-agent policy:** a Safari-compatible string with an Axiom token, frozen per milestone. No Client Hints until a corpus page needs them.
2. **An intervention mechanism outside the core:** a data file of per-site overrides (user agent only, at first). Each entry carries its corpus evidence and a condition for removing it, and none lives in engine code paths. They are listed under SB5 as approximate behaviour.

---

## RV1-15. Diagnostics for one developer

**Medium** · Program · Blocks M1 · Challenges JE27, EM9; the handover's items 28 and 30

**The claim.** JE27: debugging comes "later", and "the developer tools protocol and its process boundary are decided at that milestone". The [handover](../../handover-phase-0.md#11-areas-flagged-for-critical-review)'s items 28 and 30 raise developer tools, and "crash reporting and logs on a single-user machine".

**Evidence.**
- **Observed: nothing is designed.** No document defines a DOM, style or layout inspector, a console sink, a network log, or a local path for crash reports.
- **Inference: diagnosis dominates milestone work.** Diagnosing why a real page renders wrongly is most of the work of MA5's readability checks and TS12's corpus checks. Without dumps, each investigation rebuilds its tooling.

**Counter-proposal.**
- **M1:** the headless embedder (EM9) and a test-only command dump, as text in stable formats that tests can use: the DOM; a node's computed styles; the box and fragment trees; the display list; the network log.
- **M2:** a crashed helper's report is moved into a local directory the owner controls, with the build identifier (IP18), as S9's script already does for test crashes.
- **M3:** console messages go to the embedder's log and the test runners.
- **Later:** a developer-tools protocol (JE27), with its process boundary.

---

## RV1-16. Failure semantics: panics and overflow

**Medium** · Robustness · Blocks M1 · Challenges [ADR-0002](../../adr/0002-implementation-language.md), [ADR-0003](../../adr/0003-gc-handle-and-rooting-pattern.md), IP11, IR3, IR10; the workspace's build profiles

**Evidence.**
- **Observed: one panic rule.** The only rule on panics is IP11's "Validation never panics in the browser". ADR-0003 says that under Pattern B "a bug is a stale index: a bounds panic".
- **Observed: no release settings.** The workspace's `Cargo.toml` sets neither `panic` nor `overflow-checks` for release builds. **Background:** Rust's defaults unwind on a panic, and let integer overflow wrap in release builds.
- **Inference: unwinding.** A panic on one agent's thread, in a renderer that hosts several agents (SI3), leaves the process running with that agent half torn down.
- **Inference: wrapping.** A length computed in a parser or a validator wraps silently in release builds, while debug builds and fuzzing, with overflow checks on, panic.

**Counter-proposal: a short section on failure semantics,** in ADR-0002's decision history or the overview.
1. **`panic = "abort"` for every helper executable:** a panic is a crash, and IR10 and FT13 handle it.
2. **In the browser:**
   - message handling and validation are panic-free by construction: no indexing or `unwrap` on decoded values, with Clippy's `indexing_slicing`, `unwrap_used` and `arithmetic_side_effects` denied in validators;
   - they are fuzzed for panics (V2);
   - the browser itself aborts on a panic, since its state can't be trusted after one.
3. **Overflow checks on in release** for parsers, decoders and validators, or checked arithmetic enforced by lint, with the cost measured.
4. **At L0,** a panic on an agent's thread ends the process, as at L1. No `catch_unwind` recovery hides defects.

---

## RV1-17. The signing identity

**Medium** · Platform · Blocks M1 (the signing script, MA10) · Challenges ADR-0004 D2; the process model §4.1 and §10; TS15; the roadmap M6 (the Keychain)

**Evidence.**
- **Sourced: macOS can't track ad-hoc code across rebuilds.** Apple's TN3127 (revised 2024-04-02): "Ad hoc signed code … has a DR but it's tied to that specific version of the code. In both cases macOS can't reliably track the identity of the code." Its symptoms: "the system fails to remember your choices during development" for privacy-protected resources such as the microphone, and the keychain "presents unexpected authorization alerts" [A1].
- **Observed: Axiom rebuilds often, and plans to use both.** The browser and its helpers are signed ad hoc (D2). TS15 rebuilds whenever an advisory affects a crate that handles untrusted input. The process model plans entitlements for the camera and microphone, and the Keychain for secrets (§4.1, §10). M6 decides password saving "in the Keychain".
- **Inference: the result.** Every rebuild asks again for the camera, the microphone, the local network and other protected resources. The Keychain asks again for the browser's own items. The owner learns to click "Always Allow", a habit a browser for sensitive use shouldn't train.

**Counter-proposal: sign with a stable local identity.**
- **The identity:** a self-signed code-signing certificate created on this Mac (no paid account), with an explicit designated requirement (identifier and certificate), so privacy decisions and Keychain access persist across builds.
- **Check before M1,** by S4's method, that the hardened runtime's entitlements (`allow-jit`, `single-jit`) and the custom profiles behave as they do under ad-hoc signing: spike S15 ([actions](04-round-2-actions.md#4-proposed-spikes)).
- **Record how its private key is kept:** in the login keychain, never in the repository.

---

## RV1-18. Build cost of generated code

**Medium** · Toolchain · Blocks M1 (style's generated property code) and M3 (the bindings) · Challenges T1–T4, PF18, WB22, style's property table (ST); ADR-0002's revisit condition 3

**Evidence.**
- **Sourced: Servo's experience.** Servo, 2024-10-03 [S1]:
  - "script is also a monolith, with over 170 000 lines of hand-written Rust plus another 520 000 lines of generated Rust";
  - "it has long dominated Servo's build times to the point of being unwieldy";
  - splitting it cut "a typical Linux-only build from over half an hour to under 8 minutes".
- **Observed: Axiom's thresholds, on 8 GB.** T2 targets a clean build in 5 minutes, and T3 3 GiB of summed compiler and linker memory, on an 8 GB machine (ADR-0002). PF18 counts the generator's output in all four thresholds.
- **Observed: the measurement waits for the generator.** WB22 says "Step 6 measures the generated code's size, its build time", but no measurement exists yet, because there is no generator.
- **Inference: the two largest generated crates arrive before their measurements.** They are the bindings for several hundred interfaces and style's property table.

**Counter-proposal.**
1. **Design the crate topology for compile time now:**
   - the bindings split by interface group into several crates;
   - table-driven glue, static descriptors read by a shared runtime, over per-member functions wherever behaviour allows (WB22 already leans this way);
   - `split-debuginfo = "unpacked"`;
   - the linker chosen and recorded.
2. **Spike S13 before M3:** generate bindings for webref's IDL with stub implementations, and measure T1–T4. Measure style's property code at M1's first checkpoint.

---

## RV1-19. Rendering-timing side channels

**Low** · Security · Blocks M5 (cross-site frames with logins) · Challenges the [threat model](../../security/threat-model.md#72-non-goals) §7.2 (NG4) and §9; PL6, PL9; SI3

**Evidence.**
- **Sourced: GPU.zip.** "Chrome is vulnerable to the pixel stealing attack", because it lets cross-origin iframes load with cookies, renders SVG filters on iframes, and delegates rendering to the GPU. "Other browsers, like Firefox and Safari, do not meet all these criteria and are therefore not vulnerable" (IEEE Symposium on Security and Privacy, 2024) [G1].
- **Observed: Axiom meets two of the criteria.** Axiom blocks third-party cookies (NW11), Safari's position on the first criterion. But its compositor applies a parent's effect nodes (filters, blend modes, opacity) to a child frame's output (PL6, PL9).
- **Observed: the threat model doesn't cover it.** NG4 excludes "cross-process microarchitectural attacks", not rendering-timing attacks across frames.
- **Inference: credentials can still reach the child.** Same-site cross-origin frames carry cookies, and a Storage Access API grant (the process model §5) would give credentials back to cross-site frames.

**Counter-proposal.**
- **A threat-model entry** for rendering-timing attacks across frames (pixel stealing).
- **A rule in PL9:** data-dependent effects (SVG filters, `backdrop-filter`, blend modes) are not applied to a cross-origin child frame's pixels, or only in forms whose cost doesn't depend on the pixels.
- **A test under V4.**

---

## RV1-20. A stale fact: rustls and ECH

**Low** · Networking · Blocks M5's plan (DNS over HTTPS and ECH) · Challenges the networking design's §10 question 4 and §8; the handover §4.9

**Evidence.**
- **Observed: the documents wait for rustls.**
  - The [networking](../../architecture/networking.md#10-open-questions) design's question 4: "Encrypted Client Hello, once the client side is available in rustls and the DNS records are deployed";
  - the handover §4.9: "Encrypted Client Hello, once rustls has the client side".
- **Sourced: rustls already has it.** On 2024-06-13 the Rustls project announced "experimental support for client-side Encrypted Client Hello (ECH)" [M1].

**Counter-proposal.**
- **Correct both documents.**
- **Plan ECH with DNS over HTTPS** at M5's plan. ECH's keys come from DNS records, and the server's name stays private only if the DNS lookup is private too (background).

---

## RV1-21. Passkeys without the restricted entitlement

**Low** · Security · Blocks M6 · Challenges OQ-16; the threat model §9

**Evidence.**
- **Observed: OQ-16's current analysis.** The platform authenticator probably needs `com.apple.developer.web-browser.public-key-credential`, which ad-hoc signing can't claim. Its candidates are "roaming security keys over USB HID, or obtaining the entitlement".
- **Sourced: Chromium implements the hybrid transport itself.** The hybrid transport (a phone as the authenticator, caBLE version 2) lives in `device/fido/cable/`, with `v2_handshake`, `v2_discovery`, `fido_tunnel_device` and `noise` [C4].
- **Inference: two paths need no restricted entitlement.** Hybrid needs Bluetooth Low Energy through CoreBluetooth, behind a privacy prompt rather than a restricted entitlement (to verify), and the tunnel service. With USB HID, that makes two entitlement-free paths to passkey-only accounts.

**Counter-proposal.**
- **Add hybrid to OQ-16's candidates.**
- **Check feasibility before M6:** CoreBluetooth under the browser's signing identity (RV1-17).

---

## RV1-22. Daily-use gaps and an orphaned decoder

**Low** · Product · Blocks M6, the owner's daily use · Challenges the [standards](../../standards.md#53-not-planned) §5.3 (not planned); the handover's items 27–29; ADR-0012's risk row; IM2

**Evidence.**
- **Observed: daily-use features have no milestone.**
  - No design plans a PDF viewer, and printing comes after M9.
  - Downloads, find in page, zoom and session restore have no milestone (handover item 29).
  - WebRTC is "staged" with no milestone (item 27).
- **Observed: JPEG's decoder is unmaintained.** `jpeg-decoder` is "in maintenance mode" (the [register](../../technology/dependency-register.md)). ADR-0012's mitigation is "a JPEG decoder of our own (IM2)".

**Counter-proposal.**
- **PDFs:** open in Preview, downloaded and quarantined. In-engine viewing goes on the not-planned list, since an in-engine PDF parser would be a large new attack surface (inference).
- **A milestone for each daily feature** the owner needs at L3 (downloads, find in page, zoom, session restore), probably M5–M6.
- **WebRTC:** not planned until a corpus page needs it.
- **JPEG:** decide at M1 between Axiom's own decoder, which adds scope (RV1-08), and grading an alternative.

---

## Sources

All read on 2026-10-21. Repository documents are linked where they are cited, and observations name their files or commands.

- [A1] Apple, TN3127 "Inside Code Signing: Requirements", first published 2022-05-03, revised 2024-04-02 — https://developer.apple.com/documentation/technotes/tn3127-inside-code-signing-requirements
- [C1] Chromium, `sandbox/policy/mac/renderer.sb` (main) — https://github.com/chromium/chromium/blob/main/sandbox/policy/mac/renderer.sb
- [C2] Chromium, `services/viz/README.md` (main) — https://github.com/chromium/chromium/blob/main/services/viz/README.md
- [C3] Chromium, `third_party/blink/renderer/platform/fonts/shaping/harfbuzz_face.cc` (main): on Apple platforms, `HarfBuzzSkiaFontFuncs::GetFunctions` returns HarfBuzz's own advances only for a font with a `trak` table and no `sbix` table — https://github.com/chromium/chromium/blob/main/third_party/blink/renderer/platform/fonts/shaping/harfbuzz_face.cc
- [C4] Chromium, `device/fido/cable/` (main), listed through GitHub's API — https://github.com/chromium/chromium/tree/main/device/fido/cable
- [F1] Firefox, `browser/extensions/webcompat/` (main), listed through GitHub's API — https://github.com/mozilla-firefox/firefox/tree/main/browser/extensions/webcompat
- [FG] Figma Help Center, "What are the system requirements for Figma?" (undated) — https://help.figma.com/hc/en-us/articles/360039827194
- [G1] Y. Wang, R. Paccagnella, Z. Gang, W. R. Vasquez, D. Kohlbrenner, H. Shacham, C. Fletcher, "GPU.zip: On the Side-Channel Implications of Hardware-Based Graphical Data Compression", IEEE Symposium on Security and Privacy 2024; project page — https://www.hertzbleed.com/gpu.zip/
- [L1] Ladybird, newsletter, 2026-05-31 — https://ladybird.org/newsletter/2026-05-31
- [L2] Ladybird, newsletter, 2026-06-30, as the [roadmap](../../roadmap.md) cites it ([L3] there) — https://ladybird.org/newsletter/2026-06-30
- [M1] Prossimo (ISRG), "Encrypted Client Hello (ECH) Support for Rustls", 2024-06-13 — https://memorysafety.org/blog/rustls-ech-support
- [R1] rust-lang/rust pull request 121207, "Add `-Z external-clangrt`", merged 2024-03-15 — https://github.com/rust-lang/rust/pull/121207 ; The Unstable Book, `external-clangrt` — https://doc.rust-lang.org/unstable-book/compiler-flags/external-clangrt.html
- [R2] Miri, `README.md` (master) — https://github.com/rust-lang/miri
- [R3] loom, `README.md` (master) — https://github.com/tokio-rs/loom
- [R4] Shuttle, `README.md` (main) — https://github.com/awslabs/shuttle
- [S1] Servo, "This month in Servo", 2024-10-03 — https://servo.org/blog/2024/10/03/this-month-in-servo/
- [V1] U. Degenbaev, M. Lippautz, H. Payer, "Concurrent marking in V8", V8 blog, 2018-06-11 — https://v8.dev/blog/concurrent-marking
- [V2] V8, `include/cppgc/README.md` (Oilpan; main) — https://github.com/v8/v8/blob/main/include/cppgc/README.md
- [W1] WebKit bug 254725, "Enable GPU Process and UI side compositing on macOS", reported 2023-03-29, fixed 2023-04-05 (262629@main) — https://bugs.webkit.org/show_bug.cgi?id=254725
- [W2] WebKit bug 275564, "GPU Process: DOM Rendering SoundCloud", comment 1, 2024-06-17 — https://bugs.webkit.org/show_bug.cgi?id=275564
- [W3] WebKit bug 219368, "GPU Process: IOSurfaces should not be mapped into the Web Content Process", reported 2020-11-30, resolved fixed — https://bugs.webkit.org/show_bug.cgi?id=219368
- [W4] WebKit, `Source/WebKit/WebProcess/com.apple.WebProcess.sb.in` (main) — https://github.com/WebKit/WebKit/blob/main/Source/WebKit/WebProcess/com.apple.WebProcess.sb.in
- [W5] WebKit, `Source/WebCore/page/Quirks.cpp` (main) — https://github.com/WebKit/WebKit/blob/main/Source/WebCore/page/Quirks.cpp
- [W6] F. Pizlo, "Introducing Riptide: WebKit's Retreating Wavefront Concurrent Garbage Collector", WebKit blog, 2017-01-20 — https://webkit.org/blog/7122/
- **Local:** WPT at `0f26972422` and its manifest, from S9, under `target/s9/` (not committed); S8's raw census records under `target/s8-results/` (not committed); the workspace's `Cargo.toml`
