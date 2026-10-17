# ADR-0008: Script, agents and the heap

| | |
|---|---|
| **Status** | **Proposed** (2026-10-17). Acceptance was pre-approved at Gate B, effective once spike S3-Q1 and S3-Q2 pass. **Spike S3 (2026-10-18): S3-Q2 passes; S3-Q1 passes for reads, not for pushes as written. The owner decides at Gate C** ([S3 results](../research/s3-results.md)) |
| **Date** | 2026-10-17 |
| **Deciders** | Project owner (only the owner can move an ADR to Accepted) |
| **Related** | [Script and heap](../architecture/script-and-heap.md) (SH1–SH14); [ADR-0003](0003-gc-handle-and-rooting-pattern.md) (C1–C5); [ADR-0004](0004-process-model-and-sandbox.md) (J1–J7); [S1 results](../research/s1-results.md); [S3 results](../research/s3-results.md); [Step 4 tracker](../research/step-4-architecture-proposal.md) §10 (spike S3's design); OQ-5, OQ-13, OQ-14 |

## Decision history

| Date | Event |
|---|---|
| 2026-10-17 | Proposed at Step 4, CP12, with spike S3's design for the owner's authorization |
| 2026-10-17 | **Owner's Gate B decision:** "Pre-approved; will move to Accepted upon S3-Q1 and S3-Q2 passing." Spike S3 is "Authorized for the full scope (questions S3-Q1 through S3-Q5 under spikes/s3/)", and S3-Q2's threshold is "Confirmed at ≤10% of runtime spent rooting" |
| 2026-10-18 | **Spike S3's results** ([S3 results](../research/s3-results.md)).<br>• **S3-Q2 passes.** Executing bytecode performs no root operations, and with C1's stack rooting takes 0.9–1.1% of the callback-heavy workload, against a 10% threshold.<br>• **S3-Q1 passes for reads** (0.51 ns, Miri-clean) but **not for pushes as written.** By S1's method a push costs 2.03 ns, the same as S1's stack: the method imposes a store-to-load round trip on every stack. Back to back, a push costs 0.56 ns.<br>• **The pre-approved condition is not met,** so the ADR stays Proposed for the owner's decision at Gate C.<br>• Decisions 5–7 and 10 now carry S3's evidence; C2 names struct-of-arrays link columns |

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
- **C1.** Without borrow tracking, a rooted value is read in 0.51 ns and a frame opened and closed in 0.75 ns. A push stays at 2.03 ns by S1's method, on every stack, because the method forces a store-to-load round trip on the stack's length. Pushed back to back, as a builtin roots its arguments, it costs 0.56 ns. Rooting takes about 1% of the callback-heavy workload, and bytecode roots nothing.
- **C2.** One-based struct-of-arrays link columns traverse faster than both of S1's patterns in every case. Power-of-two strides cost up to 22% once the nodes outgrow the cache.
- **C3.** Generated setters cost nothing without a barrier; card marking costs about 3% on mutation-heavy work.
- **Dispatch** (J1's evidence): a `match` loop is fastest on stable Rust; explicit tail calls are faster again on nightly, where the feature is still flagged incomplete.

## Evidence and sources

[Script and heap](../architecture/script-and-heap.md) Sources; [S1 results](../research/s1-results.md) §5; [S3 results](../research/s3-results.md); SDK 27 `os/os_sync_wait_on_address.h`.

## Decision (proposed)

1. **One heap per agent, on its event loop's thread.** DOM objects are the JavaScript objects, with no wrappers (SH1).
2. **Between agents, only copies, transfers and shared buffers.** Backing stores live outside the heap, and `Atomics.wait` uses `os_sync_wait_on_address` (SH2–SH3).
3. **Script runs only in tasks and microtasks.** An atomic interrupt flag can stop it (SH4–SH5).
4. **The root set:** registers and frames traced as regions; native holds on the root stack; the agent's own roots (SH6).
5. **C1: a root stack without borrow tracking, with 8-byte slots** (SH7). Spike S3's recommended implementation is `RawStack`: one lazily committed array of `Cell` slots, whose only `unsafe` is three blocks in an allowlisted module, Miri-clean. It reads in 0.51 ns and pushes in 0.56 ns back to back, or 2.03 ns by S1's method. `CellStack`, with no `unsafe`, is the measured fallback: equal on reads, 11% slower on DOM dispatch.
6. **C2: one-based struct-of-arrays `u32` columns for hot link fields** (SH8), the fastest layout in every case S3 measured.
7. **C3:** generated setters for every handle-valued field, the single place for any future barrier (SH9). S3 measured them as free without a barrier, and card marking at about 3%.
8. **C4:** stop-the-world mark–compact now. Then incremental marking in idle periods and a generational nursery, ordered by corpus measurements, with the pause budget from Step 6 (SH10).
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
| C1's push misses its target, as S1's method measures it | Occurred (S3) | Low | Measured: rooting is about 1% of interpreter time with C1's stack, reads meet the target, and pushes back to back cost 0.56 ns. The owner decides at Gate C |
| Stop-the-world pauses on large heaps | High (for large apps) | Medium | SH10's plan, with SH9's barrier point already in place |
| The `unsafe` in the root stack hides a soundness bug | Low | High | One small module; Miri under Stacked and Tree Borrows; review (V5) |

## Conditions for revisiting

- S3-Q1 fails, or S3-Q2 shows rooting dominating an interpreter-shaped workload. S3-Q1's push criterion did not pass as written, and the [S3 results](../research/s3-results.md) §3.3 and §9 are the input to the owner's decision at Gate C. S3-Q2 showed rooting at about 1%.
- Agents need to share GC objects across threads, contrary to ADR-0003's assumption.

## Consequences

- **Acceptance is the owner's decision at Gate C.** S3-Q2 passed; S3-Q1 did not pass as written.
- **The `gc` crate's only `unsafe` is the root stack:** `RawStack`'s three blocks if it is chosen, or none with `CellStack`.
- OQ-5 and OQ-14 resolve with this ADR. OQ-13's design part resolves here; its budget is set in Step 6.
