# S3 Results: Rooting, Dispatch, Layout and Write Barriers on the M1

| | |
|---|---|
| **Status** | **Complete** (2026-10-18), with an addendum (CP16) that measured C1's final stack, `SliceStack`, and corrected §3. ADR-0008 accepted at Gate C. **The build** wasn't recorded (macOS 27.2) (Round 4, RV4-13) |
| **Question** | Spike S3 (Step 4 tracker §10.1). **S3-Q1:** can a root stack without borrow tracking cost at most 1 ns per root and per read (ADR-0003's C1)? **S3-Q2:** how much rooting does an interpreter-shaped workload do, and where? **S3-Q3:** how fast is each bytecode dispatch technique on stable Rust? **S3-Q4:** do layouts without index arithmetic speed up traversal (C2)? **S3-Q5:** what do generated setters and a write barrier cost (C3)? |
| **Feeds** | [ADR-0008](../adr/0008-script-agents-and-heap.md) (C1, and the dispatch technique as evidence for J1); [ADR-0003](../adr/0003-gc-handle-and-rooting-pattern.md) (C1–C3); OQ-14; Step 5 (the JavaScript engine) |
| **Plan and raw log** | [Step 4 tracker](step-4-architecture-proposal.md): design §10, per-checkpoint results §11 (CP13–CP16) |
| **Code** | `spikes/s3/*` on branch `step-4`. CP14's measurements ran at commit `05185cb`, the addendum's at `17775c4` |
| **Last updated** | 2026-10-18 |

**Evidence labels** follow the [research conventions](README.md):
- **Measured** numbers come from this repository's reproducible benchmarks, with the commands in §10.
- **Observed** facts come from a named command or file on this Mac.
- **Inference** marks an explanation drawn from measurements, profiles or disassembly.
- **Hypothesis** marks an untested claim; each one names its validation step.

---

## Summary

**What was measured.** A copy of S1's Pattern B heap with interchangeable root stacks:
- S1's `RefCell` design;
- a safe stack of `Cell` chunks;
- `RawStack`, a contiguous array whose only `unsafe` is three Miri-checked blocks: S3's first candidate for C1, now archived;
- **`SliceStack`, C1's stack since Gate C:** the same contiguous array, borrowed safely from a zeroed buffer, with its length first in the struct; and a control that differs only in putting the length last. The addendum (CP16) added both.

On top of it runs a register-machine interpreter whose builtins call back into bytecode while holding values, as JavaScript's array methods do, built four ways to compare dispatch techniques. S1's own code ran in the same processes as the baseline. All formal runs were on AC power, from a clean tree.

| Question | Answer |
|---|---|
| **S3-Q1** Can a root stack without borrow tracking cost ≤ 1 ns per root and per read? | **Reads yes; pushes no, as S1's method measures them, on every stack.**<br>• C1's stack, `SliceStack`, reads a rooted value in **0.53 ns** (S1's stack: 1.70 ns in the same run) and has no `unsafe`.<br>• By S1's method a push costs **2.12 ns** on `SliceStack`, as on `RawStack` (2.10) and S1's stack (2.12). That figure depends on how the loads and stores around the push are compiled: the same push measured 1.02 ns in a binary whose harness compiled differently (§3.2).<br>• **Field order matters in real code:** in S1's dispatch case `SliceStack` is 13–17% faster than `RawStack`, and 14–20% faster than its control, which differs only in where the length sits. It cuts Pattern B's dispatch time by 36%, closing about 70% of the gap to Pattern A.<br>**As written, S3-Q1 does not pass** |
| **S3-Q2** How much rooting does an interpreter-shaped workload do, and where? | **Passes.**<br>• Executing bytecode performs no root operations, in any workload.<br>• All of W3's 650,146 root operations are in builtins; 99.99% are reads after callbacks.<br>• With C1's stack, **rooting takes 0.8–1.0% of W3's time**, and none of the other workloads' (threshold 10%). S1's `RefCell` design takes 8–10% by profile, about 14% by timing.<br>• ADR-0003's assumption holds |
| **S3-Q3** Dispatch on stable Rust | **A `match` loop is fastest:** 2.64 ns per instruction on W1. A table of handlers is 54% slower, and handlers that return the next handler 50% slower. Nightly explicit tail calls beat `match` by 5–22%, but rustc flags the feature as incomplete |
| **S3-Q4** C2: layouts without index arithmetic | **Struct-of-arrays `u32` link columns, indexed from 1.**<br>• In cache-friendly orders they traverse 31–36% faster than Pattern B and 12–23% faster than Pattern A.<br>• After random moves they are about three times faster than either.<br>• Power-of-two strides help only while the nodes fit in cache, and cost up to 22% beyond it |
| **S3-Q5** C3: generated setters and barriers | **Free without a barrier; about 3% with card marking.**<br>• A setter without a barrier costs what a plain write does: 1.26 against 1.28 ns.<br>• Card marking adds 0.03–0.05 ns per write, and 2.6–3.1% on S1's mutation case.<br>• Setters replace writes one for one. The ceremony they add is per-type declarations |

**Decisions at Gate C** (2026-10-18; the owner's words are in the [tracker](step-4-architecture-proposal.md) §2 and ADR-0008's decision history):
- **C1's implementation is `SliceStack`:** "The `gc` crate remains strictly `#![forbid(unsafe_code)]`." `RawStack` stays in the spike, archived as benchmark history.
- **ADR-0008 is accepted** "immediately upon completion of the formal addendum measurements", and Step 4 merges into `main`.
- **S3-Q1's push criterion still fails as written.** The addendum corrects its explanation (§3.2). CP15 said S1's method imposes the same 2 ns round trip on any stack that keeps its top in memory. In fact the cost depends on how the loads and stores around the push are compiled, which is also why field order changes real code (§3.3).

---

## 1. What was built and verified

| Crate | Contents | `unsafe` |
|---|---|---|
| `s3-gc` | S1's Pattern B heap (`s1-b-gc` stays untouched), plus NaN-boxed values with `u32` handles inside; **five root stacks behind one `RootStack` trait**; a write-barrier hook and a `setters!` macro | 3 blocks, all in `src/raw/` (the archived `RawStack`), under invariants R1–R3. C1's `SliceStack` has none |
| `s3-vm` | A register machine with 28 operations; builtins `forEach`, `map` and `sortBy`, and a `valueOf`-style coercion hook, all calling back into bytecode; four dispatch techniques; a reference evaluator; workloads W1–W4 with native models | None (`forbid`) |
| `s3-bench` | S1's harness, generalized to any number of variants; `s3-probe` and `s3-count`; S1's Pattern B DOM ported to `s3-gc`; four node layouts | None (`forbid`) |

**The root stacks:**

| Stack | Storage | A rooted value is found by |
|---|---|---|
| `RefCellStack` | `RefCell<Vec<Value>>`: S1's borrow-tracked design, with 8-byte slots (S1's were 24) | the stack and an index; every access checks and updates the borrow flag |
| `CellStack` | chunks of 1,024 `Cell<Value>` slots, allocated on first use and never moved | a reference to its slot |
| `RawStack` (archived) | one fixed-capacity array of `Cell<Value>` slots, allocated zeroed so that its pages are committed lazily; its `unsafe` allocates, views and frees the array | a reference to its slot |
| **`SliceStack` (C1)** | one contiguous array of 8-byte `Cell<u64>` slots, borrowed from a zeroed buffer that its owner allocates (`slot_buffer`), so it needs no `unsafe`; the length comes first in the struct (`repr(C)`) | a reference to its slot |
| `SliceStackLenLast` (control) | the same, with the length after the slice | a reference to its slot |

In all of them, frames borrow the stack rather than share it through S1's `Rc`, so opening and closing one costs no reference counting. `CellStack` was added during CP13: ADR-0008 says the `gc` crate's only `unsafe` is the root stack "only if C1 needs it", and a stack that is safe and free of borrow tracking tests exactly that.

**`SliceStack` and its control were added by the CP16 addendum.** A probe at Gate C showed that a safe stack can have `RawStack`'s layout:
- `Cell::from_mut` and `Cell::as_slice_of_cells` turn a unique borrow of a zeroed buffer into shared cells, so the borrow checker enforces what `RawStack`'s invariants R1–R3 state in comments.
- The push compiles to the same instructions (§3.2).
- Slots are `u64`s rather than `Value`s because the standard library allocates `vec![0; n]` zeroed only for types it knows to be zero.

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

**Verification** (tracker §11, CP13 and CP16):
- **The oracle.** Every workload, and 34 edge-case programs, give the reference evaluator's result on every technique and stack, at default pacing and when collecting at every allocation, with debug epochs checking every handle. The reference evaluator is a separate implementation of the same semantics: doubles only, a heap that never collects, and poisoned registers wherever a program must not read. It agrees with every workload's native model.
- **Counted builds** show that bytecode performs no root operations, and that the interpreter executes exactly the reference evaluator's instructions.
- **The oracle catches rooting bugs.** A builtin changed to hold its callback across callbacks fails with a stale-handle panic.
- **Miri** (Stacked and Tree Borrows, strict provenance) finds no undefined behaviour or leak in `s3-gc`'s tests, which run on every stack, `SliceStack` included, or in the VM running every workload on `RawStack` with a collection at every allocation.

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

**The addendum (CP16)** re-ran everything at `17775c4`: every suite, with counts and the nightly runs; S1's dispatch suite; the profiles; and the footprints. It differs from CP14 in four ways:
- **The Mac was in use.** The load average was 3.1–5.4 before the runs (`sysctl vm.loadavg`), with VS Code, Chrome and other applications running (observed afterwards with `top`). Absolute times came out higher and noisier.
  - S1's frame cost measured 4.22 ns, against 4.08.
  - S1's dispatch case measured 1,389 ns inside `s3-bench`, against 1,031 ns in S1's own suite four minutes later; at CP14 the two agreed within 2%.
  - Every case rotates its variants sample by sample, so ratios within a run stay comparable. §3.3 compares those, and they reproduce CP14's for the stacks both runs measured.
- **The `vm` suite keeps one stack per workload** across its runs, as an agent keeps its stack; CP14 made a fresh stack per run. That changes at most one page fault or allocation per W3 run (inference).
- **Counts run on `SliceStack`.** They count the operations of frames and scoped values, which are the same on every stack, and they match CP14's exactly.
- **S1's suite and the profiles ran with uncommitted documentation changes in the tree:** Markdown only; the code was `17775c4`.

§5–§7 keep CP14's numbers: the addendum changed nothing they measure, and its run was noisier.

---

## 3. S3-Q1: the root stacks (C1)

### 3.1 Unit costs

**Measured** (`tools/s3-bench.sh`, S1's loops; the addendum's run, median of 3 run medians, every spread 3% or less except one read at 7%):

| Operation | S1's stack (`Rc<RefCell<Vec>>`, 24-byte slots) | `RefCellStack` | `CellStack` | `RawStack` (archived) | `SliceStackLenLast` (control) | **`SliceStack` (C1)** | C1's target |
|---|---:|---:|---:|---:|---:|---:|---:|
| Open and close a frame | 4.22 ns | 3.62 ns | 0.55 ns | 0.77 ns | 0.55 ns | 0.77 ns | — |
| Root a handle (S1's method: a barrier after each push) | 2.12 ns | 2.15 ns | 2.41 ns | 2.10 ns | 2.10 ns | **2.12 ns** | ≤ 1.0 ns |
| Read it back | 1.70 ns | 1.47 ns | 0.52 ns | 0.52 ns | 0.52 ns | **0.53 ns** | ≤ 1.0 ns |
| *Supplementary:* 64 pushes back to back, one barrier | 0.92 ns | 0.73 ns | 0.88 ns | 0.58 ns | 0.58 ns | 0.58 ns | — |

S1's own stack measures 3–4% above its published figures in the same process (4.22 against 4.08 ns per frame, 2.12 against 2.05 per push): the Mac was busier than at CP14 (§2). CP14's quieter run gave the same picture for the stacks it measured: `RawStack` 0.75, 2.03, 0.51 and 0.56 ns; `CellStack` 0.53, 2.32, 0.50 and 0.86 ns.

- **Reads: 0.53 ns on C1's stack,** a third of S1's, as on every stack without borrow tracking. A read is one load through the reference that `Scoped` keeps.
- **Frames: 0.55–0.77 ns** on every stack without borrow tracking. Opening and closing a frame is the same code on all four (read the length; compare and store it), so their differences come from code alignment. In this build `SliceStack` and `RawStack` measure 0.77 ns and the other two 0.55 ns; at CP14 `RawStack` measured 0.75 and `CellStack` 0.53 (limitation 5).
- **Pushes by S1's method: about 2.1 ns on every stack,** C1's included. Back to back, 0.58 ns.

### 3.2 What S1's push measures (the addendum's correction)

CP15 explained the 2 ns this way: S1's loop puts `black_box` after every push, so every push must reload the stack's length, which the previous push stored, and "any stack that keeps its top in memory pays" that store-to-load round trip. The first half holds; the second does not.

**Observed** (`objdump -d` on `target/release/s3-probe` at `17775c4`, and on a probe that compiled `s3-bench`'s push loop as a function of its own in a separate binary; 2026-10-18; the probe's timings are informal, three runs in rotation, and its code is not committed):

| Binary | Stack | How the push reloads the length | S1's push |
|---|---|---|---:|
| `s3-probe` | `RawStack` | `ldp x10, x11, [x28, #0x8]`: one 16-byte paired load of the length and the capacity | 2.10 ns |
| `s3-probe` | `SliceStackLenLast` | `ldp x11, x10, [x22, #0x8]`: the slice's length and the length | 2.10 ns |
| `s3-probe` | `SliceStack` | `ldr x10, [x22]`: the length alone | 2.12 ns |
| probe | `RawStack` | the same `ldp` | 2.00 ns |
| probe | the slice stack in Rust's field order | an `ldp` of the slice's length and the length | 2.00 ns |
| probe | `SliceStack` | `ldr`: the length alone | **1.02 ns** |

- **The previous push's 8-byte store covers only half of a 16-byte paired load.** Wherever the length is reloaded that way, S1's push costs about 2 ns, in both binaries.
- **Loaded on its own, the length can cost half as much,** as in the probe. In `s3-probe` it does not: there the stack's instructions are the same as in the probe, but the harness's are not. `s3-probe` passes the handle and the rooted slot to `black_box` through one stack slot (`str w19, [sp, #0x70]`, then `str x12, [sp, #0x70]`), where the probe uses two.
- **Hypothesis:** the M1 forwards a store to a load quickly only when the load reads exactly what one store wrote, and stalls on partial overlaps. Examples are a paired load half covered by a store, or a 4-byte reload of a slot just written by an 8-byte store and then a 4-byte one. That fits all six rows. It does not explain CP13's calibration counter, a plain 8-byte reload of one 8-byte store, which measured 2.18 ns. Not tested further: it concerns how S1's loop compiles on this core, not the stack's design.
- **So S1's method measures the loads and stores around a push as much as the push itself.** Its 2 ns is not inherent to keeping the length in memory, and field order alone does not remove it from that loop.

### 3.3 S1's dispatch case

**Measured:** S1's `dispatch` case (20,000 elements, 60,000 listeners, 50,000 events, 682,346 listener invocations). It ran on S1's Pattern B inside `s3-bench` and on the port of it to `s3-gc`, which differs only in its root stack (borrowed, with 8-byte slots) and in generated setters, which cost nothing (§7). Pattern A and B also ran in S1's own suite in the same session. Each port is given as a **ratio to Pattern B in the same process,** because this run's absolute times were inflated by other load (§2):

| | × Pattern B, same process: median of the three runs' ratios (range) | CP14 | × Pattern A (inference) |
|---|---:|---:|---:|
| Pattern A (S1's suite) | — | — | 1.00 |
| Pattern B (S1's suite) | — | — | 2.03 (CP14: 2.12) |
| Port on `RefCellStack` | 0.95 (0.93–0.95) | 0.92 | 1.92 |
| Port on `CellStack` | 0.87 (0.82–0.89) | 0.83 | 1.77 |
| Port on `RawStack` | 0.74 (0.74–0.78) | 0.74 | 1.50 |
| Port on `SliceStackLenLast` (control) | 0.78 (0.74–0.80) | — | 1.59 |
| **Port on `SliceStack` (C1)** | **0.64 (0.61–0.67)** | — | **1.30** |

The last column multiplies each ratio by Pattern B's time over Pattern A's in S1's suite (2.03 in this session).

- **C1's stack is the fastest.** In each run it was 13–17% faster than `RawStack` and 14–20% faster than its control (per-run ratios 0.83–0.87 and 0.80–0.86). It cuts Pattern B's dispatch time by 36% and closes about 70% of B's gap to Pattern A, where `RawStack` closes about half (inference from the ratios).
- **Field order alone accounts for the difference:** `SliceStack` and its control run the same code with the length in a different place.
- **The ratios that CP14 also measured reproduce it within 5%:** 0.74 for `RawStack` in both runs, 0.87 against 0.83 for `CellStack`, 0.95 against 0.92 for `RefCellStack`.
- **Where the gain arises is not resolved.** The profiles attribute about the same share of dispatch time to the root-stack code on all three contiguous stacks: 15.2% on `SliceStack`, 13.4% on `RawStack` and 13.8% on the control, against 26.3% on `CellStack` and 30.5% on `RefCellStack`. Most of what remains of the gap to Pattern A is the loop that builds each listener snapshot, and the listener lookups (inference from CP14's line profile).

### 3.4 Lazy commitment

**Observed** (`/usr/bin/time -l target/release/s3-probe footprint <stack> <roots>`, peak memory footprint, three runs each):
- **Baseline:** without a stack, a process peaks at 1,184 KiB.
- **1,000 roots** add 16 KiB on `SliceStack` in every run, and on `RawStack` in two runs of three. On `CellStack` they add nothing measurable: its 8 KiB chunk and directory fit in memory the allocator already held.
- **100,000 roots** add 784 KiB on all three: the slots themselves.

Each stack reserves room for 1,048,576 roots (8 MiB of address space) and commits only what pushes touch. `SliceStack` relies for this on the standard library allocating `vec![0; n]` zeroed. That is an optimization, not a documented guarantee; without it, footprint would grow, but soundness would not suffer (`slot_buffer`'s documentation). CP15's check of `RawStack`, in a separate program, found 32 KiB for 1,000 roots.

### 3.5 Verdict

S3-Q1 passes if "the C1 stack measures ≤ 1.0 ns per root and ≤ 1.0 ns per read, hot, and its module is Miri-clean" (tracker §10.4). C1's stack is now `SliceStack`:

| Part | Result | |
|---|---|---|
| Per read | 0.53 ns | **Pass** |
| Miri | No `unsafe` to check; its tests pass under Miri, with Stacked and Tree Borrows, as `RawStack`'s do | **Pass** |
| Per root, by S1's method | 2.12 ns | **Fail** |

**As written, S3-Q1 does not pass,** as at CP15. What changed is the explanation of the failing part (§3.2), and the stack: C1's stack has no `unsafe`, reads as fast as any, and is the fastest in S1's dispatch case. The owner accepted ADR-0008 with it at Gate C (§9).

## 4. S3-Q2: rooting traffic

### 4.1 Where root operations happen

**Measured** (`s3-count` on C1's `SliceStack`, bench sizes). The counts are identical for all three techniques, and the same as CP14's on `RawStack`: they count the operations of frames and scoped values, which do not depend on the stack.

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

**Measured** (`tools/s3-profile.sh`, the addendum's run: the share of samples whose inlined frames include root-stack code; about 5,000 samples per case):

| Workload and stack | `match` | `table` | `threaded` |
|---|---:|---:|---:|
| W1, W2, W4, on every stack | 0.0% | 0.0% | 0.0% |
| **W3 on `SliceStack` (C1)** | **1.0%** | **1.0%** | **0.8%** |
| W3 on `RawStack` (archived) | 1.3% | 1.1% | 1.1% |
| W3 on `CellStack` | 1.2% | 1.2% | 1.0% |
| W3 on `RefCellStack` (S1's design) | 9.7% | 8.2% | 9.2% |

CP14's run gave 0.9–1.1% on `RawStack`, 0.9–1.2% on `CellStack` and 8.1–9.0% on `RefCellStack`.

Two cross-checks, both inference:
- **An upper bound for C1.** W3's 650,124 reads at the probe's cost for an isolated read, 0.53 ns, would take 0.34 ms of its 5.1 ms run: 6.7%. In the builtins' loops reads overlap other work, so the profile's 1% is plausible, and even the bound is under the threshold.
- **`RefCell`'s cost is larger than its profile share.** W3 runs 1.02 ns per instruction slower on `RefCellStack` than on `SliceStack` (14–15% slower in each run): 0.72 ms, 14% of its run, above the threshold. Profiles charge only the instructions inlined from the root-stack code, so knock-on costs land in the code around them. `SliceStack`, `RawStack` and `CellStack` run W3 within 1.5% of each other in every run.

**Verdict: S3-Q2 passes.** Executing bytecode performs no per-value root operations, and with C1's stack rooting takes at most 1.0% of any workload's time, against the owner's 10% threshold. **ADR-0003's assumption holds:** rooting traffic comes from builtins, not from executing bytecode. In this workload it is almost entirely the re-reading of roots after callbacks: 650,124 reads against 16 pushes.

## 5. S3-Q3: dispatch

**Measured** (ns per executed instruction, `RawStack`; CP14's run, median of 3 run medians, spreads ≤ 1% except where shown). The stable build runs three techniques; the nightly build adds `tail` and re-measures `match`, so `tail` is compared with the same compiler:

| Technique | W1 (arithmetic) | W2 (allocation) | W3 (callbacks) | W4 (calls, slots) |
|---|---:|---:|---:|---:|
| `match` (Rust 1.94.1) | **2.64** | **2.03** | **6.94** | **2.60** |
| `table` | 4.07 (+54%) | 3.35 (+65%) | 7.78 (+12%) | 3.66 (+41%) |
| `threaded` | 3.96 (+50%) | 3.10 (+53%) | 7.79 (+12%) | 3.46 (+33%) |
| `match` (nightly 1.96) | 2.70 | 2.17 | 7.04 | 2.77 |
| `tail` (nightly, `become`) | **2.50 (−7%)** | **1.78 (−18%)** | **6.67 (−5%)** | **2.17 (−22%)** |

Percentages compare with `match` from the same compiler. The root stack makes no difference here, except in W3 (§4.2). The addendum's run (§2) shows the same ordering on every stack, `SliceStack` included, and its nightly runs, which were quieter, reproduce the nightly rows within 3% on `RawStack` and `SliceStack` alike.

- **On stable Rust, the `match` loop is fastest, by a third to two thirds** wherever bytecode dominates (W1, W2, W4). Inference: `match` keeps `pc`, the register window, the code and the constants in registers inside one function, behind one shared indirect jump. `table` and `threaded` call a function per instruction and keep that state in a struct in memory, so every handler loads and stores it. A table lookup (`table`) or a load of the next handler from the decoded instruction (`threaded`) adds little beside that.
- **W3 compresses the differences** (12%): most of its time is in native calls back into bytecode, not in dispatch.
- **Explicit tail calls are fastest:** 5–22% faster than `match` from the same compiler, and 4–17% faster than stable `match`. Each handler ends in its own indirect branch, and the state stays in argument registers. But `become` is a nightly feature that rustc itself flags as "incomplete and may not be safe to use and/or cause compiler crashes" (issue #112788).
- **Scale:** 2.6 ns per instruction is about 8 cycles at 3.2 GHz. The operations check types, overflow and bounds in safe Rust. Step 5's interpreter design (instruction set, value checks, inline caches) is where that comes down, not the dispatch technique.

## 6. S3-Q4: node layouts (C2)

**Measured** (ns per node, S1's four traversal cases; CP14's run, median of 3 run medians, spreads ≤ 3%). Pattern A and B from S1's own suite in the same session:

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

**Measured** (CP14's run, median of 3 run medians):

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
5. **One machine, one core type, one compiler.** Sub-nanosecond costs also move with code alignment: the same `RawStack` frame loop measured 0.54 and 0.75 ns in development builds that differed only elsewhere. **They also move with how the code around them compiles:** the same `SliceStack` push measured 1.02 and 2.12 ns by S1's method in two binaries (§3.2).
6. **The tail-call technique needs an unstable feature** that the pinned nightly flags as "incomplete and may not be safe to use and/or cause compiler crashes" (rustc's own warning; issue #112788).
7. **The addendum's run was noisier than CP14's,** because the Mac was in use (§2). Its dispatch comparisons rest on ratios within each run, and §5–§7 keep CP14's numbers.
8. **Where `SliceStack`'s dispatch gain arises is not resolved** (§3.3), and field-order effects depend on code generation. A compiler upgrade, or different code around the stack, could remove or move them.

---

## 9. Recommendations and decisions

1. **ADR-0008: accepted at Gate C** (2026-10-18), once the addendum's measurements were complete, as the owner instructed.
   - S3-Q1's push criterion does not pass as written (§3.5). S3-Q2 passes (§4).
   - What C1 was for is met: reads cost 0.53 ns against a 1.0 ns target, and rooting takes about 1% of the callback-heavy workload.
   - ADR-0003's revisit condition 1 requires rooting that "stays well above 1 ns per operation *and* dominates an interpreter-shaped workload", and it does neither for reads.
2. **C1's implementation: `SliceStack`,** the owner's decision at Gate C.
   - It has no `unsafe`, so the `gc` crate is `#![forbid(unsafe_code)]`.
   - It reads as fast as `RawStack`, and it is the fastest stack in S1's dispatch case (§3.3).
   - `RawStack` stays in the spike as benchmark history, allowlisted and checked by Miri as before.
3. **C2: SH8 should name one-based struct-of-arrays link columns.** They are the fastest layout in every traversal case, faster than both of S1's patterns. Power-of-two strides should be dropped: they cost up to 22% once the nodes outgrow the cache (§6).
4. **C3: keep SH9 as designed.** Setters cost nothing until a barrier is added, and card marking costs about 3% on mutation-heavy work (§7).
5. **Dispatch for Step 5: a `match` loop,** with operations written as functions over the machine state, as here, so that the technique can change. Explicit tail calls are the measured next step, once `become` is stable and not flagged incomplete (§5).
6. **Three inputs for Step 5's engine design.**
   - **The value type should carry the scope lifetime,** as handles do, so the compiler catches a value held across a collection (limitation 3).
   - **Code that roots many values at once,** like the listener snapshot in event dispatch, can root one heap array instead of each value, as `sortBy` roots its scratch array. That would remove most of dispatch's remaining push traffic (inference from §3.3's profile).
   - **Keep the root stack's length where the compiler loads it on its own, and keep S1's dispatch case, or a successor, as a benchmark.** Field order alone changed that case by 13–17%, through effects that depend on code generation (§3.2–§3.3, limitation 8).
7. **OQ-14 is resolved** with ADR-0008.

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
/usr/bin/time -l target/release/s3-probe footprint slice 1000   # what a stack commits; also none, cell, raw
cargo test -p s3-gc -p s3-vm -p s3-bench    # the oracle; add --features count for the rooting check
cargo +nightly miri test -p s3-gc           # Miri: every stack's tests, and RawStack's unsafe; also -p s3-vm --test miri
```

Results land in `target/s3-results/` and `target/s1-results/`, which are not committed. The runs behind this note were:

| Kind | Directory |
|---|---|
| S3 suites, counts, nightly tail calls (CP14) | `target/s3-results/bench-20261018-005914` (at `05185cb`, clean tree, AC power) |
| S1's `dispatch` and `traverse` suites, same session (CP14) | `target/s1-results/bench-20261018-010643`, `bench-20261018-010948` |
| Profiles (CP14) | `target/s3-results/profiles-20261018-011414` (at `11be3a7`, which changes only `tools/samply-summary.py`) |
| Ceremony | `target/s3-results/ceremony-20261018-002529` |
| Counts (CP14) | `target/s3-results/counts-05185cb.md` |
| S3 suites, counts, nightly tail calls (addendum) | `target/s3-results/bench-20261018-083106` (at `17775c4`, clean tree, AC power, the Mac in use) |
| S1's `dispatch` suite, same session (addendum) | `target/s1-results/bench-20261018-083905` |
| Profiles (addendum) | `target/s3-results/profiles-20261018-084214` (at `17775c4`, with uncommitted Markdown changes only) |
| Discarded (Low Power Mode) | `target/s3-results/bench-20261018-001555` |

---

## Sources

1. This repository: the [Step 4 tracker](step-4-architecture-proposal.md) §10–§11, [S1 results](s1-results.md), [ADR-0003](../adr/0003-gc-handle-and-rooting-pattern.md), [ADR-0008](../adr/0008-script-agents-and-heap.md), [script and heap](../architecture/script-and-heap.md), `spikes/s3/*`, and `tools/s3-*.sh`.
2. rustc 1.96.0-nightly (2026-04-08), the warning for `#![feature(explicit_tail_calls)]`: "the feature `explicit_tail_calls` is incomplete and may not be safe to use and/or cause compiler crashes", pointing to https://github.com/rust-lang/rust/issues/112788 (observed 2026-10-18).
3. samply 0.13.1. https://github.com/mstange/samply
