# ADR-0017: The JavaScript Engine and Bindings

| | |
|---|---|
| **Status** | **Accepted** (2026-10-20), by the owner at Step 5's Gate C, as proposed |
| **Date** | 2026-10-19 |
| **Deciders** | Project owner (only the owner can move an ADR to Accepted) |
| **Related** | [The JavaScript engine](../architecture/javascript-engine.md) (JE1–JE32); [WebIDL bindings](../architecture/bindings.md) (WB1–WB23); [ADR-0001](0001-scope-and-target.md) (R2); [ADR-0003](0003-gc-handle-and-rooting-pattern.md); [ADR-0004](0004-process-model-and-sandbox.md) (J1–J7); [ADR-0008](0008-script-agents-and-heap.md); [ADR-0011](0011-text-fonts-and-unicode.md) (TX1, TX2); [ADR-0015](0015-html-parsing-and-the-dom.md); [S3 results](../research/s3-results.md); decision-map item P7; [OQ-3](../open-questions.md) |

## Decision history

| Date | Event |
|---|---|
| 2026-10-19 | Proposed at Step 5, CP18 |
| 2026-10-20 | **Accepted** by the owner at Step 5's Gate C: "ADR-0017 (JavaScript Engine and Bindings): Accepted as proposed." |

## Question

How is Axiom's own JavaScript engine built, from parser to builtins, and how are the WebIDL bindings generated and called? This settles the handed-over questions 17 (dispatch), 18 (the value type, rooting and the dispatch benchmark) and 19 (ropes and slices), and script and heap's open question 2, and records evidence for OQ-3.

## Requirements and constraints

- **ADR-0001 R2, DP1 (g), (h):** the engine and the WebIDL generator are Axiom's own, in one heap with the DOM.
- **ADR-0003, SH1–SH14:** Pattern B, with `SliceStack` rooting, generated setters, a mark–compact collector, and memory for platform APIs outside the heap.
- **J1–J7:** interpreter first; any JIT only in `renderer-jit`, from L2, on evidence.
- **S3:** a `match` loop is fastest on stable Rust; values should carry the scope lifetime; many values can be rooted as one heap array.
- **PL11, TX1, TX2:** 8- and 16-bit strings; ICU4X for Unicode and `Intl`.
- **ADR-0002 rule 7:** generated code is safe.

## Alternatives considered

- **Embedding an engine** (V8, SpiderMonkey, JavaScriptCore, Boa, Nova).
- **A stack machine** (Nova).
- **Interpreter handlers generated as assembly** (Ladybird's Flap).
- **Reusing `temporal_rs`, `regress` or `num-bigint`** for Temporal, `RegExp` and BigInt.
- **Values without lifetimes,** checked only by debug epochs.
- **Wrapper objects** for platform objects, and **hand-written bindings**.
- **The proposal:** an engine and a generator of Axiom's own, a register machine dispatched by a `match` loop, lifetime-carrying values, and platform objects as heap entries.

## Historical context and modern developments

- **V8's Ignition** is a register machine with an accumulator; its jitless mode ran Speedometer 2.0 about 40% slower, half of it in the `RegExp` interpreter. **SpiderMonkey's Stencil** decouples compilation from the heap for thread-agnostic caching. V8 builds Temporal on `temporal_rs`.
- **Rust engines:** Nova 1.0 (stack machine, compacting collector, `bind` and `unbind`); Boa (ICU4X, `temporal_rs`, `regress`); Brimstone (>97% of test262's language tests, a "very unsafe" collector). **Ladybird's LibJS** has an optimizing JIT whose only baseline is its interpreter's caches.
- **Servo and Ladybird keep two objects per DOM object,** a native one and its reflector or wrapper, as Blink, Gecko and WebKit do (background); Ladybird moved to wrappers built as host objects on 2026-10-05.

Sourced in [the JavaScript engine](../architecture/javascript-engine.md) §3 and [WebIDL bindings](../architecture/bindings.md) §3.

## Analysis

| | Proposal | Alternatives |
|---|---|---|
| Ownership | Core under DP1 (g) and (h); one heap with the DOM | Embedding breaks R2; reusing `temporal_rs` or `regress` breaks DP1 (g) |
| Speed without a JIT | S3's fastest dispatch; inline caches as data; a fast `RegExp` interpreter | Assembly handlers are faster but unsafe; a stack machine runs more instructions |
| Safety | No `unsafe` in the engine or the generated glue; missed roots are compile errors | Debug epochs catch missed roots only under stress tests |
| Memory | No wrappers; lazy compilation and lazy realms; ropes and slices | Wrappers double the objects and need preservation |

## Evidence and sources

[The JavaScript engine](../architecture/javascript-engine.md) §2–§4 and [WebIDL bindings](../architecture/bindings.md) §2–§4, with their sources: ECMA-262 2026 and the 2027 draft, ECMA-402, TC39's finished proposals, test262, Web IDL, HTML, V8, SpiderMonkey, Nova, Boa, `regress`, Brimstone, Ladybird, Blink, Gecko, WebKit, Servo and webref. [S1](../research/s1-results.md) and [S3](../research/s3-results.md) results.

## Decision

**Accepted: JE1–JE32 and WB1–WB23** (2026-10-20). In summary:
1. **The whole engine is Axiom's own** (DP1 (g)): parser, bytecode, interpreter, object model, builtins, `RegExp` and collector, on ICU4X and Rust's standard library (JE1–JE24).
2. **Heap-free compiled scripts,** so compilation runs on the helper pool and functions compile lazily (JE2, JE3).
3. **A register machine with an accumulator, dispatched by a `match` loop** (handed-over question 17; script and heap's open question 2), with explicit tail calls once `become` is stable (JE5–JE7).
4. **A NaN-boxed value that carries the scope lifetime** (handed-over question 18), after Nova's `bind` and `unbind` (JE9).
5. **Ropes and slices from 13 code units,** with the collector copying out slices that would pin a dead parent (handed-over question 19) (JE12).
6. **ECMAScript 2026 and the 2027 draft's finished proposals.** `RegExp` is a backtracking interpreter with a linear-time fallback; `Intl` runs on ICU4X; Temporal and BigInt are Axiom's own, with `temporal_rs` and `num-bigint` as differential references (JE19–JE22).
7. **A JIT stays under J1–J7;** the interpreter's feedback is its input. **OQ-3's evidence** is recorded; the decision stays open (JE30, JE31).
8. **The WebIDL compiler is Axiom's own,** reading webref's curated IDL and emitting safe Rust; **platform objects are heap entries, not wrappers;** conversions and overloads as Web IDL specifies, with rooting only where a later conversion may collect, checked by the compiler (WB1–WB16).
9. **Event dispatch** roots two heap arrays, benchmarked by S1's dispatch case; **remote window proxies** offer only HTML's cross-origin properties, each operation a message to the browser (WB17–WB21).

## Rationale

- R2 and DP1 settle ownership; S3's measurements settle dispatch.
- Lifetime-carrying values close the gap S3 found, so the compiler, not a stress test, catches a value held across a collection.
- Platform objects as heap entries follow from SH1, and remove the wrapper machinery every other engine carries.
- `RegExp` speed matters most without a JIT, on V8's evidence.

## Assumptions

- A well-designed interpreter, with inline caches, meets the first milestones' budgets on the owner's corpus (OQ-2); Step 6 measures it.
- ICU4X's formatters and calendars cover ECMA-402 and Temporal (TX2).
- The generator's static analysis of which conversions may run script is complete for Web IDL's types; the compiler rejects any gap.

## Risks and unresolved issues

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Interpreter speed falls short on app-web sites | Medium | High | J1's evidence path to a JIT in `renderer-jit`; the interpreter's feedback is already its input (JE30) |
| Temporal, `Intl` and `RegExp` are large to build | High | Medium | No deadline (charter principle 7); differential references shorten the path to correctness (JE32) |
| Generated bindings inflate the executable | Medium | Low | Shared helpers and static tables; measured in Step 6 (WB22) |
| WasmGC needs deep integration with this heap | Medium | Medium | Recorded as OQ-3 evidence (JE31); decided before any Wasm milestone |

## Conditions for revisiting

- Step 6 or the first script milestone shows the interpreter can't meet its budgets: J1's evidence for a JIT.
- `become` stabilizes without the incomplete-feature warning: adopt tail calls (JE6).
- OQ-3 is decided for an embedded WebAssembly engine: revisit the shared-heap interfaces of JE31.

## Consequences

- [HTML and the DOM](../architecture/html-and-dom.md)'s JavaScript facet (HD30) is closed by WB4–WB9.
- Every subsystem that exposes an interface supplies IDL (from webref), a configuration entry and a Rust implementation (WB1).
- The browser's frame tree answers remote window proxies' operations (FT6, WB20).
