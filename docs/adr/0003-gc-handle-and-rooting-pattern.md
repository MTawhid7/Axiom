# ADR-0003: GC handle and rooting pattern

| | |
|---|---|
| **Status** | **Accepted** (2026-10-17), as proposed, with obligations C1–C5 |
| **Date** | 2026-10-17 |
| **Deciders** | Project owner |
| **Related** | [S1 results](../research/s1-results.md) (all evidence), [Step 2b tracker](../research/step-2b-validation-spikes.md), [ADR-0001](0001-scope-and-target.md) (R2, R3), [ADR-0002](0002-implementation-language.md), [language evaluation](../technology/language-evaluation.md) §7, §15.3 |

## Decision history

| Date | Event |
|---|---|
| 2026-10-17 | Proposed after spike S1 (Step 2b, CP9) |
| 2026-10-17 | **Owner accepts it as proposed**: Pattern B, with obligations C1–C5. On ceremony (S1-Q2): "Nova's lexical ceremony (scope tokens, bind/unbind) is acceptable. It is checked by the compiler and far preferable to the architectural burden of turning the JS interpreter and DOM dispatch into resumable state machines under gc-arena." |
| 2026-10-17 | **Owner makes spike S3 a required benchmark**, to run during Step 4 as part of the interpreter and root-stack design. It validates obligation C1 (OQ-15) |

## Question

How does Arbor's code refer to objects in the garbage-collected heap that JavaScript and the DOM share, and how does it keep them alive and correct across collections? The answer fixes:
- the type of a heap reference;
- where collections may happen;
- how references held by native (Rust) code survive a collection;
- how heap writes are expressed;
- what the GC crate's `unsafe` boundary is.

## Requirements and constraints

- **Our own JS engine, with one heap for JS and the DOM** (ADR-0001 R2). Hundreds of builtins can call back into script. `ToPrimitive` alone reaches user code from most arithmetic, comparison and property-key operations.
- **Security posture fit for sensitive logins** (R3). `unsafe` stays confined and audited (evaluation §15.3).
- **Efficiency is a primary goal** (charter G4): memory first on an 8 GB machine, then CPU and power.
- **A path to incremental or generational collection.** Pause times will matter as heaps grow.
- **One heap per agent**, with site isolation (ADR-0001) and workers (OQ-5). Heaps are single-threaded, and `SharedArrayBuffer` data lives outside them.
- **One developer with AI assistance.** Ceremony that the compiler checks is preferable to ceremony that review must catch.

## Alternatives considered

| | Pattern | Built in S1? |
|---|---|---|
| **A** | **gc-arena style.** Branded `Gc<'gc, T>` pointers to individually allocated boxes; mutation inside `Arena::mutate` closures; write barriers through `Gc::write`; a derived, `unsafe` `Trace`; collection only *between* mutation scopes | Yes, measured |
| **B** | **Nova style.** `u32` handles into typed arenas; `GcScope`/`NoGcScope` tokens that make an unrooted handle held across a collection a compile error; an explicit root stack (`RootFrame`, `Scoped`); a safe `Trace`; a mark–compact collector that can run at any allocation | Yes, measured |
| D′ | An `unsafe` core with typed handles and an explicit shadow-stack root set: the classic precise design (evaluation §7.3) | No. The owner chose the two canonical patterns. Its rooting costs resemble B's, with less help from the compiler (inference) |
| E | Conservative stack scanning (JavaScriptCore, Ladybird's LibGC) | No. It needs `unsafe` assumptions about stack contents and layout, pins every object a stack word might point to, and so rules out compaction (inference; evaluation §7.2) |

## Historical context and modern developments

The evaluation's §7 covers this; the facts below were checked on 2026-10-17 ([S1 results](../research/s1-results.md), sources 1–4).
- **gc-arena is sound, and incremental.** It keeps an intrusive list of every object, sweeps it object by object, and paces marking and sweeping as allocation debt, with real write barriers. piccolo, the Lua VM built on it, is stackless because "no garbage collection can take place during a call to `mutate`". Its Rust callbacks that call back into Lua are state machines polled by an executor.
- **Nova roots scoped values in `agent.stack_refs`**, a `RefCell<Vec<…>>`. Its author describes the resulting `bind`/`unbind` ceremony as a "soup" found in nearly 800 places, and notes that gc-arena is the sound alternative available today.

## Analysis

The full data and its attribution are in [S1 results](../research/s1-results.md) §3–§6. In summary:

| Consideration | Pattern A | Pattern B | Favours |
|---|---|---|---|
| Builtins and an interpreter that call back into script | Every long or re-entrant path becomes a resumable state machine with its state in the root (S1 dispatch: 128 lines), plus a stackless interpreter | Ordinary functions that root what they hold across a call that may collect | **B, strongly** |
| `unsafe`, and what a GC bug becomes | 24 blocks in one module plus 2 macro templates. A tracing bug is a use-after-free, which is undefined behaviour | No `unsafe` anywhere. A bug is a stale index: a bounds panic or the wrong object of the right type. Debug builds catch it deterministically | **B** |
| Memory | Baseline: 24-byte headers, 8-byte pointers, one allocation per object | 13% less per live node, everything included; 13–22% less process memory in every scenario | **B** |
| Mutator CPU | Cache-friendly traversal 18–22% faster. Code that holds many references across possible collections is faster (dispatch 2.06×) | Allocation-heavy work takes 30% less time. It pays about 2 ns per root and 1.6 ns per root read (hot), and an index multiply on each dependent hop | Mixed |
| Collection | Faster on small heaps. The sweep walks a linked list, so it degrades once the heap outgrows the caches | 1.3–2.8× faster from 64k elements; lower maximum pauses in every scenario | **B today** |
| Incremental or generational collection | Barrier insertion points already exist; gc-arena is incremental | Plain writes work only with a stop-the-world collector | **A** |
| Build | `syn` adds about 1.8 s to a clean build | No proc-macros | B, minor |

## Evidence and sources

- [S1 results](../research/s1-results.md). It reports measurements over 3 process runs, per-process memory and instruction counts, line-level profiles and disassembly, and the ceremony counts. Sources 1–4 there cover gc-arena, piccolo and Nova.
- Tracker §8, CP3–CP7: correctness evidence, including oracle equality on every workload, A ≡ B, and Miri.

## Decision

**Adopt Pattern B, the Nova style, for the JS and DOM heap:**
- typed arenas addressed by `u32` handles;
- handles bound to `GcScope`/`NoGcScope` tokens;
- an explicit root stack with RAII frames for values that native code holds across a possible collection;
- a safe `Trace`;
- a compacting collector that may run at any allocation.

It comes with the **obligations** C1–C5 below. Each is settled during the architecture step (Step 4), before the JS object model is fixed, unless noted.

| # | Obligation | Validation |
|---|---|---|
| C1 | **Cheaper rooting:** at most about 1 ns per root and per read, against 2.05 and 1.58 ns measured. **Hypothesis:** a root stack without `RefCell` borrow tracking (an `UnsafeCell`-based stack in an allowlisted module, as gc-arena does for its gray queue), with 8-byte slots | **Spike S3** (required; runs in Step 4 with the interpreter and root-stack design). Also `s1-probe roots` and the dispatch suite (OQ-14) |
| C2 | **No index arithmetic on hot dependent loads.** Power-of-two strides, or struct-of-arrays `u32` link columns, so the scaling folds into the load's addressing mode | The `traverse` suite on the new layout |
| C3 | **A write-barrier API** for fields that hold handles, designed before any incremental or generational collector, so the type system enforces it | Recount S1-Q2's ceremony tokens with the barrier added (OQ-13) |
| C4 | **A pause budget**, and a plan for incremental or generational collection. The stop-the-world pause is about 4 ms at 130k live nodes and 15 ms at 333k objects | The performance section (Step 6); measurement on a recorded corpus of real pages |
| C5 | **Native interop.** Pinning, or a non-moving space, for script-visible memory handed to platform APIs. S2 found that glyph and pixel buffers need not live in the GC heap. Also: 8-byte mark-worklist entries | GPU-facing APIs, later; re-run `tools/s1-memory.sh` |

## Rationale

1. **Fit with our own JS engine (R2) decides it.**
   - **Under A,** `ToPrimitive`, accessors, `Proxy` traps, iterator protocols, `Array.prototype` callbacks and `Promise` jobs would all have to be resumable, or collection would wait for every native frame to return.
   - **Suspending collection doesn't bound memory.** The pressure test showed a peak of about 131k live nodes without mid-dispatch collection, against about 2.2k with it.
   - **B's cost is local.** It is checked by the compiler, and it never changes the shape of the code.
2. **Containment and failure mode (R3).** With B, a GC bug cannot corrupt memory. It remains a correctness bug, and the scope discipline and debug epochs are built to catch it.
3. **Memory (G4).** B is smaller in every measured scenario.
4. **Collector scaling.** B's dense, compactable arenas collect faster at the sizes that matter for real pages and heaps. They are also the natural base for a generational nursery (inference).

## Assumptions

- **Most rooting traffic comes from native builtins, not from bytecode execution** (inference). An interpreter's value stack and registers are roots traced as a whole; per-value `scope`/`get` is needed only where Rust code holds values across a call that may allocate. S1 measured the worst case, a builtin holding every listener of every visited node. Spike S3 validates this in Step 4 (C1).
- **A barrier API can be added for incremental or generational collection** without approaching A's write ceremony (C3).
- **Heaps are per agent and single-threaded.** B's root stack (`Rc<RefCell<…>>`) is deliberately not `Send`.
- **`u32` handles per arena are enough** for one agent's heap: about 4 billion objects of each type.

## Risks and unresolved issues

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Root traffic dominates interpreter-heavy workloads | Medium | High | C1; an interpreter design that keeps live values in rooted VM state; spike S3 in Step 4 |
| Barriers for incremental or generational GC add write ceremony | Medium | Medium | Design C3 early, with the ceremony recounted |
| Stop-the-world pauses on large heaps | High (for large apps) | Medium | C4: a pause budget, then a generational nursery or incremental marking |
| A "use after GC" (stale handle) reaches a release build | Medium | High (logic errors in a security boundary) | Compile-time scope discipline; debug epochs in tests and fuzzing; consider a generation check in release builds (Nova notes spare handle bits) |
| Index-arithmetic overhead on hot traversals | High | Low to medium | C2 |
| Mistakes in hand-written tracing | Medium | Medium | A **safe** `#[derive(Trace)]` that generates `mark` and `relocate`; it costs about 1.8 s of clean build (`syn`), measured in S1. Debug builds already detect missed marks |
| A moving collector clashes with native interop | Low to medium | Medium | C5 |

## Conditions for revisiting

- **C1 fails.** Rooting stays well above 1 ns per operation and dominates an interpreter-shaped workload.
- **C3 makes B's writes as ceremonious as A's, *and* a JS-engine design emerges in which only a small, fixed set of re-entry points must be resumable.** Together, these would remove A's main architectural cost.
- **Real engine code shows the lexical ceremony to be much heavier than S1's.** The owner judged S1's ceremony acceptable on 2026-10-17.
- **Agents need to share GC handles across threads.**

## Consequences

- **Every GC-managed type lives in a typed arena; references are handles.** Code that may allocate takes a `GcScope`. Native code roots what it holds across such calls. The interpreter's stack and registers are roots.
- **Builtins, the DOM and the bindings are ordinary Rust functions.** WebIDL code generation emits the handle and rooting glue mechanically (evaluation §7.5).
- **The GC crate's `unsafe` boundary is small or empty.** The S1 prototype needed none. C1 may add a small audited root-stack module, registered in `tools/unsafe-policy.txt`.
- **ADR-0002's assumption about GC references is confirmed.** This ADR becomes the GC section's constraint in the architecture (Step 4).
- **Follow-up work:**
  - OQ-13 (incremental or generational strategy) tracks C3 and C4.
  - OQ-14 (root-stack design) tracks C1.
  - Spike S3, required in Step 4, validates C1.
