# Process Confinement on macOS: Mechanisms and Browser Precedent

| | |
|---|---|
| **Status** | Draft, for review with ADR-0004 |
| **Question** | How can Axiom confine its helper processes on macOS 27, building with the Command Line Tools only and signing ad hoc (no paid developer account)? How do Chromium, WebKit and Firefox do it, and what does that imply for Axiom? |
| **Feeds** | ADR-0004; [process model](../security/process-model.md); [threat model](../security/threat-model.md) §9; spike S4 (§7) |
| **Last updated** | 2026-10-17 |

**Evidence labels** follow the [research conventions](README.md):
- **Observed** facts were read on this Mac (macOS 27.2, build 26B5112a; Command Line Tools with SDK 27.0) using system tools that change nothing. §9 lists the files and commands.
- **Sourced** facts cite §10.
- **Inference** and **hypothesis** are marked; each hypothesis names the S4 question that tests it.

---

## Summary

| Finding | Evidence |
|---|---|
| **Every major browser on macOS confines its content processes with Seatbelt profiles applied through libsandbox functions that Apple doesn't document.** Chrome's renderer helper calls `sandbox_init_with_parameters` and `sandbox_apply`; Firefox calls `sandbox_init_with_parameters`; WebKit ships deny-by-default profiles inside `WebKit.framework` | Observed (`nm -u`; profile files) and sourced [1][3] |
| **The public face of that API is closing.** `sandbox_init` has been deprecated since macOS 10.8. In the macOS 27 SDK, its man page adds that the named profiles "must not be used when building against macOS SDK >= 27.0. Processes opting into them will be killed" | Observed (SDK 27.0 header and man page) |
| **Custom profiles still work for SDK 27 binaries.** Chrome Canary 158 is built against SDK 27.0 and its helpers apply custom profiles | Observed (`otool -l`, `nm -u`). **Inference:** the kill rule covers the named profiles, not custom ones. S4-Q1 tests this directly |
| **App Sandbox, the supported mechanism, is built for apps, not renderers.** Its base profile allows 170 global Mach services, including the window server, font server, pasteboard and LaunchServices. WebKit's content profile denies exactly those | Observed (`application.sb`; WebKit's `com.apple.WebProcess.sb`) |
| **App Sandbox can't make a child stricter than its parent.** Children inherit the parent's sandbox, and a process that is already sandboxed can't apply another profile. Safari gets per-process profiles only because its helpers are XPC services launched by launchd | Observed (sandbox(7), `sandbox.h`, Safari's and WebKit's entitlements) and sourced [7] |
| **Getting one XPC service process per renderer needs a private key.** WebKit's XPC services set `_MultipleInstances` in their `Info.plist`. Apple's public XPC documentation doesn't describe multiple instances | Observed; sourced [11] (silence) |
| **Entitlements belong to executables, so each set of privileges needs its own helper binary.** Chrome gives `allow-jit` only to its renderer and GPU helpers. WebKit ships three content executables: one with a JIT, and two without (Lockdown Mode and "EnhancedSecurity") | Sourced [8]; observed |
| **Seatbelt can forbid JIT independently of entitlements.** WebKit's content profile contains `(deny dynamic-code-generation)` for its no-JIT variants | Observed |
| **Ad-hoc signing covers what Axiom needs.** macOS needs no provisioning profile for App Sandbox or hardened-runtime entitlements, including `allow-jit`. Only restricted entitlements need one | Sourced [10]. **Hypothesis:** that holds for ad-hoc signatures; tested by S4-Q1, S4-Q4 and S4-Q6 |
| **BrowserEngineKit is not an option.** The SDK declares its process classes for macOS 14.3, but Apple documents it for iOS, iPadOS and Mac Catalyst, with entitlements that must be requested per developer account | Observed (SDK headers); sourced [12]. **Inference:** unavailable to an ad-hoc-signed browser |

**Recommendation for ADR-0004 (§6).**
- The browser process spawns small helper executables, one per privilege set.
- Each helper applies its own deny-by-default Seatbelt profile as the first thing in `main`, before it holds any untrusted input or resource. This is Chromium's "V2" model [1].
- The browser process stays unsandboxed but runs under the hardened runtime, as Chrome's and Firefox's do (observed).
- Spike S4 must confirm that this works on this machine before the ADR is accepted (§7).

---

## 1. Constraints

| Constraint | Consequence |
|---|---|
| Command Line Tools only (charter §4) | No Xcode build phases or capability editor. Bundles, `Info.plist` files and entitlements are written by hand and signed with `codesign` |
| No paid Apple Developer Program membership (status, Step 3 inputs) | Ad-hoc signatures only (`codesign -s -`). No provisioning profiles, so no restricted entitlements [10] |
| macOS 27.2 on an M1 | The newest APIs may be used (charter §6). Hardware memory tagging needs an M5 or later [13], so it isn't available here |
| Code in Rust; `unsafe` confined to designated crates (ADR-0002) | Every libsandbox, Mach and spawn call is FFI in the `platform` crate. Message *decoding* stays safe Rust (rule 5) |

## 2. Mechanisms

### 2.1 Seatbelt (`sandbox(7)`)

**How it works.**
- A process restricts itself with a profile written in SBPL, a Scheme-like language. Profiles start from `(deny default)` and allow specific operations: file paths, Mach service lookups, IOKit classes, sysctls, system calls (observed, in WebKit's profiles).
- **Inheritance:** "New processes inherit the sandbox of their parent" (observed, sandbox(7)).
- **Enforcement happens when a resource is acquired:** "if the application already has a file descriptor opened for writing, it may use that file descriptor regardless of restrictions" (observed, sandbox(7)).
- **No second profile:** "If the process is already in a sandbox, the new profile is ignored and sandbox_init() returns an error" (observed, `sandbox.h`).

**Consequences** (inference):
1. A helper must enter its sandbox before it opens anything it shouldn't keep.
2. The browser must not leak file descriptors into helpers. `posix_spawn`'s `POSIX_SPAWN_CLOEXEC_DEFAULT` closes all of them except those passed explicitly (observed in `sys/spawn.h`).
3. Capabilities can be handed over as already-open descriptors and Mach ports, rather than as paths the sandbox must allow.

**API status in the macOS 27 SDK** (observed):

| Interface | Status |
|---|---|
| `sandbox_init`, `sandbox_free_error` (`sandbox.h`) | `API_DEPRECATED("No longer supported", macos(10.5, 10.8))`. The header says it "may be removed in a future release" and points to App Sandbox. Its only documented flag is `SANDBOX_NAMED` |
| Named profiles (`kSBXProfileNoInternet`, `kSBXProfilePureComputation`, …) | No longer declared in the header, though still exported. The man page: "These profiles must not be used when building against macOS SDK >= 27.0. Processes opting into them will be killed on attempt to do so." |
| `sandbox_init_with_parameters`, `sandbox_check`, `sandbox_extension_issue_file`, `sandbox_extension_consume`, … | Exported by `libsystem_sandbox` (SDK `.tbd` stub); not declared in any SDK header |
| `sandbox_compile_string`, `sandbox_apply`, `sandbox_create_params`, `sandbox_set_param`, … | Exported by `libsandbox.1` (SDK `.tbd` stub); not declared in any header |
| `sandbox-exec` | "DEPRECATED" (man page), but still installed at `/usr/bin/sandbox-exec` |

**Who uses the private functions** (observed with `nm -u`):
- **Chrome Canary 158.0.8108.0** (built against SDK 27.0, minimum macOS 13.0):
  - its renderer helper executable (170 KB) imports `sandbox_init_with_parameters`, `sandbox_apply` and `dlopen`;
  - its framework imports `sandbox_compile_string`, `sandbox_create_params`, `sandbox_set_param` and the `sandbox_extension_*` functions.
- **Firefox (Tor Browser 16.0a11):** `XUL` imports `sandbox_init_with_parameters`.
- **Chromium's source** says of these functions: "OS X deprecated these functions, but did not provide a suitable replacement" [3].

**What WebKit's profiles show the language can express** (observed; macOS 27.2's `com.apple.WebProcess.sb` has 1,834 lines):
- `(deny default)`;
- per-name `mach-lookup` rules. The content profile **denies** `com.apple.fonts`, `com.apple.windowserver.active`, `com.apple.CoreServices.coreservicesd`, `com.apple.lsd.mapdb` and `com.apple.DiskArbitration.diskarbitrationd`;
- `(deny dynamic-code-generation)`, filtered by signing identifier, for the no-JIT content variants;
- allowlists for BSD system calls, Mach traps and MIG routines, each with a stricter Lockdown Mode branch;
- state flags such as `BlockIOKitInWebContentSandbox`, which tighten the sandbox after launch;
- no outbound network access, apart from syslog in Apple-internal builds.

### 2.2 App Sandbox

**How it works.**
- An executable is sandboxed by signing it with the entitlement `com.apple.security.app-sandbox`. Further entitlements grant capabilities such as `network.client`, `files.user-selected.read-write` and device access [6].
- It is the supported mechanism, and required in the Mac App Store [6].
- macOS needs no provisioning profile for "those [entitlements] used to enable and configure the App Sandbox" [10].

**The base profile is broad** (observed). `/System/Library/Sandbox/Profiles/application.sb` has 1,002 lines and imports `system.sb` and `appsandbox-common.sb`. Every sandboxed app may look up 170 distinct global Mach services, including:
- `com.apple.windowserver.active` and `com.apple.CARenderServer`;
- `com.apple.fonts`;
- `com.apple.pasteboard.1`;
- `com.apple.CoreServices.coreservicesd`;
- `com.apple.tccd`.

**Inference.** These services suit an app with a user interface. For a renderer they are the attack surface that WebKit's content profile removes, and an entitlement can't take them away.

**Children.**
- A helper tool inside a sandboxed app is signed with `com.apple.security.app-sandbox` and `com.apple.security.inherit`, and Apple warns that "Adding other entitlements to the tool can cause problems" [7].
- Together with inheritance and the no-second-profile rule (§2.1), this means a child can't be given a sandbox **stricter** than its parent's (inference).
- **Safari** is App-Sandboxed: `app-sandbox`, `network.client`, `files.user-selected.read-write`, plus exceptions (observed). Its WebContent, GPU and Networking processes are XPC services, which launchd starts, each with its own profile and entitlements (observed).

**Containers.** Each sandboxed executable gets a container directory under `~/Library/Containers` (background).

### 2.3 Hardened runtime and per-executable entitlements

- The hardened runtime (`codesign -o runtime`) "protects the runtime integrity of your software by preventing certain classes of exploits, like code injection, dynamically linked library (DLL) hijacking, and process memory space tampering". It disallows JIT compilation unless an exception entitlement allows it [8].
- Exceptions: `allow-jit`, `allow-unsigned-executable-memory`, `allow-dyld-environment-variables`, `disable-library-validation`, `disable-executable-page-protection`, `debugger` [8].
- **Entitlements are per executable:** "You add entitlements only to executables. Shared libraries, frameworks, and in-process plug-ins inherit the entitlements of their host executable" [8].

**Consequence:** each distinct privilege set needs its own executable (inference, from [8]).

**What the installed browsers do** (observed with `codesign -d`):

| Executable | Hardened runtime | Entitlements (summary) |
|---|---|---|
| Chrome: main app | Yes; library validation on | Devices, Keychain access groups, `web-browser.public-key-credential`; **no App Sandbox** |
| Chrome: Helper (Renderer), Helper (GPU) | Yes; library validation **off** | `com.apple.security.cs.allow-jit` only |
| Chrome: Helper (plain), Helper (Alerts) | Yes; library validation on | None |
| Firefox (Tor Browser): main app | Yes | `allow-jit`, `disable-library-validation`, devices; **no App Sandbox** |
| Firefox: `plugin-container.app` | Yes | `allow-jit` |
| WebKit: WebContent | No runtime flag; it is an Apple platform binary with library validation. It carries the `hardened-process` entitlements instead | `allow-jit`, `single-jit`, `jit-write-allowlist`, and private ones such as `verified-jit` and `security.message-filter` |
| WebKit: WebContent.EnhancedSecurity, WebContent.CaptivePortal | As WebContent | As WebContent, but **without `allow-jit`**. CaptivePortal adds `com.apple.imageio.allowabletypes` |
| WebKit: GPU, Networking | As WebContent | No `allow-jit`; private entitlements |

**Notes.**
- The WebKit profile names the CaptivePortal variant `lockdown-mode`.
- A 2025 WebKit change is titled "Fix JIT entitlement logic for EnhancedSecurity processes" [14].
- Which Safari feature selects the EnhancedSecurity variant is not documented in the sources consulted.

### 2.4 JIT on Apple silicon

All sourced from [9] or observed in the man pages:
- `MAP_JIT` lets a mapping be both writable and executable under the hardened runtime, and "requires the caller to have the `com.apple.security.cs.allow-jit` entitlement on macOS" (observed, mmap(2)). The entitlement "is required only when an app adopts the Hardened Runtime capability" [9].
- With the hardened runtime and `allow-jit`, a process "can only create one memory region with the MAP_JIT flag set" [9].
- **Per-thread W^X.** `pthread_jit_write_protect_np` switches the calling thread between writing and executing the region (observed, man page).
- **A stricter variant.** `pthread_jit_write_with_callback_np` allows writes only from callbacks listed statically with `PTHREAD_JIT_WRITE_ALLOW_CALLBACKS_NP`. Under the `jit-write-allowlist` entitlement, `pthread_jit_write_protect_np` is unavailable [9]. Every WebKit process carries `jit-write-allowlist` (observed).
- Instruction caches aren't coherent with data caches, so generated code must be flushed with `sys_icache_invalidate` before it runs [9].
- **Defence in depth:** a sandbox profile can deny `dynamic-code-generation` even where the entitlement exists (observed, WebKit). Whether that denial blocks `MAP_JIT` itself is tested by S4-Q4.

### 2.5 Launching processes and connecting them

| Mechanism | What it is | Used by | For Axiom |
|---|---|---|---|
| **`posix_spawn`** of helper executables | The parent controls the environment, the inherited descriptors (`POSIX_SPAWN_CLOEXEC_DEFAULT`) and special ports (`posix_spawnattr_setspecialport_np`) (observed, headers and man page) | Chrome (`posix_spawn` and file actions imported, observed; design doc [1]). Firefox's launch mechanism wasn't verified | **Recommended.** Any number of instances; no launchd registration; the helper applies its own profile |
| **XPC services** in `Contents/XPCServices` | launchd launches the service and connects client and listener [11]. Each service has its own entitlements, so its own sandbox (inference, from §2.2 and Safari) | WebKit, with the private `_MultipleInstances` key for one process per content instance (observed) | Without that private key, one instance per service (inference; the public documentation is silent [11]). That conflicts with one renderer per site |
| **BrowserEngineKit** | Extension processes (`BEWebContentProcess`, `BENetworkingProcess`, `BERenderingProcess`) with a libxpc connection. The SDK declares them `API_AVAILABLE(macos(14.3), ios(17.4))` (observed) | Third-party browsers on iOS | Documented for iOS, iPadOS and Mac Catalyst only; the entitlements are requested per developer account and vary by region [12]. **Not available** (inference) |
| **Enhanced Security helper extensions** | "Create Enhanced Security helper extensions on iOS, iPadOS, and macOS to enforce tighter sandbox restrictions." "The system runs your extension in a sandbox that restricts access to most system services and frameworks." Launched as an `AppExtensionProcess` [13] | New; no browser seen using one | **Unknown.** It is a public route to tight sandboxes, but it is built as an Xcode extension target. Whether it works without Xcode or a paid account, and with many instances, is unknown (S4-Q6, optional) |

**Rendezvous and IPC transport.**
- **Chrome and Firefox** both import `bootstrap_check_in` and `bootstrap_look_up` (observed). Chromium's renderer profile allows exactly one browser-specific service, `BUNDLE_ID.MachPortRendezvousServer.BROWSER_PID` [4].
- **Mach messages** carry port rights natively: IOSurfaces, memory entries and further channel endpoints. The kernel attaches an audit token identifying the sender (background).
- **Unix-domain socket pairs** inherited at spawn are simpler and can pass descriptors (background). IOSurfaces still need Mach ports, though (§2.6).
- **The choice is open:** S4-Q3 measures both.

**Peer validation in XPC** (observed, headers):
- `xpc_connection_set_peer_code_signing_requirement` (macOS 12);
- `xpc_listener` (macOS 14);
- `xpc_peer_requirement` (macOS 26).

Ad-hoc signatures carry no Team ID, so requirements would have to pin a code hash (inference). With `posix_spawn`, the browser knows each child from the moment it launches it and doesn't need signature-based lookup.

### 2.6 Graphics across processes

All verified in the SDK headers:
- An IOSurface can be passed between processes as a Mach port: `IOSurfaceCreateMachPort` and `IOSurfaceLookupFromMachPort`.
- Metal can wrap one as a texture: `newTextureWithDescriptor:iosurface:plane:`, available since macOS 10.11.

Both are public.

**What the browsers do with the window server:**
- **Chrome** imports `CGSSetDenyWindowServerConnections` (observed), a private CoreGraphics function. **Inference:** renderers use it to cut their own window-server connection.
- **WebKit's content profile** denies the window-server lookup outright (observed).

**Inference for Axiom.** A renderer needs no window-server connection. It sends display lists (or, later, IOSurfaces) to the process that presents them.

### 2.7 Memory pressure

`DISPATCH_SOURCE_TYPE_MEMORYPRESSURE` reports normal, warning and critical levels (observed, `dispatch/source.h`). The browser can use it to discard background renderers.

## 3. Browser precedent

| | Chrome (Chromium) | Safari (WebKit) | Firefox | Ladybird |
|---|---|---|---|---|
| **Browser or UI process** | Unsandboxed; hardened runtime (observed) | App Sandbox (observed) | Unsandboxed; hardened runtime (observed) [5] | — |
| **Content process** | Helper executables started with `posix_spawn` [1]. A per-type Seatbelt profile is applied before `ChromeMain`, and the framework is `dlopen()`ed afterwards so its static initializers run sandboxed [1]. "V2" covers every process type except GPU [1] | XPC services, using the private `_MultipleInstances` key (observed). Deny-by-default profiles shipped in `WebKit.framework` (observed) | `plugin-container.app`. Seatbelt through `sandbox_init_with_parameters` (observed). Content sandbox level 3: no file writes, no home-directory reads, no network, exec or fork. The window server and microphone are still allowed, and "work is ongoing to remove" them [5] | One WebContent process per tab; one ImageDecoder process per image. `pledge()`/`unveil()` on its original OS; the document is "partly aspirational" [16] |
| **JIT entitlement** | Renderer and GPU helpers only (observed) | WebContent only; two no-JIT content variants (observed) | Main app and `plugin-container` (observed) | — |
| **Network process** | Yes (background) | Yes: Networking XPC service with its own profile (observed) | Socket process; the sources conflict on whether it is sandboxed on release builds [5] | RequestServer [16] |
| **GPU process** | Yes; still unsandboxed while it warms up [1] | Yes: GPU XPC service with its own profile (observed) | "Mac builds do not use a GPU process" [5] | — |
| **Site isolation** | Site-per-process on desktop [2] | In development: "As of January 2025 … step 2" [14]. A shared ("slush bucket") process for cross-site iframes landed in July 2025 [14] | Fission, shipped 2021 [5] | Per tab [16] |

**Inference.**
- Every engine that runs untrusted script at scale on macOS uses Seatbelt profiles, not App Sandbox entitlements, for its content processes.
- The two engines that ship their own app (Chrome and Firefox) leave the browser process unsandboxed.
- The one that uses App Sandbox for its UI process (Safari) has to launch its helpers through private XPC support.

## 4. Options for Axiom

| | Option | Strength against AD2 (threat model) | Public API? | Fits per-site processes? | Key risk |
|---|---|---|---|---|---|
| **O1** | `posix_spawn` + a custom Seatbelt profile applied by each helper at startup | **Strongest:** deny by default; per-name Mach rules; system-call allowlists; JIT denial | No; private libsandbox functions | Yes | Apple removes or restricts custom profiles. Mitigation: Chrome and Firefox depend on it, so a change would be visible in betas first (inference) |
| **O2** | `posix_spawn` + App Sandbox entitlements on each helper (no `inherit`) | **Weaker:** the base profile exposes the window server, font server, pasteboard and LaunchServices (§2.2) | Yes | Yes | Whether a non-inheriting sandboxed child of an unsandboxed parent works as expected (S4-Q6); a container per helper |
| **O3** | XPC services with their own sandbox | As O1 or O2, depending on how each service sandboxes itself | Launch: yes. Multiple instances: no | Only with the private `_MultipleInstances` key | Private key; launchd lifecycle |
| **O4** | BrowserEngineKit | Designed for this purpose | Yes | Yes | Entitlements unavailable (§2.5) |
| **O5** | Enhanced Security helper extensions | Apple's description: "restricts access to most system services and frameworks" | Yes | Unknown | Feasibility without Xcode, a paid account, or many instances (S4-Q6) |

**The browser process** (inference):
- **Sandbox it, as Safari does.** This needs XPC (O3) for every helper, because spawned children would inherit the browser's sandbox and couldn't tighten it (§2.2).
- **Leave it unsandboxed but hardened, as Chrome and Firefox do.** Then its protection rests on the Rule of 2: it handles untrusted data only in safe Rust, and never parses web content. That is ADR-0002's policy for parsers, network policy and IPC decoding.

## 5. Implications for Axiom

1. **Helpers are separate executables, one per privilege set:** renderer, a renderer with JIT (later), network, GPU (later), utility. Each is a small Rust binary inside the app bundle, signed ad hoc with the hardened runtime and only the entitlements it needs.
2. **Each helper enters its sandbox first.** It applies its profile at the top of `main`, before it creates threads, connects IPC or loads heavy frameworks. Following Chromium, the helper links little and loads the engine afterwards. Rust has few static initializers, but frameworks such as CoreText and AppKit run code when they load (background). Renderers therefore don't link AppKit at all.
3. **Profiles live in the repository,** in a `platform`-owned module. Each is short, deny-by-default and reviewed like `unsafe` code, starting from what S4 proves necessary rather than from a browser's full profile.
4. **Capabilities, not paths.** The browser opens files (for example, an upload the user chose) and passes descriptors or ports. Profiles then never need to allow user paths. `sandbox_extension_*` stays a fallback.
5. **JIT is a property of an executable.** Only the JIT renderer variant carries `allow-jit`. Every other profile also contains `(deny dynamic-code-generation)`, as WebKit does. This underpins the JIT policy in ADR-0004.
6. **Expect breakage when macOS changes.** WebKit's own profile has rules that change with OS features. Axiom should run its sandbox test suite (threat model V1) on every macOS update and beta it adopts.

## 6. Recommendation

**Adopt O1 for every helper, and keep the browser process unsandboxed under the hardened runtime.**
- Revisit the browser-process choice if O5 or a public multi-instance XPC mechanism proves workable.
- Treat O2 as the documented fallback if S4 shows that O1 no longer works for SDK 27 binaries.
- Accepting ADR-0004 should depend on S4-Q1 and S4-Q2 passing.

## 7. Spike S4 (proposed; needs the owner's authorization)

**Goal.** Validate O1 on this machine and measure what a sandboxed helper costs, before ADR-0004 is accepted.

| ID | Question | Pass criterion |
|---|---|---|
| **S4-Q1** | Can an ad-hoc-signed, hardened-runtime Rust helper, built with the Command Line Tools against SDK 27.0, enter a custom deny-by-default profile on macOS 27.2 without being killed? Tried two ways: `sandbox_init_with_parameters`, and `sandbox_compile_string` followed by `sandbox_apply` | The profile applies, and `sandbox_check` reports the helper as sandboxed |
| **S4-Q2** | Inside the profile, are these denied: reading a file in the home directory; writing anywhere; outbound TCP and UDP; `posix_spawn`; looking up the window server, font server, pasteboard and LaunchServices? Do allocation, threads, the inherited IPC channel and logging to an inherited descriptor still work? | Every probe behaves as specified, verified by automated probes in the spirit of threat-model check V1 |
| S4-Q3 | What does a helper cost? Measure spawn-to-ready latency, idle memory footprint, and round-trip latency and 1 MB transfer throughput, over Mach messages and over a Unix-domain socket pair | Reported. Feeds the memory budgets ([process model](../security/process-model.md) §7) and the IPC transport choice |
| S4-Q4 | JIT. With `allow-jit`, can a helper map one `MAP_JIT` region, write a trivial function under W^X, flush it and run it? Without the entitlement, does the mapping fail? Does `(deny dynamic-code-generation)` block it even with the entitlement? | Reported. Feeds the JIT policy |
| S4-Q5 | Can CoreText lay out and rasterize text with system fonts (S2's code path) inside a profile that denies the window server and font server lookups? Which font rules are needed? | Reported. Feeds where text rasterization may live (Step 5) |
| S4-Q6 | Fallbacks. Does a spawned, ad-hoc-signed helper with `app-sandbox` (and no `inherit`) get its own App Sandbox, and which S4-Q2 probes does App Sandbox deny? Optionally: can an Enhanced Security helper extension be built and launched without Xcode? | Reported |

**Design.**
- Two crates under `spikes/s4/`: `s4-broker`, which stands in for the browser, and `s4-helper`, a probe binary with one mode per question.
- A signing script in `tools/` applies `codesign -s - -o runtime --entitlements …` per helper variant.
- All FFI (libsandbox, Mach, `posix_spawn`) goes in the helper's `src/platform/`, allowlisted the way S2's was. No new dependencies are expected.
- Results go in `docs/research/s4-results.md`.

**If S4-Q1 or S4-Q2 fails:** ADR-0004 switches to O2, with the weaker guarantees of §2.2 stated, and the threat model records the gap.

**Size:** three checkpoints, in the style of CP8 in Step 2b:
- S4a: the probes;
- S4b: the measurements;
- S4c: the results note.

## 8. Open questions

- **Longevity of custom profiles.** Does macOS 27's tightening of named profiles extend to custom profiles in a later release? Watch macOS betas; Chrome and Firefox would be affected too.
- **Which Mach services and IOKit classes a renderer really needs** for CoreText, CoreGraphics and IOSurface allocation. S4-Q5 starts on this, and the profile grows only by evidence.
- **IPC transport** (Mach messages or socket pairs). Decided in Step 4 from S4-Q3's measurements.
- **Enhanced Security helper extensions** as a public alternative (S4-Q6).

## 9. Observations on this machine

Read on 2026-10-17. `$SDK` is `/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk`, a link to `MacOSX27.0.sdk`.

| Fact | Source |
|---|---|
| OS 27.2 (26B5112a) | `sw_vers` |
| `sandbox_init` deprecation; the header's comments | `$SDK/usr/include/sandbox.h` |
| Named profiles kill the process under SDK ≥ 27.0 | `man 3 sandbox_init` (SDK man page) |
| Inheritance; enforcement at acquisition | `man 7 sandbox` |
| `sandbox-exec` deprecated and present | `man 1 sandbox-exec`; `/usr/bin/sandbox-exec` |
| Exported sandbox functions | `$SDK/usr/lib/system/libsystem_sandbox.tbd`; `$SDK/usr/lib/libsandbox.1.tbd` |
| WebKit's XPC services, their `Info.plist` keys and entitlements | `/System/Library/Frameworks/WebKit.framework/Versions/A/XPCServices/*.xpc` with `plutil -p` and `codesign -d --entitlements -` |
| WebKit's profiles, which are self-contained (no `import`) | `…/WebKit.framework/Versions/A/Resources/com.apple.WebProcess.sb`, `com.apple.WebKit.GPUProcess.sb`, `com.apple.WebKit.NetworkProcess.sb` |
| WebKit's code-signing flags: library validation only | `codesign -dv` on each XPC service |
| The App Sandbox base profile | `/System/Library/Sandbox/Profiles/application.sb`, `appsandbox-common.sb` |
| Safari's entitlements | `codesign -d --entitlements - /Applications/Safari.app` |
| Chrome Canary 158.0.8108.0: entitlements, signing flags, imports, SDK | `codesign -d`, `codesign -dv`, `nm -u`, `otool -l` on the app, its helpers and `Google Chrome Framework` |
| Tor Browser 16.0a11: entitlements and imports | `codesign -d` on the app and `plugin-container.app`; `nm -u` on `XUL` |
| BrowserEngineKit availability | `$SDK/System/Library/Frameworks/BrowserEngineKit.framework/Headers/BEWebContentProcess.h`, `BEExtensionProcess.h`, `BECapability.h` |
| `MAP_JIT`, W^X functions | `man 2 mmap`; `man 3 pthread_jit_write_protect_np` |
| XPC peer requirements, listener | `$SDK/usr/include/xpc/connection.h`, `peer_requirement.h`, `listener.h` |
| IOSurface over Mach ports; Metal IOSurface textures | `IOSurface.framework/Headers/IOSurfaceRef.h`; `Metal.framework/Headers/MTLDevice.h` |
| `POSIX_SPAWN_CLOEXEC_DEFAULT`; special ports | `$SDK/usr/include/sys/spawn.h`, `spawn.h`; `man 3 posix_spawnattr_setspecialport_np` |
| Memory-pressure source | `$SDK/usr/include/dispatch/source.h` |
| A minimal process's footprint: 1.03 MB peak (three identical runs) | `/usr/bin/time -l /bin/sleep 0.2` |

## 10. Sources

All web sources were checked on 2026-10-17.

1. Chromium, "Mac Sandbox V2 Design Doc" (status Final; last updated 2025-01-30; authors kerrnel@ and rsesek@): removal of the unsandboxed warm-up; the helper "will immediately initialize the sandbox" before `ChromeMain()`; the framework is `dlopen()`ed afterwards; "Chromium now uses the V2 sandbox for all process types except the GPU process." https://chromium.googlesource.com/chromium/src/+/HEAD/sandbox/mac/seatbelt_sandbox_design.md
2. Chromium, "Process Model and Site Isolation". https://chromium.googlesource.com/chromium/src/+/HEAD/docs/process_model_and_site_isolation.md
3. Chromium, `sandbox/mac/seatbelt.cc` and `sandbox/mac/README.md`: wrappers for `sandbox_init_with_parameters`, `sandbox_compile_string`, `sandbox_apply` and `sandbox_check`; profiles compiled and passed to new processes over Mojo (`SeatbeltExecClient`). https://chromium.googlesource.com/chromium/src/+/HEAD/sandbox/mac/
4. Chromium, `sandbox/policy/mac/renderer.sb` and `common.sb`: deny by default; font directories readable; Mach lookups limited to a short list plus `BUNDLE_ID.MachPortRendezvousServer.BROWSER_PID`; no network beyond syslog sockets; no JIT rules. https://chromium.googlesource.com/chromium/src/+/HEAD/sandbox/policy/mac/
5. Mozilla, "Security/Sandbox" (wiki; text cites May 2020) and "Process Model" (firefox-source-docs): Mac content processes at sandbox level 3; RDD and GMP sandboxed; "Mac builds do not use a GPU process". https://wiki.mozilla.org/Security/Sandbox ; https://firefox-source-docs.mozilla.org/dom/ipc/process_model.html
6. Apple, "App Sandbox". https://developer.apple.com/documentation/security/app-sandbox
7. Apple, "Embedding a command-line tool in a sandboxed app": the helper claims `app-sandbox` and `inherit`; "Adding other entitlements to the tool can cause problems"; an ad-hoc `codesign -s - … -o runtime` example. https://developer.apple.com/documentation/xcode/embedding-a-helper-tool-in-a-sandboxed-app
8. Apple, "Hardened Runtime": protections; exception entitlements; "You add entitlements only to executables." https://developer.apple.com/documentation/security/hardened-runtime
9. Apple, "Porting just-in-time compilers to Apple silicon" and "com.apple.security.cs.allow-jit": the entitlement is required only with the hardened runtime; one `MAP_JIT` region; `jit-write-allowlist`; `sys_icache_invalidate`. https://developer.apple.com/documentation/apple-silicon/porting-just-in-time-compilers-to-apple-silicon
10. Apple, TN3125 "Inside Code Signing: Provisioning Profiles": "macOS doesn't require a provisioning profile to run third-party code"; App Sandbox and hardened-runtime entitlements are unrestricted; restricted entitlements such as `keychain-access-groups` need a profile; "A standalone executable can't claim a restricted entitlement". https://developer.apple.com/documentation/technotes/tn3125-inside-code-signing-provisioning-profiles
11. Apple, "Creating XPC services": launchd launches the service and connects client and listener; nothing about multiple instances. https://developer.apple.com/documentation/xpc/creating-xpc-services
12. Apple, "BrowserEngineKit": availability iOS 17.4+, iPadOS 18.0+, Mac Catalyst; "request the relevant entitlements for your developer account"; support "varies by geographic region". https://developer.apple.com/documentation/browserenginekit
13. Apple, "Enabling enhanced security for your app" and "Creating Enhanced Security helper extensions": hardened-process entitlements; memory tagging on "Mac … with an M5 chip or later"; helper extensions "restrict access to most system services and frameworks". https://developer.apple.com/documentation/xcode/enabling-enhanced-security-for-your-app ; https://developer.apple.com/documentation/xcode/creating-enhanced-security-helper-extensions
14. WebKit, "Site Isolation" (docs.webkit.org): a Site is "the protocol and eTLD+1"; "As of January 2025 are currently on step 2". WebKit bug 296260, a shared "slush bucket" process for cross-site iframes (committed 2025-07-22). WebKit bug 300099, "Fix JIT entitlement logic for EnhancedSecurity processes" (committed 2025-10-03). https://docs.webkit.org/Deep%20Dive/SiteIsolation.html ; https://bugs.webkit.org/show_bug.cgi?id=296260 ; https://bugs.webkit.org/show_bug.cgi?id=300099
15. Chromium, "The Rule Of 2". https://chromium.googlesource.com/chromium/src/+/HEAD/docs/security/rule-of-2.md
16. Ladybird, "Process Architecture" ("partly aspirational"). https://github.com/LadybirdBrowser/ladybird/blob/master/Documentation/ProcessArchitecture.md
