# ADR-0008: Script, agents and the heap

| | |
|---|---|
| **Status** | **Accepted** (2026-10-18), by the owner at Step 4's Gate C, once the S3 addendum's formal measurements were complete ([S3 results](../research/s3-results.md)). **SH10 amended** at Step 5's Gate C (2026-10-20): compaction skips the DOM's node arena ([HTML and the DOM](../architecture/html-and-dom.md) HD11) |
| **Date** | 2026-10-17 |
| **Deciders** | Project owner (only the owner can move an ADR to Accepted) |
| **Related** | [Script and heap](../architecture/script-and-heap.md) (SH1–SH14); [ADR-0003](0003-gc-handle-and-rooting-pattern.md) (C1–C5); [ADR-0004](0004-process-model-and-sandbox.md) (J1–J7); [S1 results](../research/s1-results.md); [S3 results](../research/s3-results.md); [Step 4 tracker](../research/step-4-architecture-proposal.md) §10 (spike S3's design); OQ-5, OQ-13, OQ-14 |

## Decision history

| Date | Event |
|---|---|
| 2026-10-17 | Proposed at Step 4, CP12, with spike S3's design for the owner's authorization |
| 2026-10-17 | **Owner's Gate B decision:** "Pre-approved; will move to Accepted upon S3-Q1 and S3-Q2 passing." Spike S3 is "Authorized for the full scope (questions S3-Q1 through S3-Q5 under spikes/s3/)", and S3-Q2's threshold is "Confirmed at ≤10% of runtime spent rooting" |
| 2026-10-18 | **Spike S3's results** ([S3 results](../research/s3-results.md)).<br>• **S3-Q2 passes.** Executing bytecode performs no root operations, and with C1's stack rooting takes 0.9–1.1% of the callback-heavy workload, against a 10% threshold.<br>• **S3-Q1 passes for reads** (0.51 ns, Miri-clean) but **not for pushes as written.** By S1's method a push costs 2.03 ns, the same as S1's stack: the method imposes a store-to-load round trip on every stack. Back to back, a push costs 0.56 ns.<br>• **The pre-approved condition is not met,** so the ADR stays Proposed for the owner's decision at Gate C.<br>• Decisions 5–7 and 10 now carry S3's evidence; C2 names struct-of-arrays link columns |
| 2026-10-18 | **Owner's Gate C decisions.**<br>• "S3 Addendum Checkpoint: Authorized." It adds "the length-first safe `SliceStack`" to `s3-gc`, measures it formally, and corrects "the S3-Q1 verdict".<br>• "C1 Implementation: Adopt `SliceStack` (Safe Contiguous Stack). The `gc` crate remains strictly `#![forbid(unsafe_code)]`. A 19% dispatch improvement (628 ns vs 779 ns) with zero `unsafe` proves that hardware-aligned data layout beats manual memory micro-optimizations. Keep `RawStack` archived in spikes/s3/ as benchmark history."<br>• "We will formally accept ADR-0008 and merge `step-4` into `main` (with a subsequent push to origin) immediately upon completion of the formal addendum measurements." |
| 2026-10-18 | **The S3 addendum's results** (CP16; [S3 results](../research/s3-results.md) §3).<br>• `SliceStack` reads in 0.53 ns and has no `unsafe`. By S1's method a push costs 2.12 ns, as on every stack, so S3-Q1 still fails as written.<br>• **The explanation recorded above was wrong.** S1's method does force each push to reload the length, but the cost of that reload depends on how the loads and stores around it compile. Reloading the length in one paired load with a neighbouring field costs about 2 ns. The same `SliceStack` push measured 1.02 ns in an informal probe whose harness compiled differently.<br>• **In S1's dispatch case,** formally and run by run, `SliceStack` is 13–17% faster than `RawStack`, and 14–20% faster than its field-order control. The 19% quoted above came from the informal probe |
| 2026-10-18 | **Accepted,** the owner's condition met: the addendum's formal measurements are complete. Decision 5 names `SliceStack` |
| 2026-10-19 | **Amendment proposed, for the owner at Step 5's Gate C** ([HTML and the DOM](../architecture/html-and-dom.md) HD11): the mark–compact collector of SH10 compacts every arena except the DOM's node arena, whose slots never move and are reused with a new generation. Reason: PL2 and PL12 make a node's identity (handle and generation) stable for its life, and AX2 sends it to the browser, so a moving node would invalidate identities held outside the heap. Until the owner decides, SH10 stands as written |
| 2026-10-20 | **Amendment accepted** by the owner at Step 5's Gate C: "ADR-0008 Amendment (HD11): Accepted. Node slots in the arena never move; node identity remains stable." SH10 is amended in [script and heap](../architecture/script-and-heap.md), and decision 8 records it |

## Question

How do agents, realms and heaps fit together, and how does data cross between agents (OQ-5)? How does script integrate with the event loop? How are ADR-0003's obligations met: C1, rooting; C2, layout; C3, the barrier API; C4, the collection plan; C5, native interop (OQ-13, OQ-14)?

## Requirements and constraints

- **ADR-0001 R2:** one heap for JavaScript and the DOM.
- **ADR-0003:** Pattern B, and C1–C5 settled in Step 4 "before the JS object model is fixed". S3 is required.
- **SI7, EL12, EL15:** agent clusters within one process; one event loop and thread per agent; `SharedArrayBuffer` as the only shared mutable memory.
- **J1–J7:** interpreter first.

## Alternatives considered

- Wrapper objects joining two heaps.
- Conservative stack scanning.
- Choosing incremental or generational collection now.
- Keeping S1's `RefCell` root stack.

Each is assessed in [script and heap](../architecture/script-and-heap.md) §5.

## Historical context and modern developments

- V8 combines a generational scavenger with incremental and concurrent marking, using write barriers, and runs idle-time tasks: "45%" less JavaScript heap on an idle Gmail.
- SpiderMonkey is incremental and generational, and compacts rarely.
- S1 measured Pattern B's rooting at 2.05 ns per root and 1.58 ns per read.

All sourced in [script and heap](../architecture/script-and-heap.md) §2.

## Analysis

See [script and heap](../architecture/script-and-heap.md) §3. **C1 is the one obligation that design alone can't settle:** whether a root stack without borrow tracking reaches about 1 ns is an empirical question, which spike S3 answers.

**Spike S3's answer** ([S3 results](../research/s3-results.md) §3–§7):
- **C1.** Without borrow tracking, a rooted value is read in about 0.5 ns, and a frame opened and closed in 0.55–0.77 ns. By S1's method a push costs about 2.1 ns on every stack, C1's `SliceStack` included; the addendum showed that this figure depends on how the loads and stores around the push compile. Pushed back to back, as a builtin roots its arguments, a push costs 0.58 ns. In S1's dispatch case `SliceStack` is the fastest stack, 13–17% faster than `RawStack`. Rooting takes about 1% of the callback-heavy workload, and bytecode roots nothing.
- **C2.** One-based struct-of-arrays link columns traverse faster than both of S1's patterns in every case. Power-of-two strides cost up to 22% once the nodes outgrow the cache.
- **C3.** Generated setters cost nothing without a barrier; card marking costs about 3% on mutation-heavy work.
- **Dispatch** (J1's evidence): a `match` loop is fastest on stable Rust; explicit tail calls are faster again on nightly, where the feature is still flagged incomplete.

## Evidence and sources

[Script and heap](../architecture/script-and-heap.md) Sources; [S1 results](../research/s1-results.md) §5; [S3 results](../research/s3-results.md); SDK 27 `os/os_sync_wait_on_address.h`.

## Decision

1. **One heap per agent, on its event loop's thread.** DOM objects are the JavaScript objects, with no wrappers (SH1).
2. **Between agents, only copies, transfers and shared buffers.** Backing stores live outside the heap, and `Atomics.wait` uses `os_sync_wait_on_address` (SH2–SH3).
3. **Script runs only in tasks and microtasks.** An atomic interrupt flag can stop it (SH4–SH5).
4. **The root set:** registers and frames traced as regions; native holds on the root stack; the agent's own roots (SH6).
5. **C1: `SliceStack`, a root stack without borrow tracking, with 8-byte slots** (SH7). It is one contiguous array of `Cell` slots, borrowed from a zeroed buffer that the agent owns, with its length first. It has no `unsafe`, so **the `gc` crate is `#![forbid(unsafe_code)]`** (the owner's decision at Gate C). It reads in 0.53 ns and pushes in 0.58 ns back to back, or 2.12 ns by S1's method, and it is the fastest stack in S1's dispatch case. `RawStack`, whose `unsafe` was three Miri-checked blocks, stays in the spike as benchmark history.
6. **C2: one-based struct-of-arrays `u32` columns for hot link fields** (SH8), the fastest layout in every case S3 measured.
7. **C3:** generated setters for every handle-valued field, the single place for any future barrier (SH9). S3 measured them as free without a barrier, and card marking at about 3%.
8. **C4:** stop-the-world mark–compact now. Then incremental marking in idle periods and a generational nursery, ordered by corpus measurements, with the pause budget from Step 6 (SH10). **As amended at Step 5's Gate C** (HD11): compaction skips the DOM's node arena, whose slots never move, so a node's identity stays stable for its life.
9. **C5:** memory that reaches platform APIs lives outside the heap and never moves; mark-worklist entries are 8 bytes (SH11).
10. **Generated, safe bindings.** Room for a JIT under J1–J7 (SH12–SH13). S3-Q3's evidence for the interpreter: a `match` loop on stable Rust, with operations written as functions over the machine state so that the technique can change. Heaps are budgeted (SH14).

## Rationale

- These choices complete ADR-0003 without changing it.
- They keep every heap on one thread (AC5).
- They leave the collector's evolution to measurement rather than guesswork (AC10).

## Assumptions

- Most rooting traffic comes from native builtins, not from executing bytecode. ADR-0003 stated this. **S3-Q2 confirmed it:** bytecode performed no root operations in any workload.
- Generated setters add no measurable cost while the barrier is a no-op. **S3-Q5 confirmed it** (1.26 against 1.28 ns per write).

## Risks and unresolved issues

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| C1's push misses its target, as S1's method measures it | Occurred (S3 and its addendum) | Low | Rooting is about 1% of interpreter time with C1's stack, reads meet the target, and pushes back to back cost 0.58 ns. S1's figure depends on how the code around the push compiles. Accepted at Gate C |
| A compiler change, or different code around the stack, undoes the field-order gain | Medium | Low | The engine keeps S1's dispatch case, or a successor, as a benchmark ([S3 results](../research/s3-results.md) §9) |
| Stop-the-world pauses on large heaps | High (for large apps) | Medium | SH10's plan, with SH9's barrier point already in place |
| The `unsafe` in the root stack hides a soundness bug | Retired at Gate C | — | C1's stack has no `unsafe`, and the `gc` crate forbids it |

## Conditions for revisiting

- Rooting comes to dominate an interpreter-shaped workload. S3-Q2 measured it at about 1%. S3-Q1's push criterion did not pass as written, and the owner accepted this ADR with that result at Gate C ([S3 results](../research/s3-results.md) §3.5).
- Agents need to share GC objects across threads, contrary to ADR-0003's assumption.

## Consequences

- **Accepted at Gate C** (2026-10-18), with S3-Q2 passed and S3-Q1's push criterion not met as written.
- **The `gc` crate has no `unsafe`** (`#![forbid(unsafe_code)]`): C1's stack is safe.
- OQ-5, OQ-14 and OQ-13's design part are resolved by this ADR; OQ-13's budget is set in Step 6.
- **SH10 was amended at Step 5's Gate C** (2026-10-20): the node arena is excluded from compaction ([HTML and the DOM](../architecture/html-and-dom.md) HD11; [ADR-0015](0015-html-parsing-and-the-dom.md)). Step 6 measures the arena's density on the corpus.
