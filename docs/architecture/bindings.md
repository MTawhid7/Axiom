# WebIDL Bindings: The Generator, Platform Objects, Conversions and Cross-Origin Objects

| | |
|---|---|
| **Status** | **Proposed** (Step 5, CP17). To be decided with part 2's ADRs at CP18, for the owner at Gate C. No Step 4 interface changes (SC1) |
| **Question** | How are the WebIDL bindings generated: platform objects in the heap, argument conversion, rooting conventions, prototypes and realms, exceptions, cross-origin objects, and the hot path of event dispatch? |
| **Answers** | Step 5 question Q13 ([tracker](../research/step-5-subsystems.md) §3); the bindings part of the handed-over question 18 (§2.2); closes [HTML and the DOM](html-and-dom.md)'s JavaScript facet (HD30) |
| **Feeds** | Part 2's ADRs (CP18); every subsystem that exposes an interface to script |
| **Builds on** | [Script and heap](script-and-heap.md) (SH1, SH2, SH6, SH7, SH9, SH12); [ADR-0003](../adr/0003-gc-handle-and-rooting-pattern.md); [ADR-0002](../adr/0002-implementation-language.md) (rule 7); [the JavaScript engine](javascript-engine.md) (JE8–JE11, JE16, JE18, JE32); [HTML and the DOM](html-and-dom.md) (HD10, HD16–HD18, HD25, HD30); the [frame tree](frame-tree-and-navigation.md) (FT3–FT7); the [event loop](event-loop-and-threading.md) (EL14); the [dependency policy](../technology/dependency-policy.md) (DP1, DP2) |
| **Last updated** | 2026-10-19 |

Everything here is **planned**; nothing is implemented. Rules are numbered WB1–WB23. Precedent is sourced (§3); the design is inference unless marked otherwise.

## Summary

- **The WebIDL compiler is Axiom's own** (DP1 (h)), a build-time tool that reads the specifications' IDL through webref's curated extracts, the same files WPT's interface tests use, plus a configuration that maps interfaces to Rust types. **Generated code is safe, readable Rust** (ADR-0002 rule 7, SH12) (WB1–WB3).
- **Platform objects are heap entries, not wrappers** (SH1): a node's handle is its platform object, and every other interface's objects live in typed arenas. Brand checks are range checks on interface IDs. Interface objects and prototypes are created per realm, on first use. Ladybird, by contrast, now builds wrapper objects with preservation, and Servo keeps a SpiderMonkey reflector per object (WB4–WB9).
- **Calls go straight from the interpreter to generated functions,** and conversions and overload resolution follow Web IDL exactly. **The generator knows which conversions can run script,** so it roots a converted argument only when a later conversion may collect, and nothing in the common all-primitive case; the scope tokens make a missed root a compile error (WB10–WB13).
- **Exceptions, `[CEReactions]` and callbacks** are generated as Web IDL and HTML specify; callbacks are handles held by their owners, so storing one needs no root (WB14–WB16). **Event dispatch** roots two heap arrays and calls listeners directly, with S1's dispatch case as its benchmark (WB17).
- **Cross-origin objects:** `WindowProxy` is an exotic object in the engine, exposing only HTML's cross-origin properties across origins. **A frame in another agent is a remote `WindowProxy`** whose every operation is a message to the browser (FT5, FT6), and a proxy keeps its identity when its frame moves between local and remote (WB18–WB21).

## 1. Binding inputs

| Input | Consequence here |
|---|---|
| **DP1 (h):** the WebIDL generator is core, Axiom's own | WB1; §4 |
| **SH1, SH12, ADR-0002 rule 7:** DOM objects are JavaScript objects with no wrappers; bindings are generated, safe and mechanical; macro-generated `unsafe` counts as part of the boundary | WB2, WB4 |
| **ADR-0003, SH6, SH7, SH9:** values native code holds across a possible collection are rooted on the `SliceStack`; scope tokens make an unrooted handle held across a collection a compile error; fields that hold handles are written through generated setters | WB13, WB16 |
| **JE8–JE11, JE18:** inline caches as data; the NaN-boxed value carrying the scope lifetime; platform objects as arena entries; realms with intrinsics created on demand | WB5, WB6, WB10 |
| **HD10, HD16–HD18, HD25, HD30:** the node arena's kind column; live collections keyed by a tree version; listeners in a side table; dispatch rooting two heap arrays; custom element reactions; the DOM's JavaScript facet as an interface for CP17 | WB4, WB7, WB8, WB15–WB17 |
| **FT3–FT7:** tokens issued by the browser; a skeleton replica of the frame tree; remote window proxies with only the cross-origin properties; every operation between agents through the browser | WB19–WB21 |
| **S3 results §9, item 6; S1:** root many values as one heap array; keep S1's dispatch case as a benchmark | WB17, WB23 |

## 2. Specifications

- **WHATWG Web IDL,** last updated 2026-10-10 [W1]: the IDL grammar (§2), the JavaScript binding (§3), including extended attributes such as `[SameObject]`, `[LegacyUnforgeable]`, `[LegacyOverrideBuiltIns]` and `[LegacyWindowAlias]` (§3.3–§3.4), the security check (§3.5), the overload resolution algorithm (§3.6), interface objects, interface prototype objects and the named properties object (§3.7), platform objects and legacy platform objects (§3.8–§3.9), and observable arrays (§3.10). **"Platform objects are objects that implement an interface"; legacy platform objects are those that support indexed or named properties without `[Global]`.**
- **WHATWG HTML,** last updated 2026-10-19 [H1]:
  - the security infrastructure for `Window`, `WindowProxy` and `Location` (§7.2.1): `CrossOriginProperties` lists, for `Location`, a setter for `href` and `replace`; for `Window`, `window`, `self`, `location`, `close`, `closed`, `focus`, `blur`, `frames`, `length`, `top`, `opener`, `parent` and `postMessage`, with indexed properties "handled directly by the `WindowProxy` object";
  - the `WindowProxy` exotic object (§7.2.3), whose wrapped `Window` "is changed" when its browsing context navigates;
  - `[CEReactions]` (§4.13), which pushes an element queue onto the agent's custom element reactions stack, runs the original steps, pops the queue and invokes its reactions, then rethrows or returns;
  - the callback steps of scripting (§8.1): preparing to run a callback, and cleaning up after it.

## 3. Precedent

Read on 2026-10-19; dates in brackets are the last commits to each file, from GitHub's API.

| Engine | What the sources show |
|---|---|
| **Chromium** (Blink) | "High-performance C++ bindings between V8 and DOM per the Web IDL spec", generated by an IDL compiler written in Python with Jinja templates (`bindings/README.md` [2023-08-25]) [B1] |
| **Firefox** (Gecko) | "The Web IDL bindings are generated at build time based on two things: the actual Web IDL file and a configuration file that lists some metadata about how the Web IDL should be reflected into Gecko-internal code" (*Web IDL bindings*, Firefox source docs) [F1] |
| **WebKit** | Bindings generated by a Perl generator, `CodeGeneratorJS.pm` [2026-10-16] [K1] |
| **Servo** | A `script_bindings` crate with a Python generator (`codegen/codegen.py`) and a `Bindings.conf` configuration. Each DOM object keeps "a reference to the reflector of a DOM object", its SpiderMonkey object, with a "cached prototype ID for fast type checks" (`reflector.rs`); its stack roots carry a "# Safety" contract and an `unsafe` trait, `StableTraceObject` (`root.rs`) [S1] |
| **Ladybird** | **Wrappers as host objects:** "Each interface now gets a generated `create_x_wrapper()` function", creating "a plain `JS::HostObject` of the interface's host class with the interface prototype object and the implementation object"; the generator had emitted "one C++ class per IDL interface, 583 in all" (commit `c53fa722125d`, 2026-10-05). Wrappers are preserved "once script gives it state of its own" (`PlatformObject.h` [2026-10-05]) [L1] |
| **IDL sources** | **webref** publishes "machine-readable references" extracted from specifications, with curated extracts that apply "manually-maintained patches to fix invalid content and provide validity and consistency guarantees", and an `@webref/idl` package (`README.md` [2026-10-16]) [R1]. **WPT's `interfaces/`** copies its IDL files from `@webref/idl` "by a workflow that tries to sync the files daily", for `idlharness.js` tests (`interfaces/README.md` [2021-10-07]) [T1] |

**Observations** (inference):
- **Every engine generates its bindings at build time from IDL.** Blink's and Servo's generators are Python and WebKit's is Perl (§3); Axiom's is Rust, like the rest of its build tooling.
- **Servo and Ladybird keep two objects per DOM object,** a native one and its JavaScript reflector or wrapper (§3), as Blink, Gecko and WebKit do (background), and must keep the wrapper alive while script can observe its state: Ladybird's "preservation", for example. SH1's single heap removes both the second object and that logic.
- **The IDL itself is shared infrastructure:** webref's curated extracts already feed WPT's interface tests, so Axiom's generator and its test oracle can read the same files.

## 4. Build or reuse

| Need | Candidates | Decision |
|---|---|---|
| **The WebIDL compiler** | Blink's, Gecko's and Servo's generators (Python), WebKit's (Perl); `webidl2.js`; **our own** | **Our own** (DP1 (h)), in Rust, run at build time |
| **The IDL** | Copies kept by hand, as engines do; **webref's curated `@webref/idl`** | **webref's curated extracts,** pinned and checked into the repository as data (DP2), updated deliberately and reviewed as diffs |

No new dependency is proposed: the IDL is data, and the generator is Axiom's.

## 5. Design

### 5.1 The generator

| Rule | Decision |
|---|---|
| WB1 | **A WebIDL compiler of Axiom's own** (DP1 (h)), run at build time. It parses Web IDL's grammar; merges partial interfaces, mixins and `includes`; resolves typedefs, dictionaries, enums, callbacks and namespaces; validates the result; and generates Rust. **Its input** is the specifications' IDL as webref's curated extracts provide it, pinned in the repository (§4), plus **a configuration** saying which Rust type implements each interface and which members are implemented, as Gecko's configuration file and Servo's `Bindings.conf` do (§3) |
| WB2 | **Generated code is safe, readable Rust** (ADR-0002 rule 7, SH12): no `unsafe`, and no macros that expand to it. It passes the workspace's lints like hand-written code, and the same input always generates the same bytes, so builds stay reproducible. The generator is a build tool and never ships |
| WB3 | **Only implemented members are exposed.** A member the configuration doesn't mark as implemented is absent, not a stub that throws, so feature detection (`'x' in object`) tells the truth. `[Exposed]`, `[SecureContext]` and `[CrossOriginIsolated]` decide, per global, which interfaces and members a realm sees |

### 5.2 Platform objects

| Rule | Decision |
|---|---|
| WB4 | **Platform objects are heap entries, not wrappers** (SH1, HD30). A node's handle is its platform object; every other interface's objects (`Event`, `Range`, `DOMRect`, `Response` and the rest) are entries in their own typed arenas. A JavaScript value carries the handle with its arena's kind (JE9). There is no second object to create, cache or preserve, unlike Ladybird's preserved wrappers and Servo's reflectors (§3) |
| WB5 | **Brand checks are range checks.** Interfaces are numbered in depth-first order of their inheritance tree, so "implements `X`" is a comparison of the object's interface ID with `X`'s range. The ID comes from the arena kind and, for nodes, the kind column and local name (HD10, HD30). Servo caches a prototype ID for the same purpose (§3) |
| WB6 | **Interface objects, prototypes and named properties objects belong to their realm,** and each is created the first time script reaches it, from generated static tables of properties, so a realm that touches a few dozen of several hundred interfaces pays for those only (JE18). Legacy factory functions (`Image`, `Audio`, `Option`), `[LegacyWindowAlias]` names and namespaces follow Web IDL |
| WB7 | **Per-object state lives in rare data** (HD30): expando properties, cached `[SameObject]` results, and a property table created on the first write. **`[LegacyUnforgeable]` members** are reported as own, non-configurable properties by the object's internal methods, without being stored until the object gets a property table of its own. So every `Event` doesn't pay for its own `isTrusted` property, while script observes exactly what Web IDL specifies |
| WB8 | **Legacy platform objects** (`HTMLCollection`, `NodeList`, `DOMStringMap`, `Storage`, `HTMLFormElement` and the rest) get Web IDL's internal methods for indexed and named properties (§3.9), generated per interface, with `[LegacyOverrideBuiltIns]` and `[LegacyUnenumerableNamedProperties]`. Inline caches (JE8) don't cache their named lookups, except behind a guard on the live collection's tree version (HD16) |
| WB9 | **Declarations that generate members:** `iterable`, `async iterable`, `maplike` and `setlike` generate their methods as Web IDL specifies; `ObservableArray<T>` attributes, such as `adoptedStyleSheets`, are Web IDL's observable array exotic objects (§3.10) |

### 5.3 Conversions and calls

| Rule | Decision |
|---|---|
| WB10 | **Calls go straight from the interpreter to generated functions.** A platform operation or accessor is a native function that the interpreter calls with the receiver and its argument registers, without packing them into an array. The generated function checks the brand, converts the arguments, calls the Rust implementation, and converts the result. Call sites cache their target in their feedback (JE8) |
| WB11 | **Conversions follow Web IDL exactly,** left to right: numeric types with `[EnforceRange]` and `[Clamp]`; `DOMString` as PL11's strings, `USVString` with lone surrogates replaced, `ByteString` checked; nullable types and unions in Web IDL's order; sequences and records by iteration; dictionaries, whose members are read in order and may run getters; enums; callback types; buffer sources as views of backing stores outside the heap (SH11), with `[AllowShared]` and `[AllowResizable]`; promises. **No string passes through UTF-8** (PL11) |
| WB12 | **Overload resolution** follows Web IDL's algorithm (§3.6). The generator computes each effective overload set and its distinguishing argument index at build time, so the generated code selects by argument count, then by the distinguishing argument's type, in the order the algorithm specifies |
| WB13 | **Rooting conventions** (ADR-0003, SH6, SH7; handed-over question 18). **The generator knows which conversions can run script,** and so can collect: object-to-primitive conversions (`valueOf`, `toString`), dictionary getters, the iteration behind sequences, and callbacks. It roots an already converted handle argument, or the receiver, **only when a later conversion may collect,** and roots nothing when none can, the common case of primitive arguments. Results are bound to the scope (JE9), and because the scope tokens make an unrooted handle held across a collection a compile error, generated code that missed a root wouldn't build |
| WB14 | **Exceptions.** Implementations return a `Result` whose error is a `DOMException` name, a `TypeError` or `RangeError`, or a JavaScript value already thrown; the glue creates the exception object in the current realm. `DOMException` is a platform object, and serializable. **Promise-returning operations** turn an exception into a rejected promise, as Web IDL specifies |
| WB15 | **`[CEReactions]`** is generated around the annotated operations, attributes, setters and deleters exactly as HTML specifies (§2), on the agent's reactions stack that HD25 holds |
| WB16 | **Callbacks** (callback functions, and callback interfaces such as `EventListener`) are stored as handles in the objects that own them: the listener side table (HD17), an observer, a promise reaction. Those fields are traced and written through generated setters (SH9), so storing a callback needs no root. **Invoking one** follows Web IDL's "invoke a callback function" and "call a user object's operation", with HTML's steps to prepare to run a callback and clean up after it, and reports exceptions as specified |

### 5.4 The hot path of event dispatch

| Rule | Decision |
|---|---|
| WB17 | **Dispatch roots two arrays and calls listeners directly** (HD18). The event path and the listener snapshot are heap arrays rooted once each; each listener call is WB16's invocation, entering the interpreter with the event as its argument, without allocating per call. A callback interface's `handleEvent` is looked up as specified, through the call site's inline cache. **The benchmark** is S1's dispatch case, or a successor, measured whenever the bindings, the root stack or the interpreter change, as S3 recommended (handed-over question 18) |

### 5.5 Realms, globals and cross-origin objects

| Rule | Decision |
|---|---|
| WB18 | **Global objects:** `Window`, the worker global scopes (dedicated, shared and service) and worklet globals, each the global of its realm (JE18), with the interfaces its `[Exposed]` set names. **`Window`** has its named properties object, for named access to elements and child frames, and an immutable prototype chain, as `[Global]` requires |
| WB19 | **`WindowProxy` and `Location` within an agent.** `WindowProxy` is an exotic object in the engine (HTML §7.2.3). Its `[[Window]]` slot changes when its browsing context navigates within the agent. Its internal methods, and `Location`'s, perform HTML's checks (`IsPlatformObjectSameOrigin`), so a cross-origin access sees only `CrossOriginProperties` (§2). This happens inside one agent whenever frames are same-site but cross-origin, since clusters are keyed by site unless they ask for origin keying |
| WB20 | **Remote window proxies** (FT5, FT6). A frame whose document lives in another agent appears in this heap as a remote `WindowProxy`, a platform object holding the frame's token (FT3). It offers exactly `Window`'s `CrossOriginProperties` and indexed and named child access. FT4's skeleton answers `length`, `frames`, `top`, `parent`, `opener` and `closed` synchronously; `postMessage` (structured-cloned, SH2), a `location` assignment or `replace()` (a navigation request, FT7), `close`, `focus` and `blur` become messages to the browser. **One proxy per browsing context per agent:** each agent maps frame tokens to proxies, so identity holds, and when a frame moves between this agent and another, the same proxy object switches between a local `Window` and a remote reference, keeping its identity as HTML requires across navigations (inference) |
| WB21 | **Cross-origin functions** (`postMessage`, `focus`, `blur`, `close`, `replace` and the `location` accessors) are created per realm and kept in `[[CrossOriginPropertyDescriptorMap]]` as HTML specifies, so repeated cross-origin access returns the same function objects within a realm |

### 5.6 Size and tests

| Rule | Decision |
|---|---|
| WB22 | **Generated code has a size budget.** Several hundred interfaces generate a great deal of code; the generator shares conversion helpers and emits property tables as static data rather than per-member code wherever behaviour allows. Step 6 measures the generated code's size, its build time (T1–T4) and its effect on a ready renderer's footprint (B2) |
| WB23 | **Tests at boundaries.** **Oracles:**<br>• WPT's `idlharness.js` tests against the same IDL files [T1], and WPT's `webidl/` directory, which tests the JavaScript binding itself;<br>• the generator's golden tests: IDL in, Rust out, reviewed as diffs;<br>• the engine's stress pacing, collecting at every allocation (JE32), which exercises every generated rooting decision;<br>• S1's dispatch case, as WB17's benchmark |

## 6. Interfaces with neighbouring subsystems

| Neighbour | Interface |
|---|---|
| **The JavaScript engine** (JE8–JE11, JE16, JE18) | Native functions called with argument registers; the value type and its conversions; realms and their lazily created interfaces; promises and callbacks |
| **The heap** (SH6, SH7, SH9) | Arena entries as platform objects; generated setters; rooting by the generator's rules |
| **HTML and the DOM** (HD16–HD18, HD25, HD30) | The DOM's implementations behind generated glue; live collections; listeners and dispatch; custom element reactions |
| **The frame tree and the browser** (FT3–FT7) | Remote window proxies; operations between agents as messages; the skeleton replica for synchronous answers |
| **Every subsystem exposed to script** | Its IDL, its configuration entry, and a Rust implementation |

## 7. Security

- **The bindings are where script reaches native code,** so the generated glue is safe Rust (WB2) and its rooting is checked by the compiler (WB13). A conversion bug is a correctness bug, not memory corruption.
- **Cross-origin access exposes only HTML's list** (WB19–WB21), checked in the same agent and enforced by construction for remote frames, whose proxies can only send the browser the operations FT6 allows. A compromised renderer gains nothing from forging them: the browser validates every operation (FT6).
- **Brand checks precede every implementation call** (WB5), so script can't call an implementation on an object of the wrong interface.
- **Only implemented members are exposed** (WB3), so no stub claims capabilities that the engine doesn't have.

## 8. Staging and the complete model

| Stage | What exists |
|---|---|
| First script milestone | The generator; platform objects and brand checks; lazy interface objects; conversions, overloads and exceptions; callbacks and event dispatch; `Window` and `WindowProxy` within an agent |
| Next | Legacy platform objects in full; iterable and maplike declarations; observable arrays; `[CEReactions]`; remote window proxies (with L2's process separation, as FT5 already shapes them); worker and worklet globals |
| Later | Interfaces as their subsystems arrive, through their IDL and configuration entries |

**The complete model** (SC9): every interface on the web platform is IDL that the generator reads, a configuration entry, and a Rust implementation; new extended attributes extend the generator, not the engine.

## 9. Alternatives considered

| | Alternative | Assessment |
|---|---|---|
| A | **Wrapper objects,** as Blink, Gecko, Servo and now Ladybird use | Each DOM object would get a second object, and the engine would need wrapper caches and preservation. SH1 already rejected wrappers. Rejected |
| B | **Hand-written bindings** | Hundreds of interfaces, written by hand, would drift from their IDL, and their rooting would rest on review. Rejected (SH12) |
| C | **IDL copied by hand,** as most engines keep it | It drifts from the specifications; webref's curated extracts are maintained and already feed WPT. Rejected (§4) |
| D | **Rooting every handle argument** in generated code | Simple, but it pays root-stack traffic in the common all-primitive case. Rejected for WB13's static analysis, which the compiler checks |

## 10. Open questions

| # | Question | Where it is settled |
|---|---|---|
| 1 | How the generated code's size and build time compare with T1–T4 and B2 (WB22) | Step 6 |
| 2 | Which interfaces the first script milestone implements (WB3) | That milestone's plan |

## Sources

All checked 2026-10-19.
- [W1] WHATWG, *Web IDL Standard*, last updated 2026-10-10 — https://webidl.spec.whatwg.org/
- [H1] WHATWG, *HTML Living Standard*, last updated 2026-10-19: §7.2.1–§7.2.3 (`Window`, `WindowProxy`, `Location` security), §4.13 (`[CEReactions]`), §8.1 (callbacks) — https://html.spec.whatwg.org/multipage/
- [B1] Chromium: `third_party/blink/renderer/bindings/README.md` — https://github.com/chromium/chromium
- [F1] Firefox source docs, *Web IDL bindings* — https://firefox-source-docs.mozilla.org/dom/webIdlBindings/index.html
- [K1] WebKit: `Source/WebCore/bindings/scripts/CodeGeneratorJS.pm` — https://github.com/WebKit/WebKit
- [S1] Servo: `components/script_bindings/` (`codegen/codegen.py`, `codegen/Bindings.conf`, `reflector.rs`, `root.rs`) — https://github.com/servo/servo
- [L1] Ladybird: `Libraries/LibWeb/Bindings/PlatformObject.h`; commit `c53fa722125d` (2026-10-05) — https://github.com/LadybirdBrowser/ladybird
- [R1] W3C, webref, `README.md` — https://github.com/w3c/webref
- [T1] web-platform-tests: `interfaces/README.md`; `resources/idlharness.js` [2026-09-18]; `webidl/` — https://github.com/web-platform-tests/wpt
- This repository: [S1 results](../research/s1-results.md) (the dispatch case); [S3 results](../research/s3-results.md) §9 (rooting arrays, the benchmark).
