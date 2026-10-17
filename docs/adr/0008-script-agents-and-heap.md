# ADR-0008: Script, agents and the heap

| | |
|---|---|
| **Status** | **Proposed** (2026-10-17). Recommended for acceptance once spike S3's required questions are answered, as ADR-0004 waited for S4 |
| **Date** | 2026-10-17 |
| **Deciders** | Project owner (only the owner can move an ADR to Accepted) |
| **Related** | [Script and heap](../architecture/script-and-heap.md) (SH1–SH14); [ADR-0003](0003-gc-handle-and-rooting-pattern.md) (C1–C5); [ADR-0004](0004-process-model-and-sandbox.md) (J1–J7); [S1 results](../research/s1-results.md); [Step 4 tracker](../research/step-4-architecture-proposal.md) §10 (spike S3's design); OQ-5, OQ-13, OQ-14 |

## Decision history

| Date | Event |
|---|---|
| 2026-10-17 | Proposed at Step 4, CP12, with spike S3's design for the owner's authorization |

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

## Evidence and sources

[Script and heap](../architecture/script-and-heap.md) Sources; [S1 results](../research/s1-results.md) §5; SDK 27 `os/os_sync_wait_on_address.h`.

## Decision (proposed)

1. **One heap per agent, on its event loop's thread.** DOM objects are the JavaScript objects, with no wrappers (SH1).
2. **Between agents, only copies, transfers and shared buffers.** Backing stores live outside the heap, and `Atomics.wait` uses `os_sync_wait_on_address` (SH2–SH3).
3. **Script runs only in tasks and microtasks.** An atomic interrupt flag can stop it (SH4–SH5).
4. **The root set:** registers and frames traced as regions; native holds on the root stack; the agent's own roots (SH6).
5. **C1: a root stack without borrow tracking, with 8-byte slots,** in one Miri-checked allowlisted module, targeting about 1 ns per root and per read. **Validated by spike S3** (SH7).
6. **C2:** struct-of-arrays columns or power-of-two strides for hot link fields (SH8).
7. **C3:** generated setters for every handle-valued field, the single place for any future barrier (SH9).
8. **C4:** stop-the-world mark–compact now. Then incremental marking in idle periods and a generational nursery, ordered by corpus measurements, with the pause budget from Step 6 (SH10).
9. **C5:** memory that reaches platform APIs lives outside the heap and never moves; mark-worklist entries are 8 bytes (SH11).
10. **Generated, safe bindings.** Room for a JIT under J1–J7, informed by S3-Q3's dispatch measurements (SH12–SH13). Heaps are budgeted (SH14).

## Rationale

- These choices complete ADR-0003 without changing it.
- They keep every heap on one thread (AC5).
- They leave the collector's evolution to measurement rather than guesswork (AC10).

## Assumptions

- Most rooting traffic comes from native builtins, not from executing bytecode. ADR-0003 stated this, and S3-Q2 tests it.
- Generated setters add no measurable cost while the barrier is a no-op (S3-Q5 if authorized, or the first heap milestone).

## Risks and unresolved issues

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| C1 misses its target | Medium | High | S3 measures before the object model is fixed. ADR-0003's revisit condition 1 then applies, and the owner decides |
| Stop-the-world pauses on large heaps | High (for large apps) | Medium | SH10's plan, with SH9's barrier point already in place |
| The `unsafe` in the root stack hides a soundness bug | Low | High | One small module; Miri under Stacked and Tree Borrows; review (V5) |

## Conditions for revisiting

- S3-Q1 fails, or S3-Q2 shows rooting dominating an interpreter-shaped workload.
- Agents need to share GC objects across threads, contrary to ADR-0003's assumption.

## Consequences

- **Acceptance waits for S3** (CP13–CP15), if the owner authorizes it at Gate B.
- The `gc` crate's only `unsafe` is the root stack, and only if C1 needs it.
- OQ-5 resolves with this ADR, and OQ-14 with S3. OQ-13's design part resolves here; its budget is set in Step 6.
