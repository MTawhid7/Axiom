# S3 Results: Rooting, Dispatch, Layout and Write Barriers on the M1

| | |
|---|---|
| **Status** | **Complete** (2026-10-18), for the owner's review at Gate C |
| **Question** | Spike S3 (Step 4 tracker §10.1). **S3-Q1:** can a root stack without borrow tracking cost at most 1 ns per root and per read (ADR-0003's C1)? **S3-Q2:** how much rooting does an interpreter-shaped workload do, and where? **S3-Q3:** how fast is each bytecode dispatch technique on stable Rust? **S3-Q4:** do layouts without index arithmetic speed up traversal (C2)? **S3-Q5:** what do generated setters and a write barrier cost (C3)? |
| **Feeds** | [ADR-0008](../adr/0008-script-agents-and-heap.md) (C1, and the dispatch technique as evidence for J1); [ADR-0003](../adr/0003-gc-handle-and-rooting-pattern.md) (C1–C3); OQ-14; Step 5 (the JavaScript engine) |
| **Plan and raw log** | [Step 4 tracker](step-4-architecture-proposal.md): design §10, per-checkpoint results §11 (CP13–CP15) |
| **Code** | `spikes/s3/*` on branch `step-4`. Measurements ran at commit `05185cb` |
| **Last updated** | 2026-10-18 |

**Evidence labels** follow the [research conventions](README.md):
- **Measured** numbers come from this repository's reproducible benchmarks, with the commands in §10.
- **Observed** facts come from a named command or file on this Mac.
- **Inference** marks an explanation drawn from measurements, profiles or disassembly.
- **Hypothesis** marks an untested claim; each one names its validation step.

---

## Summary

**What was measured.** A copy of S1's Pattern B heap with three interchangeable root stacks:
- S1's `RefCell` design;
- a safe stack of `Cell` chunks;
- C1's stack, `RawStack`, whose only `unsafe` is three Miri-checked blocks.

On top of it runs a register-machine interpreter whose builtins call back into bytecode while holding values, as JavaScript's array methods do, built four ways to compare dispatch techniques. S1's own code ran in the same processes as the baseline. All formal runs were on AC power, from a clean tree.

| Question | Answer |
|---|---|
| **S3-Q1** Can a root stack without borrow tracking cost ≤ 1 ns per root and per read? | **Reads yes; pushes no, as S1's method measures them.**<br>• C1's stack reads a rooted value in **0.51 ns** (S1's: 1.64) and opens and closes a frame in 0.75 ns (S1's: 4.08). Miri finds no undefined behaviour.<br>• A push measures **2.03 ns**, the same as S1's stack. S1's method puts a barrier after every push, which forces any stack to reload its length from memory: a bare counter measures 2.18 ns under it.<br>• Back to back, a push costs 0.56 ns.<br>• S1's dispatch case runs 26% faster, closing about half of Pattern B's gap to Pattern A.<br>**As written, S3-Q1 does not pass** |
| **S3-Q2** How much rooting does an interpreter-shaped workload do, and where? | **Passes.**<br>• Executing bytecode performs no root operations, in any workload.<br>• All of W3's 650,146 root operations are in builtins; 99.99% are reads after callbacks.<br>• With C1's stack, **rooting takes 0.9–1.1% of W3's time**, and none of the other workloads' (threshold 10%). S1's `RefCell` design takes 8–9% by profile, about 13% by timing.<br>• ADR-0003's assumption holds |
| **S3-Q3** Dispatch on stable Rust | **A `match` loop is fastest:** 2.64 ns per instruction on W1. A table of handlers is 54% slower, and handlers that return the next handler 50% slower. Nightly explicit tail calls beat `match` by 5–22%, but rustc flags the feature as incomplete |
| **S3-Q4** C2: layouts without index arithmetic | **Struct-of-arrays `u32` link columns, indexed from 1.**<br>• In cache-friendly orders they traverse 31–36% faster than Pattern B and 12–23% faster than Pattern A.<br>• After random moves they are about three times faster than either.<br>• Power-of-two strides help only while the nodes fit in cache, and cost up to 22% beyond it |
| **S3-Q5** C3: generated setters and barriers | **Free without a barrier; about 3% with card marking.**<br>• A setter without a barrier costs what a plain write does: 1.26 against 1.28 ns.<br>• Card marking adds 0.03–0.05 ns per write, and 2.6–3.1% on S1's mutation case.<br>• Setters replace writes one for one. The ceremony they add is per-type declarations |

**Recommendation for ADR-0008 at Gate C: accept it, with C1 met by `RawStack`.** Confidence: medium to high.
- **The owner decides.** The pre-approved condition was that "S3-Q1 and S3-Q2 pass", and S3-Q1's push criterion fails as written.
- **The failing figure measures the method,** not the stack. It is the same for S1's `RefCell` stack, the stack C1 was meant to improve on.
- **What the criterion was for is met.** Reads are 99.99% of W3's root operations, and they cost 0.51 ns. Rooting takes about 1% of that callback-heavy workload.
- **ADR-0003's revisit condition is not met.** It needs rooting to stay well above 1 ns *and* dominate an interpreter-shaped workload.

---

## 1. What was built and verified

| Crate | Contents | `unsafe` |
|---|---|---|
| `s3-gc` | S1's Pattern B heap (`s1-b-gc` stays untouched), plus NaN-boxed values with `u32` handles inside; **three root stacks behind one `RootStack` trait**; a write-barrier hook and a `setters!` macro | 3 blocks, all in `src/raw/` (`RawStack`), under invariants R1–R3 |
| `s3-vm` | A register machine with 28 operations; builtins `forEach`, `map` and `sortBy`, and a `valueOf`-style coercion hook, all calling back into bytecode; four dispatch techniques; a reference evaluator; workloads W1–W4 with native models | None (`forbid`) |
| `s3-bench` | S1's harness, generalized to any number of variants; `s3-probe` and `s3-count`; S1's Pattern B DOM ported to `s3-gc`; four node layouts | None (`forbid`) |

**The three root stacks:**

| Stack | Storage | A rooted value is found by |
|---|---|---|
| `RefCellStack` | `RefCell<Vec<Value>>`: S1's borrow-tracked design, with 8-byte slots (S1's were 24) | the stack and an index; every access checks and updates the borrow flag |
| `CellStack` | chunks of 1,024 `Cell<Value>` slots, allocated on first use and never moved | a reference to its slot |
| `RawStack` (C1) | one fixed-capacity array of `Cell<Value>` slots, allocated zeroed so its pages are committed lazily: 1,048,576 slots reserve 8 MiB of address space, and an 8 MiB stack holding 1,000 roots adds 32 KiB to a process's peak footprint (observed, `/usr/bin/time -l`) | a reference to its slot |

In all three, frames borrow the stack rather than share it through S1's `Rc`, so opening and closing one costs no reference counting. `CellStack` was added during CP13: ADR-0008 says the `gc` crate's only `unsafe` is the root stack "only if C1 needs it", and a stack that is safe and free of borrow tracking tests exactly that.

**The interpreter** (`s3-vm/src/bytecode.rs` describes it):
- **Registers.** A frame has 256 registers in a window over one register file of `Cell<Value>`s. The collector traces the file's used prefix as one region, so **executing bytecode roots nothing value by value** (ADR-0008 SH6).
- **Calls** are Lua's: a callee's frame starts just above its arguments, which are never copied.
- **Builtins** are ordinary Rust in Pattern B's style. What they hold across a callback, which may collect, they root in a frame and read back after every callback; the scope tokens make the compiler enforce this for handles.
- **Dispatch techniques**, sharing every operation's semantics:

  | Technique | Next instruction | Where `pc` and the window live |
  |---|---|---|
  | `match` | one `match` on the opcode: a jump table with one shared indirect branch | locals |
  | `table` | an indirect call through a 256-entry table of handlers, which return to the loop | a state struct |
  | `threaded` | each handler returns the next instruction's handler, stored in the decoded instruction; a trampoline calls it | a state struct |
  | `tail` (nightly) | each handler tail-calls the next with `become`: one indirect branch per handler | arguments |

**The workloads** (`s3-vm/src/workloads.rs`; bench sizes):

| | Exercises | Instructions |
|---|---|---:|
| W1 | Collatz step counts for 1–10,000: dispatch and integer arithmetic, no allocation | 7,737,003 |
| W2 | 300 linked lists of 1,000 objects, built and walked: allocation, slot access, 36 collections | 5,104,207 |
| W3 | `map`, `sortBy` and `forEach` over 5,000 elements with closures that read and write their environment; then 5,000 objects summed and sorted through their coercion hook | 703,548 |
| W4 | a random tree of 5,000 objects, recursive sums, 5,000 subtree moves with ancestor checks | 1,697,643 |

**Verification** (tracker §11, CP13):
- **The oracle.** Every workload, and 34 edge-case programs, give the reference evaluator's result on every technique and stack, at default pacing and when collecting at every allocation, with debug epochs checking every handle. The reference evaluator is a separate implementation of the same semantics: doubles only, a heap that never collects, and poisoned registers wherever a program must not read. It agrees with every workload's native model.
- **Counted builds** show that bytecode performs no root operations, and that the interpreter executes exactly the reference evaluator's instructions.
- **The oracle catches rooting bugs.** A builtin changed to hold its callback across callbacks fails with a stale-handle panic.
- **Miri** (Stacked and Tree Borrows, strict provenance) finds no undefined behaviour or leak in `s3-gc`'s 44 tests, or in the VM running every workload on `RawStack` with a collection at every allocation.

---

## 2. Method and environment

**Machine.** MacBook Air M1 (4 performance + 4 efficiency cores, 8 GB, fanless), macOS 27.2, **on AC power with Low Power Mode off**; Rust 1.94.1, release profile; the pinned nightly (1.96.0, 2026-04-08) for the tail-call technique. No thermal or performance warning was recorded before or after any run (`pmset -g therm`).

**A first formal run was discarded.** It ran on battery, where this Mac's power settings enable Low Power Mode (`pmset -g custom`: `lowpowermode 1` on battery, `0` on AC). Every timing came out about 1.64 times slower, S1's own frame cost included (6.69 ns against its reproducible 4.08), and the shortest micro-benchmarks spread widely. Its ratios broadly agree with the reported run's; its absolute values are not used. It remains in `target/s3-results/bench-20261018-001555`.

**Runtime.** `tools/s3-bench.sh`: three separate process runs, each after a 60 s cool-down. Within each case:
- a warm-up of 300 ms, or 500 ms for whole workload runs;
- then 11 samples per variant, rotating through the variants and reversing the order every other round (for two variants, S1's ABBA).

Reported values are **the median of the three run medians**; "±x%" is half the range of the run medians, relative to that median. Root-stack unit costs use S1's exact loops (`s1-probe roots`: 65,536 operations per sample, pushes in frames of 64), and S1's own stack is measured by S1's own code in the same process.

**QoS.** The plan asked for user-interactive QoS. `taskpolicy` can only lower a process's QoS, and raising it needs a pthread call, which would have added `unsafe` FFI to the benchmark. The processes ran at the default QoS of a command-line process, as S1's did. That they ran on the performance cores at full speed is shown by S1's frame cost, reproduced in-process at S1's published value.

**Counts.** `s3-count`, built with the `count` feature, counts executed instructions per opcode, calls, collections, and root operations by context (bytecode, builtin, coercion hook). Counts are deterministic.

**Profiles.** `tools/s3-profile.sh`: `samply` at 1 kHz, 5 s per case, narrowed to the profiled loop and resolved to source lines with `atos`. The **share of time rooting** is the share of samples whose leaf address has, among its inlined frames, a frame in the root-stack code (`s3-gc/src/roots.rs` or `src/raw/`), wherever that code was inlined (`samply-summary.py --share`).

**S1's references.** S1's own `dispatch` and `traverse` suites were re-run in the same session (`tools/s1-bench.sh`), for Pattern A's times under the same conditions.

**Ceremony.** `tools/s3-ceremony.sh` applies S1-Q2's token counts to S1's Pattern B DOM and to its port with generated setters.

---

## 3. S3-Q1: the root stacks (C1)

### 3.1 Unit costs

**Measured** (`tools/s3-bench.sh`, S1's loops; median of 3 run medians, every spread 4% or less):

| Operation | S1's stack (`Rc<RefCell<Vec>>`, 24-byte slots) | `RefCellStack` | `CellStack` | `RawStack` (C1) | C1's target |
|---|---:|---:|---:|---:|---:|
| Open and close a frame | 4.08 ns | 3.50 ns | **0.53 ns** | **0.75 ns** | — |
| Root a handle (S1's method: a barrier after each push) | 2.05 ns | 2.03 ns | 2.32 ns | **2.03 ns** | ≤ 1.0 ns |
| Read it back | 1.64 ns | 1.42 ns | **0.50 ns** | **0.51 ns** | ≤ 1.0 ns |
| *Supplementary:* 64 pushes back to back, one barrier | 0.90 ns | 0.71 ns | 0.86 ns | **0.56 ns** | — |

S1's own stack reproduces its published figures in the same process: 4.08 and 2.05 ns exactly, and 1.64 against 1.58 ns for a read.

- **Reads: C1 cuts the cost from 1.64 to 0.51 ns**, 3.2 times less, well under the target. A read is now one load through the reference that `Scoped` keeps. `CellStack` matches it with no `unsafe`.
- **Frames: from 4.08 to 0.5–0.75 ns,** with no reference count and no borrow flag.
- **Pushes, by S1's method, stay at 2.03 ns,** S1's figure, on every stack, `RefCell` or not. The cause is the method, not the stack:
  - **Disassembly** (inference): `RawStack`'s push loop is 12 instructions. Its critical path is the load of the stack length that the previous push stored.
  - **Why it must reload:** S1's loop puts `black_box` after every push, a barrier the compiler must assume can read and write any memory whose address has escaped. The stack's address has escaped, as it must: the heap and the frames refer to it.
  - **Calibration** (observed, a 30-line program on this Mac): a bare counter whose address has escaped costs **2.18 ns per increment** under the same barrier; one whose address has not escaped costs 0.67 ns.
  - So S1's method measures a store-to-load round trip of about 6.5 cycles per push, at the performance cores' 3.2 GHz (background). Any stack that keeps its top in memory pays it.
- **Back to back, a push costs 0.56 ns on C1's stack.** This is a builtin rooting its arguments on entry: the length stays in a register between pushes. The 64-push loop is the supplementary measurement added at CP13 (tracker §11).

### 3.2 S1's dispatch case

**Measured:** S1's `dispatch` case (20,000 elements, 60,000 listeners, 50,000 events, 682,346 listener invocations), on S1's Pattern B and on the port of it to `s3-gc`. The port differs only in its root stack (borrowed, with 8-byte slots) and in generated setters, which cost nothing (§7). Pattern A from S1's own suite in the same session.

| | ns per dispatch | × Pattern A |
|---|---:|---:|
| Pattern A (S1's suite) | 472 | 1.00 |
| Pattern B (S1's suite) | 1,002 | 2.12 |
| Pattern B (inside `s3-bench`) | 1,017 | 2.15 |
| Port on `RefCellStack` | 932 | 1.97 |
| Port on `CellStack` | 847 | 1.79 |
| **Port on `RawStack` (C1)** | **753** | **1.59** |

**C1 cuts Pattern B's dispatch by 25–26% and closes about half (47–48%) of its gap to Pattern A.** The ranges depend on whether Pattern B is measured in S1's suite or in `s3-bench`. Borrowing the stack instead of sharing it through an `Rc`, with 8-byte slots, accounts for about 16% of the gap (`RefCellStack`). Removing the borrow flag accounts for the rest of C1's gain. Profiles put rooting at **13% of the port's dispatch time on `RawStack`**, against 29% on `CellStack`, whose push looks up a chunk, and 34% on `RefCellStack`. Most of what remains of the gap to Pattern A is the loop that builds each listener snapshot, and the listener lookups (inference from the line profile).

### 3.3 Verdict

S3-Q1 passes if "the C1 stack measures ≤ 1.0 ns per root and ≤ 1.0 ns per read, hot, and its module is Miri-clean" (tracker §10.4).

| Part | Result | |
|---|---|---|
| Per read | 0.51 ns | **Pass** |
| Miri | No undefined behaviour, Stacked and Tree Borrows | **Pass** |
| Per root, by S1's method | 2.03 ns | **Fail** |

**As written, S3-Q1 does not pass.** The failing part measures a store-to-load round trip that the method imposes on every stack whose top lives in memory: S1's `RefCell` stack measures the same 2.05 ns. Measured back to back, without that barrier, C1's push costs 0.56 ns. Whether that satisfies C1 is the owner's decision (§9).

## 4. S3-Q2: rooting traffic

### 4.1 Where root operations happen

**Measured** (`s3-count`, bench sizes; identical for all three techniques, and for all three stacks):

| | Instructions | Root operations: in bytecode | in builtins | in hooks | per instruction |
|---|---:|---:|---:|---:|---:|
| W1 | 7,737,003 | 0 | 0 | 0 | 0 |
| W2 | 5,104,207 | 0 | 0 | 0 | 0 |
| W3 | 703,548 | 0 | 650,146 | 0 | 0.924 |
| W4 | 1,697,643 | 0 | 0 | 0 | 0 |

- **Executing bytecode performs no root operation** in any workload. Registers are traced as one region, and even W4's 44,924 closure calls and W2's 36 collections need no rooting.
- **All of W3's root operations are in builtins:** 6 frames (one per builtin call), 16 pushes (the arrays, callbacks, results and scratch arrays they hold), and **650,124 reads**. Reads dominate because a builtin re-reads its roots after every one of its 251,745 callbacks into bytecode.
- **The coercion hook roots nothing**, although it is called 118,372 times from inside arithmetic and comparisons. The interpreter re-reads the other operand from its register after the hook returns, instead of holding it.

### 4.2 Time spent rooting

**Measured** (`tools/s3-profile.sh`: the share of samples whose inlined frames include root-stack code; about 5,000 samples per case):

| Workload and stack | `match` | `table` | `threaded` |
|---|---:|---:|---:|
| W1, W2, W4, on every stack | 0.0% | 0.0% | 0.0% |
| **W3 on `RawStack` (C1)** | **0.9%** | **1.1%** | **1.0%** |
| W3 on `CellStack` | 1.1% | 1.2% | 0.9% |
| W3 on `RefCellStack` (S1's design) | 8.5% | 9.0% | 8.1% |

Two cross-checks, both inference:
- **An upper bound for C1.** W3's 650,124 reads at the probe's cost for an isolated read, 0.51 ns, would take 0.33 ms of its 4.88 ms run: 6.8%. In the builtins' loops reads overlap other work, so the profile's 1% is plausible, and even the bound is under the threshold.
- **`RefCell`'s cost is larger than its profile share.** W3 runs 1.01 ns per instruction slower on `RefCellStack` than on `RawStack`: 0.71 ms, 12.7% of its run, above the threshold. Profiles charge only the instructions inlined from the root-stack code, so knock-on costs land in the code around them.

**Verdict: S3-Q2 passes.** Executing bytecode performs no per-value root operations, and with C1's stack rooting takes at most 1.1% of any workload's time, against the owner's 10% threshold. **ADR-0003's assumption holds:** rooting traffic comes from builtins, not from executing bytecode. In this workload it is almost entirely the re-reading of roots after callbacks: 650,124 reads against 16 pushes.

## 5. S3-Q3: dispatch

**Measured** (ns per executed instruction, `RawStack`; median of 3 run medians, spreads ≤ 1% except where shown). The stable build runs three techniques; the nightly build adds `tail` and re-measures `match`, so `tail` is compared with the same compiler:

| Technique | W1 (arithmetic) | W2 (allocation) | W3 (callbacks) | W4 (calls, slots) |
|---|---:|---:|---:|---:|
| `match` (Rust 1.94.1) | **2.64** | **2.03** | **6.94** | **2.60** |
| `table` | 4.07 (+54%) | 3.35 (+65%) | 7.78 (+12%) | 3.66 (+41%) |
| `threaded` | 3.96 (+50%) | 3.10 (+53%) | 7.79 (+12%) | 3.46 (+33%) |
| `match` (nightly 1.96) | 2.70 | 2.17 | 7.04 | 2.77 |
| `tail` (nightly, `become`) | **2.50 (−7%)** | **1.78 (−18%)** | **6.67 (−5%)** | **2.17 (−22%)** |

Percentages compare with `match` from the same compiler. The root stack makes no difference here, except in W3 (§4.2).

- **On stable Rust, the `match` loop is fastest, by a third to two thirds** wherever bytecode dominates (W1, W2, W4). Inference: `match` keeps `pc`, the register window, the code and the constants in registers inside one function, behind one shared indirect jump. `table` and `threaded` call a function per instruction and keep that state in a struct in memory, so every handler loads and stores it. A table lookup (`table`) or a load of the next handler from the decoded instruction (`threaded`) adds little beside that.
- **W3 compresses the differences** (12%): most of its time is in native calls back into bytecode, not in dispatch.
- **Explicit tail calls are fastest:** 5–22% faster than `match` from the same compiler, and 4–17% faster than stable `match`. Each handler ends in its own indirect branch, and the state stays in argument registers. But `become` is a nightly feature that rustc itself flags as "incomplete and may not be safe to use and/or cause compiler crashes" (issue #112788).
- **Scale:** 2.6 ns per instruction is about 8 cycles at 3.2 GHz. The operations check types, overflow and bounds in safe Rust. Step 5's interpreter design (instruction set, value checks, inline caches) is where that comes down, not the dispatch technique.

## 6. S3-Q4: node layouts (C2)

**Measured** (ns per node, S1's four traversal cases; median of 3 run medians, spreads ≤ 3%). Pattern A and B from S1's own suite in the same session:

| | 10k, document order | 100k, document order | 100k, breadth-first | 100k, after 100k moves |
|---|---:|---:|---:|---:|
| Pattern A (S1) | 6.5 | 7.1 | 8.5 | 44.3 |
| Pattern B (S1) | 8.0 | 9.1 | 9.4 | 40.4 |
| Port (S1's code on `s3-gc`) | 8.63 | 9.25 | 9.45 | 41.00 |
| `aos104`: 104 bytes, `raw − 1` | 7.48 | 8.25 | 8.42 | 47.04 |
| `aos104-1b`: 104 bytes, 1-based | 7.05 | 7.65 | 7.84 | 46.56 |
| `aos128-1b`: 128 bytes, 1-based | 6.25 | 9.30 | 7.16 | 54.93 |
| **`soa-1b`: `u32` link columns, 1-based** | **5.13** | **6.23** | **6.53** | **14.28** |

The four layouts share one payload per node (a precomputed kind code), so `aos104` is faster than the port for reasons other than layout. Comparisons among the four layouts isolate it.

- **One-based indexing alone saves 6–7%** in the cache-friendly orders, and 1% after random moves (`aos104` → `aos104-1b`). A dummy element 0 lets a handle's stored value (index + 1) index directly, removing the subtraction from the chain.
- **A power-of-two stride helps only while the nodes fit in cache.** At 10k nodes (1.7 MB) it saves another 11%. With 100k document-ordered nodes it costs 22%, and after random moves 18%: 128-byte nodes take 16.6 MB, against 13.5 MB at 104 bytes, beyond the performance cores' shared 12 MiB L2 (observed, `sysctl hw.perflevel0.l2cachesize`).
- **Struct-of-arrays link columns are fastest in every case:**
  - **31–36% faster than Pattern B, and 12–23% faster than Pattern A,** in the three cache-friendly orders.
  - **About three times faster than either after random moves:** 14.3 ns against 40.4 (B) and 44.3 (A). Inference: the three link columns hold 4 bytes per node, 1.6 MB for 130,000 nodes, and stay in L2 when hops are random, while 104-byte nodes do not.
  - Each hop's scaling is the load's own addressing mode, so no arithmetic sits between dependent loads.

## 7. S3-Q5: generated setters and barriers (C3)

### 7.1 Ceremony

**Measured** (`tools/s3-ceremony.sh`), comparing S1's Pattern B DOM, where handle-valued fields are public and written by assignment, with its port, where they are private, read through getters, and written through setters that `s3_gc::setters!` generates, each calling the heap's barrier:

| Handle-valued fields, at call sites | S1 | Port |
|---|---:|---:|
| Link writes: `x.link = …` → `heap.set_link(x, …)` | 14 | 14 |
| Listener-list writes: `listeners.push`/`remove` → `push_listener`/`remove_listener_at` | 2 | 2 |
| Link reads: `x.link` → `x.link()` | 10 | 10 |
| Declarations | — | 5 getters, 5 setter entries, 3 constructors |

S1-Q2's GC-API tokens are unchanged (the scope tokens, `bind`, frames, `scope`, `get`, safepoints, tracing), except `.unbind()`, 7 → 11, inside the new constructors and listener methods. The whole file grows from 416 to 509 lines, almost all of it the accessor declarations; the operations themselves change by a few lines each (`remove` 19 → 21, `insert_before` 29 → 28, `add_listener` 28 → 23, `remove_listener` 7 → 11, `dispatch` unchanged at 74).

**Inference:** generated setters replace each write one for one, so the ceremony a barrier API adds is the declarations, written once per type, not a cost at every write.

### 7.2 What a write costs

**Measured** (median of 3 run medians):

| | One handle write (65,536 per sample) | S1's `mutate` case (100k moves, inserts, removes, attribute writes) |
|---|---:|---:|
| S1: a plain field write | 1.28 ns (±1%) | 66.8 ns per op (±3%) |
| Setter, no barrier | 1.26 ns (±1%) | 66.5 ns (±3%) |
| Setter, card marking | 1.31 ns (±5%) | 68.5 ns (±3%) |

- **A generated setter without a barrier costs nothing measurable.** It compiles to the same store.
- **Card marking costs 0.03–0.05 ns per write**, within the probe's spread, **and 2.6–3.1% on the mutation case**, about its spread. The ranges depend on whether the baseline is S1's plain write or the barrier-free setter. A card mark is a shift, a bounds check and a byte store, all off the critical path.

---

## 8. Limitations

1. **The interpreter is JavaScript-shaped, not JavaScript.** There are no strings, prototypes, exceptions or inline caches, and objects have four fixed slots. Rooting traffic depends on how builtins are written, and a real engine has hundreds of them.
2. **W3 does not collect at default pacing** (its allocations stay under the 256 KiB budget). Its builtins root and re-read as if it might, as Pattern B requires, so the cost measured is the real cost; correctness under collection mid-builtin is covered by the stress-pacing oracle.
3. **Values carry no lifetime.** A value held across a collection is caught by debug epochs under stress tests, not by the compiler. The engine's value type should carry the scope lifetime, as handles do.
4. **Profile shares count code inlined at the sampled address.** Time in a separate compiled function that root-stack code calls (a vector's out-of-line growth, in `RefCellStack`) is not counted as rooting.
5. **One machine, one core type, one compiler.** Sub-nanosecond costs also move with code alignment: the same `RawStack` frame loop measured 0.54 and 0.75 ns in development builds that differed only elsewhere.
6. **The tail-call technique needs an unstable feature** that the pinned nightly flags as "incomplete and may not be safe to use and/or cause compiler crashes" (rustc's own warning; issue #112788).

---

## 9. Recommendations

1. **ADR-0008: accept at Gate C, with C1 met by `RawStack`; the owner decides.**
   - The pre-approved condition, "S3-Q1 and S3-Q2 passing", is not met as written: S3-Q2 passes, and S3-Q1's push criterion does not.
   - The push figure measures a store-to-load round trip that S1's method imposes on every root stack, S1's included (§3.1).
   - What C1 was for is met. Reads, 99.99% of W3's root operations, cost 0.51 ns against a 1.0 ns target. Back-to-back pushes cost 0.56 ns. Rooting takes about 1% of the callback-heavy workload (§4.2).
   - ADR-0003's revisit condition 1 requires rooting that "stays well above 1 ns per operation *and* dominates an interpreter-shaped workload", and it does neither for reads.
2. **C1's implementation: `RawStack` rather than `CellStack`.**
   - Both meet the read target, and they are equal on the workloads.
   - `RawStack` pushes faster (0.56 against 0.86 ns back to back), which makes S1's dispatch case 11% faster (753 against 847 ns). Event dispatch and other DOM code root as they go.
   - Its `unsafe` is three blocks that allocate a zeroed array, view it, and free it, under three invariants, checked by Miri under both borrow models.
   - `CellStack` remains the measured fallback without `unsafe`, if the owner prefers none in the `gc` crate. ADR-0008's consequence anticipated this choice: "only if C1 needs it".
3. **C2: SH8 should name one-based struct-of-arrays link columns.** They are the fastest layout in every traversal case, faster than both of S1's patterns. Power-of-two strides should be dropped: they cost up to 22% once the nodes outgrow the cache (§6).
4. **C3: keep SH9 as designed.** Setters cost nothing until a barrier is added, and card marking costs about 3% on mutation-heavy work (§7).
5. **Dispatch for Step 5: a `match` loop,** with operations written as functions over the machine state, as here, so that the technique can change. Explicit tail calls are the measured next step, once `become` is stable and not flagged incomplete (§5).
6. **Two inputs for Step 5's engine design.**
   - **The value type should carry the scope lifetime,** as handles do, so the compiler catches a value held across a collection (limitation 3).
   - **Code that roots many values at once,** like the listener snapshot in event dispatch, can root one heap array instead of each value, as `sortBy` roots its scratch array. That would remove most of dispatch's remaining push traffic (inference from §3.2's profile).
7. **OQ-14** resolves with ADR-0008's decision.

---

## 10. Reproducing these results

```sh
tools/s3-bench.sh                  # every suite: 3 runs, 60 s cool-downs; counts; nightly tail calls
tools/s3-profile.sh                # samply profiles, with rooting, builtin and collector shares
tools/s3-ceremony.sh               # S3-Q5's ceremony counts
tools/s1-bench.sh --filter dispatch   # S1's Pattern A and B, in the same session
tools/s1-bench.sh --filter traverse
target/release/s3-probe roots      # root-stack unit costs (after cargo build --release -p s3-bench)
target/release/s3-probe sizes      # object sizes
cargo test -p s3-gc -p s3-vm -p s3-bench    # the oracle; add --features count for the rooting check
cargo +nightly miri test -p s3-gc           # Miri on C1's unsafe; also -p s3-vm --test miri
```

Results land in `target/s3-results/` and `target/s1-results/`, which are not committed. The runs behind this note were:

| Kind | Directory |
|---|---|
| S3 suites, counts, nightly tail calls | `target/s3-results/bench-20261018-005914` (at `05185cb`, clean tree, AC power) |
| S1's `dispatch` and `traverse` suites, same session | `target/s1-results/bench-20261018-010643`, `bench-20261018-010948` |
| Profiles | `target/s3-results/profiles-20261018-011414` (at `11be3a7`, which changes only `tools/samply-summary.py`) |
| Ceremony | `target/s3-results/ceremony-20261018-002529` |
| Counts | `target/s3-results/counts-05185cb.md` |
| Discarded (Low Power Mode) | `target/s3-results/bench-20261018-001555` |

---

## Sources

1. This repository: the [Step 4 tracker](step-4-architecture-proposal.md) §10–§11, [S1 results](s1-results.md), [ADR-0003](../adr/0003-gc-handle-and-rooting-pattern.md), [ADR-0008](../adr/0008-script-agents-and-heap.md), [script and heap](../architecture/script-and-heap.md), `spikes/s3/*`, and `tools/s3-*.sh`.
2. rustc 1.96.0-nightly (2026-04-08), the warning for `#![feature(explicit_tail_calls)]`: "the feature `explicit_tail_calls` is incomplete and may not be safe to use and/or cause compiler crashes", pointing to https://github.com/rust-lang/rust/issues/112788 (observed 2026-10-18).
3. samply 0.13.1. https://github.com/mstange/samply
