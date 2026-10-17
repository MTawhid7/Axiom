# S1 Results: A Shared GC Heap for JavaScript and the DOM in Rust

| | |
|---|---|
| **Status** | **Reviewed** by the owner (2026-10-17). The recommendation was accepted as [ADR-0003](../adr/0003-gc-handle-and-rooting-pattern.md); the owner judged Pattern B's ceremony acceptable (S1-Q2) |
| **Question** | Can a garbage-collected heap shared by JS objects and DOM nodes be written in Rust with `unsafe` confined to the GC, at acceptable ergonomic and runtime cost on the target machine? Which of the two candidate patterns should the engine use? |
| **Feeds** | [ADR-0002](../adr/0002-implementation-language.md) (final acceptance); ADR-0003, the GC handle and rooting pattern (to be proposed at CP9) |
| **Plan and raw log** | [Step 2b tracker](step-2b-validation-spikes.md): questions §3, design §4, methods §7, per-checkpoint results §8 |
| **Code** | `spikes/s1/*` on branch `step-2b`. Measurements ran at commit `d5d8272` |
| **Last updated** | 2026-10-17 |

**Evidence labels** follow the [research conventions](README.md):
- **Measured** numbers come from this repository's reproducible benchmarks, with the commands in §11.
- **Inference** marks an explanation drawn from measurements, profiles or disassembly.
- **Hypothesis** marks an untested claim; each one names its validation step.

---

## Summary

**What was compared.** Two canonical designs, built from scratch, each running the same DOM: nodes, attributes, listeners, closures with node ⇄ closure cycles, weak references, and event dispatch.
- **Pattern A, gc-arena style:**
  - branded `Gc<'gc, T>` pointers to individually allocated boxes;
  - mutation inside `Arena::mutate` closures;
  - write barriers through `Gc::write` and `unlock!`;
  - a derived, `unsafe` `Trace`;
  - non-moving mark–sweep, which runs only *between* mutation scopes.
- **Pattern B, Nova style:**
  - `u32` handles into typed arenas;
  - `GcScope`/`NoGcScope` tokens that make holding an unrooted handle across a collection a compile error;
  - an explicit root stack (`RootFrame`, `Scoped`);
  - a hand-written, safe `Trace`;
  - a mark–compact collector that can run at *any* allocation.

Both reproduce the reference interpreter exactly on every tested script, and agree with each other (tracker §8, CP4 and CP6).

| Question | Answer |
|---|---|
| **S1-Q1** Is `unsafe` contained? | **Yes, for both.** Pattern A keeps it in one module (24 `unsafe` blocks under six documented invariants), plus two audited macro templates; its DOM crate is `#![forbid(unsafe_code)]`. **Pattern B has no `unsafe` anywhere**: not in the GC, its macro, or the DOM. Miri finds no undefined behaviour in either |
| **S1-Q2** Is the ceremony acceptable? | **Owner judgment.** The volume is about the same; the kind differs. A's cost is architectural: anything that runs long or calls back into script must become a resumable state machine. B's cost is lexical: `bind`/`unbind`/`reborrow`/`scope` on handles, plus hand-written tracing. My assessment is that B's kind is the acceptable one for an engine that includes its own JS engine (§4) |
| **S1-Q3** What does each cost at run time? | **Neither dominates.** B allocates and collects faster at scale (full collections 1.3–2.8× faster from 64k elements; the allocation-heavy scenario takes 30% less time), has lower maximum pauses, and uses 13% less memory per live node, and 21–22% less process memory in three of the four scenarios. A is faster at pointer chasing in cache-friendly order (B takes 18–22% longer) and at code that holds many references across possible collection points (B's dispatch takes 2.06× as long). Whole scenarios range from B 30% faster to B 8% slower |
| **S1-Q4** What does it cost to build? | **Little, for both** (stack builds clean in under 2.5 s, the whole S1 workspace in 4.2 s, every `rustc` under 255 MiB). B's stack builds clean 4× faster (0.58 s against 2.37 s), because A's derive pulls in `syn`. Incremental builds are equal (0.13 s) |

**Recommendation for ADR-0003: Pattern B, conditionally.** Confidence: **medium**. Five conditions are in §9.

The deciding factor is the requirement to write our own JavaScript engine (ADR-0001 R2). Under Pattern A, collection waits until every mutation scope has returned. So the interpreter, and every builtin that can call back into script, would have to be resumable with its state in the GC root. Under Pattern B, they stay ordinary functions that root what they hold.

B's measured costs, rooting traffic and index arithmetic on dependent loads, are real but local, and each has a named mitigation to validate. B's main open risk is the move to incremental or generational collection: its plain field writes would need a write-barrier API.

**For ADR-0002:** the central hypothesis holds, more strongly than the language evaluation assumed. Revisit condition 1 (`unsafe` cannot stay inside the GC) does not trigger.

---

## 1. What was built and verified

| | Pattern A (`s1-a-gc`, `s1-a-gc-derive`, `s1-a-dom`) | Pattern B (`s1-b-gc`, `s1-b-dom`) |
|---|---|---|
| Reference to the heap | `Gc<'gc, T>`: an 8-byte pointer, branded by an invariant lifetime | `Handle<'a, T>`: a 4-byte index (`NonZeroU32`); debug builds add a collection epoch |
| Allocation | One system-allocator allocation per object, with a 24-byte header; an intrusive list of every object | Push onto a typed `Vec` arena; no header |
| Tracing | `unsafe trait Trace`, derived; the derive also proves that the type has no `Drop` impl | Safe `Trace` with `mark` and `relocate`, written by hand |
| Writes | `Gc::write(mc, x)` + `unlock!(…)` for every field that holds a `Gc` | Plain field assignment through the arena |
| When collection can run | Only between `Arena::mutate` calls. Long work must yield | At any allocation (a safepoint). Handles held across one must be rooted |
| Collector | Stop-the-world mark–sweep: tri-colour mark, then a sweep over the object list | Stop-the-world mark–compact: bitmap mark, rank-based forwarding, order-preserving compaction, relocation of every handle |
| Event dispatch | A resumable state machine stored in the arena root | An ordinary function that roots its event path and listener snapshots |

**Fidelity to the originals** (sourced, checked 2026-10-17):
- **gc-arena.** It keeps the same intrusive list, `all: Cell<Option<GcPtr>>`, and sweeps it one object at a time, deallocating each dead one [1]. Unlike our Pattern A, gc-arena is **incremental**: marking and sweeping are paid off as allocation debt, and its backward and forward write barriers do real work [1]. Our A is a stop-the-world simplification, so its pause numbers overstate what gc-arena would show.
- **piccolo**, the Lua VM built on gc-arena, is stackless for exactly the reason measured here: "no garbage collection can take place during a call to `mutate`", so Rust callbacks that call back into Lua are written as `Sequence`s that an executor polls [2].
- **Nova.** Its scoped roots live in `agent.stack_refs`, a `RefCell<Vec<…>>`. Rooting is `borrow_mut()` plus `push`; reading back is `borrow()` plus a bounds-checked `get` [3]. Pattern B's `Rc<RefCell<Vec<RootSlot>>>` has the same cost model. Nova's author describes the `bind`/`unbind` ceremony as a "soup", with "nearly 800 places" using the chain in Nova [4].
- Nova's collector design was **not** checked here. Pattern B's mark–compact collector is this spike's own design (tracker §4.3).

**Verification behind every number below** (tracker §8):
- Each DOM reproduces the reference outcome (listener log, tree digests, live counts after every collection, weak-reference liveness) on:
  - 10 edge-case scripts;
  - 100 seeds × 4 workloads;
  - Medium-size seeds.
- That holds both at default pacing and when collecting at every opportunity.
- A ≡ B on every workload.
- Miri (strict provenance) is clean:
  - on both GC crates, under Stacked Borrows and Tree Borrows;
  - on both DOM crates under small workloads: Pattern A under both models, Pattern B under Stacked Borrows. B has no `unsafe` for Miri to check.
- Every benchmark case first checks both patterns against reference results before it times anything (§2).

---

## 2. Method and environment

**Machine.** MacBook Air M1 (4 performance + 4 efficiency cores, 8 GB, fanless), macOS 27.2, on AC power; rustc 1.94.1, release profile. No thermal warning was recorded before or after any run (`pmset -g therm`).

**Inputs.** Generated by the CP2 generators and validated op by op by the reference interpreter. Each input carries the tree digests and dispatch logs both patterns must reproduce. Micro-benchmark trees are allocated in document order, as a parser would; breadth-first and shuffled orders are measured as traversal variants.

**Runtime.** `tools/s1-bench.sh`: three separate process runs, each after a 60 s cool-down. Within each case:
- a 300 ms warm-up (500 ms for whole scenarios), because Apple Silicon raises the clock only after sustained load;
- then 11 samples per pattern (5 for scenarios), alternating A and B in ABBA order, so that drift hits both equally.

Reported values are **the median of the three run medians**. "±x%" is half the range of the run medians, relative to that median.

**Memory and CPU work.** `tools/s1-memory.sh`:
- Each measurement is a fresh process that runs a pre-generated script file under `/usr/bin/time -l`.
- It records peak memory footprint, maximum RSS, and instructions retired, three times each.
- A parse-only run is subtracted, so the generator's own memory is excluded.

**Profiles.** `tools/s1-profile.sh`: samply at 1 kHz, narrowed to the timed loop, and resolved to source lines with `dsymutil` and `atos`.

**Run quality.**
- Background load was not controlled. The load average was 3–4, except during bench run 2, when it reached about 8. A's scenario timings in that run were up to about 50% slower than in runs 1 and 3 (`listeners`: 244 ms against 159–172 ms), and B's up to 13% slower; the median of medians absorbs one disturbed run.
- Memory and instruction measurements varied by at most 1.3% between runs.

---

## 3. S1-Q1: is `unsafe` contained?

| | Pattern A | Pattern B |
|---|---|---|
| `unsafe` in the GC crate | 24 `unsafe {}` blocks, 5 `unsafe fn`, 16 `unsafe impl` lines, 1 `unsafe trait`, 41 `// SAFETY:` comments. All under `s1-a-gc/src/raw/`, governed by invariants I1–I6 | **None** (`#![forbid(unsafe_code)]`) |
| `unsafe` emitted by macros (evaluation §15.3, rule 7) | Two audited templates: the derive's `unsafe impl Trace`, and `Write::assume` inside `unlock!` | **None** (`heap!` expands to safe code) |
| DOM crate | `#![forbid(unsafe_code)]`; no hand-written `unsafe` | `#![forbid(unsafe_code)]`; none anywhere in its dependency chain |
| What a GC bug becomes | A missed `trace` is a **use-after-free: undefined behaviour** | A missed root or relocation is a **stale index**: a bounds panic, or a type-correct read of the wrong object. It is a logic bug, not undefined behaviour |
| How such bugs are caught | Six compile-fail tests pin the branding rules. Miri checks the unsafe core under DOM workloads | The borrow checker rejects a bound handle used after a possible collection (pinned as E0502). Debug-build epochs turn a stale handle into a deterministic panic, and a tracing bug into "some Trace::mark missed it"; tests prove both fire |

Both patterns pass the S1-Q1 criterion (tracker §3). **Inference:** B's failure mode is the better fit for the security posture (ADR-0001 R3), because a GC bug cannot corrupt memory. It is still a correctness and security bug class ("use after GC"). Release builds could keep a cheap generation check in handles: Nova's author notes 24 spare bits in its handles that could serve this purpose [4].

---

## 4. S1-Q2: ceremony

**Measured** by `tools/s1-ceremony.sh` on the two DOM crates. Comments are excluded.

| Concern | Pattern A | Count | Pattern B | Count |
|---|---|---:|---|---:|
| Write barrier | `Gc::write(…)` | 12 | — | 0 |
| Field projection behind the barrier | `unlock!(…)` | 16 | — | 0 |
| GC-pointer fields needing interior mutability | `Lock<…>` / `RefLock<…>` | 5 / 1 | — | 0 |
| Tracing: types | derived (`#[derive(…Trace…)]`) | 5 | `impl Trace for` / `impl Roots for` | 3 / 1 |
| Tracing: per-field work | `#[trace(require_static)]` (orphan rule) | 9 | `marker.mark…` / `forwarding.relocate…` calls | 11 / 12 |
| Collection context passed to functions | `&Mutation<…>` parameters | 8 | `GcScope<…>` parameters | 6 |
| Scope handling | `mutate(` / `mutate_root(` | 1 / 4 | `.reborrow()` / `.nogc()` / `.into_nogc()` | 8 / 4 / 1 |
| Handle lifetime changes | — | 0 | `.bind(…)` / `.unbind()` | 5 / 9 |
| Rooting across a possible collection | — (state lives in the root) | 0 | `.frame()` / `.scope(&…)` / `Scoped::get()` | 4 / 5 / 5 |
| Collection points | `Step::Yielded` / `wants_collection` / `collect_debt(` | 2 / 1 / 2 | `safepoint(` | 4 |
| **Code size** (non-blank, non-comment lines) | | **506** | | **560** |

Lines per DOM operation are within a few lines of each other:
- `remove`: 18 (A) against 19 (B);
- `insert_before`: 28 against 29;
- `add_listener`: 30 against 28;
- dispatch: 99 against 103.

The patterns differ in the *kind* of ceremony, not its amount.

**Writes.** The same operation, detaching a node:

```rust
// Pattern A: every write to a Gc-holding field goes through the barrier.
match prev {
    Some(prev) => unlock!(Gc::write(mc, prev), Node, next_sibling).set(next),
    None => unlock!(Gc::write(mc, parent), Node, first_child).set(next),
}

// Pattern B: plain assignment through the arena.
match prev {
    Some(prev) => heap.nodes[prev].next_sibling = next,
    None => heap.nodes[parent].first_child = next,
}
```

**A builtin that calls back into script.** The language evaluation's S1 brief (§16) asked how such builtins are expressed. Event dispatch is the case here: listeners are script code, and may allocate.

- **Pattern A.** `dispatch.rs` is a resumable state machine: 128 lines holding a flattened visit sequence, a cursor, the snapshot and a stop flag, all stored in the arena root. The driver calls `step` repeatedly and collects whenever it yields. A listener that itself ran long would need the same treatment, and so would the interpreter (as piccolo's is [2]).
- **Pattern B.** `dispatch` is an ordinary loop. It roots the path and each snapshot, and re-reads handles after every listener call:

  ```rust
  for listener in &snapshot {
      let listener = &agent.heap.listeners[listener.get().bind(gc.nogc())];
      // … filter, log …
      run_closure(agent, listener.closure, &mut stop, gc.reborrow()); // may collect
  }
  ```

**Assessment (inference).** For this project, the deciding consideration is how each kind of ceremony scales to a whole ECMAScript engine.
- **A's cost spreads through the engine.** Many abstract operations can call user code: `ToPrimitive` calls `valueOf`/`toString`; there are getters and setters, `Proxy` traps, iterator protocols, `Array.prototype` callbacks and `Promise` jobs. Under A, each of these, and the interpreter, would have to be resumable, or collection would have to be suspended while native frames are active.
- **Suspending collection doesn't bound memory.** The pressure test shows what happens without mid-dispatch collection: the peak reaches about 131k live nodes instead of about 2.2k (tracker §8, CP4).
- **B's cost stays local.** It is noisy, as the Nova author acknowledges [4], but the compiler enforces it, and it never changes the shape of the code.

S1-Q2's pass criterion is your judgment.

---

## 5. S1-Q3: runtime cost

### 5.1 Mutator (no GC runs inside these cases)

| Case | Unit | A | B | B/A | Spread A / B |
|---|---|---:|---:|---:|---|
| build: 10k-element tree (12,982 nodes) | ns per node | 77.6 | 70.9 | 0.91 | ±3% / ±3% |
| build: 100k-element tree (129,839 nodes) | ns per node | 80.9 | 81.5 | 1.01 | ±1% / ±2% |
| traverse: 10k, document order | ns per node | 6.7 | 7.9 | 1.18 | ±10% / ±8% |
| traverse: 100k, document order | ns per node | 7.7 | 9.4 | 1.22 | ±7% / ±4% |
| traverse: 100k, breadth-first allocation | ns per node | 9.2 | 9.5 | 1.03 | ±4% / ±1% |
| traverse: 100k, after 100k random moves | ns per node | 49.1 | 45.8 | 0.93 | ±10% / ±9% |
| mutate: 100k moves, inserts, removes, attribute writes | ns per op | 73.4 | 76.8 | 1.05 | ±8% / ±8% |
| dispatch: 20k elements, 60k listeners, 50k events | ns per dispatch | 520 | 1,073 | **2.06** | ±4% / ±4% |

The dispatch case makes 682,346 listener invocations, about 14 per dispatch: 38 ns per invocation for A, 79 ns for B.

**Unit cost of rooting** (Pattern B; `s1-probe roots`; three runs, identical to two decimals):

| Operation | Cost |
|---|---:|
| Open and close a root frame | 4.08 ns |
| Root a handle (`Handle::scope`) | 2.05 ns |
| Read it back (`Scoped::get`) | 1.58 ns |

These are best-case numbers, with everything hot in cache. Pattern A's counterpart, using a `Gc` inside a mutation scope, is a pointer copy.

### 5.2 Collector: pause against live-set size

**A full collection with everything live:** median pause, and time per live object.

| Elements | Objects | Heap bytes A / B | Pause A | Pause B | B/A | ns/object A / B |
|---:|---:|---|---:|---:|---:|---|
| 1k | 1,325 | 0.20 / 0.13 MiB | 21.7 µs | 31.6 µs | 1.46 | 16.4 / 23.8 |
| 4k | 5,212 | 0.80 / 0.52 MiB | 102 µs | 161 µs | 1.58 | 19.6 / 31.0 |
| 16k | 20,728 | 3.2 / 2.1 MiB | 422 µs | 685 µs | 1.62 | 20.4 / 33.1 |
| 64k | 83,148 | 12.7 / 8.2 MiB | 4.11 ms | 3.16 ms | 0.77 | 49.4 / 38.0 |
| 256k | 332,702 | 50.8 / 33.0 MiB | 34.8 ms | 14.6 ms | **0.42** | 104.5 / 43.8 |

**Half garbage:** every live node is followed by one unreachable node in allocation order, so B must move every survivor.

| Elements | Objects before | Pause A | Pause B | B/A |
|---:|---:|---:|---:|---:|
| 1k | 2,649 | 41.1 µs | 39.8 µs | 0.97 |
| 4k | 10,423 | 176 µs | 204 µs | 1.16 |
| 16k | 41,455 | 733 µs | 981 µs | 1.34 |
| 64k | 166,295 | 12.4 ms | 4.86 ms | 0.39 |
| 256k | 665,403 | 63.2 ms | 22.5 ms | **0.36** |

There is a crossover between 16k and 64k elements, as the heap outgrows the caches:
- **Below it, A is faster:** its two passes cost fewer instructions than B's three.
- **Above it, A's cost per object rises five-fold**, while B's rises by about a third.

### 5.3 Whole scenarios (CP2 workloads, Large, through the verified drivers, default pacing)

| Scenario | Ops | A | B | B/A | Collections A / B | Max pause A / B | Peak GC-heap bytes A / B |
|---|---:|---:|---:|---:|---|---|---|
| tree | 590,151 | 43.3 ms | 34.5 ms | 0.80 | 8 / 7 | 7.8 / 4.1 ms | 19.8 / 12.9 MiB |
| listeners | 214,158 | 171.7 ms | 168.0 ms | 0.98 | 27 / 26 | 6.0 / 3.8 ms | 14.0 / 7.8 MiB |
| churn | 452,979 | 170.2 ms | 184.0 ms | 1.08 | 77 / 76 | 4.2 / 3.5 ms | 11.8 / 7.0 MiB |
| pressure | 12,312 | 85.0 ms | 59.5 ms | 0.70 | 459 / 509 | 0.14 / 0.11 ms | 1.34 / 0.82 MiB |

- **Max pause** is the median of six recorded maxima: three bench verification runs and three memory runs. Single maxima are noisy, because the scheduler can preempt a collection: B's 6 `churn` maxima ranged from 3.4 ms to 20.9 ms.
- **The pressure test.** Garbage is allocated inside long dispatches, so collections must happen mid-dispatch. Peak live nodes were 8,643 for A and 8,179 for B, against 2.1 million nodes allocated.

### 5.4 Memory

**Measured:** process memory, with the parse-only baseline subtracted (median of 3).

| Input | Peak footprint A | Peak footprint B | B/A | Instructions A | Instructions B | B/A |
|---|---:|---:|---:|---:|---:|---:|
| tree (Large) | 31.4 MiB | 25.0 MiB | 0.79 | 0.35 G | 0.36 G | 1.04 |
| listeners (Large) | 27.2 MiB | 21.1 MiB | 0.78 | 0.95 G | 1.66 G | 1.75 |
| churn (Large) | 18.0 MiB | 14.1 MiB | 0.78 | 0.56 G | 1.38 G | 2.46 |
| pressure (Large) | 2.1 MiB | 1.8 MiB | 0.87 | 1.19 G | 1.12 G | 0.94 |
| all-live tree, 520,065 nodes | 132.3 MiB | 113.1 MiB | 0.85 | 1.40 G | 1.53 G | 1.09 |

**Bytes per live node, everything included.** This includes attributes, text, the driver's id table and allocator overhead. The slope between 129,839 and 520,065 live nodes cancels fixed costs: **A 271 B, B 236 B (−13%)**.

**Object sizes** (`s1-probe sizes`):

| Object | A (box) | B (arena entry) |
|---|---|---|
| Node | 160 B (24 header + 136) | 104 B |
| Listener | 40 B | 12 B |
| Closure | 64 B | 32 B |
| Reference | 8 B | 4 B (`Option` also 4) |

Why B's process-level saving (13–22%) is smaller than its object saving (35–44% of GC-heap bytes) (inference):
- Attribute and text storage is identical in both patterns.
- At the final collection, B's mark worklist briefly holds every rooted id, at 16 bytes per entry; that is about 8 MB at 520k nodes, against 4 MB for A's 8-byte gray stack. An 8-byte work item would remove most of the difference (condition C5 in §9).

### 5.5 CPU work and energy

B retires 1.75× (listeners) and 2.46× (churn) the instructions of A for wall times within ±8%:
- **B's extra instructions** are root-stack operations, bounds checks, and its relocation pass.
- **A's time** goes to cache misses (§6).

Energy was not measured (`powermetrics` needs root). Because instructions and time disagree here, neither is a reliable energy proxy.

---

## 6. Attribution: rooting discipline, representation, collector

The patterns differ in three confounded ways (tracker §4.4). Profiles and disassembly assign each measured difference to one of them.

| Observation | Dominant factor | Evidence |
|---|---|---|
| build: equal (B/A 0.91–1.01) | **Neither.** Both spend 35–48% of build time in `malloc` for attributes and text, which they allocate identically | Profiles `build-a`, `build-b` |
| traverse, cache-friendly order: B +18–22% | **Representation.** Each hop in B decodes the index on the dependent chain: load the link, subtract one, bounds check, `madd` by 104, then the next load. In A a hop is one load | Disassembly of B's loop. Hottest line: B's handle decode (`handle.rs:54`, 28%), against A's `Lock::get` (32%) |
| traverse, shuffled: B −7% | **Representation.** Once each hop is a cache miss, B's 35% smaller nodes help more than the index arithmetic costs | Measured crossover across the three orders |
| mutate: B +5% | **Representation**, on the write path. B's index decode is 21% of samples; A pays the barrier (a counter, plus a `Cell` write) | Profiles `mutate-a`, `mutate-b` |
| dispatch: B 2.06× | **Rooting discipline.** Root-stack pushes and reads take about half of B's dispatch time. The rest costs about what A's whole dispatch does | Line profile: `RootFrame::push` 18%, `Scoped::get` 13%, the extend loop that drives the pushes 18%. Unit costs: §5.1 |
| collection, ≤16k elements: B 1.5–1.6× slower | **Collector.** B makes three passes (mark, rank, relocate every handle in every live object and root), A makes two (mark, sweep). Everything is in cache, so instruction count decides | Profile `collect-live-b`: mark 56%, relocate 16%, rank 12% |
| collection, ≥64k elements: B 1.3–2.4× faster (all live), 2.5–2.8× (half garbage) | **Collector and representation.** A's sweep is a dependent pointer chase through every individually allocated box, then a `free` per dead box. B's passes run over dense arrays | Profile `collect-live-a`: sweep 69% (`heap.rs:211` loads the next link; `:215` touches the header). Real gc-arena sweeps the same list [1] |
| pressure: B −30% | **Representation**, specifically the allocator. A spends 40% of the scenario in the system allocator (one `malloc` and one `free` per object); B's arena push is amortized | Profile `scenario-pressure-a` |
| memory: B −13% per node; −35% to −44% GC-heap bytes | **Representation.** No 24-byte header, 4-byte handles, no `RefCell` borrow flags, no allocator rounding | §5.4 |

**Instrumentation overhead, common to both.** Both DOMs carry the S1 oracle's thread-local live counters, which are updated on every node allocation and drop. The profiles put their cost at 4% (A) and 12% (B) of the allocation-heavy `pressure` scenario. Without them, B's lead there would be slightly larger.

**On roots.** The collector numbers depend on how roots relate to allocation order. Here the driver's id table roots every live node, so marking starts in id order. B's cost at 256k elements was 33 ns per object with breadth-first trees, against 44 ns with document-order trees; the 33 ns comes from one shakedown run, not the formal method. A real engine's roots (the document, the JS stack and globals) are far fewer.

---

## 7. S1-Q4: build cost

**Measured** by `tools/measure-build.sh --runs 3` (isolated target directory; medians):

| Build | Clean | Incremental (touch the top crate) | `rustc` invocations (clean) | Largest `rustc` RSS | Aggregate compiler peak |
|---|---:|---:|---:|---:|---:|
| A stack, dev | 2.37 s | 0.13 s | 11 | 250 MiB (`syn`) | 204–285 MiB |
| A stack, release | 2.41 s | 0.26 s | 11 | 247 MiB (`syn`) | 290–291 MiB |
| B stack, dev | 0.58 s | 0.13 s | 4 | 169 MiB | 242–271 MiB |
| B stack, release | 0.78 s | 0.32 s | 4 | 167 MiB | 283–294 MiB |
| All of S1, release (`s1-bench`) | 3.58 s | 1.09 s | 15 | 251 MiB (`syn`) | 387–404 MiB |
| Whole workspace, dev, all targets | 4.19 s | 0.72 s | 31 | 253 MiB (`syn`) | 573–734 MiB |

- The A stack's extra clean-build time is the proc-macro toolchain (`syn`, `quote`, `proc-macro2`), a one-time cost. Incremental builds are equal in dev.
- For scale, installing `samply` (281 crates, release) took 83 s wall (411 s CPU), with a largest single `rustc` of 911 MiB and an aggregate compiler peak of 2.0 GiB (tracker §8, CP7).
- S1 gives no sign that build cost threatens iteration on this machine. Evaluation §15.6 assigns the thresholds for revisit condition 3 to S2 (CP8).

---

## 8. Threats to validity

1. **These are spikes, not engines.**
   - A is a stop-the-world simplification of gc-arena, which is incremental [1]: real gc-arena would slice its pauses, at the cost of barrier work.
   - B copies Nova's rooting design [3], but its collector is this spike's own.
   - Neither is tuned. Both run at the default release profile: no LTO, 16 codegen units.
2. **There is no JavaScript interpreter.** Dispatch stands in for "a builtin that calls back into script". How much root traffic an interpreter loop generates under B is **unmeasured**, which is why it is condition C1.
3. **The root set is unrealistic.** The driver roots every live node through its id table, which affects marking order and collector costs (§6).
4. **One machine, uncontrolled background load.** One bench run was disturbed (§2). Medians of three runs are reported, with spreads. Placement on performance or efficiency cores cannot be pinned from safe code.
5. **Synthetic workloads.** They are seeded and validated, with realistic tree shapes, but they are not recorded from real pages.
6. **Allocator-dependent results.** A's per-object allocation runs on macOS's "xzone" malloc, some of whose paths read the clock (`mach_absolute_time`). Its cost shapes A's `build` and `pressure` results.
7. **The code changed during CP7.** Both dispatch implementations dropped incidental allocations. The change was symmetric, re-verified, and measured before and after: B/A went from 2.59 to 2.12 in the shakedown runs (tracker §8, CP7).

---

## 9. Recommendation for ADR-0003 (GC handle and rooting pattern)

**Recommend Pattern B (Nova style)**: typed arenas, `u32` handles bound to scope tokens, an explicit root stack, and a compacting collector. **Confidence: medium.** The decision is yours at CP9.

| Consideration (source) | Pattern A | Pattern B | Favours |
|---|---|---|---|
| Builtins and an interpreter that call back into script (ADR-0001 R2; evaluation §16) | Resumable state machines and a stackless interpreter | Ordinary functions that root what they hold | **B, strongly** |
| `unsafe`, and what a GC bug becomes (R3; evaluation §15.3) | 24 blocks plus 2 macro templates; a tracing bug is undefined behaviour | None; a bug is a stale index | **B** |
| Memory (charter G4) | baseline | 13% less per live node all-in; 13–22% less process memory in every scenario | **B** |
| Mutator CPU (G4) | Faster at cache-friendly pointer chasing (B takes 18–22% longer) and at root-heavy code (B's dispatch takes 2.06× as long) | Faster at allocation (pressure: 30% less time) | **Mixed** |
| Collection and pauses | A two-pass collector that is faster on small heaps; the sweep degrades once the heap outgrows cache | 1.3–2.8× faster from 64k elements (2.4–2.8× at 256k); lower maximum pauses in every scenario. Both are stop-the-world | **B today** |
| Path to incremental or generational collection | Barrier insertion points are already in the API; gc-arena is incremental [1] | Plain writes work only with a stop-the-world collector; barriers would have to be added | **A** |
| Build | `syn` adds 1.8 s to a clean build | No proc-macros | **B, minor** |

**Conditions to settle in ADR-0003, or by follow-up experiments:**

| # | Condition | Hypothesis | Validation |
|---|---|---|---|
| C1 | **Cheaper rooting.** Target at most 1 ns per root and per read, against 2.05 and 1.58 ns measured | A root stack without `RefCell` borrow tracking (an `UnsafeCell`-based stack inside the GC crate, as gc-arena does for its gray queue [1]), with 8-byte slots | `s1-probe roots`, the `dispatch` suite, and an interpreter-shaped micro-benchmark (the evaluation's optional S3) |
| C2 | **Index arithmetic out of the dependent chain** for hot link fields | Power-of-two strides, or struct-of-arrays `u32` link columns, let the scaling fold into the load's addressing mode | The `traverse` suite on the new layout |
| C3 | **A write-barrier design** before any incremental or generational collector | A field-write API for handle-holding fields can be enforced by the type system without A's `unlock!` projection | Count S1-Q2's tokens with the barrier added |
| C4 | **A pause budget.** B's stop-the-world pause is about 4 ms at 130k live nodes and 15 ms at 333k objects | Large pages plus JS heaps will need incremental or generational collection (this needs C3) | Set the budget in the architecture's performance section; measure with a recorded real-page corpus |
| C5 | **Native interop and memory peaks.** Pinning or a non-moving space for objects whose memory reaches platform APIs; 8-byte mark work items | — | S2 (CP8) shows which objects cross into Metal and CoreText; re-run `s1-memory.sh` |

**What would change this recommendation:**
- C1 fails: rooting stays well above 1 ns per operation and dominates an interpreter-shaped workload.
- C3 makes B's write ceremony as heavy as A's, *and* a JS-engine design emerges in which only a small, fixed set of re-entry points must be resumable. Together, these would remove A's main architectural cost.
- You judge B's lexical ceremony unacceptable (S1-Q2).

---

## 10. Implications for ADR-0002 and Axiom

- **The ADR-0002 hypothesis holds.** A shared GC heap can be written in Rust with `unsafe` confined to one GC module (A), or with none at all (B), at measured, explainable cost. Revisit condition 1 does not trigger.
  - **Recommendation:** accept ADR-0002 at CP9, subject to S2.
- **The GC design is now an engine-wide constraint.** The choice in ADR-0003 decides how the interpreter, builtins, DOM and bindings are written. It belongs in the architecture before any engine code exists.
- **New open questions** ([open-questions.md](../open-questions.md)):
  - OQ-13: the incremental or generational GC strategy for B (C3, C4);
  - OQ-14: the root-stack design (C1);
  - OQ-15: whether S3, the interpreter micro-benchmark, becomes required rather than optional.

---

## 11. Reproducing these results

```sh
tools/s1-bench.sh              # runtime suites: 3 runs, 60 s cool-downs; summary.md
tools/s1-memory.sh             # peak footprint, RSS, instructions per process
tools/s1-profile.sh            # samply profiles + line-level summaries
tools/s1-ceremony.sh           # GC-API tokens and code size
target/release/s1-probe roots  # rooting unit costs (after cargo build --release -p s1-bench)
target/release/s1-probe sizes  # object sizes
tools/measure-build.sh -p s1-a-dom [--release] --runs 3   # likewise s1-b-dom, s1-bench
```

Results land in `target/s1-results/` and `target/measure/`, which are not committed. The runs behind this note were:

| Kind | Directory |
|---|---|
| Runtime suites | `bench-20261017-070142` |
| Memory | `memory-20261017-070549` |
| Profiles | `profiles-20261017-070732` |
| Builds | `target/measure/20261017-0710*` to `20261017-0713*` |

---

## Sources

1. gc-arena, `src/context.rs`: the intrusive `all` list, the `Phase` enum (Mark, Sweep, Sleep, Drop), allocation-debt pacing, the per-object sweep, backward and forward write barriers, and the `UnsafeCell`-based `Queue`. https://github.com/kyren/gc-arena/blob/master/src/context.rs (checked 2026-10-17)
2. piccolo, README: a stackless VM, because "no garbage collection can take place during a call to `mutate`"; callbacks as `Sequence`s polled by an `Executor`. https://github.com/kyren/piccolo (checked 2026-10-17)
3. Nova, `nova_vm/src/engine/rootable/scoped.rs`: `agent.stack_refs.borrow_mut()` plus `push`, and `stack_refs.borrow().get(…)`. https://github.com/trynova/nova/blob/main/nova_vm/src/engine/rootable/scoped.rs (checked 2026-10-17)
4. Nova blog, "Garbage collection is contrarian" (2026-01-09, updated 2026-01-11): the bind/unbind "soup", "nearly 800 places", the 24 spare handle bits, and gc-arena as a sound alternative. https://trynova.dev/blog/garbage-collection-is-contrarian (checked 2026-10-17)
5. samply 0.13.1. https://github.com/mstange/samply
6. This repository: the [Step 2b tracker](step-2b-validation-spikes.md) §8 (all checkpoint results), `spikes/s1/*`, and `tools/s1-*.sh`.
