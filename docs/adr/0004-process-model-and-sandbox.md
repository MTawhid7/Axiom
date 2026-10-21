# ADR-0004: Process model, sandbox and JIT policy

| | |
|---|---|
| **Status** | **Accepted** (2026-10-17). The owner accepted its parts with amendments, and instructed that it move to Accepted once spike S4-Q1 and S4-Q2 passed on this Mac. They passed the same day ([S4 results](../research/s4-results.md)). The owner then confirmed the refinements S4 made. **D1 amended by the architecture review's Round 1** (2026-10-21): the GPU/compositor process from M2, real processes from M1, and renderers without IOKit (RV1-02, RV1-07, RV1-09); D2's signing identity, conditional on S15 (RV1-17) |
| **Date** | 2026-10-17 |
| **Deciders** | Project owner (only the owner can move an ADR to Accepted) |
| **Related** | [Threat model](../security/threat-model.md), [process model](../security/process-model.md), [macOS confinement note](../research/macos-process-confinement.md), [Step 3 tracker](../research/step-3-security-and-process-model.md), [ADR-0001](0001-scope-and-target.md) (R3, consequence 4), [ADR-0002](0002-implementation-language.md) (`unsafe` policy), [ADR-0003](0003-gc-handle-and-rooting-pattern.md) (one heap per agent), [S2 results](../research/s2-results.md), [S4 results](../research/s4-results.md) |

## Decision history

| Date | Event |
|---|---|
| 2026-10-17 | Proposed at the end of Step 3 (CP5) |
| 2026-10-17 | **Owner's review.** Guiding principle: "we have no calendar deadline or fixed time limit on this project. We will not cut architectural corners or accept weaker security boundaries for convenience. Where we stage features, the underlying architecture must be designed for the complete, uncompromising model from day one." Decisions: the threat model is accepted; security levels L0–L3 and checks V1–V8 are accepted, with **full out-of-process iframes made a mandatory prerequisite for L3** (D4, D5); the JIT policy J1–J7 is accepted, with global and per-site switches (D7); SI1–SI7 are accepted as staged, with SI3 permitted only at L1 and non-sensitive L2 (D4); memory targets B1–B7 are accepted as initial baselines (D6); spike S4 is authorized. **The ADR stays Proposed until S4-Q1 and S4-Q2 pass, then moves to Accepted** |
| 2026-10-17 | **Spike S4: S4-Q1 and S4-Q2 pass** ([S4 results](../research/s4-results.md)). A custom deny-by-default profile applies to an ad-hoc-signed, hardened-runtime helper built against SDK 27.0, by three methods. All 18 probes behave as specified. S4 also refines three decisions, recorded below: the browser compiles profiles (D2); J3's explicit rule is required (D7); J5 adds `single-jit` (D7) |
| 2026-10-17 | **Accepted**, by the owner's instruction of the same day: "Once S4-Q1 and S4-Q2 pass, ADR-0004 moves to Accepted" |
| 2026-10-17 | **Owner confirms S4's three refinements:** browser-compiled profile bytecode (D2), the required explicit `(deny dynamic-code-generation)` (J3), and `com.apple.security.cs.single-jit` on `renderer-jit` (J5). They "are well-justified by the S4 empirical data and strengthen both our security posture and memory budgets" |
| 2026-10-21 | **Amendments accepted** in the owner's dispositions of the architecture review's Round 1 ([the review](../review/README.md) §5.1):<br>• **RV1-02:** "Formally design the sandboxed GPU/compositor process topology now (Renderer → GPU Process → Browser CALayer tree)." "Build the separate GPU/compositor process at Milestone 2; split Budget B1 accordingly." "Ensure Level 3 (L3) security certification at Milestone 6 is performed on the final multi-process topology."<br>• **RV1-07** ("Accepted (Recommended Option)"): "Run real, sandboxed renderer and network processes from Milestone 1 using the profiles validated in Spikes S4, S5, and S7." "Retain single-process mode strictly for rapid automated testing, fuzzing, and Miri."<br>• **RV1-09:** "Renderers write pixels into shared memory entries; only the compositor owns IOSurfaces; renderer profiles explicitly deny iokit-open."<br>D1 is amended below, and the [process model](../security/process-model.md) records the changes (§3 note 1, §4, §7, §9, §10, and its change record). Spike S10 is authorized to measure the hop, validation and zero-copy sampling before M2 |
| 2026-10-21 | **Amendment accepted** in the owner's dispositions of the architecture review's Round 1 ([the review](../review/README.md) §5.1), RV1-17: "Validate local self-signed codesigning persistence via Spike S15." D2 is amended: a stable local signing identity with an explicit designated requirement, conditional on spike S15; ad-hoc signing stays the fallback |
| 2026-10-21 | **Spike S10's results** ([S10 results](../research/s10-results.md)): the GPU/compositor process's profile, each rule checked by removal (§6.1), entered by D2's method, the browser compiling it, which kept the sandboxed GPU process's footprint equal to an unsandboxed one's (§7). **One departure from D2, for the owner's decision:** Metal finds its shader cache through the user's cache directory, which can't be resolved inside the profile, so the GPU process resolves it first, one libSystem call before its profile, where D2 has a helper apply its profile "as its first act" (§6.2). Without it every start compiles the shaders, about 0.45 s against 20 ms |
| 2026-10-21 | **Spike S15's results** ([S15 results](../research/s15-results.md)): under a self-signed identity, S4's 18 probes, the JIT matrix with `allow-jit` and `single-jit`, CoreText in its profile, start-up and IPC behave exactly as under ad-hoc signing; a rebuild under the identity keeps its Keychain item, where an ad-hoc rebuild is refused (`errSecAuthFailed`), because the item trusts the designated requirement. D2's condition is met; TCC's persistence is checked by hand at M1's shell checkpoint |

## Question

How is Axiom split into processes, and how is each process confined on macOS? Which rules govern communication between processes, and how are sites assigned to them? Under what conditions may a JIT exist (OQ-4)?

The answer must meet ADR-0001 R3: a compromised renderer must expose neither credentials, nor other sites' data, nor the file system.

## Requirements and constraints

- **R3 (ADR-0001):** a posture fit for sensitive logins, with process isolation, a sandbox and site isolation reachable. Sensitive use is gated until isolation is verified.
- **JIT (ADR-0001, consequence 4):** interpreter first; any JIT only after isolation and the sandbox exist; a no-JIT mode always available.
- **`unsafe` policy (ADR-0002):** parsers, network policy and IPC message decoding contain no `unsafe`. FFI lives in `platform`.
- **One single-threaded GC heap per agent (ADR-0003).**
- **Memory (charter G4, ADR-0001 consequence 7):** per-process budgets on 8 GB.
- **Environment:** macOS 27.2; Command Line Tools only; ad-hoc signing only, with no paid developer account. That rules out provisioning profiles and restricted entitlements ([macOS note](../research/macos-process-confinement.md) §1).

## Alternatives considered

**Process architecture**

| | Alternative | Summary |
|---|---|---|
| A1 | **One process,** relying on Rust's memory safety | No OS boundary between web content and the user's authority |
| A2 | **A process per tab,** no site isolation | Chrome before 2018; Safari historically |
| A3 | **Full site isolation with out-of-process iframes from the start** | Chrome on desktop today |
| **A4** | **Staged** (recommended): sandboxed renderers and a network process first; then site-locked renderers, with cross-site iframes in their embedder's process under compensating rules; out-of-process iframes before sensitive use (L3) | The [process model](../security/process-model.md) |

**Confinement mechanism on macOS** ([macOS note](../research/macos-process-confinement.md) §4)

| | Mechanism |
|---|---|
| **O1** | **Recommended:** `posix_spawn` plus a custom Seatbelt profile that each helper applies at startup |
| O2 | App Sandbox entitlements on each helper |
| O3 | XPC services |
| O4 | BrowserEngineKit |
| O5 | Enhanced Security helper extensions |

## Historical context and modern developments

- **Chrome** has been multi-process since its 2008 launch ([decision map](../decision-map.md) §2, item 3). It deployed Site Isolation to all desktop users in 2018 as a mitigation for Spectre. Total memory rose by 9–13%: at the median, private memory per renderer fell by 51.5% while the number of renderer processes rose by 43.5% [3]. Sourced.
- **Firefox** took years to retrofit multiple processes (Electrolysis) and then site isolation (Fission, 2021) ([decision map](../decision-map.md) §2, item 3). Sourced there.
- **WebKit** was still building out-of-process iframes in 2025. In July 2025 it added a shared process for cross-site iframes "for performance reasons" [macOS note, source 14]. Sourced.
- **On macOS, every major engine** confines content with Seatbelt profiles applied through undocumented libsandbox functions. In the macOS 27 SDK, Apple kills processes that use the old *named* profiles, while Chrome's SDK 27 build still applies custom ones. Observed on this machine ([macOS note](../research/macos-process-confinement.md), Summary).
- **The JIT is the riskiest component.** Of the Chrome exploits seen in the wild from 2021 to 2023 that began with memory corruption in a renderer, 60% exploited V8 bugs. Many V8 bugs are logic bugs that a memory-safe implementation language doesn't prevent [5]. Sourced.
- **Disabling the JIT is an established defence:**
  - Microsoft Edge's enhanced security mode turns the JIT off, for unfamiliar sites in "balanced" mode or for all sites in "strict" mode [6].
  - WebKit on macOS 27.2 ships two content-process variants without the JIT entitlement and with `(deny dynamic-code-generation)` (observed).
  - In 2019, V8 measured its JIT-less mode as about 40% slower on Speedometer 2.0, 80% slower on the Web Tooling Benchmark, and 6% slower in JS execution in a simulated browsing session of YouTube's TV interface ("Living Room") [7].

## Analysis

| Consideration | A1 | A2 | A3 | **A4** |
|---|---|---|---|---|
| A compromised renderer (threat model AD2) reaches user files | Yes | No | No | No |
| … reaches other sites' data | Yes | Yes, any site in the same tab | No | No for top-level sites. Embedded cross-site frames carry only partitioned state; sensitive sites are never embedded in another site's process |
| Spectre-class reads of cross-site data | Yes | Yes | No | As the row above |
| Implementation cost | Lowest | Moderate | Highest: frame-tree replication, cross-process compositing and input routing | A3's full cost, paid before L3; the architecture carries it from the start (SI4) |
| Memory | Lowest | Low | +9–13% over A2 in Chrome [3] | Between A2 and A3 |
| Path to A3 | Rewrite | Hard | — | Designed in (SI4) |

**Why O1 over O2.** App Sandbox's base profile lets every sandboxed process reach the window server, font server, pasteboard and LaunchServices: 170 Mach services in all. A child also can't be made stricter than its parent. O1 is what Chrome, Firefox and WebKit use for content. Its cost is reliance on private interfaces ([macOS note](../research/macos-process-confinement.md) §2.1–§2.2, §4).

## Evidence and sources

- [Threat model](../security/threat-model.md), [process model](../security/process-model.md) and [macOS confinement note](../research/macos-process-confinement.md), with their sources. The note records the observations made on this machine (§9).
- [S2 results](../research/s2-results.md): the footprint of a window with a Metal device; an event loop that costs nothing when idle.
- [S4 results](../research/s4-results.md): the sandbox mechanism validated on this Mac; start-up, footprint, IPC, JIT and CoreText measurements.

## Decision (proposed)

**D1. Process types.**

| Process | Role | Confinement |
|---|---|---|
| Browser | UI, navigation, frame tree, policy, profile; the CoreAnimation layer tree, scrolling and compositor animation on layers, input routing. **As amended in Round 1:** composites display lists only at M1, as a thread behind the compositor's interface | Hardened runtime, no sandbox. Never parses web content; from M2, consumes no display lists |
| Renderer (many) | All interpretation of web content | Deny-by-default Seatbelt profile; no files, network, window server, IOKit or process launching. Pixels go only into Mach memory entries (Round 1, RV1-09) |
| Network (1) | HTTP, TLS, DNS, cache, cookie jar, web storage, ORB, CORS | Seatbelt: outbound network and its own data directory only |
| GPU/compositor (1, from M2) | **As amended in Round 1** (RV1-02): validates every renderer's display lists, rasterizes tiles into IOSurfaces it owns, and sends the browser layer commits. Takes no web-supplied GPU work; WebGL and WebGPU (M9) run where M9's plan places them | Seatbelt: Metal only |
| Utility (on demand) | Media decoding and other isolated jobs | Seatbelt, per kind of job |

The full capability matrix is in the [process model](../security/process-model.md) §4.

**D1 as amended in Round 1** (2026-10-21; RV1-02, RV1-07, RV1-09):
- **The topology is renderer → GPU/compositor process → browser,** from M2. The browser holds the layer tree and scrolls and animates layers itself, so input never waits on the GPU process ([process model](../security/process-model.md) §3, note 1). L3's checks at M6 run on this topology.
- **Real, sandboxed renderer and network processes from M1,** under the profiles S4, S5 and S7 wrote. The compositor runs as a browser thread at M1, behind the interface the GPU process serves from M2. Single-process mode is kept strictly for automated tests, fuzzing and Miri.
- **Renderers have no IOKit:** they write pixels into Mach memory entries, the compositor samples them and owns every IOSurface, and every renderer profile denies `iokit-open` explicitly, as J3 denies code generation.

**D2. Confinement.**
- **Option O1.** Each helper is its own executable, signed ad hoc with the hardened runtime and only the entitlements it needs. It applies its own deny-by-default profile at the top of `main`, before any untrusted input or resource reaches it.
- **The browser compiles each profile once**, with the helper's parameters, and sends the bytecode over the inherited channel. The helper applies it as its first act (S4 refinement). Measured: the helper is ready 2.1 ms after launch with a 1.1 MiB footprint, the same as an unsandboxed helper. Compiling in the helper would cost about 6 ms and 2.8 MiB per process.
- **Profiles** live in the repository and are reviewed like `unsafe` code.
- **The browser process stays unsandboxed** under the hardened runtime.
- **Fallback:** option O2, with its weaker guarantees stated, if custom profiles stop working.
- **The signing identity, as amended in Round 1** (RV1-17; the owner: "Validate local self-signed codesigning persistence via Spike S15."): ad-hoc code "has a DR but it's tied to that specific version of the code", so macOS forgets privacy decisions and Keychain approvals at every rebuild (Apple's TN3127, revised 2024-04-02, as RV1-17 cites it). **The browser and its helpers are signed with a stable local identity:** a self-signed code-signing certificate created on this Mac, with a designated requirement naming the identifier and the certificate, so decisions persist across builds. **Conditional on spike S15** (authorized 2026-10-21): S4's probes, `allow-jit` with `single-jit`, and the custom profiles must behave as under ad-hoc signing. Ad-hoc signing stays the fallback. The private key stays in the owner's login keychain, never in the repository; creating it there is the owner's act. **S15's result** (2026-10-21; [S15 results](../research/s15-results.md)): the condition is met for S4's probes and the JIT entitlements, and for the Keychain, where a rebuild under the identity kept access that an ad-hoc rebuild lost; privacy permissions are checked by hand at M1's shell checkpoint.

**D3. Launch and IPC.**
- Only the browser launches processes, using `posix_spawn` with no inherited descriptors (`POSIX_SPAWN_CLOEXEC_DEFAULT`).
- IPC follows requirements R1–R6 and trust rules IR1–IR10 ([process model](../security/process-model.md) §6). In particular, decoding is safe Rust, decisions come from the receiver's own records, and any violation kills the sender.
- The transport (Mach messages or socket pairs) is chosen in Step 4 (OQ-17). S4-Q3's measurements favour Mach messages: 24% faster round trips, port rights, and the kernel's audit trailer for peer identity.

**D4. Site isolation.**
- Rules SI1–SI7 ([process model](../security/process-model.md) §5): site locks from L2; process limits never merge sites.
- **Cross-site iframes.** At L1 and non-sensitive L2 only, a cross-site iframe may render in its embedder's process under three compensating rules (SI3): partitioned third-party state, ORB, and no sensitive site in another site's process.
- **Out-of-process iframes are mandatory before L3** (owner, 2026-10-17; OQ-18 resolved). From its first line of code, the Step 4 data model has a browser-owned frame tree, per-frame display lists and browser-routed input (SI4), so that moving to out-of-process iframes needs no architectural rewrite.
- **Sensitive sites** are the owner's list plus Chromium's heuristics for sites likely to hold user data: password entry, OAuth login, COOP.

**D5. Security levels and the gate.**
- Levels L0–L3, and the checks V1–V9 that gate sensitive use ([threat model](../security/threat-model.md) §8). V9, added by the owner's amendment, verifies out-of-process iframes.
- Email, banking and accounts with write access wait for L3.

**D6. Memory.**
- Initial targets B1–B7 ([process model](../security/process-model.md) §7): about 1 GB for a typical session; a soft limit of 16 renderers on 8 GB; same-site process reuse; tab discarding under memory pressure.
- Final values are set in Step 6, from measurements.

**D7. JIT policy (resolves the policy part of OQ-4).**

| # | Rule |
|---|---|
| J1 | **Interpreter first. Whether to build a JIT is decided by evidence:** spike S3 in Step 4, and target-corpus measurements once OQ-2 is answered |
| J2 | **No JIT before L2,** and not before the JIT renderer variant passes the sandbox escape tests (V1) |
| J3 | **Only one executable, `renderer-jit`, carries `com.apple.security.cs.allow-jit`.** Every other Axiom executable runs under the hardened runtime without it, so the kernel refuses to give it executable memory (S4: `EINVAL`). Every profile except `renderer-jit`'s **must** contain an explicit `(deny dynamic-code-generation)`: S4 showed that `(deny default)` doesn't cover it, while the explicit rule refuses `MAP_JIT` even to a holder of `allow-jit` |
| J4 | **Sensitive sites, and every document rendered in a sensitive site's process, run in the no-JIT renderer.** A **global switch** moves every site to the no-JIT renderer, like Lockdown Mode, whose WebKit content process has no JIT entitlement (observed). A **per-site switch** does the same for any site the owner chooses. A mode that disables the JIT on unfamiliar sites, like Edge's "balanced" mode, can be added later as a setting |
| J5 | **Generated code is written only through statically allowlisted callbacks** (`pthread_jit_write_with_callback_np` with `jit-write-allowlist`), as in every WebKit process. Code is flushed with `sys_icache_invalidate`. **`renderer-jit` also carries `com.apple.security.cs.single-jit`,** so that code lives in a single `MAP_JIT` region. S4 found that the limit applies only with this entitlement, which ad-hoc signing honours |
| J6 | **A WebAssembly compiler counts as a JIT** and follows J1–J5. An interpreter keeps WebAssembly working in no-JIT renderers, as in Edge's enhanced security mode [6] |
| J7 | **Before a JIT ships,** it needs: differential testing against the interpreter (test262 and fuzzing); a design for containing memory corruption within the JS heap; and a tested global switch (V7). JIT bugs are typically logic bugs that become memory corruption [5]. **Hypothesis:** Pattern B's 32-bit handles into arenas (ADR-0003) are a natural basis for that containment design, to be examined if J1 leads to a JIT |

**D8. Validation before acceptance: done.**
- Spike S4 (authorized 2026-10-17) passed S4-Q1 and S4-Q2 on 2026-10-17 ([S4 results](../research/s4-results.md)).
- S4-Q3 to S4-Q5 refined D2, D3, D6 and D7, as recorded above. S4-Q6, the fallbacks, wasn't needed.

## Rationale

1. **R3 needs OS boundaries.**
   - Rust lowers the rate of renderer compromise but doesn't remove it: there will be system parsers, `unsafe` FFI and, later, a JIT.
   - Spectre reads data within a process without any bug at all [3].
   - So web content must run in processes that hold no user authority and, from L2, only one site's data (A4, D1, D4).
2. **Staging (A4) puts each cost where it buys the most.**
   - Sandboxed renderers and a network process protect files and credentials early (L1).
   - Site locks for top-level sites cost about a tenth more memory [3] and protect sites from one another (L2).
   - Out-of-process iframes, the most expensive part, come last, but before sensitive use (L3). Until then, partitioned third-party state limits what an embedded frame exposes, and sensitive sites are never embedded in another site's process.
3. **O1 is the only option that gives renderers a WebKit-class sandbox** without Apple's approval. Chrome and Firefox show it works for third-party browsers on macOS 27; WebKit's profiles show what it can express.
4. **IR1–IR10 turn ADR-0002's "no `unsafe` in IPC decoding" into a complete trust model.** Decoding, deciding and failing closed are each specified.
5. **The JIT policy enforces ADR-0001's intent with mechanisms rather than conventions.** Entitlements are per executable and the kernel enforces them; profiles deny code generation as well. The data supports the trade-off. In V8's 2019 measurements, the JIT-less slowdown was large on compute-heavy benchmarks but small in a browsing session [7], and sensitive sites are mostly of the browsing kind (inference).

## Assumptions

- Custom Seatbelt profiles keep working for binaries built against current SDKs. S4 confirmed this for SDK 27 binaries on macOS 27.2; the risk of a later change is below.
- A helper can apply its profile before it needs any resource the profile denies (S4-Q2, S4-Q5).
- Partitioning third-party cookies and storage by top-level site is compatible with the target corpus. Safari and Firefox do it by default (background).
- The owner can name the sites they consider sensitive.

## Risks and unresolved issues

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Apple removes or restricts custom Seatbelt profiles | Low to medium; the macOS 27 SDK already kills processes that use named profiles | High | Fallback O2. Chrome and Firefox would break too, so a change would be visible in betas (inference). Run the sandbox test suite (V1) on every macOS update |
| Profiles break when macOS frameworks change what they access | Medium | Medium | Minimal profiles grown by evidence; V1 tests; logging of sandbox denials in development |
| Cross-site iframes in the embedder's process leak more than partitioning suggests | Medium | Medium, limited to L1 and non-sensitive L2 | SI3's sensitive-site rule. Out-of-process iframes are mandatory before L3 |
| Memory targets prove too tight | Medium | Medium | Measure in S4 and Step 6. Same-site reuse and tab discarding. Revisit B5 rather than merge sites |
| IPC decoders become the main attack surface of the browser | High (by design) | High | Generated safe Rust decoders; fuzzing (V2); IR2 and IR3 |
| No WebAuthn platform authenticator without a restricted entitlement | Medium | Medium | OQ-16 |

## Conditions for revisiting

- **S4-Q1 or S4-Q2 fails,** or a later macOS release stops custom profiles: switch D2 to O2 and restate the guarantees.
- **A public route to strict, multi-instance helper sandboxes appears,** such as Enhanced Security helper extensions (S4-Q6) or public multiple XPC instances. Re-evaluate D2, including sandboxing the browser process.
- **Memory measurements show B5 or B6 can't be met without merging sites.** Re-decide SI6 with the owner.
- **Evidence shows SI3's compensating rules are insufficient** even at L1 or non-sensitive L2. Bring out-of-process iframes forward.

## Consequences

**Step 4 must design:**
- message-based interfaces at every future process boundary (PM6);
- a frame tree owned by the browser, with per-frame display lists and input routed by the browser, from the first line of code (SI4), so that out-of-process iframes need no architectural rewrite;
- one agent cluster per process (SI7);
- a display list that never requires the browser to decode images or parse fonts.

**Step 5 must decide:**
- where glyphs are rasterized (S4-Q5);
- how the network process enforces initiator locks and ORB;
- the TLS stack. If it is Apple's, it must stay out of the browser process (Rule of 2).

**Step 6 sets:**
- final memory budgets;
- which milestone reaches each security level;
- the routine of advisory checks and rebuilds (threat model §9).

**Code structure:**
- the `platform` crate gains the sandbox, spawn and IPC-transport FFI, under the existing `unsafe` policy;
- helper binaries stay small and enter their sandbox before loading the engine.

**OQ-4's policy part is answered by D7.** Whether to build a JIT stays evidence-driven (J1).

## Sources

1. [Threat model](../security/threat-model.md), §11.
2. [Process model](../security/process-model.md), §11.
3. C. Reis, A. Moshchuk, N. Oskov, "Site Isolation: Process Separation for Web Sites within the Browser", USENIX Security 2019, §5.2–5.3. https://www.usenix.org/conference/usenixsecurity19/presentation/reis
4. [macOS confinement note](../research/macos-process-confinement.md), §9–§10.
5. V8, "The V8 Sandbox" (2024-04-04). https://v8.dev/blog/sandbox
6. Microsoft Learn, documentation of Edge's enhanced security mode (applies to Edge 111 and later; checked 2026-10-17). https://learn.microsoft.com/en-us/deployedge/microsoft-edge-security-browse-safer
7. V8, "JIT-less V8" (2019-03-13), V8 v7.4. https://v8.dev/blog/jitless
