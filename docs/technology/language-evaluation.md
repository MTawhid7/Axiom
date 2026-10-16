# Implementation Language Evaluation

| | |
|---|---|
| **Status** | Reviewed by the owner (2026-10-16); §15.3 amended with rule 7 |
| **Decision record** | [ADR-0002](../adr/0002-implementation-language.md), Provisionally accepted |
| **Phase** | Phase Zero, Step 2a |
| **Inputs** | [ADR-0001](../adr/0001-scope-and-target.md) (scope and target), [decision map](../decision-map.md) |
| **Last updated** | 2026-10-16 |

**How to read the evidence.**
- A statement with a link is a **sourced fact**. Bracketed numbers such as [3] point to §17.
- **Inference** marks my reasoning from sourced facts.
- **Hypothesis** marks a claim that needs validation, usually by a spike (§16).
- **Background** marks well-established facts that were not re-verified for this document.
- The scores in §14 are structured judgments, not measurements.

---

## 1. Summary

**Recommendation: Rust, for the whole system.** That includes the browser shell, the engine, the JavaScript engine with its garbage collector, the DOM, and the macOS platform layer through the `objc2` bindings. An explicit policy (§15.3) confines `unsafe` code to a few audited crates:
- the GC heap;
- JIT code emission (later);
- platform FFI;
- hot paths, admitted only when a benchmark justifies them.

**Confidence.**
- **High** that Rust is the best of the four candidates for this project's requirements.
- **Medium** on *how* Rust code will hold references into the garbage-collected heap. This is the one area where Rust is weakest and C++ strongest. Validation spike S1 (§16) should settle it before ADR-0002 is accepted.

**The argument in brief.**

1. **The security posture rules out C++ and Zig as the default.** Arbor must be safe enough for sensitive logins ([ADR-0001](../adr/0001-scope-and-target.md) R3).
   - Memory-unsafe code is the dominant source of severe vulnerabilities in the best-studied browser codebase: about 70% of Chromium's high-severity security bugs [1].
   - The most effective known remedy is writing *new* code in memory-safe languages. Android cut memory-safety bugs from 76% to 24% of its vulnerabilities between 2019 and 2024, mainly by doing that [2].
   - All of Arbor's code is new, so C++ and Zig would need some other large advantage to compensate.
2. **The JS engine requirement is Rust's weak spot, but working designs exist.** Arbor will run its own JS engine, and that engine's garbage collector also manages the DOM (ADR-0001 R2). This is C++'s home ground. Rust already has three working approaches, each with known costs (§7):
   - sound but restrictive: `gc-arena`, used in production by Ruffle;
   - checked by the borrow checker but verbose: Nova;
   - unsafe but contained: Brimstone, which passes over 97% of test262's language tests.

   The cost is real and bounded. It falls on one layer (GC, DOM, bindings), not on parsers, networking, style, layout or inter-process communication (IPC).
3. **The app-web target needs parallelism, and Rust checks data races at compile time.** Arbor must eventually run parallel and multi-agent: workers, `SharedArrayBuffer`, parallel style and layout (R1).
   - Rust's compile-time data-race checking is the strongest available.
   - Mozilla parallelized its style system in Rust after two failed attempts in C++ [4].
4. **Swift is safe and Apple-native, but fits a GC heap poorly.**
   - It is memory-safe and the best fit for Apple frameworks.
   - Its reference-counting memory model works against a tracing GC heap.
   - No JS engine has been written in Swift to learn from.
   - Apple itself defers Swift in WebKit's performance-sensitive WebCore [21].
5. **The newest independent engine reached the same conclusion.**
   - In 2024, Ladybird cited Rust's poor fit for "garbage collection and deep inheritance" as its reason to reject it.
   - In 2026 it adopted Rust anyway [5]. It has since moved CSS parsing, style and painting to Rust [6].

**What would change this recommendation** (details in §15.6):
- S1 cannot keep `unsafe` inside the GC crate with acceptable ergonomics.
- The owner's fluency makes Rust's learning cost prohibitive.
- Measured builds on the 8 GB M1 make iteration impractical despite mitigations.

The matrix analysis (§14) shows exactly which combination of weakened assumptions makes C++ win.

---

## 2. The question, and why it is foundational

**Question:** In which language or languages will Arbor be written? That covers the browser engine, its JavaScript engine, and its macOS shell.

**The cost of reversal is not only a rewrite.** A language fixes the system's *invariants*: how objects refer to each other, how memory is reclaimed, how threads share data, and where unsafe code lives. Each candidate pushes toward different answers:
- Rust: handles and explicit ownership.
- C++: raw-pointer graphs with conservative stack scanning.
- Swift: reference counting.

**A 2026 development changes the economics.**
- Ladybird translated about 25,000 lines of its JS frontend from C++ to Rust in roughly two weeks, using AI agents under human direction [5].
- They verified it byte-for-byte against the old pipeline: 52,898 test262 tests, 12,461 regression tests, and lockstep browsing.
- **Inference:** mechanical translation checked against a strong test oracle is now cheap. Changing invariants is not.
- So the decision still matters, but mainly for the invariants it encourages.
- Strong oracles are worth adopting from the first milestone whatever the language: test262 for JS, WPT for the web platform, html5lib-tests for the parser.

---

## 3. Requirements that drive the choice

| Source | Requirement | What it asks of the language |
|---|---|---|
| ADR-0001 R2 | Our own JS engine; **one GC heap for JS objects and DOM nodes** | A tracing GC, interpreter, object model, string representations and regex engine. GC references must be held safely throughout the engine |
| ADR-0001 R1 | Full app web must be reachable | Multi-threaded agents (workers), SharedArrayBuffer and Atomics, an eventual JIT, Wasm (open question), Metal-backed GPU APIs, VideoToolbox media. That means concurrency guarantees, performance headroom and good Apple FFI |
| ADR-0001 R3 | Security posture fit for sensitive logins | Memory safety for parsers, decoders, networking and IPC message decoding. Multi-process with sandboxing. Easy to fuzz |
| ADR-0001 defaults | Apple Silicon and current macOS only | Cross-platform support is **not** a criterion; Apple-framework integration **is** |
| Environment | 8 GB, fanless M1; Command Line Tools only | Build time and build memory matter. Tooling should work without Xcode |
| Team | One human developer with AI assistance (assumption, OQ-9) | The compiler should catch errors early; low ceremony; strong test oracles |

---

## 4. Historical context

### 4.1 Why every mainstream engine is C++

**Background:**
- Gecko, WebKit (from KHTML) and Blink (from WebKit) are written in C++.
- So are V8, JavaScriptCore and SpiderMonkey; SpiderMonkey began in C.
- These engines were started between the late 1990s and 2008.

**Inference:** at that time, no mainstream language offered C-level performance, direct OS and GUI access, and memory safety together. Garbage-collected languages (Java, C#) brought runtime overhead, unpredictable pauses, and a second memory manager underneath any JS engine. C++ was the practical choice. Since then, the industry has been paying down its safety cost layer by layer:

| When | Mitigation in C++ engines | Evidence |
|---|---|---|
| 2008 | Multi-process, sandboxed renderers (Chromium) | Background |
| 2010s | Oilpan, a tracing GC for Blink's C++ DOM, built to "get rid of memory leaks and use-after-free issues" | [10] |
| 2018 | Site isolation, motivated by Spectre | [47] |
| 2022–2024 | MiraclePtr: mitigated about 57% of use-after-free bugs in Chrome's privileged processes, at +5.5–8% browser-process memory on desktop | [3] |
| 2020s | Chromium's unsafe-buffer warnings and "spanification"; WebKit's smart-pointer rules, `-Wunsafe-buffer-usage` and safer-C++ checker | [23], [21] |
| 2025 | C++26 adopts standard-library hardening (P3471) | [24] |
| 2025 | The "Safe C++" proposal is abandoned, and "profiles" miss C++26 | Secondary sources: [25], [26] |

In 2020, Chromium's security team concluded that sandboxing and site isolation were reaching their limits. The remaining lever was preventing bugs at the source [1].

### 4.2 What has changed since those engines were designed

- **Memory-safe systems languages matured.**
  - Rust 1.0 shipped in 2015 (background).
  - Firefox shipped Stylo, a parallel style system in Rust, in Firefox 57 (2017).
  - Stylo replaced about 160,000 lines of C++ with about 85,000 lines of Rust.
  - Mozilla had tried to parallelize its C++ style system twice and failed both times [4].
- **There is now data on new code.** Android's memory-safety share of vulnerabilities fell from 76% to 24% between 2019 and 2024 [2]. Google attributes this to prioritizing memory-safe languages for *new* development.
- **Incumbent browsers now use Rust where attack surface is highest.**
  - Chromium uses the Rust `png` crate [44].
  - Chrome ships a pure-Rust JPEG XL decoder (jxl-rs) [45].
- **Servo became a product.** It is published as an embeddable crate (0.1.0, April 2026) with long-term-support releases [7].
- **Swift gained systems features.**
  - Swift 6 (2024) added compile-time data-race safety (background).
  - Swift 6.2 (2025) added `Span`, `InlineArray`, and opt-in strict memory-safety checking (SE-0458) [20].
  - WebKit is adopting Swift in its WebKit layer, starting with a WebGPU backend. It defers WebCore "because WebCore is perf sensitive, and vast majority of code is C++" [21].
- **Ladybird changed language twice.**
  - It picked Swift in 2024.
  - In February 2026 it removed Swift: "C++ interop never quite got there" and platform support outside Apple's ecosystem was limited.
  - It then adopted Rust, while acknowledging that Rust's ownership model is "not a natural fit" for the web's object model [5].
- **Hardware memory tagging arrived, but not on our machine.**
  - Apple's Memory Integrity Enforcement (built on EMTE) is available on A19 and M5 chips and later [22].
  - **Inference:** on our M1, C or C++ code gets no hardware backstop against use-after-free.
- **AI-assisted translation** has made rewrites cheaper (see §2).

---

## 5. Criteria and weights

The weights sum to 100 and come from ADR-0001.

| # | Criterion | Weight | Why this weight |
|---|---|---|---|
| C1 | Memory safety of the non-GC majority of code: parsers, decoders, networking, style, layout, IPC | **20** | R3. Most of the attack surface lives here |
| C2 | Suitability for a GC heap shared by JS and the DOM | **15** | R2. The hardest implementation problem in the project |
| C3 | Runtime performance and predictability | **12** | App-web target; 8 GB memory budget; no hidden costs |
| C4 | Concurrency and data-race safety | **10** | Parallel style and layout; workers; SharedArrayBuffer |
| C5 | macOS platform integration | **8** | AppKit, Metal, CoreText, Security.framework, XPC, sandbox |
| C6 | Tooling on this machine: debugging, profiling, sanitizers, fuzzing, undefined-behaviour (UB) detection | **8** | Quality bar, without Xcode for now |
| C7 | Ecosystem for supporting infrastructure | **8** | The charter allows mature libraries for fonts, images, TLS and similar |
| C8 | Suitability for writing an interpreter and JIT | **5** | Our own JS engine; a JIT comes later |
| C9 | Language stability, governance, longevity | **5** | Multi-year project |
| C10 | Learning curve; fit with AI-assisted development | **5** | Team assumption |
| C11 | Build cost on an 8 GB M1 | **4** | Iteration speed |

Each criterion is scored from 1 (poor) to 5 (excellent). The maximum weighted total is 500.

---

## 6. The candidates

### 6.1 Rust

**Strengths**
- **Safe by default.** Memory safety and data-race freedom are enforced at compile time. `unsafe` is explicit, local and searchable (background).
- **Browser-scale track record:**
  - Stylo in Firefox [4].
  - Servo, now an embeddable crate [7].
  - Ladybird, migrating core subsystems [6].
  - Chromium's Rust image decoders [44], [45].
- **Ecosystem.** The deepest supporting ecosystem for browser work of any memory-safe language (§12).
- **Tooling that works on this machine without Xcode** (§11): `cargo`, the `samply` profiler, `cargo-fuzz` on Apple Silicon, and Miri.
- **Apple frameworks via `objc2`.** It auto-generates bindings to AppKit, Foundation, Metal and other Apple frameworks [40]. wgpu moved its Metal backend to `objc2-metal` in January 2026 [40].

**Weaknesses**
- **GC object graphs work against the ownership model** (§7). This was Ladybird's 2024 objection [5].
- **No direct equivalent for an inheritance-heavy DOM** (EventTarget → Node → Element → HTMLElement → …). It has to be modelled with enums, traits and generated code. Trait upcasting, stable since Rust 1.86 (April 2025), removes one long-standing obstacle for designs based on trait objects [33].
- **No stable guaranteed tail calls and no computed goto.** This limits some interpreter-dispatch techniques (§8). Stabilization of explicit tail calls is targeted for 2027 at the earliest [32].
- **Builds are slow and memory-hungry** (background). On 8 GB this is a risk to measure (S2).
- **Steep learning curve** (background).

### 6.2 C++ (C++23/26 with modern hardening)

**Strengths**
- **Every production browser and JS engine is written in it**, so it has the most reference material.
- **Production GC designs are proven** (§7): a C++ GC library (Oilpan, published as cppgc) [10], and conservative stack scanning [11], [12].
- **Direct Apple-framework access** through Objective-C++ and Apple's `metal-cpp` (background).
- **Mature interpreter and JIT techniques**: computed goto, `[[clang::musttail]]` (background).
- **Incumbent supporting libraries**: HarfBuzz, ICU, Skia, FreeType, BoringSSL (background).

**Weaknesses**
- **Memory safety is opt-in and partial.**
  - Hardening reduces out-of-bounds bugs: C++26 library hardening [24], unsafe-buffer warnings [23].
  - Use-after-free remains the core problem. MiraclePtr mitigates about 57% of use-after-free bugs in Chrome's privileged processes, at a memory cost [3].
  - "Safe C++" was abandoned, and profiles missed C++26 [25], [26].
- **No compile-time data-race checking.** Mozilla's two parallel-style attempts in C++ failed [4].
- **No hardware memory tagging on the M1** (Apple's Memory Integrity Enforcement needs A19/M5 or later) [22].
- **Even Apple's WebKit has open problems here.** With all of Apple's resources, it lists JSC GC-timing bugs and reference cycles as unsolved memory-safety problems [21].
- **Fuzzing needs a separate toolchain.** Apple's clang ships without libFuzzer, so fuzzing requires a separately installed LLVM [34].

### 6.3 Swift (6.x)

**Strengths**
- **Memory-safe by default**: automatic reference counting (ARC), bounds checks, exclusivity enforcement (background).
- **Swift 6 data-race safety** (background). Swift 6.2 adds `Span`, `InlineArray` and a strict memory-safety mode [20].
- **Low-level concurrency primitives**: `Atomic` and `Mutex` in the Synchronization module [36].
- **First-class Apple integration.** Apple uses Swift inside WebKit's WebKit layer [21].

**Weaknesses**
- **ARC is the wrong memory model for a tracing GC heap.** **Inference:** JS objects and DOM nodes in our heap can't be ordinary Swift classes, so there are only two options:
  - Pay reference-counting traffic on every reference copy, *and still* need a collector that can reclaim cycles.
  - Bypass ARC and write the whole heap layer with unsafe pointers. This is Swift's least ergonomic and least tool-supported mode.
- **No JavaScript engine written in Swift exists to learn from.** My search found none. The main Swift artifact in this space is Fuzzilli, a JS-engine *fuzzer* [49].
- **Performance predictability needs expertise.** **Inference:** reference-counting traffic, exclusivity checks, and costs of protocol types and generics require expertise to control. **Fact:** Apple defers Swift in WebCore because WebCore is performance-sensitive [21].
- **Weaker tooling for unsafe code.**
  - There is no Miri equivalent for checking unsafe code.
  - libFuzzer integration depends on swift.org toolchains, and users report friction on macOS [35].
- **Ladybird abandoned Swift in 2026**, but its reasons (C++ interop, non-Apple platforms) mostly don't apply to a new, macOS-only project. **Inference:** this is weak evidence either way.

### 6.4 Zig (0.16+)

**Strengths**
- **C-level control with better ergonomics**: explicit allocators, `comptime`, optionals. Debug and ReleaseSafe builds check bounds (background).
- **Good for interpreters.** A labelled `switch` that can be the target of `continue` (since 0.14) supports threaded dispatch [30]. `@call(.always_tail, …)` gives explicit tail calls (background).
- **Fast compiles** (background).
- **There is a precedent, with a caveat.** Kiesel, a JS engine in Zig by a former LibJS maintainer, shows the path. However, it hands garbage collection to the Boehm collector (bdwgc) instead of its own [19].

**Weaknesses**
- **No protection against use-after-free and no data-race checking** (background). It fails C1 the way C++ does.
- **Pre-1.0, with breaking changes in each release.** For example, 0.16 reworked I/O as an interface [31].
- **Small ecosystem.** Apple frameworks are reachable only through the C ABI and hand-written Objective-C runtime calls (inference).

### 6.5 Excluded options

- **Garbage-collected languages (Go, Java/Kotlin, C#).** **Inference:** a JS engine must control object layout, memory reclamation and generated machine code. A host GC underneath ours adds pauses and double bookkeeping.
- **C.** Covered by the C++ and Zig analysis, with fewer tools for abstraction.
- **Experimental languages (Carbon, Hylo, Vale).** **Inference:** not stable enough in 2026 to found a multi-year engine on.
- **Ada/SPARK.** **Inference:** strong safety story, but almost no browser or macOS ecosystem.

---

## 7. The crux: a garbage-collected heap shared by JS and the DOM

### 7.1 The problem

ADR-0001 commits Arbor to one tracing GC heap holding both JS objects and DOM nodes, the same model Ladybird uses. That has consequences in any language.

**The graph is cyclic and shared.** Typical edges:
- node ⇄ parent and children;
- node → event listener → closure → node;
- extra JS properties ("expandos") on DOM objects.

**This creates the *rooting problem*.**
1. Native code holds references to GC objects *while it allocates*. That covers DOM algorithms, bindings, the event loop, and the parser building the tree.
2. Any allocation can trigger a collection.
3. A reference the collector doesn't know about then becomes *dangling* (the object was freed) or *stale* (a compacting collector moved the object).

**Beyond correctness, the heap must provide:**
- fast allocation;
- short pauses, to sustain 60 fps rendering;
- compaction, to fit the 8 GB budget;
- one heap per agent, for workers;
- `WeakRef` and `FinalizationRegistry`;
- write barriers, if the collector becomes incremental or generational.

**The GC core (allocator, marker, sweeper) is `unsafe` code in every language, including Rust.** So the real question is how *the other ~90% of the engine* handles GC references.

### 7.2 How existing engines solve it

| Engine | Heap and DOM model | How roots are found | Objects move? | Evidence |
|---|---|---|---|---|
| Blink + V8 | DOM objects in Oilpan, a C++ GC library, traced together with V8 as one unified heap | Exact (precise) tracing of heap references; conservative scan of the native stack | — | [10] |
| WebKit + JSC | DOM is reference-counted; JS wrappers are kept alive via "opaque roots"; JSC's heap is collected by Riptide | Conservative scan of the stack and registers | No: "our collector does not move objects" | [11]; opaque roots are background |
| Gecko + SpiderMonkey | DOM is reference-counted plus a cycle collector; the JS heap is collected precisely | Precise rooting (`Rooted<T>`, `Handle<T>`) | Yes (compacting, generational) | Background |
| Servo | DOM objects are owned by SpiderMonkey's GC through "reflectors" | Rust rooting types, checked by a custom lint (crown) that "may not catch all possible mistakes" | Follows SpiderMonkey | [8], [9] |
| Ladybird | One GC (LibGC) for JS and the DOM | Conservative scan of the stack and registers; precise heap edges (`visit_edges`) | No | [12] |

**Inference: the three engines with the deepest investment (V8 + Blink, JSC, Ladybird) all scan the native stack conservatively.**
- Every word on the stack that looks like a pointer into the heap is treated as a root.
- That is what makes GC references cheap to use in C++: native code needs no rooting ceremony.
- The costs are that objects referenced from the stack can't move (they are "pinned"), and stray stack words can keep dead objects alive.

### 7.3 Design patterns available in Rust

**A. Branded-lifetime arenas (`gc-arena`)**
- **How it works:** all GC access happens inside a `mutate` closure. Pointers carry a lifetime tied to that one call, so the borrow checker stops them from escaping or surviving a collection.
- **Soundness:** fully sound, checked at compile time.
- **Track record:** Ruffle's ActionScript VM and the piccolo Lua VM [13].
- **Costs:** collection can only run *between* `mutate` calls. That forces a "stackless" interpreter: builtins that call back into JS must be written as resumable state machines [14]. Dynamically sized types are awkward [13].

**B. Borrow-checked handles with explicit safepoints (Nova)**
- **How it works:** handles carry the lifetime of a GC scope. Functions `bind` handles to a no-GC scope, and `unbind` them before any call that may collect. Long-lived values are rooted explicitly.
- **Soundness:** checked by the borrow checker. A planned redesign to make this less verbose is currently "unsound and incomplete" [14].
- **Track record:** `nova_vm` 1.0.0 [15]:
  - data-oriented heap: 32-bit handles, one arena per type, compaction;
  - performance "acceptable, but not fast";
  - RegExp support incomplete, and no Wasm.
- **Costs:** the author calls the approach both "excellent" and "awful" [14]. It turns code into "a soup of" bind/unbind calls across about 800 call sites, and a colleague called it "worse than C++".

**C. Precise `Gc<T>` smart pointers with a `Trace` derive and root counting (the rust-gc lineage; Boa)**
- **How it works:** smart pointers implement `Trace`, and roots are tracked at run time.
- **Soundness:** mostly a safe API.
- **Track record:** Boa's docs claim support for more than 90% of ECMAScript [17]. A secondary 2026 snapshot reports about 51,000 of 53,000 test262 tests passing [16]. However, Boa's own 2026 roadmap calls its GC "a prototype" that needs "significant redesign" [16].
- **Costs:** root bookkeeping overhead and performance limits.

**D. Unsafe core with handles and a compacting collector (Brimstone)**
- **How it works:** raw pointers inside an unsafe GC.
- **Soundness:** correctness rests on discipline inside the GC.
- **Track record:** Brimstone [18] passes over 97% of the ECMAScript language tests in test262. Its compacting GC is "written in *very* unsafe Rust", and the project says it is "not ready for use in production".
- **Costs:** a larger unsafe surface that is harder to audit.

**E. Conservative stack scanning from Rust**
- **How it works:** scan the native stack for words that look like heap pointers, as JSC and Ladybird do.
- **Soundness, hypothesis:** probably workable in practice, but in tension with Rust's rules on pointer provenance and aliasing.
- **Track record:** no mainstream Rust precedent found.
- **Costs:** pinned objects, and no clear soundness story under Miri.

**F. A foreign C++ GC (for example cppgc) driven from Rust**
- **How it works:** Rust code hosts objects in a C++ GC library.
- **Soundness:** unsafe at every boundary.
- **Track record:** Servo does this with SpiderMonkey, guarded by custom lints [9].
- **Costs:** it defeats the safety argument for exactly the code that matters most.

### 7.4 The same problem in C++, Swift and Zig

- **C++ is the most direct path.** Adopt either a conservative-stack design (as JSC and LibGC do) or Oilpan-style handles (`Member<T>`, `Persistent<T>`). A GC-managed DOM removes most DOM use-after-free bugs by construction; that was Oilpan's original rationale [10]. The remaining risk is all memory that is not GC-managed, plus GC-timing bugs: WebKit still lists JSC GC-timing bugs as unsolved [21].
- **Swift needs unsafe code for the whole heap.** ARC manages class instances, not a tracing heap (§6.3). **Inference:** the heap layer would be `UnsafeMutableRawPointer` code. The DOM could not be idiomatic Swift classes without bringing back the reference-counting and wrapper complexity (WebKit's opaque roots) that ADR-0001's unified heap is meant to avoid.
- **Zig is natural to write but checks nothing.** Manual memory management and `comptime` layouts make a GC straightforward to write, but nothing protects the references. Kiesel uses Boehm's conservative collector instead of its own [19].

### 7.5 What this means for Arbor

The following are **inferences, to be validated by S1**:

1. **Contain the GC-reference burden.** Only three layers must hold references into the GC heap: the JS engine, the DOM, and the generated WebIDL bindings.
   - Style, layout, paint, networking and the HTML tokenizer don't need to. They can work on node IDs and produce their own non-GC outputs: immutable fragment trees and display lists.
   - This matches the LayoutNG model in the [decision map](../decision-map.md) [48].
   - It confines Rust's weak spot to a fraction of the codebase.
2. **Let code generation absorb the ceremony.** In every engine, most DOM entry points are generated from WebIDL. Generated code can carry rooting boilerplate mechanically (pattern B), and Servo generates its bindings the same way (background).
3. **Prefer a design that keeps `unsafe` inside the GC crate.** Shortlist for S1:
   - Pattern A: sound, but it forces builtins into a resumable style.
   - Pattern B: checked, but verbose.
   - Pattern D′: an unsafe core with typed handles and an explicit shadow-stack root set. This is the classic precise design.
4. **Hypothesis: a data-oriented heap is an opportunity, not only a workaround.** Nova shows that typed arenas, 32-bit handles and splitting frequently used fields from rarely used ones can coexist with code that follows the specification's structure [15]. On an 8 GB budget, compact handles and compaction directly serve our memory goals.

---

## 8. Interpreter and JIT

Arbor's JS engine starts as a bytecode interpreter, with any JIT gated behind isolation (ADR-0001, consequence 4).

**Interpreter dispatch** differs by language:

| Language | Dispatch options | Source |
|---|---|---|
| C/C++ | Computed goto (a GNU extension); `[[clang::musttail]]` tail-call threading | Background |
| Zig | Labelled `switch` with `continue`, designed for this purpose (0.14+); `@call(.always_tail, …)` | [30]; background |
| Rust | `match` in a loop. Explicit tail calls (`become`) are unstable, with stabilization targeted for 2027 at the earliest | [32] |
| Swift | No computed goto and no guaranteed tail calls | Background |

**Hypothesis:** the dispatch technique matters less than bytecode design, value representation and inline caches. Spike S3 can measure the difference on the M1.

**JIT constraints on Apple Silicon** apply to every language [27]:
- Under the hardened runtime, `mmap` with `MAP_JIT` requires the `com.apple.security.cs.allow-jit` entitlement.
- With that entitlement, the process may create only *one* `MAP_JIT` region.
- A page is never writable and executable at the same time. Each thread switches between the two with `pthread_jit_write_protect_np`.

**The JIT is the riskiest component.**
- Apple's Lockdown Mode disables JavaScript JIT compilation [28].
- Microsoft Edge's enhanced security mode disables it on sites the user rarely visits [29].
- ADR-0001 already records the policy: interpreter first, JIT later, and a no-JIT mode kept available.

**Cranelift is a possible future backend.** It is a code generator written in Rust and used by Wasmtime (background). It could serve as an optimizing tier or as the Wasm compiler. **Inference:** this advantage is specific to Rust, because Cranelift can be reused without crossing a language boundary.

---

## 9. macOS integration

| Need | Rust | C++ | Swift | Zig |
|---|---|---|---|---|
| AppKit: windows, events, menus, text input (`NSTextInputClient`) | `objc2` generated bindings; subclassing through objc2's macros [40] | Objective-C++ | Native | C ABI plus manual `objc_msgSend` |
| Metal, `CAMetalLayer`, IOSurface | `objc2-metal`, used by wgpu since January 2026 [40] | Objective-C++ or `metal-cpp` | Native | Manual |
| CoreText, CoreGraphics (C APIs) | Direct C FFI | Native | Native | C ABI |
| Security.framework (certificate trust evaluation) | C FFI or objc2 | Native | Native | C ABI |
| XPC, Mach ports, sandbox (C APIs) | C FFI | Native | Native | C ABI |
| NSAccessibility | AccessKit's `accesskit_macos` adapter [41] or our own objc2 code | Objective-C++ | Native | Manual |

**Inference:**
- Most of the platform surface Arbor needs is a C or Objective-C API.
- Swift's advantage is ergonomics, not capability.
- If the AppKit shell later grows complex (preferences, menus, rich UI chrome), a thin Swift shell is a contained, later option (§13).

---

## 10. Concurrency and parallelism

What Arbor will need:
- parallel style computation (Stylo-like);
- parallel layout, which LayoutNG's immutable fragments make possible [48];
- one JS heap per agent, on worker threads;
- `SharedArrayBuffer`.

| Language | Guarantees |
|---|---|
| Rust | `Send`/`Sync` checked at compile time; scoped threads; work-stealing via rayon, which Stylo's cascade uses [4] |
| Swift | Swift 6 strict concurrency (actors, `Sendable`); `Atomic` and `Mutex` [36] |
| C++ | `std::atomic` and the memory model; races are found only at run time, by ThreadSanitizer |
| Zig | Atomics and threads; no static race detection |

**Inference:**
- `SharedArrayBuffer` deliberately exposes racy shared memory to JS, so its implementation needs `unsafe` in Rust as well. It belongs in the `gc` crate's audited boundary.
- Thread placement on performance versus efficiency cores is set through the C API `pthread_set_qos_class_self_np` (background), which works the same from any language.

---

## 11. Tooling on this machine (Command Line Tools only)

| Need | Rust | C++ | Swift | Zig |
|---|---|---|---|---|
| Build | `cargo` | CMake/GN/Bazel with Apple clang | SwiftPM | `zig build` |
| CPU profiling without Xcode | `samply` (Firefox Profiler UI) works on macOS [37] | `samply` works for native binaries (background) | same | same |
| Instruments | Needs Xcode, for every language (OQ-7) | | | |
| Fuzzing | `cargo-fuzz` on Apple Silicon (nightly toolchain) [38] | Apple clang lacks libFuzzer; needs a separate LLVM [34] | libFuzzer via swift.org toolchains; friction reported [35] | Built-in fuzzer, still maturing [31] |
| UB and memory-error detection | Miri for unsafe code (no inline asm, limited FFI; background [39]); AddressSanitizer on nightly | AddressSanitizer and UBSan in Apple clang (background) | ASan and TSan (background) | Safety-checked build modes (background) |
| JS-engine fuzzing with Fuzzilli | Hypothesis: feasible via LLVM sanitizer coverage, the instrumentation `cargo-fuzz` already uses | Native fit; Fuzzilli's targets are C/C++ engines [49] | — | Hypothesis |
| Debugger | lldb | lldb | lldb | lldb |

---

## 12. Ecosystem, licensing, and supply chain

**Supporting libraries available in Rust.** These are examples of what exists, **not decisions**; each will need its own justification:

| Area | Option | Evidence |
|---|---|---|
| Unicode, i18n | ICU4X 2.0, a Unicode Consortium project (May 2025) | [42] |
| Text shaping | HarfRust, a port of HarfBuzz. 0.13.x tracks HarfBuzz 14.3.1, and the library has no unsafe code except via `bytemuck`. It is not a complete port | [43] |
| Images | `png` (used by Chromium); `jxl-rs` (Chrome's JPEG XL decoder) | [44], [45] |
| GPU | wgpu (Metal through `objc2-metal`) | [40] |
| Accessibility | AccessKit (`accesskit_macos`) | [41] |
| TLS, HTTP | rustls, hyper, quinn | Background |
| Engines to study (not use) | Servo (embeddable crate), Ladybird's Rust code, Boa, Nova, Brimstone | [7], [6], [16], [15], [18] |

**The C/C++ incumbents** (HarfBuzz, ICU, Skia, FreeType, BoringSSL) are mature. Every C/C++ decoder is still attack surface: the 2023 libwebp overflow was exploited in the wild [46].

**Supply-chain risk (inference).** crates.io makes dependency sprawl easy. Mitigations, to be finalized in the dependency policy:
- written justification for every dependency, as the charter requires;
- `cargo-deny` and `cargo-vet`;
- a license allow-list;
- minimal feature flags;
- preferring crates already used by Chromium or Firefox.

---

## 13. Mixed-language architectures

| Option | Assessment |
|---|---|
| **Rust everywhere** (recommended) | One toolchain, one safety model, one build. Platform access through objc2 and C FFI |
| Rust engine + thin Swift shell | Viable *later* if AppKit UI work grows. Costs two toolchains and an FFI boundary. Kept as an option, not adopted |
| C++ VM and DOM + Rust elsewhere (Ladybird's transitional shape) | FFI-heavy, and the most security-critical code (DOM, bindings, JIT) stays unsafe. Considered only if S1 fails (§15.6) |
| Swift engine + a JS VM in another language | Splits the unified heap across languages; contradicts ADR-0001. Rejected |
| Zig VM + Rust elsewhere | Two language boundaries around the most sensitive code. Rejected |

---

## 14. Comparison matrix

Scores run from 1 to 5. The weighted total is out of 500.

| Criterion (weight) | Rust | C++ | Swift | Zig |
|---|---|---|---|---|
| C1 Memory safety, non-GC code (20) | **5**: safe by default; unsafe is opt-in and auditable | **2**: hardening covers bounds; use-after-free remains; no tagging on M1 | **4**: safe by default; unsafe needed in performance code | **2**: bounds checks only; no use-after-free protection |
| C2 Shared GC heap + DOM (15) | **3**: viable patterns, each with a known cost (§7.3) | **5**: natural; production designs and libraries exist | **2**: ARC conflicts; the heap would be unsafe Swift; no precedent | **3**: natural to write; no reference safety; Kiesel uses Boehm |
| C3 Performance and predictability (12) | **5**: no GC or refcount; control over layout; SIMD | **5** | **3**: ARC, exclusivity and protocol/generic overheads need expertise | **5** |
| C4 Concurrency safety (10) | **5**: Send/Sync; Stylo precedent | **2**: runtime detection only | **4**: Swift 6 strict concurrency | **2** |
| C5 macOS integration (8) | **4**: objc2 generated bindings; C FFI | **4**: Objective-C++ | **5**: native | **2**: manual |
| C6 Tooling on this machine (8) | **4**: samply, cargo-fuzz, Miri; Instruments needs Xcode | **4**: sanitizers; libFuzzer needs another LLVM | **3**: fuzzing friction; no Miri equivalent | **3**: maturing |
| C7 Supporting ecosystem (8) | **5**: ICU4X, HarfRust, Chromium decoders, wgpu, AccessKit | **5**: incumbents (all memory-unsafe) | **3**: Apple frameworks, usable from any language | **2** |
| C8 Interpreter/JIT (5) | **4**: no stable tail calls; Cranelift available | **5** | **2** | **5** |
| C9 Stability, longevity (5) | **5** | **5** | **4**: Apple-governed; fast evolution | **2**: pre-1.0 churn |
| C10 Learning curve, AI fit (5) | **4**: steep, but compiler errors give a tight feedback loop (inference) | **3**: undefined behaviour is silent (inference) | **3** | **2**: small corpus plus churn (inference) |
| C11 Build cost on 8 GB (4) | **2**: known slow and memory-hungry | **3** | **3** | **5** |
| **Weighted total (/500)** | **432** | **376** | **331** | **286** |

### Sensitivity analysis

| Scenario | Rust | C++ | Swift | Zig | Leader |
|---|---|---|---|---|---|
| Baseline | 432 | 376 | 331 | 286 | Rust |
| GC suitability (C2) weight doubled, 15 → 30 | 477 | 451 | 361 | 331 | Rust |
| Memory safety (C1) weight halved, 20 → 10 | 382 | 356 | 291 | 266 | Rust |
| **Both of the above** | 427 | **431** | 321 | 311 | **C++, narrowly** |
| macOS integration (C5) weight doubled, 8 → 16 | 464 | 408 | 371 | 302 | Rust |
| Rust's C2 score collapses to 1 (S1 goes badly) | 402 | 376 | 331 | 286 | Rust |
| **Rust's C2 → 1 *and* C1 → 3** (unsafe leaks out of the GC crate) | **362** | **376** | 331 | 286 | **C++** |

**What the analysis shows:**
- C++ wins only in two situations:
  - GC convenience is weighted twice as heavily *and* memory safety half as heavily. That would mean ranking implementation convenience above the sensitive-login posture chosen in ADR-0001.
  - Rust's GC story fails badly *and* the failure spreads `unsafe` into the DOM and bindings.
- So S1's real test is not "is GC in Rust pleasant?" but **"can `unsafe` stay contained?"** That is the revisit condition in §15.6.

---

## 15. Recommendation

### 15.1 Decision (proposed)

**Rust** (stable toolchain, 2024 edition) for all Arbor code:
- the browser shell;
- the engine;
- the JavaScript engine, including its GC and any future JIT;
- the platform layer, through `objc2` and C FFI.

The nightly toolchain is allowed only for development tools (fuzzing, Miri, sanitizers). It is never required to build the product.

### 15.2 Justification

1. **Security (R3 → C1).** Rust is the safest default for the code with the most attack surface: parsers, decoders, network policy and IPC. The strongest evidence available shows that the language chosen for *new* code is the most effective lever on memory-safety vulnerabilities [2]. All of our code is new.
2. **Parallelism and multiple agents (R1 → C4).** Data races are ruled out at compile time. The only production parallel style system was built this way after C++ attempts failed [4].
3. **Our own JS engine (R2 → C2).** This is Rust's weakest criterion, and we accept that deliberately:
   - viable designs exist, with evidence (§7.3);
   - the burden is confined to the JS, DOM and bindings layers (§7.5);
   - generated code absorbs the ceremony;
   - S1 validates the approach before acceptance.
4. **Tooling and ecosystem (C6, C7).** The tools work without Xcode, and key supporting libraries are proven in Chromium and Firefox.
5. **Platform (C5).** `objc2` and C FFI cover what Arbor needs. Swift's advantage is ergonomic, not a capability we would otherwise lack.
6. **Independent corroboration.** Ladybird reached the same conclusion in 2026 despite raising the same GC and inheritance concern [5], [6].

### 15.3 Unsafe-code policy (draft; finalized with the architecture)

1. Every crate starts with `#![forbid(unsafe_code)]`.
2. `unsafe` is allowed only in designated crates:
   - `gc`: heap, collector, handles, and the `SharedArrayBuffer` backing store;
   - `jit`: code emission and W^X switching, later;
   - `platform`: objc2 and C FFI for AppKit, Metal, CoreText, Security, XPC and the sandbox;
   - narrowly scoped performance modules, admitted only with a benchmark that shows the need.
3. Every `unsafe` block carries a `// SAFETY:` comment stating the invariant it relies on, enforced by Clippy's `undocumented_unsafe_blocks` lint (background).
4. Unsafe crates get extra verification: Miri where applicable, fuzzing, sanitizer builds and focused review.
5. **No `unsafe`** in:
   - parsers: HTML, CSS, URL, encodings;
   - network policy: Fetch, CORS, cookies;
   - IPC message decoding;
   - image and font parsing wrappers.
6. Each unsafe crate exposes a safe API. A soundness bug in an unsafe crate is treated as a security bug.
7. **Macro-generated `unsafe` is part of the unsafe boundary** (added 2026-10-16).
   - **The gap:** `#![forbid(unsafe_code)]` does not fire on `unsafe` produced by *external* macros, whether a proc-macro derive or a `macro_rules!` from another crate. It does reject hand-written `unsafe`. This was verified on rustc 1.94.1 with a minimal three-crate experiment ([Step 2b plan §4.5](../research/step-2b-validation-spikes.md#45-finding-recorded-before-implementation-2026-10-16)).
   - **What `forbid` proves:** that a crate has no *hand-written* `unsafe`. It does not prove the expanded code has none.
   - **The rule:** any macro that can emit `unsafe` (for example `#[derive(Trace)]`) must live in a designated crate, be audited like the `unsafe` code it generates, and be registered in `tools/unsafe-policy.txt`.

### 15.4 Toolchain and dependencies (draft)

- Stable Rust, pinned in `rust-toolchain.toml` when implementation begins. It is not pinned now, because no code exists yet.
- Each dependency needs a written justification and must state whether it is a required platform facility, a replaceable implementation detail, or a foundational dependency (charter).
- Dependencies are audited with `cargo-deny` and `cargo-vet` against a license allow-list.
- Performance claims are measured on the target M1.

### 15.5 Risks and mitigations

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| GC references are awkward in the JS, DOM and bindings code | High | High | S1; layering (§7.5); generated bindings; choose the pattern before ADR-0002 is accepted |
| Modelling the inheritance-heavy DOM | Medium | Medium | Enum node kinds, traits and generated dispatch; trait upcasting; study Servo's and Ladybird's models |
| Build time and memory on 8 GB | Medium | Medium | Split the workspace; incremental builds; Cranelift backend for debug builds (supports macOS on AArch64 [50]); measure in S2 |
| Interpreter dispatch without tail calls | Medium | Low–medium | Benchmark in S3; revisit when `become` stabilizes [32] |
| objc2 is awkward for AppKit and accessibility | Medium | Low | AccessKit; thin Swift shell as a later option (§13) |
| Owner's Rust fluency unknown | Unknown | Medium–high | OQ-6; learning plan; AI-assisted review |
| Supply-chain sprawl | Medium | Medium | Dependency policy (§15.4) |
| `SharedArrayBuffer` and the JIT need `unsafe` | Certain | Medium | Confined to the `gc` and `jit` crates; fuzzing; Miri where possible |

### 15.6 Conditions that would reopen this decision

1. **S1 shows `unsafe` cannot stay inside the `gc` crate.** For example, DOM or binding code needs `unsafe` to hold GC references, and ergonomics are poor. This is the scenario in §14 where C++ overtakes Rust (362 vs 376). The response would be to re-evaluate a C++ GC and DOM core with Rust elsewhere, or a different GC design.
2. **The owner is fluent in C++ or Swift but not in Rust**, and the time budget (OQ-1) can't absorb the learning cost.
3. **Measured clean or incremental builds on the M1 make iteration impractical**, even after mitigations. S2 sets the thresholds.
4. **The evidence changes materially.** Examples:
   - Rust gains GC-friendly language support, such as a sound form of the "contravariant handles" approach.
   - Swift gains an ownership model that suits a tracing GC.
   - C++ gains enforceable protection against use-after-free.

---

## 16. Proposed validation spikes (not authorized; they need explicit approval)

**S1: GC and DOM heap (highest priority).** A throwaway Rust prototype.
- **Contents:** a heap of DOM-like nodes (tree plus attributes); JS-like objects and closures with cycles (listener → closure → node); weak references; an allocation-heavy workload.
- **Designs to compare, two or three of them:**
  - A: `gc-arena` style;
  - B: Nova-style scoped handles;
  - D′: an unsafe core with typed handles and a precise shadow-stack root set.
- **Measured on the M1:**
  - allocation throughput;
  - longest pause;
  - memory overhead per object;
  - ceremony per typical DOM operation;
  - whether `unsafe` stays inside the GC crate;
  - how builtins that call back into JS are expressed.
- **Output:** a research note and the chosen pattern.
- **Acceptance:** at least one design keeps `unsafe` inside the GC crate, with ceremony the owner judges acceptable.

**S2: Platform and build.** An objc2 AppKit window with a `CAMetalLayer` and a CoreText glyph run drawn to pixels.
- Measure clean and incremental build times.
- Measure the peak memory (RSS) of `rustc` and the linker on the 8 GB machine.
- Set the thresholds used by revisit condition 3.

**S3 (optional): interpreter dispatch micro-benchmark.** Compare a `match` loop against alternatives on the M1.

None of these commits the project to an implementation. Each produces a research note.

---

## 17. Sources

1. Chromium Security, "Memory safety" — https://www.chromium.org/Home/chromium-security/memory-safety/
2. Google Security Blog, "Eliminating Memory Safety Vulnerabilities at the Source" (2024-09-25) — https://security.googleblog.com/2024/09/eliminating-memory-safety-vulnerabilities-Android.html
3. Google Security Blog, "MiraclePtr: protecting users from use-after-free vulnerabilities on more platforms" (2024-01) — https://security.googleblog.com/2024/01/miracleptr-protecting-users-from-use.html
4. Rust Blog, "Fearless Concurrency in Firefox Quantum" (2017-11-14) — https://blog.rust-lang.org/2017/11/14/Fearless-Concurrency-In-Firefox-Quantum/
5. Ladybird, "Ladybird adopts Rust, with help from AI" (2026-02-23) — https://ladybird.org/posts/adopting-rust/
6. Ladybird, "This Month in Ladybird — August 2026" — https://ladybird.org/newsletter/2026-08-31/
7. Servo, "Servo is now available on crates.io" (2026-04-13) — https://servo.org/blog/2026/04/13/servo-0.1.0-release/
8. Mozilla Research, "JavaScript: Servo's only garbage collector" (2014-08-26) — https://research.mozilla.org/2014/08/26/javascript-servos-only-garbage-collector/
9. Anderson et al., "Experience Report: Developing the Servo Web Browser Engine using Rust" — https://arxiv.org/pdf/1505.07383
10. V8 Blog, "Oilpan library" — https://v8.dev/blog/oilpan-library
11. WebKit Blog, "Introducing Riptide: WebKit's Retreating Wavefront Concurrent Garbage Collector" — https://webkit.org/blog/7122/
12. Ladybird source, `Libraries/LibGC/Heap.cpp` — https://github.com/LadybirdBrowser/ladybird/blob/master/Libraries/LibGC/Heap.cpp
13. `gc-arena` — https://github.com/kyren/gc-arena ; https://docs.rs/crate/gc-arena/0.6.1
14. Nova, "Garbage collection is contrarian" (2026-01-09) — https://trynova.dev/blog/garbage-collection-is-contrarian
15. `nova_vm` 1.0.0 documentation — https://docs.rs/crate/nova_vm/latest
16. Boa, "Roadmap 2026" — https://boajs.dev/roadmap (the test262 snapshot is a secondary source: https://www.issoh.co.jp/tech/details/3100/)
17. `boa_engine` documentation — https://docs.rs/crate/boa_engine/latest
18. Brimstone — https://github.com/hans-halverson/brimstone
19. Kiesel — https://codeberg.org/kiesel-js/kiesel
20. Swift.org, "Swift 6.2 Released" — https://swift.org/blog/swift-6.2-released/
21. Web Engines Hackfest 2026, "Memory Safety in WebKit" (session notes) — https://github.com/Igalia/webengineshackfest/wiki/2026-Memory-Safety-in-WebKit
22. Apple Platform Security, Memory Integrity Enforcement — https://support.apple.com/guide/security/sec8b776536b/web
23. Chromium, "Unsafe Buffers" — https://chromium.googlesource.com/chromium/src.git/+/lkgr/docs/unsafe_buffers.md
24. Herb Sutter, trip report, February 2025 ISO C++ meeting (P3471 adopted) — https://herbsutter.com/2025/02/
25. Whole Tomato, "C++26 memory safety: what actually ships" (secondary) — https://www.wholetomato.com/blog/c26-memory-safety-what-actually-ships-for-existing-code/
26. App Developer Magazine, "Safe C++ proposal abandoned" (secondary) — https://appdevelopermagazine.com/safe-c-plus-plus-proposal-abandoned-after-community-pushback/
27. Apple, "Porting just-in-time compilers to Apple silicon" — https://developer.apple.com/documentation/apple-silicon/porting-just-in-time-compilers-to-apple-silicon
28. Schneier on Security, "Apple's Lockdown Mode" (quotes Apple) — https://www.schneier.com/blog/archives/2022/07/apples-lockdown-mode-2.html
29. PortSwigger, "Microsoft Edge … enhanced security mode" — https://portswigger.net/daily-swig/microsoft-edge-deepens-defenses-against-malicious-websites-with-enhanced-security-mode
30. Zig 0.14.0 release notes (labelled switch) — https://ziglang.org/download/0.14.0/release-notes.html
31. LWN, "Zig 0.16.0 released" — https://lwn.net/Articles/1067634/
32. Trifecta Tech, "Rust should have stable tail calls" (2026) — https://trifectatech.org/blog/tail-calls-project-goal/ ; https://doc.rust-lang.org/std/keyword.become.html
33. Rust Blog, "Announcing Rust 1.86.0" (trait upcasting) — https://blog.rust-lang.org/2025/04/03/Rust-1.86.0/
34. Apple Developer Forums, libFuzzer absent from Apple clang — https://developer.apple.com/forums/thread/818248
35. Swift, libFuzzer integration docs (older branch) — https://github.com/apple/swift/blob/swift-4.2-branch-06-11-2018/docs/libFuzzerIntegration.rst
36. Swift Evolution SE-0410 (Atomics), SE-0433 (Mutex) — https://forums.swift.org/t/accepted-with-modifications-se-0410-atomics/69244 ; https://forums.swift.org/t/se-0433-synchronous-mutual-exclusion-lock/71174
37. samply — https://github.com/mstange/samply
38. cargo-fuzz — https://github.com/rust-fuzz/cargo-fuzz
39. Miri (README, limitations; background) — https://github.com/rust-lang/miri
40. objc2 — https://docs.rs/objc2/latest/objc2/ ; wgpu migration to objc2-metal — https://github.com/gfx-rs/wgpu/pull/5641
41. AccessKit — https://github.com/AccessKit/accesskit
42. Unicode Blog, ICU4X 2.0 released (2025-05-29) — https://blog.unicode.org/search/label/ICU4X
43. HarfRust 0.13.2 — https://docs.rs/crate/harfrust/0.13.2
44. Phoronix, Rust PNG decoder used by Chrome (2026) — https://phoronix.com/news/Rust-PNG-Image-Faster
45. Chrome for Developers, "Shipping JPEG XL in Chrome" — https://developer.chrome.com/blog/jpeg-xl-in-chrome
46. Isosceles, "The WebP 0day" — https://blog.isosceles.com/the-webp-0day/
47. Site Isolation (USENIX Security 2019) — https://research.google/pubs/site-isolation-process-separation-for-web-sites-within-the-browser/
48. Chrome for Developers, "RenderingNG deep-dive: LayoutNG" — https://developer.chrome.com/docs/chromium/layoutng
49. Fuzzilli — https://github.com/googleprojectzero/fuzzilli
50. rustc_codegen_cranelift (macOS AArch64 support) — https://github.com/rust-lang/rustc_codegen_cranelift
