# Script, Agents and the Heap

| | |
|---|---|
| **Status** | **Binding** since ADR-0008's acceptance (2026-10-18, Gate C). Written at Step 4, CP10; C1 settled by spike S3 and its addendum (CP16), which the owner chose `SliceStack` from ([S3 results](../research/s3-results.md)). SH10 amended at Step 5's Gate C (2026-10-20; [HTML and the DOM](html-and-dom.md) HD11). **Amended by the architecture review's Round 1** (the owner's dispositions, 2026-10-21; [the review](../review/README.md) §5): SH1 for RV1-03 (CP3); SH10 and SH13, the collector's end-state and the slot width (RV1-04, RV1-06; CP6). **Amended by Round 4** (the owner's dispositions, 2026-10-22; [the review](../review/README.md) §5.9): §7 for RV4-14 (CP7) |
| **Question** | How do agents, realms and heaps fit together; how do messages, transfers and `SharedArrayBuffer` cross between agents; how does script integrate with the event loop; and how does the engine meet ADR-0003's obligations C1–C5: cheaper rooting, heap layout, a write-barrier API, a plan for incremental or generational collection, and native interop? What must spike S3 measure? |
| **Answers** | Step 4 questions Q4 (second half) and Q15 ([tracker](../research/step-4-architecture-proposal.md) §3); OQ-5, OQ-13, OQ-14 |
| **Feeds** | ADR-0008; spike S3 ([tracker](../research/step-4-architecture-proposal.md) §10); Step 5 (the JavaScript engine and bindings, HTML and DOM) |
| **Builds on** | [ADR-0003](../adr/0003-gc-handle-and-rooting-pattern.md) (Pattern B, C1–C5); [ADR-0004](../adr/0004-process-model-and-sandbox.md) (J1–J7); [S1 results](../research/s1-results.md); [event loop and threading](event-loop-and-threading.md) (EL12, EL15–EL17); [IPC](ipc.md); [rendering pipeline](rendering-pipeline.md) (PL2, PL11) |
| **Last updated** | 2026-10-21: Round 1's accepted findings |

Everything here is **planned**; nothing is implemented. Rules are numbered SH1–SH14. Precedent is sourced (§2); the design is inference unless marked otherwise. The JavaScript engine's internals (parser, bytecode set, object shapes, inline caches) belong to Step 5.

## Summary

- **One heap per agent, on its event loop's thread** (SH1). Realms in one agent share its heap. **DOM objects are the JavaScript objects:** one heap, so no wrapper objects and no cross-heap references.
- **Between agents, only copies, transfers and shared buffers** (SH2, SH3).
  - Messages are structured-cloned into the receiving heap.
  - Transfers move ownership of backing stores that live outside the heap.
  - `SharedArrayBuffer` memory is mapped into the cluster's one process, and `Atomics.wait` uses macOS's public wait-on-address primitive.
- **Script runs only inside tasks and microtasks** (SH4, SH5). Promise jobs run in the HTML microtask queue in order, and an atomic interrupt flag lets the browser stop a runaway script.
- **The root set** (SH6) is the interpreter's registers and frames, traced as whole regions; values that native code holds across a possible collection, on the root stack; and the realms' and the event loop's own roots.
- **C1, cheaper rooting** (SH7): `SliceStack`, a root stack without `RefCell` borrow tracking, with 8-byte slots and no `unsafe`. S1 measured 2.05 ns per root and 1.58 ns per read; the target is about 1 ns. **Spike S3 and its addendum measured it:** 0.53 ns per read; 0.58 ns per push back to back, but about 2.1 ns by S1's method on every stack, a figure that depends on how the code around the push compiles. It is the fastest stack in S1's dispatch case. Bytecode roots nothing, and rooting takes about 1% of a callback-heavy workload ([S3 results](../research/s3-results.md)).
- **C2–C5** (SH8–SH11):
  - **C2:** hot link fields are laid out so that no index arithmetic sits in a dependent chain of loads.
  - **C3:** every write of a handle-valued field goes through generated setters, the insertion point for any future barrier.
  - **C4:** version 0 is a stop-the-world mark–compact collector. **As amended in Round 1** (RV1-04), the end-state is fixed before M3, not ordered by later measurement: a generational nursery, incremental marking driven by allocation as well as idle periods, parallel marking in pauses, and non-moving tenured arenas with selective evacuation (SH10), with spike S11's measurements.
  - **C5:** memory handed to platform APIs lives outside the heap and never moves.
- **Bindings are generated, safe and mechanical** (SH12). The value representation leaves room for a JIT under J1–J7 (SH13).

## 1. Binding inputs

| Input | Consequence here |
|---|---|
| **ADR-0001 R2:** our own JavaScript engine; JavaScript and the DOM in one GC heap | SH1, SH12 |
| **ADR-0003:** Pattern B (typed arenas, `u32` handles, scope tokens, root stack, safe `Trace`, mark–compact); obligations C1–C5; spike S3 required in Step 4 | SH6–SH11, S3 |
| **SI7, EL12, EL15:** an agent cluster never spans processes; one event loop and thread per agent; `SharedArrayBuffer` is the only shared mutable memory | SH1–SH3 |
| **EL16–EL17:** idle tasks; no nested event loops; interrupts by flag | SH4, SH5, SH10 |
| **PL2, PL11:** DOM arenas without interior mutability; one string representation | SH1, SH8 |
| **J1–J7:** interpreter first; any JIT only in `renderer-jit`, from L2 | SH13 |

## 2. Precedent

From sources read on 2026-10-17 (Sources).

| Topic | What engines do |
|---|---|
| **Generational collection** | V8 builds on "The Generational Hypothesis": "most objects die young". It uses a semi-space young generation, and survivors of a second collection are "evacuated into the old generation". Parallel scavenging cut young-generation time on the main thread "by about 20%–50%" [V1] |
| **Incremental and concurrent marking** | In V8, "the main thread does a small amount of work intermittently", or marking runs "entirely in the background". "Write barriers are used to keep track of new references between objects that JavaScript creates" [V1]. SpiderMonkey's "incremental collector breaks its execution into a number of small slices"; it has "a single young generation (the nursery) and a single old generation"; and compaction "is not incremental so it only happens rarely" [M1] |
| **Idle-time collection** | V8 posts "Idle Tasks" that run in spare time before the next frame, and reports that they "can reduce Gmail's JavaScript heap memory by 45% when it is idle" [V1] |
| **Rooting** | SpiderMonkey's stack roots use "C++ wrapper classes that must be used for stack roots and handles", and its collector is "precise" [M1]. Nova roots scoped values in a `RefCell`-protected vector (ADR-0003, from its 2026 post) |
| **Axiom's own measurements** | Pattern B's root stack, an `Rc<RefCell<Vec<…>>>`, costs 4.08 ns to open and close a frame, 2.05 ns to root a handle and 1.58 ns to read it, hot in cache. Listener dispatch, which holds many handles across calls that may collect, takes 2.06× Pattern A's time [S1] |

## 3. Design

### 3.1 Agents, realms and messages (OQ-5)

| # | Rule |
|---|---|
| SH1 | **One heap per agent, on its event loop's thread** (ADR-0003, EL15). All realms of an agent share its heap: the same-site frames of one browsing context group, or same-origin ones when clusters are origin-keyed, form one agent with several global objects. **DOM objects are JavaScript objects:** a DOM node's script-visible face is the node itself, a platform object in a typed arena, with no wrapper and no reference into another heap (ADR-0001 R2). References between agents exist only as messages. **As amended in Round 1** (RV1-03): documents of different sites are never one agent, in any process configuration and from M1. A cross-site frame's document always gets an agent, event loop and heap of its own, reached through remote window proxies (WB20) |
| SH2 | **Copies and transfers between agents.** `postMessage` and `MessagePort` payloads are structured-serialized in the sender's heap and deserialized into the receiver's heap, whether the receiver is in this process or another (IP14). **Transferable objects:** an `ArrayBuffer`'s backing store lives outside the heap (SH11), so transferring it within a process moves ownership without copying. Across processes it is copied, unless it was allocated in shared memory. `MessagePort`s become channels that the browser creates (IP4). Later transferables follow the same rule: their bulk state lives outside the heap |
| SH3 | **`SharedArrayBuffer`** memory is allocated outside every heap and mapped into the cluster's one process (SI7). Agents on different threads of that cluster share it (EL12). `Atomics` operations are atomic instructions. **`Atomics.wait` and `notify`** use macOS's public wait-on-address primitive, `os_sync_wait_on_address` and `os_sync_wake_by_address_*`. It exists since macOS 14.4, and its process-local mode suits this case (observed, `os/os_sync_wait_on_address.h`). Only dedicated and shared workers may wait (EL, §4.6) |

### 3.2 Script on the event loop

| # | Rule |
|---|---|
| SH4 | **Script runs only inside tasks and microtasks.** The engine's host hooks (enqueueing promise jobs, calling job callbacks, timeouts) are implemented by the event loop (EL14). Promise jobs go to the HTML microtask queue and run in order. The microtask checkpoint runs after each task and whenever script finishes running, as HTML specifies ([event loop](event-loop-and-threading.md) §2). Nothing re-enters script except the callbacks the specifications define |
| SH5 | **Interrupts are a flag.** The interpreter reads an atomic interrupt flag on loop back-edges and calls: one relaxed load. The renderer's control thread sets the flag when the browser asks it to stop a script (EL17). The flag also lets the browser end an agent's script at navigation and shutdown |

### 3.3 Roots, layout and barriers (C1–C3)

| # | Rule |
|---|---|
| SH6 | **The root set has three parts.** (1) **The interpreter's registers and frames**, traced as whole regions: values are tagged, so tracing them is precise, and executing bytecode roots nothing value by value. (2) **Values native code holds across a possible collection**, on the root stack: builtins, the DOM and the bindings use Pattern B's frames and scoped handles. (3) **The agent's own roots:** each realm's intrinsics, the event loop's queues and timers, module maps. These are held from roots the collector knows |
| SH7 | **C1: a root stack without borrow tracking.** The hypothesis from ADR-0003 is a root stack whose 8-byte slots live in storage without `RefCell` borrow tracking: an `UnsafeCell`-based stack in one allowlisted module of the `gc` crate, with stated invariants, checked by Miri. Pattern B's scope tokens, not runtime borrow checks, already prevent misuse. **Target:** about 1 ns per root and per read, against S1's 2.05 and 1.58 ns. **Validated by spike S3** (S3-Q1, S3-Q2) before the JavaScript object model is fixed. **If it fails,** ADR-0003's first revisit condition applies, and the owner decides. **Decided at Gate C** (2026-10-18): **`SliceStack`**, one contiguous array of `Cell` slots borrowed from a zeroed buffer that the agent owns, with its length first. It needs no `unsafe`, so the `gc` crate is `#![forbid(unsafe_code)]`; the hypothesis's allowlisted module is not needed. **Measured** ([S3 results](../research/s3-results.md) §3–§4, with the CP16 addendum):<br>• `SliceStack` reads in 0.53 ns, opens and closes a frame in 0.55–0.77 ns (code alignment), and pushes in 0.58 ns back to back.<br>• By S1's method a push costs about 2.1 ns on every stack. What that method measures depends on how the loads and stores around the push compile: the same push measured 1.02 ns in another binary.<br>• In S1's dispatch case it is 13–17% faster than `RawStack` (S3's first candidate, three Miri-checked `unsafe` blocks, now archived) and 14–20% faster than the same stack with its length last |
| SH8 | **C2: no index arithmetic in dependent chains.** Hot link fields, such as a node's parent, first child and next sibling, are `u32` columns in a struct-of-arrays layout, indexed from 1 (a dummy element 0 lets a handle's stored index + 1 index directly), so that scaling folds into the load's addressing mode and no subtraction remains. **Measured by S3-Q4** ([S3 results](../research/s3-results.md) §6): the fastest layout in every traversal case, faster than both of S1's patterns, and about three times faster after random moves. Power-of-two strides, the other candidate, cost up to 22% once the nodes outgrow the cache |
| SH9 | **C3: a typed write-barrier API from the start.** A field that holds a handle is private to its type, and is written only through a setter that the type's derived `Trace` support generates. With the stop-the-world collector the setter is a plain store. It is the single place where an incremental barrier (marking the old or new value) or a generational barrier (recording old-to-young references) is later added, with no change at call sites. S1-Q2's ceremony is recounted with the setters in place. **Measured by S3-Q5** ([S3 results](../research/s3-results.md) §7): setters replace writes one for one and cost nothing with the barrier a no-op; card marking adds about 3% to mutation-heavy work |

### 3.4 Collection and interop (C4, C5)

| # | Rule |
|---|---|
| SH10 | **C4: the collection plan.** Version 0 is ADR-0003's stop-the-world mark–compact collector. S1 measured about 4 ms at 130,000 live nodes, and 15 ms at 333,000 objects. Step 6 sets the pause budget. Two additions are then taken in the order measurements on the corpus demand: **incremental marking** in idle periods (EL16), using SH9's barrier, and **a generational nursery** with bump allocation and a remembered set from the same barrier. V8 and SpiderMonkey both combine the two [V1, M1]. Compaction stays a full, rarer step, as in SpiderMonkey [M1]. Collection is also triggered by memory pressure and idle time, and V8's idle tasks show the memory this saves [V1].<br>**Amended at Step 5's Gate C** (2026-10-20; [HTML and the DOM](html-and-dom.md) HD11): **compaction skips the DOM's node arena,** whose slots never move and are reused with a new generation. So the identities of nodes held outside the heap (side tables, the browser's accessibility IDs, hit-test results, messages) stay valid for each node's life. Step 6 measures the arena's density on the corpus; if it degrades, the fallback compacts one document's nodes while remapping its side tables (HD11).<br>**The end-state, fixed in Round 1** (RV1-04; the owner: "Fix the GC end-state before Milestone 3 via an amendment to ADR-0008: generational nursery, allocation-driven incremental marking, parallel marking in pauses, and non-moving tenured arenas with selective evacuation." "Evaluate pause times on a 1M-object heap in Spike S11."). Version 0 stays the first step; the end-state replaces "in the order measurements on the corpus demand":<br>1. **A generational nursery,** with bump allocation and a remembered set written by SH9's setters.<br>2. **Incremental marking driven by allocation** as well as by idle periods: each allocation budget advances marking by a slice, so a page that is never idle, a feed being scrolled or an editor being typed in, still finishes its cycle. EL16's idle tasks stay a second driver.<br>3. **Parallel marking in pauses:** during a pause, helper threads mark arenas the event-loop thread lends them read-only, with mark state in atomic bitmaps outside the arenas (EL11, as amended).<br>4. **Non-moving tenured arenas with selective evacuation:** no whole-heap compaction. Sparse pages are chosen for evacuation, references into them are recorded while marking, and only those are rewritten, so no pause is proportional to the heap. The DOM's node arena never moves (HD11).<br>**Decided with spike S11, before M3:** whether concurrent marking stays possible, which would require arenas of chunks that never move and slot types that can become relaxed atomics; and the barrier's semantics, snapshot-at-the-beginning or incremental update, with black allocation. **S11 measures pauses on a synthetic heap of a million objects** for each strategy against PF15, together with RV1-06's slot width (SH13). The arenas, setters and mark state M3 builds are designed for this end-state, and its parts are built before the milestone whose pages would break PF15 |
| SH11 | **C5: memory that reaches platform APIs lives outside the heap, and never moves.** This covers `ArrayBuffer` and `SharedArrayBuffer` backing stores, canvas pixels, decoded images, glyph atlases (PL7, PL8), and buffers that WebGL or Web Audio hand to the system. These are non-moving, reference-counted allocations, and heap objects refer to them by handle, so compaction never relocates memory that a platform API holds. S2 already found that glyph and pixel buffers need not live in the GC heap. Mark-worklist entries are 8 bytes |

### 3.5 Bindings, the JIT and memory

| # | Rule |
|---|---|
| SH12 | **Bindings are generated, safe and mechanical.** A WebIDL generator (Step 5) emits the glue between JavaScript and DOM types in the same heap: argument conversion, scope tokens, and rooting of what each method holds across calls that may allocate (ADR-0003, consequences). Generated code contains no `unsafe` (ADR-0002 rule 7) |
| SH13 | **Room for a JIT, without building one** (J1–J7). The interpreter is the baseline. The value representation (tagged 64-bit values, handles inside them; **as amended in Round 1,** RV1-06, the width of heap slots is decided with spike S11 before M3, JE9) and the object model leave room for inline caches and a baseline JIT in `renderer-jit`, from L2 and only if J1's evidence calls for it. S3-Q3's dispatch measurements are part of that evidence: a `match` loop is fastest on stable Rust, and explicit tail calls are faster again on nightly ([S3 results](../research/s3-results.md) §5). J7's hypothesis that handle-based heaps help contain JIT bugs is examined if a JIT is ever built |
| SH14 | **Heaps are budgeted.** Each agent's heap grows by policy, triggered by allocation volume and idle periods. Memory pressure triggers a full collection and cache trims ([process model](../security/process-model.md) §7). Step 6 sets the budgets against B2, B3 and B7 |

## 4. Staging

| Level | What changes |
|---|---|
| L0 | All of it. Agents and heaps are per event-loop thread, in renderer processes from M1 (as amended in Round 1, RV1-07; one process only in single-process test mode) |
| L1–L3 | Nothing. Each agent's heap moves with its event loop into a renderer. Cross-process messages are copied, as cross-thread messages in another process would be (SH2) |
| From L2 | A JIT may appear in `renderer-jit` if J1's evidence supports it (SH13) |

## 5. Alternatives considered

| | Alternative | Assessment |
|---|---|---|
| A | **Wrapper objects** that join a DOM heap and a JavaScript heap | Cross-heap cycles were the problem ADR-0001 R2 set out to remove. Rejected for SH1 |
| B | **Conservative stack scanning** for native roots | Rejected in ADR-0003 (it prevents compaction). SH6 keeps rooting precise |
| C | **Choosing incremental or generational collection now** | Premature. The barrier API (SH9) keeps both open, and corpus measurements decide (SH10) |
| D | **Keeping S1's `RefCell` root stack** | Measured at 2.05 and 1.58 ns. It is S3's baseline, kept if the alternative fails to beat it safely. S3: the alternative reads 3.2 times faster and roots back to back 1.6 times faster; by S1's push method they are equal |

## 6. Criteria check

| Criterion | How this design meets it |
|---|---|
| AC5 | One heap per agent, on one thread; only copies, transfers and shared buffers between agents (SH1–SH3) |
| AC8 | Workers, `SharedArrayBuffer`, transfers and Wasm threads, whose memory is a shared buffer (SH3), have their place |
| AC9 | No new `unsafe`: C1's root stack is safe, and the `gc` crate is `#![forbid(unsafe_code)]` (SH7) |
| AC10 | C1 measured by spike S3; C4's order waits for the corpus |

## 7. Open questions

| # | Question | Where it is settled |
|---|---|---|
| 1 | C1: does the root stack without borrow tracking reach about 1 ns per root and per read? | **Settled at Gate C** (2026-10-18): reads yes (0.53 ns); pushes by S1's method no (about 2.1 ns on every stack). ADR-0008 is accepted with `SliceStack` (decision 5) |
| 2 | Which bytecode dispatch technique the interpreter uses | **Answered** at Step 5: a `match` loop (JE6) (Round 4, RV4-14) |
| 3 | The pause budget, and the order of incremental and generational collection | **Answered:** the budget at Step 6 (PF15); the end-state fixed before M3, with spike S11 (RV1-04) (Round 4, RV4-14) |
| 4 | Where shared and service workers run when no same-site renderer exists | **Answered:** in a renderer locked to their site ([event loop](event-loop-and-threading.md) §4.6; SW9) (Round 4, RV4-14) |

## Sources

All checked on 2026-10-17.

- [V1] P. Marshall, "Trash talk: the Orinoco garbage collector", V8 blog (2019-01-03) — https://v8.dev/blog/trash-talk
- [M1] Firefox source docs, "Garbage collection" (SpiderMonkey) — https://firefox-source-docs.mozilla.org/js/gc.html
- [S1] [S1 results](../research/s1-results.md) §5: Pattern B's rooting unit costs and the dispatch suite
- Local: SDK 27 header `os/os_sync_wait_on_address.h`
