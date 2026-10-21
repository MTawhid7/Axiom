# Dependency and Supply-Chain Policy

| | |
|---|---|
| **Status** | **Binding** since [ADR-0010](../adr/0010-dependency-and-supply-chain-policy.md)'s acceptance (2026-10-18, Step 5's Gate B). Written at Step 5, CP2; DP10 refined at CP3 (§6). **Amended by the architecture review's Round 1** (the owner's dispositions, 2026-10-21; [the review](../review/README.md) §5): DP1 (g) and DP2 for RV1-08 (CP4) |
| **Question** | Which code does Axiom write itself, and which may it reuse? How is each dependency justified, classed, audited and placed, and how do the `unsafe` and C code inside dependencies count against ADR-0002's rules and the Rule of 2? |
| **Answers** | Step 5 question Q1 ([tracker](../research/step-5-subsystems.md) §3); decision-map item X4, added by the owner at Step 5's Gate A (2026-10-18) |
| **Feeds** | ADR-0010 (CP9); the build-or-reuse decisions of every Step 5 subsystem checkpoint; the [dependency register](dependency-register.md) |
| **Builds on** | [Charter](../charter.md) G1 and principle 4; [ADR-0002](../adr/0002-implementation-language.md) and the [language evaluation](language-evaluation.md) §12, §15.3–§15.4; the [threat model](../security/threat-model.md) TA4, TA5 and §9; the [process model](../security/process-model.md) PM2 and §4; the [decision map](../decision-map.md)'s layers |
| **Last updated** | 2026-10-21: Round 1's accepted findings |

This document sets rules; it adds no dependency. Rules are numbered DP1–DP23. Precedent is sourced (§2); the rules are inference from the binding inputs and the precedent, unless marked otherwise.

## Summary

- **The core is Axiom's own; supporting infrastructure may be reused** (DP1, DP2). The line follows charter G1 and the decision map's layers:
  - the mechanisms G1 lists, and the security policies, are written in the project;
  - the decision map's infrastructure layer (shaping and fonts, the graphics backend, HTTP, TLS and DNS, decoders, the accessibility bridge), along with Unicode data, encodings and compression, may be reused behind interfaces Axiom owns.

  Axiom never implements cryptography itself (DP3), and adds a dependency only to save substantial, specialized work (DP4).
- **Every dependency has a class and a record** (DP5–DP7): required platform facility, replaceable implementation detail, or foundational dependency, with a register entry that covers its whole transitive tree.
- **The `unsafe` and C code inside dependencies count** (DP8–DP12). Each crate gets a soundness grade on Google's scale, from `ub-risk-0` (no `unsafe`) to `ub-risk-4`. Each also gets a group, as Chromium's crates do: `trusted` (may run in any process), `sandboxed` (helpers only), or `build` and `test`.
  - A reused parser of untrusted input must be pure Rust, at `ub-risk-2` or better across its whole tree, and runs in sandboxed helpers. In the browser it needs `ub-risk-1` or better. (Refined at CP3: see DP10.)
  - C, C++ or assembly that parses untrusted input counts as memory-unsafe code under the Rule of 2: it may run only in a sandboxed helper, only when no pure-Rust option meets the need, and only with an ADR.
- **The supply chain is locked and audited** (DP13–DP20):
  - crates.io only, with `Cargo.lock` and `--locked` builds;
  - audits recorded with `cargo-vet`, importing the organizations whose audits Firefox imports;
  - `cargo-deny` for advisories, licenses, duplicate crates and sources;
  - minimal features, and costs measured against T1–T4.
- **Apple's frameworks are required platform facilities,** reached only through `platform` modules and only from the processes that the process model and DP21 allow.
- **Enforcement** (DP22): `cargo-deny` and `cargo-vet` become pre-commit checks once the owner authorizes installing them. A repository tool, designed here, checks that every crate in the lock has a register entry, and that no browser-process crate depends on a `sandboxed` one.

## 1. Binding inputs

| Input | Consequence here |
|---|---|
| **Charter principle 4:** "Implement the core ourselves. Reuse supporting infrastructure only with an explicit justification", stating whether it is a required platform facility, a replaceable implementation detail, or a foundational dependency. **G1** lists the core mechanisms | DP1, DP2, DP5 |
| **Decision map:** the infrastructure layer (I) is "swappable, if we own the interfaces in front of them"; platform pieces are deferrable "only if we own the interfaces in front of them" (§2, item 5) | DP2 |
| **ADR-0002, evaluation §15.3:** `unsafe` only in designated crates; no `unsafe` in parsers, network policy, IPC decoding, or image and font wrappers (rule 5); macro-generated `unsafe` is part of the boundary (rule 7). **§15.4:** every dependency justified and classed, audited with `cargo-deny` and `cargo-vet` against a license allow-list. **§12:** the mitigations for supply-chain sprawl are "to be finalized in the dependency policy" | DP8–DP11, DP15–DP17 |
| **ADR-0002, thresholds T1–T4** ([S2 results](../research/s2-results.md) §6). The targets: an incremental rebuild in ≤ 5 s, a clean build in ≤ 5 min, ≤ 3 GiB of peak aggregate compiler memory, ≤ 1.5 GiB for one `rustc`. The revisit levels, after mitigations: 30 s, 20 min, 5 GiB and 3 GiB | DP19 |
| **Process model PM2, the Rule of 2:** no component combines untrusted input, memory-unsafe code ("`unsafe` Rust and Apple's C, C++ and Objective-C libraries") and high privilege. **Threat model TA5:** Apple's complex-format parsers are not trusted with untrusted input in a privileged process | DP9, DP10, DP21 |
| **Threat model TA4:** the toolchain and dependencies are trusted at the versions pinned in `Cargo.lock`, a residual risk. **§9:** fixes reach a locally built browser only when the owner rebuilds; Step 6 sets the routine | DP12–DP14, DP20 |
| **ADR-0004, consequences:** if the TLS stack is Apple's, it stays out of the browser process | DP21 |

## 2. Precedent

Read on 2026-10-18. The dates are the last commits to each file, from GitHub's API.

| Project | How it governs third-party code |
|---|---|
| **Chromium** | Third-party Rust crates are vendored with its `gnrt` tool. Each crate has a **`group`**, documented in `gnrt_config.toml` (last changed 2026-10-15) [C1]:<br>• **`safe`:** "The library satisfies the rule-of-2 and can be used in any process."<br>• **`sandbox`:** "The library does not satisfy the rule-of-2 and must be used in a sandboxed process such as the renderer or a utility process."<br>• **`test`.**<br>"The group of least privilege is inherited by a crate C from all crates that depend on C transitively." Per-crate settings include `allow_unsafe`, and `ban_features` for "crate-feature-gated code [that] failed or hasn't gone through a security audit".<br>The import guide says a crate with `unsafe` whose security review finds it doesn't satisfy the Rule of 2 moves to the `sandbox` group "to make it clear it can't be used in a privileged process" [C2].<br>Chromium's `cargo-vet` audits define soundness criteria `ub-risk-0` to `ub-risk-4`, plus `crypto-safe` and `does-not-implement-crypto` [C3]. That file was last changed on 2025-03-22, and the current import guide doesn't mention `cargo vet` (observed) |
| **Google** | The auditing standards behind those criteria [G1]:<br>• `ub-risk-0`: crates that "do not contain unsafe Rust code".<br>• `ub-risk-1`: "near-zero risk of introducing undefined behavior", "suitable for the most demanding situations".<br>• `ub-risk-2`: "a trivial risk", "suitable for most applications".<br>• `ub-risk-3`: "a nontrivial, but not necessarily unacceptable risk"; projects "need to make judgement calls about where, when, and by whom these crates may be used".<br>• `ub-risk-4`: "a risk of causing undefined behavior with typical use", a "necessary evil".<br>Grading `ub-risk-1` to `ub-risk-4` requires unsafe-Rust expertise |
| **Mozilla and Firefox** | Mozilla's `cargo-vet` "matches all of a project's third-party dependencies against a set of audits performed by the project authors or entities they trust". It supports relative audits between versions, and exemptions that "can be ratcheted down over time" [M1]. Its built-in criteria [M2]:<br>• **`safe-to-run`:** a crate "can be compiled, run, and tested on a local workstation or in controlled automation without surprising consequences".<br>• **`safe-to-deploy`:** a crate "will not introduce a serious security vulnerability to production software exposed to untrusted input". Auditors "must review enough to fully reason about the behavior of all unsafe blocks and usage of powerful imports". For crates that generate deployed code, such as procedural macros, "reasonable usage of the crate should output code which meets the above criteria".<br>Firefox's configuration (last changed 2026-10-15) imports audits from the Bytecode Alliance, Embark Studios, Google, ISRG and Mozilla. It records 1,429 audits of its own, and 96 exemptions [M3] |
| **Servo** | `cargo-deny` [S1], with a license allow-list of 14 licenses, including MPL-2.0 and Unicode-3.0, and `multiple-versions = "deny"` (`deny.toml`, last changed 2026-10-17) [S2] |
| **Ladybird** | At its fork from SerenityOS (2024-06-03): "Ladybird will have a relaxed NIH policy (instead of "no 3rd party code!")" [L1]. Its `vcpkg.json` (last changed 2026-10-17) lists 47 dependency entries [L2]. Among them are HarfBuzz, ICU, Skia, curl, OpenSSL, libjpeg-turbo, libpng, libwebp, libavif, WOFF2, SQLite and FFmpeg. LibWeb and LibJS are its own [L3] |
| **WebKit** | `Source/ThirdParty` holds ANGLE, dav1d, libwebrtc, PDF.js, Skia and test frameworks, among others (observed with GitHub's API; last changed 2026-10-16) [W1] |
| **The RustSec database** | "a repository of security advisories filed against Rust crates published via crates.io", maintained by the Rust Secure Code Working Group. `cargo-audit` and `cargo-deny` read it [R1] |

**Observations** (inference):
- **Every engine reuses infrastructure.** All of them write their own engine core, and reuse codecs, font shaping, TLS, Unicode data and graphics libraries. Ladybird, which began with "no 3rd party code", relaxed that rule for exactly those layers.
- **Chromium ties a dependency's soundness to the privilege of the process that may load it,** and makes the restriction transitive. That is the Rule of 2 applied to code Chromium didn't write, which is the gap X4 closes for Axiom.
- **Chromium and Firefox share audit work.** Both import audits from other organizations rather than auditing every crate themselves.

## 3. What Axiom writes, and what it may reuse

| # | Rule |
|---|---|
| DP1 | **The core is Axiom's own.** The project writes the mechanisms charter G1 lists, and the decision map's F and P layers that carry them: **(a)** parsing HTML and CSS, and the DOM; **(b)** style, layout and paint; **(c)** rendering orchestration: display lists, property trees, and the compositor's logic and validation; **(d)** incremental updates; **(e)** navigation, the frame tree, and the coordination of loading, including Fetch's algorithm; **(f)** browser security policies: site isolation, origins and sites as security principals, CORS, ORB, CORP and COEP, cookie and storage policy, CSP, mixed content, HSTS decisions, Private Network Access, permissions; **(g)** the JavaScript engine: parser, bytecode, interpreter, object model, builtins, `RegExp` and the collector (ADR-0001 R2, ADR-0003). **As amended in Round 1** (RV1-08): the arithmetic behind two builtins, Temporal's calendars, durations and time zones, and BigInt's arbitrary-precision operations, is supporting infrastructure under DP2, behind interfaces the engine owns; the builtins themselves stay core; **(h)** the IPC layer and its generator, the event loop, the embedder API and the WebIDL generator (ADRs 0005, 0006, 0009; SH12). A core component may sit on reused data or primitives (DP2), but never on a reused implementation of its own mechanism |
| DP2 | **Supporting infrastructure may be reused, behind an interface Axiom owns.** It includes:<br>• the decision map's infrastructure layer: text shaping and font parsing, the graphics backend and rasterization, HTTP, QUIC, TLS and DNS, image and media decoders, the accessibility bridge (I1–I5), and WebAssembly (I6, OQ-3);<br>• Unicode and locale data, and the algorithms the Unicode Consortium specifies over them;<br>• the Encoding Standard's codecs, URL parsing (origins and sites stay core under DP1 (f)), compression, storage engines, and build-time and test tooling;<br>• **added in Round 1** (RV1-08): Temporal's calendar and time-zone arithmetic, and BigInt's arbitrary-precision arithmetic.<br>Each reused component sits behind a module boundary whose types are Axiom's, so that replacing it stays inside one crate. A foundational dependency (DP5) is the recorded exception |
| DP3 | **Never Axiom's own cryptography.** Cryptographic algorithms come from a platform facility or a reviewed library, recorded with Chromium's crypto criteria (`crypto-safe` or `does-not-implement-crypto`, §2). This covers TLS, certificate verification, hashing for security, and later the Web Crypto API |
| DP4 | **A dependency must save substantial, specialized work:** a mature implementation of a large specification (fonts, codecs, TLS, Unicode), or work where mistakes are subtle (cryptography, compression). Small utilities are written in the project, and the standard library comes first. Every dependency costs audit time, build time and supply-chain exposure for as long as Axiom exists |

## 4. Classes

| # | Rule |
|---|---|
| DP5 | **Every dependency has one class, which sets how it is approved and how it is left:**<br>• **Required platform facility:** a macOS framework, library or service that Axiom can't do without, or must use for platform behaviour. Examples: AppKit, Metal, the sandbox, certificate trust evaluation, libSystem. Reached only through `platform` modules (ADR-0002), and only from the processes DP21 allows. Approved by the ADR that needs it.<br>• **Replaceable implementation detail:** a crate behind an Axiom-owned interface (DP2), replaceable within one crate. Approved with its subsystem's design document and ADR.<br>• **Foundational dependency:** a crate whose types or build role spread across many crates, such as objc2 across `platform` and the shell, or whose replacement would be a project in itself. Approved by an ADR of its own, or a named decision in one, with a written revisit condition |

## 5. Records

| # | Rule |
|---|---|
| DP6 | **The [register](dependency-register.md) records every dependency before it is added.** Each entry gives:<br>• the crate or facility, its version and its source;<br>• its license, purpose, and the Axiom crates that use it;<br>• the processes it runs in, and whether it handles untrusted input;<br>• its class, group (DP9) and soundness grade (DP8);<br>• the `unsafe` and non-Rust code inside it, and its transitive crates;<br>• its alternatives, writing it ourselves included;<br>• its maintenance (maintainers, last release, advisory history) and its precedent in other engines;<br>• its build cost against T1–T4, and its binary-size and memory cost where measured;<br>• the features it enables and bans;<br>• the interface in front of it, and the condition that would make Axiom revisit it.<br>Next to each dependency, its manifest carries a short justification, as the workspace manifest already does for the spikes' crates (observed: "External dependencies. Each needs a justification") |
| DP7 | **Transitive crates count.** Every crate in `Cargo.lock` is a dependency under this policy. A direct dependency's entry lists the crates it brings, each of them is audited (DP15), and a dependency is judged by its whole tree |

## 6. `unsafe` and C code in dependencies

| # | Rule |
|---|---|
| DP8 | **Every crate gets a soundness grade on Google's scale** (§2), recorded as a `cargo-vet` audit:<br>• `ub-risk-0`, no `unsafe`;<br>• `ub-risk-1`, near-zero risk;<br>• `ub-risk-2`, trivial risk.<br>**`ub-risk-3` and `ub-risk-4` are not admitted.** Axiom has no deadline that would justify a "necessary evil" (charter principle 7). C, C++ or assembly inside a crate, whether compiled by a build script or linked, can't be graded on a Rust scale, and counts as memory-unsafe code under PM2 |
| DP9 | **Every crate has a group, which says where it may run** (Chromium's groups, §2):<br>• **`trusted`:** may run in any process, the browser included. It needs a grade of `ub-risk-2` or better and no C, C++ or assembly. Where it handles data from a less trusted source in the browser, it needs `ub-risk-1` or better.<br>• **`sandboxed`:** may run only in sandboxed helpers (renderer, network, GPU, utility).<br>• **`build`:** build scripts, procedural macros and generators, never linked into a shipped executable.<br>• **`test`:** tests, fuzzers and benchmarks only.<br>**The least-privileged group is inherited transitively,** as in Chromium: a crate that a `sandboxed` crate depends on is itself `sandboxed`, unless it is separately approved as `trusted` |
| DP10 | **Parsers of untrusted input.** These are the categories of ADR-0002's rule 5 (HTML, CSS, URLs, encodings, network policy, IPC decoding, images, fonts), together with network protocols, compression and media containers. **Axiom's own are safe Rust** (rule 5). **A reused one must be pure Rust, at `ub-risk-2` or better across its whole tree, and runs in sandboxed helpers. In the browser it needs `ub-risk-1` or better** (DP9). **C, C++ or assembly that parses untrusted input** is memory-unsafe code for the Rule of 2, admitted only if all three hold:<br>• it runs in a sandboxed helper;<br>• no pure-Rust alternative meets the requirement, as a source or a measurement shows;<br>• an ADR accepts it.<br>Apple's parsers (CoreText, ImageIO, Apple's TLS) follow the same placement as platform facilities: sandboxed helpers only (SC2, TA5).<br>**Refined at CP3** (2026-10-18). CP2's version required `ub-risk-1` across the whole tree, in every process, and treated anything weaker as memory-unsafe code. Google grades `bytemuck`, `zerovec` and `crc32fast` at `ub-risk-2`, and they sit under every Rust font stack, ICU4X and the `png` crate. So that rule excluded every Rust font parser, while CoreText, which is C, remained admissible through the exception. That was the reverse of the risk ordering it meant to express |
| DP11 | **Macro-generated `unsafe` is Axiom's boundary** (ADR-0002 rule 7). An external macro that expands `unsafe` into Axiom's code, such as objc2's class-definition macros or a `bytemuck` derive, may be used only in a crate listed in `tools/unsafe-policy.txt`, and only in its listed modules |
| DP12 | **Build-time code runs with the owner's authority.** Build scripts and procedural macros execute on this Mac at every build. Any crate that has them is audited `safe-to-run` at least, and its generated code `safe-to-deploy` (cargo-vet's rule, §2). Crates without build scripts are preferred. A build script that compiles C brings C into the product, and DP8 and DP10 apply. Build scripts may not reach the network: builds run from the lock and the local registry cache |

## 7. The supply chain

| # | Rule |
|---|---|
| DP13 | **Sources:** crates.io only, by default. A git dependency needs a reason, is pinned to a commit, and is replaced by a release once one exists. A patched crate records its patch in the register, as Firefox's configuration records local fixes (§2) |
| DP14 | **Locking and updates:** `Cargo.lock` is committed, as it already is, and builds use `--locked`. No wildcard versions. Every update is a deliberate commit that reviews the lock's diff and audits each new or changed crate (cargo-vet's relative audits) |
| DP15 | **Audits with `cargo-vet`.** Every third-party crate in the lock is covered by an audit meeting what its group requires:<br>• `safe-to-deploy`, its grade (DP8) and a crypto criterion (DP3), for shipped code;<br>• `safe-to-run`, for `build` and `test` code.<br>Audits are Axiom's own, or imported from the organizations Firefox imports (§2): Mozilla, Google, the Bytecode Alliance, ISRG and Embark Studios. Each import is named in the configuration with its reason. Exemptions exist only for crates present when the policy starts, and are ratcheted down; a new crate never gets one. Each audit names who performed it and what was read; the owner approves the audits with the dependency |
| DP16 | **`cargo-deny`, all four checks:**<br>• **advisories,** from RustSec, including its unmaintained and unsound notices;<br>• **licenses** (DP17);<br>• **bans:** no two versions of one crate in a shipped executable, as Servo denies (§2), and banned crates, such as a second implementation of something a decided dependency already covers;<br>• **sources** (DP13) |
| DP17 | **Licenses.** Axiom's own license is "MIT OR Apache-2.0", chosen by the owner at Gate B (2026-10-18; open question 1, OQ-21), with `LICENSE-MIT` and `LICENSE-APACHE` at the repository root. Every license on the allow-list is compatible with it; MPL-2.0's copyleft stays with its own files:<br>• permissive licenses: MIT, Apache-2.0 (with or without the LLVM exception), BSD-2-Clause, BSD-3-Clause, ISC, Zlib, BSL-1.0, CC0-1.0, 0BSD;<br>• Unicode-3.0, for Unicode's data;<br>• MPL-2.0, whose copyleft is per file, and which Servo allows (§2).<br>GPL, LGPL, AGPL, SSPL, and any non-OSI or unidentified license are not admitted. A font license such as OFL-1.1 would be needed only if a font were ever bundled, and none is planned |
| DP18 | **Features are minimal:** `default-features = false`, and only the features Axiom uses, recorded in the register. Banned features are recorded too, after Chromium's `ban_features` (§2). S2 measured why: `s2-window` built clean in 6.0 s with only the framework headers it used, and in 29 s with objc2's default features, which compile every header, for an identical binary ([S2 results](../research/s2-results.md), S2-Q6) |
| DP19 | **Costs are measured.** Each dependency's build cost against T1–T4 is measured when it is added, in a spike or a milestone. A dependency that would push the workspace past a T1–T4 target needs that cost accepted in its approval; passing a revisit level triggers ADR-0002's revisit condition 3. Its binary size and memory are recorded where they can be measured (G4; B1–B7) |
| DP20 | **Advisories reach Axiom only through rebuilds** (threat model §9). Each advisory against a crate in the lock is assessed. A vulnerability in a crate that handles untrusted input means rebuilding before the browser is next used for anything it could affect. Step 6 sets the routine and its cadence |

## 8. Apple's frameworks

| # | Rule |
|---|---|
| DP21 | **Platform facilities run only where this table allows,** and only through `platform` modules (ADR-0002). Apple's parsers of untrusted bytes run only in sandboxed helpers (TA5, SC2). Private interfaces need an ADR with a recorded fallback, as ADR-0004 records for libsandbox. Deprecated interfaces are avoided, as `CVDisplayLink` was ([macOS runtime note](../research/macos-runtime-facilities.md)). The minimum OS is the current macOS (ADR-0001) |

| Facility | Processes | Purpose | Decided by |
|---|---|---|---|
| libSystem: Mach messages, kqueue, `posix_spawn`, wait-on-address | All | IPC, waiting, launch, `Atomics.wait` | ADRs 0005, 0006, 0008 |
| libsandbox, through private interfaces | Browser compiles profiles; helpers apply them | Sandboxing | ADR-0004 (invariant X11; a residual risk, threat model §9) |
| AppKit, Foundation, QuartzCore, Metal, IOSurface | Browser; a GPU process later, for Metal | Windows, presentation, compositing | ADRs 0007, 0009 |
| AppKit's `NSAccessibility` protocols | Browser (main thread) | The accessibility bridge | ADR-0007 (IN9); CP8 |
| CoreText and CoreGraphics | Renderer (and a GPU process, if glyph rasterization moves there) | Text, if CP3 chooses it. **Never the browser, for web fonts** (AC4) | S4-Q5; CP3 |
| ImageIO | Renderer only | Image decoding, if CP5 chooses it | CP5 |
| Security: certificate trust evaluation | Network | TLS certificate verification | CP6 |
| Security: Keychain | Browser | Secrets, if any are stored | Process model §4.1 |
| Network and CFNetwork | Network only | Transport and TLS, if CP6 chooses Apple's stack | ADR-0004; CP6 |
| VideoToolbox, AVFoundation, CoreMedia, AudioToolbox | Utility (media) | Media decoding and audio output | Process model §3; CP5 |

## 9. Enforcement, and the spikes

| # | Rule |
|---|---|
| DP22 | **Checks.** `cargo deny check` and `cargo vet` join the pre-commit checks once the owner authorizes installing them, proposed with the first engine crate. A repository tool, `tools/check-dependencies.py`, is designed here and built when authorized. It will check:<br>• that every third-party crate in `Cargo.lock` has a register entry;<br>• from a file mapping each workspace crate to its processes and each dependency to its group, that no crate in the browser process reaches a `sandboxed` dependency (DP9);<br>• that every crate with a build script or procedural macro is flagged for DP12.<br>Until then, the register and the manifests' justifications are checked by review at each gate |
| DP23 | **The spikes are exempt from the register.** They are throwaway code ([spikes README](../../spikes/README.md)), and each spike's manifest justifies its dependencies, as S1–S4 did. Their crates are listed in the register's last section, so that the lock is fully accounted for. A crate proposed for the engine because a spike used it still needs a full entry |

## 10. Adding a dependency

1. **A design document or milestone names the need.**
2. **Core or infrastructure?** Under DP1, core work is written in the project, and the procedure stops.
3. **Platform facility, crate, or writing it?** Consider whether a platform facility serves, within DP21's placement. Survey the crates. Weigh writing it under DP4.
4. **Judge each candidate:**
   - its tree, its grade and group (DP7–DP10);
   - its license (DP17);
   - its maintenance and precedent;
   - its build cost (DP19);
   - the interface Axiom would own in front of it (DP2).
5. **Write the register entry** (DP6), and the manifest's justification.
6. **Approval follows the class** (DP5): with the subsystem's ADR, or with an ADR of its own for a foundational dependency. Like every ADR, it needs the owner.
7. **Add it,** with minimal features (DP18), behind its interface, with audits recorded (DP15) and the checks passing (DP22).

## 11. Criteria check

| Criterion | How this policy meets it |
|---|---|
| SC2 | DP10 and DP21: Apple's parsers and memory-unsafe parsers run only in sandboxed helpers |
| SC3 | DP8–DP11 carry ADR-0002's rules 5 and 7 into dependencies, graded and placed by group |
| SC4 | DP1–DP5: charter principle 4, made operational |
| SC6 | DP18–DP19: features and costs, against T1–T4 and B1–B7 |
| SC10 | §2: every rule's precedent is sourced and dated |

## 12. Alternatives considered

| | Alternative | Assessment |
|---|---|---|
| A | **No reuse at all,** as SerenityOS's rule was | The charter allows reusing supporting infrastructure. Writing TLS, codecs and shaping would multiply the schedule and enlarge the attack surface, and writing cryptography would be unsafe in itself. Ladybird left this rule behind (§2). Rejected |
| B | **Reuse judged case by case, with no rules** | It invites the sprawl the evaluation warns of (§12), and would leave rule 5 empty for code Axiom doesn't write. Rejected |
| C | **No `unsafe` in any dependency** | The standard library, SIMD primitives and every Objective-C binding contain `unsafe`, so this would rule out objc2, which ADR-0009 requires. Grading and placement (DP8–DP10) target the risk instead. Rejected |
| D | **Counting `unsafe` (cargo-geiger) as the criterion** | A count isn't a soundness judgement; cargo-geiger itself "is not meant to advise directly whether the code ultimately is truly insecure or not" [T1]. Kept as a screening tool for audits. Rejected as the criterion |
| E | **Vendoring every crate into the repository,** as Chromium does | It makes builds reproducible offline, but bloats a public repository, and `Cargo.lock`'s checksums already pin each crate's contents. Deferred: revisited at the first engine milestone |

## 13. Open questions

| # | Question | Where it is settled |
|---|---|---|
| 1 | **Axiom's own license.** The repository was public with no license file (observed 2026-10-18), so by default no one else could reuse it | **Resolved at Gate B** (2026-10-18): "MIT OR Apache-2.0" (OQ-21; DP17) |
| 2 | When `cargo-deny` and `cargo-vet` are installed, and `tools/check-dependencies.py` built | Proposed with the first engine crate; the owner authorizes |
| 3 | Vendoring (alternative E) | The first engine milestone |

## Sources

All read on 2026-10-18. Dates in brackets are the last commits to each file, from GitHub's API.

- [C1] Chromium, `third_party/rust/chromium_crates_io/gnrt_config.toml` [2026-10-15] — https://github.com/chromium/chromium/blob/main/third_party/rust/chromium_crates_io/gnrt_config.toml
- [C2] Chromium, `third_party/rust/README-importing-new-crates.md` — https://github.com/chromium/chromium/blob/main/third_party/rust/README-importing-new-crates.md
- [C3] Chromium, `third_party/rust/chromium_crates_io/supply-chain/audits.toml` [2025-03-22] — https://github.com/chromium/chromium/blob/main/third_party/rust/chromium_crates_io/supply-chain/audits.toml
- [G1] Google, "Auditing standards" (rust-crate-audits) [2025-03-07] — https://github.com/google/rust-crate-audits/blob/main/auditing_standards.md
- [M1] Mozilla, "Cargo Vet" — https://mozilla.github.io/cargo-vet/
- [M2] Mozilla, "Built-in Criteria" (Cargo Vet) — https://mozilla.github.io/cargo-vet/built-in-criteria.html
- [M3] Firefox, `supply-chain/config.toml` and `supply-chain/audits.toml` [both 2026-10-15] — https://github.com/mozilla-firefox/firefox/tree/main/supply-chain
- [S1] Embark Studios, cargo-deny (README; version 0.19.9 in its example) — https://github.com/EmbarkStudios/cargo-deny
- [S2] Servo, `deny.toml` [2026-10-17] — https://github.com/servo/servo/blob/main/deny.toml
- [L1] A. Kling, "Forking Ladybird and stepping down as SerenityOS BDFL" (2024-06-03) — https://awesomekling.substack.com/p/forking-ladybird-and-stepping-down-serenityos
- [L2] Ladybird, `vcpkg.json` [2026-10-17] — https://github.com/LadybirdBrowser/ladybird/blob/master/vcpkg.json
- [L3] Ladybird, `README.md` — https://github.com/LadybirdBrowser/ladybird
- [W1] WebKit, `Source/ThirdParty` [2026-10-16] — https://github.com/WebKit/WebKit/tree/main/Source/ThirdParty
- [R1] RustSec Advisory Database — https://rustsec.org/
- [T1] cargo-geiger, README — https://github.com/geiger-rs/cargo-geiger
