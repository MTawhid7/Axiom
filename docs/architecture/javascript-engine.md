# The JavaScript Engine: Parser, Bytecode, Interpreter, Objects and Builtins

| | |
|---|---|
| **Status** | **Proposed** (Step 5, CP16). To be decided with part 2's ADRs at CP18, for the owner at Gate C. No Step 4 interface changes (SC1) |
| **Question** | How is Axiom's own JavaScript engine built: the front end, the bytecode and its interpreter, the value type, objects and strings, execution of functions, generators and modules, the standard library with `RegExp`, `Intl` and dates, errors and interrupts, and room for a JIT? What would a WebAssembly engine need from it? |
| **Answers** | Step 5 question Q12 ([tracker](../research/step-5-subsystems.md) §3); the handed-over questions 17 (dispatch), 18 (the value type, rooting, the benchmark) and 19 (ropes and slices), and evidence for 21 (OQ-3) (§2.2); [script and heap](script-and-heap.md) open question 2 |
| **Feeds** | Part 2's ADRs (CP18); CP17 (bindings: the value type, conversions, platform objects); [OQ-3](../open-questions.md), as evidence |
| **Builds on** | [ADR-0003](../adr/0003-gc-handle-and-rooting-pattern.md) (Pattern B); [script and heap](script-and-heap.md) (SH1–SH14); [ADR-0004](../adr/0004-process-model-and-sandbox.md) (J1–J7); [S3 results](../research/s3-results.md); the [rendering pipeline](rendering-pipeline.md) (PL11); the [event loop](event-loop-and-threading.md) (EL9, EL14, EL17, EL24); the [frame tree](frame-tree-and-navigation.md) (FT11); [text and fonts](text-and-fonts.md) (TX1, TX2); [HTML and the DOM](html-and-dom.md) (HD18, HD30); [style](style.md) (ST30); the [dependency policy](../technology/dependency-policy.md) (DP1, DP2, DP5) |
| **Last updated** | 2026-10-19 |

Everything here is **planned**; nothing is implemented. Rules are numbered JE1–JE32. Precedent is sourced (§3); the design is inference unless marked otherwise.

## Summary

- **Everything in the engine is Axiom's own** (DP1 (g)): parser, bytecode, interpreter, object model, builtins, `RegExp` and the collector. It sits on ICU4X's data and algorithms (TX1, TX2) and on Rust's standard library, never on another engine's implementation of its own mechanisms.
- **The front end produces heap-free compiled scripts,** as SpiderMonkey's Stencil does: bytecode, constants and scopes with no heap references, instantiated into an agent's heap on its thread. Compilation can therefore run on the helper pool, functions compile lazily, and a bytecode cache stays possible (JE1–JE4).
- **A register machine with an accumulator,** dispatched by a `match` loop, the fastest technique on stable Rust in spike S3. Frames are windows of one register file, traced as one region, so executing bytecode roots nothing. Generators suspend by moving their registers into the heap (JE5–JE8).
- **The value type is NaN-boxed and carries the scope lifetime,** as Pattern B's handles do and as Nova's `bind` and `unbind` discipline shows, so the compiler rejects a value held across a possible collection (JE9). Objects use shapes, inline caches kept as data in the interpreter, and element kinds; strings use PL11's representation with ropes and slices (JE10–JE13).
- **The standard library is ECMAScript 2026 plus the 2027 draft's finished proposals, Temporal among them** (JE19). `RegExp` is a backtracking engine with a linear-time fallback, interpreted, never compiled to machine code (JE20). `Intl` runs on ICU4X (JE21). Temporal is Axiom's own over ICU4X's calendars and compiled-in time zone data, with `temporal_rs`, which V8 and Boa use, as its differential reference (JE22).
- **Interrupts, limits and debugging** come from SH5's flag and bounded re-entry (JE25–JE27). **Room for a JIT** is the interpreter's feedback, which a later tier in `renderer-jit` would consume under J1–J7 (JE30). **Evidence for OQ-3** is recorded, not decided (JE31). **The oracle is test262**, with differential references for `RegExp`, Temporal and BigInt (JE32).

## 1. Binding inputs

| Input | Consequence here |
|---|---|
| **ADR-0001 R2, DP1 (g):** Axiom's own engine, in one heap with the DOM; parser, bytecode, interpreter, object model, builtins, `RegExp` and collector are core | JE1–JE24; §4 |
| **ADR-0003, SH6–SH9:** Pattern B: typed arenas, `u32` handles, scope tokens, a root stack (`SliceStack`), safe `Trace`, generated setters; the interpreter's registers traced as whole regions | JE7, JE9, JE10 |
| **SH1–SH5:** one heap per agent on its thread; copies, transfers and shared buffers between agents; script only in tasks and microtasks; an atomic interrupt flag | JE16, JE18, JE23, JE26 |
| **SH10, SH11, SH14:** mark–compact now, incremental and generational later; memory that reaches platform APIs lives outside the heap; heaps are budgeted | JE12, JE23, JE28 |
| **SH13, J1–J7:** interpreter first; any JIT only in `renderer-jit`, from L2, on evidence; generated code only in that executable | JE6, JE20, JE30 |
| **S3 results §5, §9:** a `match` loop is fastest on stable Rust; operations as functions over machine state; the value type should carry the scope lifetime; root many values as one heap array; keep S1's dispatch case as a benchmark | JE6, JE9, JE19, JE32 |
| **PL11:** one string representation, 8-bit or 16-bit; names as atoms | JE1, JE12 |
| **EL9, EL14, EL17, EL24:** helper threads never touch a heap; the event loop owns the task and microtask queues; no nested event loops; a deterministic test scheduler | JE3, JE16, JE29, JE32 |
| **TX1, TX2:** ICU4X is the one source of Unicode data and algorithms, compiled in; `Intl` uses its formatters directly, as a foundational dependency | JE20–JE22 |
| **FT11:** scripts and modules are fetched through the document's loader endpoint | JE17 |
| **HD18, HD30:** many values rooted as one heap array; a node's handle is its platform object | JE9, JE11 |

## 2. Specifications

- **ECMA-262:** *ECMAScript 2026*, the 17th edition (June 2026) [E1], and the living draft, *ECMAScript 2027*, dated 2026-10-16 [E2]. Proposals finished for 2027 include **Temporal**, explicit resource management, `Atomics.pause`, and iterator chunking, joining, `includes` and joint iteration [E3]. Those finished for 2026 include `Array.fromAsync`, `Error.isError`, `Math.sumPrecise`, `Uint8Array` base64, iterator sequencing, `JSON.parse` source text access and upsert; those for 2025 include iterator helpers, JSON modules, import attributes, RegExp modifiers and `RegExp.escape`, duplicate named groups, new `Set` methods, `Float16Array` and `Promise.try` [E3].
- **ECMA-402,** *ECMAScript 2027 Internationalization API*, draft dated 2026-10-12 [E4].
- **WHATWG HTML** [H1]: scripts, module scripts, import maps and the host hooks (§8.1); the event loop's microtask checkpoint; CSS and JSON module scripts.
- **test262,** at `d53ebc8b1931` (2026-10-13) [T1].

**Approximate behaviour, kept apart** (SC5): `Error.prototype.stack` has no specification, so its format follows the engines (JE25); `Date.parse` accepts implementation-defined formats beyond the specified one, so the legacy formats follow the engines where they agree (JE22). Everything else is implemented as specified.

## 3. Precedent

Read on 2026-10-19; dates in brackets are the last commits to each file, from GitHub's API.

| Engine | What the sources show |
|---|---|
| **V8** (Chromium) | **Ignition** "is a register machine, with each bytecode specifying its inputs and outputs as explicit register operands"; "a special accumulator register is an implicit input and output register for many bytecodes", which "reduces the size of bytecodes" (2016-08-23) [V1]. **JIT-less mode** (2019-03-13): "all JS user code runs through the Ignition interpreter, and regular expression pattern matching is likewise interpreted"; Speedometer 2.0 "is around 40% slower", and "roughly half of the regression" comes from the regular expression interpreter, then "originally intended as a debugging aid" [V2]. **Pointer compression** reduced "V8 heap size up to 43%" (2020-03-30) [V3]. **Strings:** cons strings and sliced strings each have a minimum length of 13 (`string.h` [2026-10-15]). **Generators** suspend and resume through `SuspendGenerator` and `ResumeGenerator` bytecodes (`bytecodes.h` [2026-10-06]). **A non-backtracking `RegExp` engine** runs in linear time, with a fallback "on excessive backtracks", 50,000 by default (2021-01-11) [V4]. **Temporal** is built on `temporal_rs` through `temporal_capi` (`src/objects/DEPS` [2025-06-10]), with time zone data "either loaded from ICU4C or static data" (`js-temporal-zoneinfo64.h` [2025-09-11]) [V5] |
| **SpiderMonkey** (Firefox) | **Values** are NaN-boxed ("punboxing" on 64-bit), with NaNs canonicalized before storing (`js/public/Value.h` [2026-10-13]). **Stencil:** "the result of parsing and bytecode emission", from which scripts are allocated on the heap; "by decoupling from the GC and other runtime systems, robust caching and speculation systems can be built that are more thread-agnostic"; its `ParserAtom` "does not involve the garbage collector", and a function's `ScriptStencil` "may or may not have bytecode" (`frontend/Stencil.h` [2026-09-07]) [M1] |
| **Nova** | Released 1.0.0 (2026-03-14). A "stack-based bytecode interpreter", with a data-oriented heap of vectors and struct-of-arrays vectors addressed by handle (`ARCHITECTURE.md` [2026-03-14]). Its collector compacts, its handles are unrooted, and the borrow checker enforces scoping through `GcScope`, `NoGcScope`, `bind`, `unbind` and `Scoped` (`GARBAGE_COLLECTOR.md` [2026-03-11]). It "aims to conform fairly strictly to the ECMAScript specification in terms of both code layout and structure" [N1] |
| **Boa** | A Rust engine whose workspace uses ICU4X 2.3 for `Intl`, `temporal_rs` 0.2.6 with `timezone_provider`, `regress` 0.12 with its UTF-16 feature, `num-bigint` 0.5.1, `ryu-js` and `fast-float2` (`Cargo.toml` [2026-10-17]) [B1]. `regress` is "a backtracking regular expression engine implemented in Rust, which targets JavaScript regular expression syntax" (`README.md` [2026-05-02]) [R1] |
| **Brimstone** | "A JavaScript engine written from scratch in Rust", passing ">97% of the ECMAScript language in test262", with a "bytecode VM, heavily inspired by the design of V8's Ignition", a "compacting garbage collector, written in *very* unsafe Rust", and its own `RegExp` engine and parser (`README.md` [2026-06-28]) [S1] |
| **Ladybird** (LibJS) | **Two tiers:** "the bytecode interpreter, whose instructions keep per-site caches and feedback, and this optimizing JIT", a Rust crate compiling on separate threads; "There is no baseline tier. The interpreter's caches are the baseline"; it can be built without the JIT (`JIT/ARCHITECTURE.md` [2026-10-19]). The interpreter's handlers are written in Flap, which compiles them "through semantic analysis, SSA construction and optimization, machine lowering, register allocation, and target-specific assembly emission" (`Flap/src/lib.rs` [2026-10-10]) [L1] |

**Observations** (inference):
- **Register machines dominate:** Ignition, Brimstone (after Ignition) and S3's spike; Nova's stack machine is the exception. An accumulator buys smaller bytecode, which matters for memory (G4).
- **A heap-free compile output is what makes off-thread compilation and caching safe,** in SpiderMonkey's words "more thread-agnostic". EL9 asks exactly that of anything on the helper pool.
- **The interpreter's own speed matters most for `RegExp`:** half of V8's jitless slowdown was its regular expression interpreter. Axiom's `RegExp` must be fast as an interpreter from the start (JE20).
- **Rust engines differ most in their collectors:** Brimstone's is "very unsafe"; Nova's and Axiom's Pattern B are safe, at the cost of scoping discipline that the borrow checker enforces.
- **Even Ladybird's new JIT keeps the interpreter's caches as its only baseline,** so feedback designed for the interpreter (JE8) is the right preparation for any later tier (JE30).
- **V8 reuses `temporal_rs` for Temporal.** DP1 (g) makes builtins Axiom's own, so Axiom uses it as a reference instead (JE22).

## 4. Build or reuse

| Need | Candidates | Decision |
|---|---|---|
| **Parser, bytecode, interpreter, objects, builtins** | Other engines (V8, SpiderMonkey, Boa, Nova) | **Our own** (DP1 (g)) |
| **`RegExp`** | `regress` (Boa's); `regex`, which lacks backreferences and lookaround; **our own** | **Our own** (DP1 (g)); `regress` is the differential reference (JE20) |
| **`Intl`** | ICU4X, already a foundational dependency (TX2) | **ICU4X's formatters,** behind Axiom's `Intl` builtins (JE21) |
| **Temporal** | `temporal_rs`, used by V8 and Boa; **our own** | **Our own** builtins (DP1 (g)) over ICU4X's calendars and compiled-in time zone data; `temporal_rs` is the differential reference (JE22) |
| **BigInt arithmetic** | `num-bigint` (Boa's); **our own** | **Our own** (DP1 (g)); `num-bigint` is a differential reference (JE13) |
| **Number formatting and parsing** | `ryu-js`, `fast-float2` (Boa's); Rust's standard library | **The standard library's** shortest round-trip formatting and correctly rounded parsing (background), under ECMA-262's layout rules (JE13) |
| **Time zone data** | ICU4X's, compiled in; the system's `/usr/share/zoneinfo` | **Compiled in,** like ICU4X's other data (TX1), so no file is read in the sandbox |

No new dependency is proposed: ICU4X is already admitted (TX2), and the differential references are test tools (DP2), each recorded in the register when a test first uses it.

## 5. Design

### 5.1 The front end

| Rule | Decision |
|---|---|
| JE1 | **The parser is Axiom's own,** a direct implementation of ECMA-262's grammar with its early errors, in safe Rust. Source text stays in PL11's form, 8-bit or 16-bit, and the tokenizer is generic over the code unit, as the CSS parser is (ST1), so script text never passes through UTF-8. Identifiers and property names become atoms (PL11). Nesting depth is bounded, so a hostile script can't exhaust the stack (§7) |
| JE2 | **Lazy compilation.** A script's inner functions are pre-parsed: their syntax and early errors are checked, as the specification requires before any code runs, and their free variables recorded, but their bytecode is generated on first call. SpiderMonkey's per-function stencils likewise "may or may not have bytecode" (§3). Functions that are called at once (immediately invoked function expressions, by the usual heuristic) compile eagerly |
| JE3 | **Compilation produces a heap-free compiled script,** as SpiderMonkey's Stencil does (§3): bytecode, constant pools, scope descriptions, literal templates and a script-local atom table, with no heap references. **Instantiation** turns it into heap objects on the agent's event-loop thread. So large scripts and module graphs compile on the helper pool, which never touches a heap (EL9), and a **bytecode cache** keyed by the response and its partition can be added later; anything read from it is validated as untrusted before use (staged) |
| JE4 | **Scopes are resolved at compile time.** Variables that no closure captures live in registers; captured ones live in environment objects created only for scopes that need them. Direct `eval` and `with` make the scopes they can reach dynamic, as specified; everything else binds statically. Private names (`#x`) are per-class atoms |

### 5.2 Bytecode and the interpreter

| Rule | Decision |
|---|---|
| JE5 | **A register machine with an accumulator.** Instructions name their register operands and use an implicit accumulator for chains of operations, as Ignition does, which keeps bytecode small (§3). Operands are 8-bit by default, with prefixes for wider ones. S3 measured a register machine without an accumulator; the accumulator, which a `match` loop keeps in a machine register, is measured when the instruction set is fixed |
| JE6 | **Dispatch is a `match` loop** (handed-over question 17), with each operation written as a function over the machine state, so that the technique can change. S3 measured it fastest on stable Rust, by a third to two thirds wherever bytecode dominates. **Explicit tail calls** (`become`) were 5–22% faster on nightly; they are adopted once stable and no longer flagged incomplete. The interrupt flag is read on loop back edges and calls (SH5) |
| JE7 | **Frames are windows of one register file** of `Cell<Value>`s, as in S3: a callee's window begins above its arguments, which are never copied, and the collector traces the used prefix as one region, so executing bytecode roots nothing (SH6). **Generators and async functions** suspend by copying their window into a heap array held by the generator object, and resume by copying it back, as V8's `SuspendGenerator` and `ResumeGenerator` bytecodes do (§3). The interpreter doesn't recurse natively for calls from bytecode to bytecode |
| JE8 | **Inline caches are data in the interpreter.** Each property access, call, `in` and `instanceof` site has a feedback slot recording the shapes it saw and what they resolved to: monomorphic, then polymorphic up to a small bound, then megamorphic, falling back to a shared cache keyed by shape and name. Shape transitions and a per-prototype validity marker invalidate them. No code is generated (J3); Ladybird's interpreter also keeps "per-site caches and feedback" (§3) |

### 5.3 Values, objects and strings

| Rule | Decision |
|---|---|
| JE9 | **The value type** (SH13; handed-over question 18). **A NaN-boxed 64-bit value:** a double, canonicalized when it is a NaN, as SpiderMonkey canonicalizes (§3); or, in the NaN space, a tag with an `int32`, a boolean, `undefined`, `null`, or a heap handle (a `u32` index with its arena's kind). **It carries the scope lifetime:** `Value<'gc>` is bound to a `GcScope` like Pattern B's handles, so the compiler rejects a value held across a possible collection, as S3 recommended. Nova's `bind`, `unbind` and `Scoped` discipline is the precedent (§3). Values that native code must keep across a collection are rooted on the `SliceStack` (SH7), and code that holds many at once roots one heap array (S3's recommendation; HD18) |
| JE10 | **Objects.** An object holds a **shape** (shared by objects with the same properties in the same order, linked by transitions), a few inline slots, an optional out-of-line slot array, and an optional **elements** store for integer-indexed properties, with element kinds (packed or holey; `int32`, double, or any value). Objects with many deleted or added properties switch to **dictionary mode.** Shapes, slot arrays and elements live in their own arenas, each a struct of arrays where that pays (SH8). Every field that holds a handle is written through generated setters (SH9) |
| JE11 | **Exotic and platform objects:** arrays and their `length`; typed arrays over `ArrayBuffer` backing stores outside the heap (SH11), including resizable and growable buffers; `Proxy` with every trap; module namespaces; mapped and unmapped `arguments`; `String` objects; bound functions. **Platform objects** are the DOM's own arena entries: a node's handle is its platform object (HD30), and CP17 defines how bindings reach them |
| JE12 | **Strings** (handed-over question 19). Strings use PL11's representation, 8-bit or 16-bit. **Ropes:** concatenation above a minimum length builds a rope, flattened when indexed, hashed or compared. **Slices:** substrings above a minimum length share their parent's characters. V8 uses 13 code units as both minimums (§3), Axiom's starting point. **During compaction,** the collector copies out a slice whose parent is otherwise unreachable, so a small slice never keeps a large string alive (inference). **Atoms:** property keys and identifiers are atoms from PL11's tables, and canonical array-index keys are integers, not strings |
| JE13 | **Numbers and BigInt.** Numbers are doubles, with `int32` as a fast path. `Number::toString` lays out the standard library's shortest round-trip digits as ECMA-262 specifies, and `StringToNumber` uses the standard library's correctly rounded parsing (background). **BigInt** arithmetic is Axiom's own (DP1 (g)), in heap cells of 64-bit limbs, with schoolbook, Karatsuba and long division at sizes Step 6 tunes; `num-bigint` is a differential reference (§4) |

### 5.4 Execution

| Rule | Decision |
|---|---|
| JE14 | **Functions and classes:** closures, `this` and `new.target`, `super`, classes with fields, static blocks, private methods and accessors, and direct and indirect `eval`, all as specified. Generator, async and async generator functions use JE7's suspension |
| JE15 | **Explicit resource management** (`using`, `await using`, `DisposableStack`), finished for 2027 (§2), compiles to disposal steps in the bytecode's exception regions, like `finally` |
| JE16 | **Promises, jobs and weak references.** The host hooks are the event loop's (SH4, EL14): promise jobs go to the microtask queue; `FinalizationRegistry` cleanup is a task; kept objects of `WeakRef` are cleared at the end of each microtask checkpoint, as ECMA-262 and HTML specify. **The collector** treats `WeakMap` and `WeakSet` entries as ephemerons, and clears dead weak references after marking (SH10) |
| JE17 | **Modules** through the host: HTML's module map, import maps, `import()` and `import.meta`, with fetching through the loader endpoint (FT11). JSON modules and import attributes are standard (§2); CSS module scripts produce style's constructed sheets (ST30). Module graphs compile off-thread (JE3) and evaluate with top-level `await` as specified. Source-phase and WebAssembly imports wait for OQ-3 (JE31) |
| JE18 | **Realms.** Each global object has its realm and intrinsics (SH1). Intrinsics are created from a compiled-in description, and builtin constructors and prototypes that a realm never touches are not created (inference), so a same-site iframe costs little until its script uses them |

### 5.5 The standard library

| Rule | Decision |
|---|---|
| JE19 | **Builtins are Axiom's own** (DP1 (g)): the whole ECMAScript 2026 library, and the 2027 draft's finished proposals (§2). Each abstract operation is a Rust function named and structured after the specification, as Nova's code follows the specification's layout (§3), so a reviewer can check one against the other. Builtins follow Pattern B as S3's did: they root what they hold across a callback and read it back after (SH6) |
| JE20 | **`RegExp` is Axiom's own** (DP1 (g)): a parser for the whole pattern syntax (the `d`, `g`, `i`, `m`, `s`, `u`, `v` and `y` flags, modifiers, duplicate named groups, lookbehind, `RegExp.escape`), compiled to a compact program run by **a backtracking interpreter, never machine code** (J3), directly over PL11's 8-bit and 16-bit text. Unicode properties and case folding come from ICU4X (TX1). **Fast as an interpreter:** literal prefixes and first-character sets are scanned before matching, since half of V8's jitless slowdown came from its `RegExp` interpreter (§3). **Bounded:** matching reads the interrupt flag (SH5); and a linear-time engine runs patterns without backreferences or lookaround once backtracking exceeds a budget, as V8's experimental engine falls back after 50,000 backtracks by default (§3). `regress` is the differential reference |
| JE21 | **`Intl` runs on ICU4X's formatters** (TX2, a foundational dependency): collation, number, date, relative time, list, plural, display names, segmentation and locale handling, as ECMA-402 specifies. Boa takes the same route (§3). Locale data is compiled in (TX1), and the default locale comes from the browser, from the user's macOS settings |
| JE22 | **Dates and Temporal.** `Date` follows ECMA-262, with local time from the time zone the browser sends; a change of system time zone updates it. `Date.parse`'s legacy formats are approximate (§2). **Temporal** (finished for 2027) is Axiom's own builtins (DP1 (g)) over ICU4X's calendars and a **compiled-in time zone database,** updated with each release as ICU4X's data is. V8 builds Temporal on `temporal_rs`, and Boa uses it too (§3); Axiom uses it as the differential reference. No time zone file is read inside the sandbox |
| JE23 | **Binary data and concurrency:** `ArrayBuffer` (resizable), `SharedArrayBuffer` (growable), `DataView`, and typed arrays including `Float16Array`, with backing stores outside the heap (SH11, SH3); `Atomics`, with `wait` and `notify` on macOS's wait-on-address primitive (SH3), `waitAsync`, and `pause` as a spin-wait hint |
| JE24 | **Structured cloning and transfer** serialize in the sending heap and deserialize in the receiving one (SH2), for `postMessage`, `structuredClone` and storage. Backing stores move without copying within a process (SH2) |

### 5.6 Errors, interrupts and debugging

| Rule | Decision |
|---|---|
| JE25 | **Errors and stack traces.** A thrown error records its stack as pairs of function and bytecode offset, and source positions are computed only when `stack` is read, from compact position tables, so throwing stays cheap. **The `stack` format is approximate:** no specification defines it, so it follows the format the engines share (§2). `Error.isError` is standard (§2) |
| JE26 | **Interrupts and limits.** The interrupt flag (SH5) stops runaway script on back edges, calls and long `RegExp` matches. **Stack limits:** the register file has a bounded size, and native re-entry (builtins calling back into bytecode) is counted, so deep recursion throws a `RangeError` instead of overflowing the thread's stack |
| JE27 | **Debugging,** later: breakpoints and stepping through a per-function debug copy of the bytecode with breakpoint instructions, checked like the interrupt flag, so code without breakpoints pays nothing. The developer tools protocol and its process boundary are decided at that milestone (staged) |

### 5.7 Memory, threads, a JIT, WebAssembly and tests

| Rule | Decision |
|---|---|
| JE28 | **Memory has owners** (SH14). Bytecode belongs to its function and is generated lazily (JE2); bytecode of functions that haven't run for a long time can be discarded and regenerated from the source text (inference); realms create intrinsics on demand (JE18); ropes and slices avoid copies (JE12). Step 6 measures bytecode, shape and string memory on the corpus against B3 |
| JE29 | **Threads.** Each heap is used only by its agent's thread (SH1). Compiling scripts and modules (JE3) and `RegExp` patterns runs on the helper pool when large, since their outputs hold no heap references (EL9). Parallel marking is a later collector decision (SH10) |
| JE30 | **Room for a JIT, not a JIT** (SH13, J1–J7). The interpreter's feedback (JE8) is what an optimizing tier would consume; Ladybird's new JIT uses its interpreter's caches as its only baseline (§3). A JIT can exist only in `renderer-jit`, from L2, on J1's evidence, which includes Axiom's interpreter measured on the corpus and V8's jitless figures (§3). Nothing in JE1–JE29 depends on one |
| JE31 | **What a WebAssembly engine would need from this one** (OQ-3, as evidence; the decision stays open): the `WebAssembly` namespace and its objects; memories as buffers outside the heap with growth semantics (SH11); `externref` and `anyref` values carrying JavaScript values; if WasmGC is supported, its structs and arrays in the same collected heap, which an embedded engine with its own collector would have to share; exceptions that cross between the two; and suspension for JavaScript promise integration, which a register-file interpreter already provides (JE7) (inference) |
| JE32 | **Deterministic, and tested at boundaries** (SC8). The engine runs under the deterministic scheduler and virtual time (EL24). **Oracles:**<br>• test262 [T1], the conformance suite;<br>• differential testing of `RegExp` against `regress`, Temporal against `temporal_rs`, and BigInt against `num-bigint`;<br>• a separate reference evaluator for the bytecode compiler, as S3 used (§1);<br>• S3's stress pacing (collecting at every allocation) and debug epochs, for rooting;<br>• S1's dispatch case, or a successor, kept as a benchmark, as S3 recommended;<br>• fuzzing of the parser, the bytecode compiler and `RegExp` (Step 6) |

## 6. Interfaces with neighbouring subsystems

| Neighbour | Interface |
|---|---|
| **The heap and collector** (SH6–SH11) | Arenas, handles and scope tokens; the register file as a root region; generated setters; weak references and ephemerons |
| **Bindings** (CP17) | The value type and its conversions; platform objects; realms and prototypes; exceptions across the boundary |
| **The event loop** (EL14, EL17; SH4, SH5) | Promise jobs and microtask checkpoints; tasks for finalization; the interrupt flag; no nested event loops |
| **HTML and the DOM** (HD18, HD30) | Script elements, module scripts and import maps; listener snapshots rooted as heap arrays; platform objects in the DOM's arenas |
| **Text** (TX1, TX2) | ICU4X's Unicode properties, case mapping, normalization and segmentation; `Intl`'s formatters |
| **Networking** (FT11) | Scripts and modules through the loader endpoint; a bytecode cache, later (JE3) |
| **Style** (ST30) | CSS module scripts as constructed sheets |

## 7. Security

- **The engine handles the most hostile input in the renderer,** so it is safe Rust: no `unsafe` in the parser, compiler, interpreter, builtins or `RegExp` (rule 5, SC3), and Pattern B's collector is safe (ADR-0003, SH7). A bug is a correctness bug or a panic, not memory corruption (ADR-0003, rationale 2).
- **No generated code** outside `renderer-jit` (J3): inline caches are data (JE8), and `RegExp` is interpreted (JE20).
- **Bounded work:** parser nesting (JE1), register file and re-entry depth (JE26), `RegExp` backtracking budgets and the interrupt flag (JE20, SH5). Exhaustion ends the script with an exception, or the renderer, never the browser.
- **Nothing is read from disk inside the sandbox:** Unicode, locale and time zone data are compiled in (JE21, JE22); a bytecode cache, if added, is validated as untrusted input (JE3).
- **`SharedArrayBuffer` stays within an agent cluster's process** (SH3), under cross-origin isolation, which networking enforces.

## 8. Staging and the complete model

| Stage | What exists |
|---|---|
| First script milestone | The parser with lazy compilation; the bytecode and the `match` interpreter; values, objects with shapes and inline caches, strings with ropes; functions, closures, classes, generators and async functions; promises and jobs; modules and import maps; the core library: `Object`, `Function`, `Array`, `String`, `Number`, `Boolean`, `Symbol`, `Math`, `JSON`, `Error`, `Map`, `Set`, `WeakMap`, `WeakSet`, `Promise`, `Proxy`, `Reflect`, iterators, typed arrays and `ArrayBuffer`; `RegExp` with its backtracking interpreter; `Date`; test262 in CI |
| Next | `Intl` on ICU4X; Temporal; BigInt; `SharedArrayBuffer` and `Atomics` with workers; `WeakRef` and `FinalizationRegistry`; slices; the linear-time `RegExp` fallback; explicit resource management; off-thread compilation |
| Later | The bytecode cache; debugging; bytecode discarding; a JIT in `renderer-jit`, only on J1's evidence; WebAssembly, once OQ-3 is decided |

**The complete model** (SC9): every ECMAScript feature fits these rules. A new syntax form extends the parser and the bytecode; a new builtin is another specification-shaped function; a new value kind is another arena and tag (JE9); `Intl` grows with ICU4X; WebAssembly and a JIT have their places (JE30, JE31).

## 9. Alternatives considered

| | Alternative | Assessment |
|---|---|---|
| A | **Embedding an existing engine** (V8, SpiderMonkey, JavaScriptCore, Boa, Nova) | ADR-0001 R2 and DP1 (g) make the engine Axiom's own, in one heap with the DOM. Rejected |
| B | **A stack machine,** as Nova uses | Simpler to compile to, but larger bytecode and more instructions per operation; S3 measured a register machine. Rejected |
| C | **Interpreter handlers generated as assembly,** as Ladybird's Flap emits them (§3) | Fast, but the most exposed component would sit outside Rust's safety, against rule 5, and S3 showed a safe `match` loop in reach. Rejected |
| D | **`temporal_rs` for Temporal,** as V8 does | Mature and shared by two engines, but DP1 (g) makes builtins core. Rejected; kept as the differential reference (JE22) |
| E | **`regress` for `RegExp`** | ECMAScript syntax and Rust, but DP1 (g) names `RegExp` as core, and Axiom needs a linear-time fallback and direct 8-bit and 16-bit matching. Rejected; kept as the differential reference (JE20) |
| F | **Values without lifetimes,** checked only by debug epochs | S3 found this a limitation: a value held across a collection is caught only under stress tests. Rejected for JE9 |
| G | **A JIT from the first milestone** | J1–J7 forbid it before L2 and without evidence. Rejected (JE30) |

## 10. Open questions

| # | Question | Where it is settled |
|---|---|---|
| 1 | The accumulator and the instruction set's encoding, measured against S3's register machine (JE5) | The first script milestone |
| 2 | Explicit tail calls, once `become` is stable (JE6) | When the feature stabilizes |
| 3 | The minimum lengths for ropes and slices, and when slices are copied out (JE12) | Step 6, on the corpus |
| 4 | The bytecode cache's design and validation (JE3) | A later milestone |
| 5 | WebAssembly: our own engine or an embedded one (JE31) | OQ-3, before any Wasm milestone |

## Sources

All checked 2026-10-19.
- [E1] Ecma International, *ECMAScript 2026 Language Specification*, 17th edition (June 2026) — https://262.ecma-international.org/17.0/
- [E2] TC39, *ECMAScript 2027 Language Specification*, draft dated 2026-10-16 — https://tc39.es/ecma262/
- [E3] TC39, finished proposals [2026-10-14] — https://github.com/tc39/proposals/blob/main/finished-proposals.md
- [E4] TC39, *ECMAScript 2027 Internationalization API Specification*, draft dated 2026-10-12 — https://tc39.es/ecma402/
- [H1] WHATWG, *HTML Living Standard*, last updated 2026-10-16: scripting (§8.1) — https://html.spec.whatwg.org/multipage/
- [T1] test262, at `d53ebc8b1931d9f1541e43d59650776e4b766248` (2026-10-13) — https://github.com/tc39/test262
- [V1] R. McIlroy, "Firing up the Ignition interpreter", V8 blog (2016-08-23) — https://v8.dev/blog/ignition-interpreter
- [V2] J. Gruber, "JIT-less V8", V8 blog (2019-03-13) — https://v8.dev/blog/jitless
- [V3] I. Sheludko and S. Aboy Solanes, "Pointer Compression in V8", V8 blog (2020-03-30) — https://v8.dev/blog/pointer-compression
- [V4] M. Bidlingmaier, "An additional non-backtracking RegExp engine", V8 blog (2021-01-11) — https://v8.dev/blog/non-backtracking-regexp
- [V5] V8: `src/objects/string.h`, `src/interpreter/bytecodes.h`, `src/objects/DEPS`, `src/objects/js-temporal-zoneinfo64.h` — https://github.com/v8/v8
- [M1] Firefox: `js/public/Value.h`, `js/src/frontend/Stencil.h` — https://github.com/mozilla-firefox/firefox
- [N1] Nova: `ARCHITECTURE.md`, `GARBAGE_COLLECTOR.md` — https://github.com/trynova/nova
- [B1] Boa: `Cargo.toml` — https://github.com/boa-dev/boa
- [R1] regress: `README.md` — https://github.com/ridiculousfish/regress
- [S1] Brimstone: `README.md` — https://github.com/Hans-Halverson/brimstone
- [L1] Ladybird: `Libraries/LibJS/JIT/ARCHITECTURE.md`, `Libraries/LibJS/Flap/src/lib.rs` — https://github.com/LadybirdBrowser/ladybird
- This repository: [S3 results](../research/s3-results.md) §1, §5 and §9 (the spike interpreter, dispatch, recommendations).
