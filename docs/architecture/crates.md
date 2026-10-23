# The Crate Architecture

| | |
|---|---|
| **Status** | **Accepted** (2026-10-23), with [ADR-0022](../adr/0022-code-architecture-and-engineering-standards.md): "The 7-layer architecture, facility-split platform layer, separate spikes workspace, and check-dependencies script are approved." Written at Milestone 1's first checkpoint, at the owner's request: "we should enforce clear separation of concerns, well-defined module boundaries, high cohesion, low coupling, and explicit interfaces between components." It supersedes the [overview](overview.md)'s crate table, which called itself "a proposal for the implementation milestones; final names and splits are set when code is authorized", and which now points here. Implementation begins at M1-CP2 |
| **Question** | How is Axiom's code divided into crates? Which crate may depend on which, which processes may link each one, and how are those rules checked, so that a change stays inside the crates it concerns? |
| **Builds on** | The [overview](overview.md) §2; the [process model](../security/process-model.md) §3–§4 and IR1–IR10; the [dependency policy](../technology/dependency-policy.md) (DP2, DP9, DP21, DP22); [ADR-0002](../adr/0002-implementation-language.md)'s rules 5 and 7, and the [language evaluation](../technology/language-evaluation.md) §15.3; EM3; NW2; TX3; PL1 and PL11; IV1 and IV2; IP2, IP8 and IP18; EL24; MS4; [Milestone 1](../milestone-1.md)'s MA1 and MA7; the [engineering standards](../technology/engineering-standards.md); [observability](observability.md) |
| **Feeds** | ADR-0022; [Milestone 1's plan](../milestone-1-plan.md), whose second checkpoint builds the crate map and its checker; every crate's charter (ES15) |
| **Last updated** | 2026-10-23: open question 1 settled at M1-CP2 (`url-idna`); corrected with the crate map: groups (§4.2, §4.4), panic classes (§4.1, §4.3), and `test-support` a leaf (§4.6, CR7). Earlier: accepted; the owner's conditional approval on open question 1 |

Rules are numbered CR1–CR16. Everything here is **planned** until the checkpoint that builds it. The rules are inference from the binding documents and the precedent in §2, unless marked otherwise. Facts observed on this Mac name their command.

## Summary

- **Seven layers** (CR1): foundation, platform, runtime, subsystems, process assemblies, embedders, executables. A crate depends only on crates of lower layers, and on the crates of its own layer that its entry names.
- **Every crate has an entry in a crate map** (CR2), `tools/crate-map.toml` (planned). The entry gives the crate's layer, processes, group, allowed internal dependencies, `unsafe` status and panic class. A dependency outside the allowlist fails the commit check, so a new edge between crates is always a reviewed change.
- **The platform layer is split by facility** (CR6): Mach, events, processes, Metal, CoreAnimation, CoreText, network services and AppKit, each a crate with `unsafe` only in its `src/sys/`. The crate graph then shows where Apple's frameworks run (DP21), and that renderers can't reach IOKit (RV1-09).
- **Shared vocabulary lives at the foundation** (CR5): strings and atoms (PL11), geometry, the URL parser (NW2), the encodings (TX3) and the trace recorder. In the overview's proposal, URLs sat in `net` and encodings in `text`. That would have linked the TLS stack into renderers and the browser, and CoreText into the HTML parser and the network process.
- **Each process's composition is a library** (CR7): `browser`, `renderer` and `network`. `document` runs one document's lifecycle (IV1, IV2). Executables are a few lines of `main`, and single-process mode composes the same libraries (MS4).
- **Placement is checked per executable** (CR4):
  - the browser's closure contains no parser or decoder of web content (AC4);
  - a renderer's contains no Metal, CoreAnimation, network service or TLS stack (RV1-09, DP21);
  - the network process's contains no renderer or graphics crate.
- **Test-only capabilities are crates or executables, never Cargo features** (CR12), because Cargo unifies features across one build of several packages.
- **Names don't collide** (CR14). `core` becomes `browser`, since a crate named `core` hides Rust's own `core` from every crate that depends on it (observed, §2).
- **Common changes stay local** (§8): adding a CSS property, an HTML element, an IPC message, an image format, a platform facility, a process or a trace category, and moving work between processes, each touch only the crates they name.

## 1. Binding inputs

| Input | Consequence here |
|---|---|
| **The overview §2's dependency rules:**<br>• engine crates never depend on `shell` (EM3);<br>• renderer crates reach browser crates only through the protocol crates;<br>• `net` and `storage` never depend on renderer crates;<br>• only `platform`, `jit` and the shell's `platform` module contain `unsafe` (AC9) | CR1, CR4, CR6 |
| **DP2:** "Each reused component sits behind a module boundary whose types are Axiom's, so that replacing it stays inside one crate." **DP9:** groups, inherited transitively. **DP21:** each platform facility only in the processes its table names. **DP22:** a planned `tools/check-dependencies.py` | CR4, CR11, CR16 |
| **AC4:** the browser never decodes images or parses web fonts. **SC2:** untrusted input is parsed only where the Rule of 2 allows. **RV1-09:** renderers have no IOKit | CR4 |
| **NW2:** "One URL parser, Axiom's own, in every process." **TX3:** the encodings come from `encoding_rs` in the sandboxed helpers that decode web content; "the browser decodes only UTF-8, with the standard library" | CR5, CR4 |
| **PL11:** one string representation for JavaScript and the DOM; atoms "that every thread can compare". **PL1:** each stage has one output structure and one owner | CR5 |
| **IV1, IV2:** an explicit lifecycle per document, checked by the compiler through tokens; two entry points | CR7 |
| **MS4, IP2, EL24:** every interface from M1; single-process mode strictly for automated tests, fuzzing and Miri, composing the production threads | CR7, CR12 |
| **IP8:** generated decoders in safe Rust; **ADR-0002 rule 7:** macro-generated `unsafe` is part of the boundary. **IP18, IR8:** one build only, checked by an identifier in the hello | CR10, CR13 |
| **MA1:** Milestone 1's crates. **MA7:** "the `js` and `bindings` crates aren't dependencies of any M1 executable" (`cargo tree`) | §4, CR4 |
| **TS11:** state crosses threads "through a short list of primitives, each with one implementation in one crate" | §4.3, ES12 |
| **T1–T4:** the build's thresholds (ADR-0002, PF18) | CR1, CR6; measured at M1-CP4 |

## 2. Precedent and observations

- **Cargo unifies features across the packages of one build** (sourced; the Cargo Book's resolver chapter, read 2026-10-22): "When building multiple packages in a workspace (such as with `--workspace` or multiple `-p` flags), the features of the dependencies of all of those packages are unified. If you have a circumstance where you want to avoid that unification for different workspace members, you will need to build them via separate `cargo` invocations." And: "Features enabled on dev-dependencies will not be unified when those same dependencies are used as a normal dependency, unless those dev-dependencies are currently being built." [K1]
- **Chromium checks its layering mechanically** (sourced; read 2026-10-22). `DEPS` files hold `include_rules`: "You can add (using `+`) or subtract (using `-`) from the previously specified rules"; "`check_deps` is run by our bots"; and "`DEPS` files can be used to require a review when someone adds an `include_rules` entry on a directory" [C1]. **Axiom's crate map (CR2) plays the same role for crates** (inference).
- **Miri can't run most FFI** (sourced; Miri's README, read 2026-10-22): "the program has no access to most platform-specific APIs or FFI"; its experimental `-Zmiri-native-lib` passes "only integer and pointers arguments and return values" [M1]. So platform crates keep their foreign calls thin and their logic testable without them (CR6).
- **A crate named `core`** (observed, 2026-10-22, in a scratch workspace with cargo and rustc 1.94.1):
  - `cargo new core` creates it with a warning: "package name `core` may be confused with the package with that name in Rust's standard library. It is recommended to use a different name to avoid problems."
  - In a crate that depends on it, built-in derives and `format!` still compile.
  - But `core::mem::size_of` fails with E0433, "could not find `mem` in `core`". So does a `macro_rules!` from a third crate that expands to `::core::mem::size_of`, the form third-party macros use.
- **ICU4X's normalizer depends on `smallvec`** (sourced; crates.io's API, read 2026-10-22): `icu_normalizer` 2.3.0 lists `smallvec` as a normal, non-optional dependency [I1]. In the spikes' lock, `smallvec` 1.16.3 is pulled in by HarfRust, `icu_segmenter` and `vello_cpu`'s tree, and not by `icu_properties` (observed, `cargo tree -i smallvec`). Open question 1 follows.

## 3. Layers and the crate map

| # | Rule |
|---|---|
| CR1 | **Seven layers, ordered.** A crate depends only on crates of lower layers, and on crates of its own layer that its entry in the crate map names.<br>0. **Foundation:** vocabulary every process shares, pure Rust, no platform access (§4.1).<br>1. **Platform:** Apple's frameworks and the kernel's interfaces, behind safe APIs; the only layer with `unsafe` (§4.2).<br>2. **Runtime:** IPC, the generated protocol, event loops, the heap (§4.3).<br>3. **Subsystems:** the pipeline, networking, compositing, the browser's records, diagnostics (§4.4).<br>4. **Process assemblies:** one library per process type, which wires its threads and subsystems (§4.5).<br>5. **Embedders:** the shell and the headless embedder (§4.5).<br>6. **Executables:** thin `main`s (§4.5).<br>Build and test crates sit outside the layers. They may depend on any layer, and no shipping crate may depend on them (CR12) |
| CR2 | **The crate map.** Every workspace package has an entry in `tools/crate-map.toml`, giving:<br>• its layer;<br>• the processes that may link it: browser, renderer, network, GPU from M2, utility from M7;<br>• its group (DP9): `trusted`, `sandboxed`, `build` or `test`;<br>• the Axiom crates it may depend on, as an allowlist;<br>• whether it may contain `unsafe`, matching `tools/unsafe-policy.txt`;<br>• its panic class (ES10): `panic-free`, `checked-arithmetic` or ordinary;<br>• the milestone it arrives in.<br>A dependency on an Axiom crate outside the allowlist fails the commit checks (CR16). Adding one is an edit to the map, made and reviewed with the change that needs it. If the new edge crosses an architectural interface (CR8), it goes to the owner as an interface note |
| CR3 | **No cycles, development dependencies included.** Cargo rejects cycles among normal dependencies. A development dependency on a crate that depends back on the first one builds two copies of it in tests, and is refused by the checker as well |

## 4. The crates

Each table lists a crate's purpose, the processes that may link it, its group, its allowed Axiom dependencies, its third-party dependencies (the [register](../technology/dependency-register.md)'s, adopted at the milestone that needs them), and the rules it implements. "All" means the browser, renderer and network processes, and the GPU process from M2.

### 4.1 Foundation (layer 0)

| Crate | Purpose | Processes | Group | May depend on | Third-party | Rules |
|---|---|---|---|---|---|---|
| `trace` | Recording of spans, events, metrics and logs into per-thread single-writer buffers; the record format; the panic hook's record | All | `trusted` | — | — | [Observability](observability.md) OB5–OB9 |
| `strings` | 8- and 16-bit strings with code-unit-generic APIs and the conversions at UTF-8 boundaries; static atoms (generated) and the process-wide dynamic atom table; the engine's hash maps, beside the atoms they mostly key (ES13) | All | `trusted` | `trace` | — | PL11 |
| `geometry` | CSS and device pixels; the layout unit, 1/64 px in an `i32` with saturating arithmetic; points, sizes, rectangles, transforms | All | `trusted` | — | — | LY1 |
| `url` | The WHATWG URL parser; origins and sites; the Public Suffix List compiled in (generated). Canonical ASCII hosts in the browser for every URL (NW2, as amended at M1-CP2); full IDNA through a trait it declares, which `url-idna` implements in the helpers (CR9) | All | `trusted` | `trace` | — | NW2, DP1 (f) |
| `url-idna` | Full UTS #46 processing for `url` in renderers and the network process, and the network process's ASCII form of a host typed in the address bar (NW2, as amended at M1-CP2) | Renderer, network | **`sandboxed`** (ICU4X's normalizer depends on `smallvec`, `ub-risk-3`) | `url`, `trace` | ICU4X's normalizer | NW2, DP8 |
| `encoding` | TX3's interface: sniffing support and the Encoding Standard's codecs from `encoding_rs`, with UTF-8 through the standard library | Renderer, network | **`sandboxed`** (`encoding_rs` is `ub-risk-3`) | `strings`, `trace` | `encoding_rs` | TX3, HD3, DP8 |

**Panic classes** (corrected at M1-CP2, 2026-10-23, when `tools/crate-map.toml` and its checker were built): `trace` and `url` are `panic-free`, because each validates data from less trusted processes inside the browser, helpers' records (OB24) and URLs from other processes (NW2), where a panic would abort the browser; `encoding` is `checked-arithmetic`; `strings` and `geometry` are ordinary.

### 4.2 Platform (layer 1)

`unsafe` lives only in each crate's `src/sys/`: raw declarations from the SDK's headers, and calls, each under a numbered invariant listed in `src/sys/mod.rs`, as the spikes' platform modules did. The rest of each crate is safe Rust over them (CR6).

| Crate | Purpose | Processes | May depend on | Third-party | Rules |
|---|---|---|---|---|---|
| `platform-mach` | Ports, messages and `mach_msg_destroy`; memory entries and mappings; no-senders and dead-name notifications; audit tokens; guarded receive rights; the port space's counts | All | `trace` | — (declarations from the SDK) | IP1, IP5, IP6, IP9, IP14, IP19 |
| `platform-event` | `kevent64`; timers with leeway; run-loop sources for the AppKit and compositor threads; wait-on-address; QoS and overrides; the host clock; the memory-pressure source | All | `platform-mach`, `trace` | — | EL1–EL3, EL23, IP20, PF30 |
| `platform-process` | `posix_spawn` with a minimal environment; compiling profiles (the browser) and applying them (helpers) through libsandbox; exit watching; `proc_pid_rusage` at `RUSAGE_INFO_V6`; `getentropy` for seeds | All | `platform-mach`, `trace` | — | ADR-0004 D2, IR9, PF2 |
| `platform-metal` | Metal devices, pipelines, command buffers and timestamps; IOSurfaces created, drawn and colour-tagged | Browser at M1; GPU from M2 | `platform-mach`, `geometry`, `trace` | `objc2`, `objc2-foundation`, `objc2-metal`, `objc2-io-surface`, `block2` | GR1–GR6, GR12 |
| `platform-layers` | CoreAnimation layer trees in explicit transactions; IOSurfaces looked up from ports as layer contents; display links | Browser | `platform-mach`, `geometry`, `trace` | `objc2`, `objc2-foundation`, `objc2-quartz-core`, `objc2-io-surface` | GR2, GR7–GR9, GR14, EL19 |
| `platform-text` | CoreText's font database, descriptors and fallback; rasterization of system fonts into CoreGraphics bitmaps; the file-backed check of S5 | Renderer | `geometry`, `trace` | `objc2-core-text`, `objc2-core-graphics`, `objc2-core-foundation` | TX6, TX12, TX22 |
| `platform-net` | Security's trust evaluation; DNS-SD for addresses and HTTPS records; SystemConfiguration | Network | `trace` | `objc2-security`, `objc2-core-foundation` | NW6, NW7 |
| `platform-appkit` | The application, windows, views, events and menus; `NSTextInputClient` and `NSAccessibility` for the shell (EM11, AX8) | Browser, used by `shell` only | `platform-layers`, `geometry`, `trace` | `objc2`, `objc2-foundation`, `objc2-app-kit`, `block2` | EM1, EM11, IN9, AX8 |

**Groups** (corrected at M1-CP2, 2026-10-23, when `tools/crate-map.toml` and its checker were built): `platform-text` and `platform-net` are `sandboxed`, because CoreText and Security's trust evaluation parse complex formats from untrusted sources and run only in sandboxed helpers (DP10, TA5); the other platform crates are `trusted`.

At M2, the browser stops linking `platform-metal`. B1a's browser "holds no Metal device and no tiles, only the layer tree", which the graph then shows. Splitting Metal from CoreAnimation is what makes that checkable.

### 4.3 Runtime (layer 2)

| Crate | Purpose | Processes | Group | May depend on | Rules |
|---|---|---|---|---|---|
| `ipc` | Channels over Mach, and in memory modelling Mach's limits (IP2); endpoints as capabilities; `Untrusted<T>` and `Violation`; owners for received handles; bounded backlogs; single-producer, single-consumer rings over shared memory; the runtime the generated code calls | All | `trusted` | `platform-mach`, `trace` | IP2–IP6, IP9–IP11, IP14, IP16; TS11's channels |
| `protocol` | **Generated** from the IDL by `codegen` (CR10): every interface's types, typed IDs, canonical encoders, total decoders, dispatch traits, fuzzing entry points and generators of valid messages. One crate at M1, split by interface family if T1 calls for it (open question 2) | All | `trusted` | `ipc`, `url`, `geometry`, `trace` | IP7, IP8, IP12, IP18, IP21, IP22 |
| `event-loop` | Thread loops; the HTML event loop's task queues and sources; each watched thread's task-age store (PF19); the wait-source-and-clock interface with its production driver (EL24); the helper pools; the interrupt flag | All | `trusted` | `platform-event`, `platform-mach`, `ipc`, `trace` | EL1–EL24, PF19; TS11's interrupt flag |
| `gc` | Pattern B's heap: typed arenas, handles, scopes, `SliceStack`, the collector; read-only lending of arenas | Renderer | `trusted` | `trace` | SH1–SH14, C1–C5; TS11's frozen lending |

**Panic classes** (corrected at M1-CP2, 2026-10-23, when `tools/crate-map.toml` and its checker were built): `ipc` and `protocol` are `panic-free`, since their decoders guard the browser (IP8, IR1); `event-loop` and `gc` are ordinary.

### 4.4 Subsystems (layer 3)

**Documents, in renderers:**

| Crate | Purpose | May depend on | Third-party | Panic class | Rules |
|---|---|---|---|---|---|
| `html` | The tokenizer, the tree builder, sniffing and fragments. It declares the tree sink it builds into (CR9), so it parses without the heap: html5lib's runner and the fuzz targets use their own sinks | `strings`, `encoding`, `trace` | — | `checked-arithmetic` | HD1–HD8, HD33 |
| `xml` | The XML 1.0 document parser, non-validating, with namespaces; its own sink | `strings`, `encoding`, `trace` | — | `checked-arithmetic` | HD9 |
| `dom` | The node arena of columns, the mutation path and its fixed hooks, events for routed input, ranges and selection, forms' model. It implements the parsers' sinks | `gc`, `strings`, `url`, `html`, `xml`, `trace` | — | ordinary | HD10–HD33 |
| `css` | CSS syntax and parsers for stylesheets, selectors, values and media queries, generic over code units; the property table's generated parsers | `strings`, `url`, `geometry`, `trace` | — | `checked-arithmetic` | ST1–ST6 |
| `style` | Matching, the cascade, computed values; the rule tree and the caches; invalidation sets; the style phase's token | `css`, `dom`, `strings`, `geometry`, `trace` | — | ordinary | ST7–ST36, IV1 |
| `text` | Font matching and fallback; shaping; segmentation and line breaking; bidi; the Rust path for test and web fonts; glyph atlases | `strings`, `geometry`, `platform-text`, `trace` | HarfRust, `read-fonts`, `skrifa`, `vello_cpu`, `icu_segmenter`, `icu_properties`, `unicode-bidi` | ordinary | TX4–TX24 |
| `image` | Decoders behind IM9's interface, decoding incrementally and at display size; colour management | `geometry`, `trace` | `png`, `gif`, `image-webp`, JPEG's decoder, `moxcms` | `checked-arithmetic` | IM1–IM10 |
| `layout` | Boxes, formatting contexts and immutable fragments; hit testing; the layout phase's token | `style`, `dom`, `css`, `text`, `strings`, `geometry`, `trace` | — | ordinary | LY1–LY32, PL5, IV1 |
| `display-list` | The format renderers produce and the compositor consumes: property trees and items, their encoding into shared-memory streams, and PL9's validator | `geometry`, `ipc`, `trace` | — | **`panic-free`** (validated in the browser at M1) | PL6–PL9 |
| `paint` | Painting fragments into display items, in CSS 2.1's order; property trees; items reused by identity | `layout`, `style`, `display-list`, `text`, `geometry`, `trace` | `vello_cpu`, through `text` | ordinary | LY19–LY24, PL6, PL7 |
| `document` | One document: its heap, the parser's budgeted tasks (HD5), the rendering update and the synchronous queries (IV1, IV2), input routed to the DOM, display-list updates for its frame, and MA1's text dumps | `dom`, `html`, `xml`, `css`, `style`, `text`, `image`, `layout`, `paint`, `display-list`, `gc`, `url`, `strings`, `event-loop`, `protocol`, `geometry`, `trace` | — | ordinary | IV1, IV2, HD5, PL1, MA1's dumps |

Each of these links only in renderers, apart from `display-list`. **Groups** (corrected at M1-CP2, 2026-10-23, when `tools/crate-map.toml` and its checker were built): `html` and `xml` depend on `encoding`, which is `sandboxed`, so they, and every crate that depends on them (`dom`, `style`, `layout`, `paint`, `document`), are `sandboxed` too, under DP8's rule that a `sandboxed` crate's dependents inherit its group; `text` is `sandboxed` through `platform-text`. `css`, `image` and `display-list` are `trusted`. The first version of this section had them all `trusted`. **Panic classes:** as the table gives them; `display-list` is `panic-free`.

**Networking, in the network process:**

| Crate | Purpose | May depend on | Third-party | Panic class | Rules |
|---|---|---|---|---|---|
| `http` | HTTP/1.1 framing as a state machine without I/O; HTTP/2 at M2 | `url`, `trace` | — | `checked-arithmetic` | NW3, NW4 |
| `net` | Connections and their partitions; TLS through rustls with the system's verifier in the worker pool; DNS-SD; the in-memory cache; Fetch's network-side policy (mixed content, HSTS with its preload list compiled in, Referrer-Policy); content decoding within NW13's limits; the network log as a trace category | `http`, `url`, `platform-net`, `event-loop`, `ipc`, `protocol`, `trace` | rustls with `aws-lc-rs`, `miniz_oxide`, `brotli-decompressor`, `ruzstd` | ordinary | NW1–NW18 |

**Compositing:**

| Crate | Purpose | Processes | May depend on | Panic class | Rules |
|---|---|---|---|---|---|
| `compositor` | Validated display lists, property trees, tiles and damage, Metal pipelines, layer commits | Browser at M1 (a thread); GPU from M2 | `display-list`, `platform-metal`, `protocol`, `ipc`, `event-loop`, `geometry`, `trace` | `panic-free` | PL8, PL9, GR1–GR6, GR10–GR14 |
| `presenter` | The browser's half: each window's CoreAnimation layer tree from layer commits, scrolling and animation on layers, hit-test regions, display links | Browser | `platform-layers`, `protocol`, `ipc`, `event-loop`, `geometry`, `trace` | `panic-free` (commits come from the GPU process from M2) | PL15, PL16, GR7–GR9, GR14, IN1 |

**The browser's records:**

| Crate | Purpose | May depend on | Panic class | Rules |
|---|---|---|---|---|
| `frame-tree` | The authoritative frame tree, navigation records and the replicas' protocol | `url`, `protocol`, `trace` | `panic-free` | FT1–FT13 |
| `embedder-api` | The message API between engine and embedder: commands, events, requests for decisions with their defaults | `protocol`, `url`, `geometry`, `trace` | ordinary | EM1–EM12 |
| `diagnostics` | The collector: reads and validates helpers' trace records, merges streams, keeps the flight recorder, builds reports, retains and exports, samples health | `trace`, `ipc`, `protocol`, `platform-process`, `event-loop`, `url` | `panic-free` | [Observability](observability.md) OB10–OB24 |

These link only in the browser, in the `trusted` group.

### 4.5 Process assemblies, embedders and executables (layers 4–6)

| Crate | Layer | Purpose | Processes | May depend on | Panic class |
|---|---|---|---|---|---|
| `browser` | 4 | The browser process's engine side, formerly `core` (CR14): the core thread's authority (navigation, session and visited history, policy, site computation), the process manager, the helpers' watchdogs, the compositor's and presenter's threads at M1, the collector, the embedder API's engine end | Browser | `frame-tree`, `embedder-api`, `diagnostics`, `compositor` (until M2), `presenter`, `protocol`, `ipc`, `event-loop`, `platform-process`, `platform-mach`, `url`, `geometry`, `trace` | `panic-free` for message handling (RV1-16) |
| `renderer` | 4 | A renderer: an agent's thread and event loop for each agent (FT10), the control thread, the helper pool, the endpoints; hosts `document`s | Renderer | `document`, `event-loop`, `ipc`, `protocol`, `gc`, `trace` | ordinary |
| `network` | 4 | The network process: the I/O thread, the worker pool, the control thread and, from M2, the storage thread; hosts `net` | Network | `net`, `event-loop`, `ipc`, `protocol`, `trace` | ordinary |
| `shell` | 5 | The AppKit shell: windows, tabs, the address field, history, the shell's own text input, the accessibility bridge; drives `browser` through the embedder API | Browser | `browser`, `embedder-api`, `platform-appkit`, `platform-layers`, `geometry`, `url`, `trace` | ordinary |
| `headless` | 5 | EM9's embedder: offscreen presentation, the script-free reference-test path writing PNGs, the text dumps, synthetic input, logged decisions; single-process mode for tests; the test-only WebDriver server at M3 | Test only | `browser`, `embedder-api`, `diagnostics`, `compositor`, `renderer`, `network`, `trace` (third-party: `png`, to write PNGs) | ordinary |
| `axiom` | 6 | The executable: the shell and the browser | Browser | `shell`, `trace` | — |
| `axiom-renderer` | 6 | The executable: applies its profile as its first act, then runs `renderer` | Renderer | `renderer`, `platform-process`, `trace` | — |
| `axiom-network` | 6 | The executable: applies its profile, then runs `network` | Network | `network`, `platform-process`, `trace` | — |
| `axiom-headless` | 6 | The test executable around `headless`, which launches `axiom-renderer` and `axiom-network` as the shell's browser does | Test only | `headless`, `trace` | — |

### 4.6 Build and test crates

None may appear in a shipping executable's closure (CR12).

| Crate | Group | Purpose | Rules |
|---|---|---|---|
| `codegen` | `build` | Every generator, run by `cargo run -p codegen`, with outputs committed and checked fresh (ES19): the IDL into `protocol` (and `embedder-api`, if open question 4 says so); the property table into `css` and `style`; static atoms into `strings`; named character references into `html` and `xml`; the Public Suffix List into `url`; HSTS's preload list into `net`. Its inputs are plain-text formats it parses with the standard library | IP8, PL11, ST3, NW2, NW12 |
| `test-support` | `test` | Golden files and their blessing (ES21); the loopback HTTP server (TS8); fixtures; the counting allocator and the counter of instructions retired for PF5's first tier, in its `src/sys/`, the one `unsafe` module of a test crate. **A leaf:** it depends on no Axiom crate, so every crate's tests can use it without a cycle (CR3). Composing processes in one process, which this entry first listed, is `headless`'s (corrected at M1-CP2, 2026-10-23, when `tools/crate-map.toml` and its checker were built) | TS8, PF5 |
| `test-scheduler` | `test` | EL24's deterministic scheduler with its virtual clock, recording and replay, and schedule exploration (TS11's third check) | EL24, TS8, TS11 |
| `fault-injection` | `test` | MA8's stress configuration: random delays at the IPC layer, network data in random chunks; linked only into the stress builds of the executables (CR13) | TS8, MA8 |
| `perf-harness` | `test` | PF5's checkpoint tier: workloads, `rusage_info_v6` at each process's exit, `perf/baseline.toml` and its ratchet | PF2, PF5, TS13 |

**`fuzz/`** is its own workspace with its own lock, as cargo-fuzz arranges and as S9's fuzz crate was. Its targets depend on the engine's crates by path. The differential references (html5ever, xml5ever, and the `url` crate, whose name ours shares) live only there.

### 4.7 Later milestones' crates

Catalog only. Each arrives with its milestone's plan (principle 7: the complete model fits now).

| Crate | Milestone | Layer and processes | Purpose |
|---|---|---|---|
| `storage` | M2 | 3; network | The storage engine (SW2), the cookie jar, the disk cache; web storage and IndexedDB later (SW1–SW17) |
| `gpu`, `axiom-gpu` | M2 | 4 and 6; GPU | The GPU/compositor process's assembly and executable, hosting `compositor` unchanged; the browser stops linking `compositor` and `platform-metal` |
| `js` | M3 | 3; renderer | The JavaScript engine (JE1–JE32); TS11's `SharedArrayBuffer` atomics |
| `webidl` | M3 | build | The WebIDL compiler (WB1–WB16); it may join `codegen`, decided with spike S13 |
| `bindings-*` | M3 | 3; renderer | Generated glue, split by interface group (RV1-18) |
| `accessibility` | M4 | 3; renderer | Accessibility trees built in renderers (AX1–AX7); the browser's cache in `browser`, the bridge in `shell` |
| `platform-media`, `platform-imageio`, `media`, `utility`, `axiom-utility` | M7 | 1, 3, 4, 6; utility | VideoToolbox and AudioToolbox; ImageIO for AVIF (IM3); the media utility process |
| `jit`, `axiom-renderer-jit` | M5's gate decides | 3 and 6; `renderer-jit` only | J1–J7; the one crate besides the platform layer allowed `unsafe` |

## 5. Placement and groups

| # | Rule |
|---|---|
| CR4 | **Placement is checked per executable.** For each shipping executable, every crate in its closure of normal dependencies must list that executable's process. Every third-party crate in it must be in a group the process admits: the browser admits `trusted` only (DP9). **The exclusions that carry security rules:**<br>• **`axiom`** links none of `html`, `xml`, `css`, `style`, `text`, `image`, `layout`, `paint`, `document`, `gc`, `encoding`, `url-idna`, `http`, `net`, `platform-text`, `platform-net`, `renderer` and `network`. So it parses no web content (AC4, SC2), decodes only UTF-8 (TX3), runs no `ub-risk-3` code (DP8; `url-idna` added at M1-CP2), and holds no TLS stack (DP10).<br>• **`axiom-renderer`** links none of `platform-metal`, `platform-layers`, `platform-net`, `platform-appkit`, `http`, `net`, `compositor`, `presenter`, `frame-tree`, `embedder-api`, `diagnostics`, `browser` and `shell` (RV1-09, DP21, EM3, IR4).<br>• **`axiom-network`** links none of the renderer's crates (`html`, `xml`, `dom`, `css`, `style`, `text`, `image`, `layout`, `paint`, `document`, `gc`), nor `platform-metal`, `platform-layers`, `platform-text`, `platform-appkit`, `compositor`, `presenter`, `browser` and `shell` (the overview §2's rule, DP21).<br>• **No shipping executable** links a `build` or `test` crate, nor `headless`. At M1, none links `js` or any `bindings-*` crate (MA7).<br>The check reads the resolved graph of the whole workspace, in which features are unified across members, so it over-approximates a single executable's build. An optional dependency that a test enables shows up as a violation, which is the hazard CR12 removes |
| CR5 | **Each shared vocabulary has one home, at the foundation:** strings and atoms in `strings`, units and rectangles in `geometry`, URLs, origins and sites in `url`, the encodings in `encoding`, recording in `trace`. No other crate defines a second string type, layout unit, URL parser or origin type, and no subsystem crate holds a type that another process needs. That is what NW2's "one URL parser" and PL11's "one representation" require of the crate graph |
| CR6 | **The platform layer is split by facility, and each facility crate is placed by process** (DP21's table). `unsafe` is allowed only in a platform crate's `src/sys/`, listed in `tools/unsafe-policy.txt`. Each `unsafe` block names an invariant from `src/sys/mod.rs` (evaluation §15.3, rule 3). **Foreign calls stay thin:** a `sys` function makes the call and checks its result, and the logic around it (message layout, ownership, state machines) is safe code in the rest of the crate, where Miri and unit tests reach it (§2). The shell's AppKit code is the `platform-appkit` crate, so "the shell's `platform` module" of AC9 becomes a crate with the same boundary |
| CR7 | **Each process's composition is a library, and its executable is a few lines.** `browser`, `renderer` and `network` (and `gpu` at M2) wire their threads, endpoints and subsystems. Each executable applies its profile through `platform-process` as its first act (ADR-0004 D2) and calls its assembly. **Single-process mode composes the same assemblies in one process,** connected by in-memory channels (IP2), and exists only in test-group code, `headless`, whose tests hold the scenarios across processes, never as a switch in a shipping executable (MS4). (Corrected at M1-CP2: this rule first named `test-support` too, but a leaf that every crate's tests use can't compose the assemblies without a cycle, CR3.) **`document` owns one document's lifecycle:** it runs the phases in order, and holds the tokens that `style` and `layout` produce for the phases after them (IV1) |

## 6. Interfaces between crates

| # | Rule |
|---|---|
| CR8 | **Architectural interfaces are designed before code, and the owner approves them** (the owner's choice, 2026-10-22: "Interfaces only"). An architectural interface is a contract between processes, or between subsystems that several crates or processes depend on. Each gets an interface note from the template in the [engineering standards](../technology/engineering-standards.md) (ES27), written in `docs/architecture/interfaces/` (planned) and approved before its code. **Milestone 1's,** by the checkpoint that defines each:<br>• **M1-CP3:** the IDL's syntax and the generated API; `ipc`'s channels and `Untrusted<T>`; `event-loop`'s tasks and task ages; `trace`'s API and the diagnostics region's format and protocol.<br>• **M1-CP4:** navigation, commit and the loader's stream; the display-list format; the XML sink; what `style` and `layout` read from the DOM; computed styles as `layout` reads them; the lifecycle's tokens; `text`'s results as `layout` reads them; fragments as `paint` reads them; the embedder API's first version.<br>• **M1-CP5:** the HTML tree sink.<br>• **M1-CP11:** the loader's protocol over HTTP.<br>A crate's other public items are its own interface, designed in its charter (ES15) and reviewed asynchronously |
| CR9 | **A lower crate calls a higher one only through a trait it owns.** The parsers declare their sinks, and `dom` implements them. `trace` declares where its buffers come from, and each assembly supplies them. EL24's clock is an interface `event-loop` declares. **The seam for M3 is reserved now:** event dispatch in `dom` calls listeners through a trait that `bindings` will implement (HD18, WB16, WB17), so script never forces `dom` to depend on `js` |
| CR10 | **Each generated interface has one source of truth:** the IDL for `protocol`; the property table for `css` and `style`; the atom list for `strings`. `codegen` generates the code and the output is committed (ES19). Adding an IPC message or a CSS property is an edit to the source, regenerated, after which the compiler names every place that must handle it: dispatch is an exhaustive `match` (ES6) |
| CR11 | **No third-party type appears in an Axiom crate's public interface,** except a foundational dependency's (DP5) inside the platform layer. A reused crate is wrapped by exactly one Axiom crate, so replacing it stays inside that crate (DP2):<br>• `encoding_rs` by `encoding`;<br>• HarfRust, `read-fonts`, `skrifa`, `vello_cpu` and ICU4X's segmenter by `text`;<br>• the image decoders and `moxcms` by `image`;<br>• rustls and the decompressors by `net`;<br>• ICU4X's normalizer by `url-idna`.<br>`paint` reaches `vello_cpu` only through `text`'s interface |

## 7. Test-only code and build flavors

| # | Rule |
|---|---|
| CR12 | **Test-only capabilities are crates or executables, never Cargo features of shipping crates.** Cargo unifies features across the packages of one build (§2). So a feature that a test crate enables can reach the shell's executable whenever both are built together, which is what the commit checks do. **Test-only code comes in three kinds:**<br>• the test-group crates (§4.6);<br>• test-only executables (`axiom-headless`, and the stress builds of CR13);<br>• `#[cfg(test)]` code inside a crate, which no other crate sees.<br>The release bundle builds each executable in its own `cargo` invocation, and the checker refuses a test crate in any shipping closure (CR4) |
| CR13 | **The build identifier carries the build's flavor:** release, test or stress. IP18 already refuses a hello whose identifier differs, so a release browser never talks to a helper built with fault injection, and the reverse. **The stress builds** (MA8, TS8) are separate executables from the same assemblies with `fault-injection` linked in, launched only by `axiom-headless` in its stress mode |

## 8. Change scenarios

How common changes travel through the graph (inference, checked by hand against §4's dependencies).

| Change | What it touches | What it leaves alone |
|---|---|---|
| **A CSS property** | A row in the property table, regenerated into `css` (its parser) and `style` (its storage and accessors, ST3); its computed value in `style`; its effect in `layout` or `paint`, if it has one; its WPT tests in the expectations | `dom`, `html`, `protocol`, the compositor, the browser |
| **An HTML element with behaviour** | Its atom, regenerated into `strings`; the tree builder's rules in `html`, if parsing treats it specially; its insertion and removal steps as one of HD15's hooks in `dom`; its UA style | `protocol`, the browser, the network process |
| **An IPC message** | An entry in an IDL file, regenerated into `protocol`. The compiler lists each dispatch site, and the receiving assembly gets a handler and a validator (IP10). Its fuzzing entry point is generated | Every crate that neither sends nor receives it |
| **An image format** | A decoder wrapper behind IM9's interface in `image`, its register entry (DP6) and its fuzz target in `fuzz/` | Everything outside `image`, except the sniffing table |
| **Replacing a reused dependency** | The one crate that wraps it (CR11) | Every other crate |
| **A platform facility** | A platform crate's `src/sys/` with its invariants, its entry in `tools/unsafe-policy.txt`, its processes in the crate map | The crates above, until one is allowed to use it |
| **Moving work between processes** (M2's GPU process) | A new assembly (`gpu`) and executable; the crate map's processes for `compositor` and `platform-metal`; `browser` stops linking them | `compositor` itself, `display-list` and `protocol`'s interfaces (MS4) |
| **A trace category or metric** | Its declaration and the call sites that record it | The collector and the exports, which handle categories generically ([observability](observability.md) OB6) |
| **A process type** (M7's media utility) | A facility crate, a subsystem crate, an assembly, an executable, its IDL interfaces, its profile in the process model, and crate-map entries | The existing assemblies, apart from `browser`'s process manager |

## 9. Naming, layout and workspaces

| # | Rule |
|---|---|
| CR14 | **Names are short, lower case and hyphenated, without a project prefix.** A name may not collide with one of Rust's built-in crates (`core`, `std`, `alloc`, `proc_macro`, `test`) or with any package in the engine's lock, which the checker verifies (CR16). Executables are named `axiom` and `axiom-*`, as MA10 fixes them. **So the overview's `core` becomes `browser`** (§2's observation), and the differential `url` crate stays in `fuzz/`'s own lock |
| CR15 | **One directory per package, flat, under `crates/`, executables included.** Elsewhere in the repository:<br>• `fuzz/` (its own workspace);<br>• `perf/` (TS13's `baseline.toml`);<br>• `tests/wpt/` (wptrunner's `axiom` product and the expectations, SB8 and SB9);<br>• `spikes/` (unchanged, as MA10 says);<br>• `tools/` (the checkers, with `crate-map.toml` beside `unsafe-policy.txt`).<br>A crate's layer is recorded in the map and its charter, not in its path, so a crate that changes layer doesn't move. **Three workspaces, three locks:**<br>• the engine at the root;<br>• the spikes in `spikes/`, with the current lock moved there;<br>• `fuzz/`.<br>The engine's `Cargo.lock` then holds only the engine's dependencies, so DP7, DP15 and DP16 apply to it alone. The register covers all three (DP7) |

## 10. Enforcement

| # | Rule |
|---|---|
| CR16 | **`tools/check-dependencies.py`** (DP22's planned tool, extended; standard-library Python with `--help`, built at M1-CP2). It reads `cargo metadata --format-version 1 --filter-platform aarch64-apple-darwin` and `tools/crate-map.toml`, and checks:<br>• that every workspace package has an entry (CR2) and that its Axiom dependencies are in its allowlist (CR1, CR2);<br>• no cycles, development dependencies included (CR3);<br>• each shipping executable's closure, crates and groups (CR4, DP9);<br>• that no test or build crate is reachable from a shipping executable (CR12);<br>• names against the built-in crates and the lock (CR14);<br>• that the crates allowed `unsafe` are the platform layer's, `jit` later, and `test-support`'s allocator, matching `tools/unsafe-policy.txt` (CR6);<br>• that each crate's root declares the lints of its panic class (ES10);<br>• DP22's three checks: a register entry for every third-party crate in each lock, group inheritance, and build scripts or procedural macros flagged for DP12.<br>It runs in the commit tier of `tools/check.sh` (ES25). It is a gate on the graph, not a substitute for review: a change to the map is reviewed with the change that needs it (CR2) |

## 11. Checks against the binding rules

| Rule | What it requires | How the graph meets it | Checked by |
|---|---|---|---|
| AC4, SC2 | The browser never decodes images or parses web content | No content crate in `axiom`'s closure | CR4 |
| AC9, evaluation §15.3 | `unsafe` only where allowed | Only the platform layer, later `jit`, and `test-support`'s allocator are allowlisted | `tools/check-unsafe-policy.sh`; CR16 |
| DP2 | Replacing a reused component stays inside one crate | Each reused crate wrapped by one Axiom crate | CR11 |
| DP9 | Groups, inherited transitively | A group per crate; `trusted` closures for the browser | CR4, CR16 |
| DP21 | Facilities only in the processes its table allows | Facility crates placed by process | CR4, CR6 |
| EM3 | The engine never depends on the shell | `shell` in layer 5; only `shell` uses `platform-appkit` | CR1, CR4 |
| RV1-09 | No IOKit in renderers | `platform-metal` and `platform-layers` absent from `axiom-renderer` | CR4, with MA7's `iokit-open` probe |
| NW2 | One URL parser in every process | `url` at the foundation | CR5; open question 1 for IDNA in the browser |
| TX3 | `encoding_rs` only in helpers; the browser decodes UTF-8 with the standard library | `encoding` is `sandboxed`, absent from `axiom` | CR4 |
| PL11 | One string representation; atoms from any thread | `strings` at the foundation | CR5 |
| IV1, IV2 | One lifecycle per document, with tokens | `document` runs it; the phase crates produce the tokens | CR7 |
| MS4, IP2, EL24 | Every interface from M1; single-process mode for tests only | Assemblies as libraries; single-process composition only in test crates | CR7, CR12 |
| IP8, rule 7 | Generated decoders with no `unsafe` | `protocol` generated by `codegen`, whose output forbids `unsafe` | CR10; `tools/check-unsafe-policy.sh` |
| IP18, IR8 | One build only | The identifier carries the flavor | CR13 |
| MA7 | No `js` or `bindings` in M1's executables | CR4's exclusions | CR16 |
| TS11 | Each primitive in one crate | Channels and rings in `ipc`; frozen lending in `gc`; the interrupt flag in `event-loop`; atomics for `SharedArrayBuffer` in `js` at M3 | ES12; review |
| T1–T4 | The build's thresholds | Leaf edits rebuild only their dependents; the platform split keeps foreign bindings out of most crates | Measured at M1-CP4 with every crate present, then at each checkpoint |

## 12. Alternatives considered

| | Alternative | Assessment |
|---|---|---|
| A | **One `platform` crate,** as the overview proposed | Every executable would link every facility: Metal and Security in renderers, CoreText in the browser. DP21's table and RV1-09 couldn't be checked from the graph, and any change to a binding would rebuild every crate above it (T1). Rejected |
| B | **Layering by convention and review alone** | It drifts as the codebase grows, which is what Chromium's `DEPS` checks exist to stop (§2). Rejected |
| C | **Cargo features for test-only code** | Features unify across one build of several packages (§2), so a test capability could reach the shell's executable. Rejected (CR12) |
| D | **A build system with visibility rules,** such as Bazel or GN | Cargo is the toolchain that works here without Xcode, and a script over `cargo metadata` performs the same checks. Rejected |
| E | **An `axiom-` prefix on every package** | It rules out collisions mechanically, at the cost of longer names throughout. The collision rule and its check (CR14) give the same safety. Rejected, and easy to revisit |
| F | **A few large crates,** such as one `engine` | Slower incremental builds (T1), no placement checks, and every boundary a convention. Rejected |
| G | **A crate per module** | Hundreds of crates cost build overhead (T2, T3) and make the code harder to navigate. Rejected: crates are split by dependency set and concern (ES3) |

## 13. Open questions

1. **`smallvec`'s grade and IDNA in the browser.** NW2 has the browser's address bar use full IDNA through ICU4X, and ICU4X's normalizer depends on `smallvec` (§2). If Axiom's audit at M1-CP2 grades `smallvec` `ub-risk-2` or better, `url` links the normalizer everywhere. If it grades it `ub-risk-3`, DP8 bars it from the browser. Full IDNA then moves into a `url-idna` crate in the `sandboxed` group, linked only by helpers, and the address bar asks the network process for the ASCII form of a typed host. That is an amendment to NW2, for the owner, decided with the grade. **The owner's conditional approval** (2026-10-23): "If smallvec is graded ub-risk-3 during CP2, approve moving the IDNA domain lookup into the network process as an amendment to NW2." So M1-CP2's grade settles it, and NW2 is amended then if the grade is `ub-risk-3`. **Settled at M1-CP2** (2026-10-23): the [audit](../research/smallvec-audit.md) grades `smallvec` 1.16.3 `ub-risk-3`, so the condition is met and NW2 is amended: full IDNA lives in `url-idna`, `sandboxed`, linked only by renderers and the network process, and the browser's address bar asks the network process for a typed host's ASCII form.
2. **One `protocol` crate or several:** split by interface family if its rebuild costs too much against T1, measured at M1-CP4.
3. **One sink for both parsers or two:** settled by M1-CP5's interface note on the HTML tree sink.
4. **`embedder-api` generated from the IDL or written by hand:** settled by M1-CP3's interface note on the IDL.

## Sources

All read on 2026-10-22.

- [K1] The Cargo Book, "Dependency Resolution", sections "Features" and "Feature resolver version 2" — https://doc.rust-lang.org/cargo/reference/resolver.html
- [C1] Chromium, `buildtools/checkdeps/README.md` — https://chromium.googlesource.com/chromium/src/+/HEAD/buildtools/checkdeps/README.md
- [M1] Miri, `README.md` — https://github.com/rust-lang/miri/blob/master/README.md
- [I1] crates.io's API, the dependencies of `icu_normalizer` 2.3.0 — https://crates.io/api/v1/crates/icu_normalizer/2.3.0/dependencies
