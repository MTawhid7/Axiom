# Threat Model

| | |
|---|---|
| **Status** | **Accepted** by the owner (2026-10-17), with one amendment: out-of-process iframes are a prerequisite for L3 (§8, check V9). **Amended by the architecture review's Round 1** (the owner's dispositions, 2026-10-21; [the review](../review/README.md) §5): L1 and V1 for the GPU/compositor process and renderers without IOKit (RV1-02, RV1-09; CP2); NG4 and §9 for rendering-timing side channels and passkeys (RV1-19, RV1-21; CP5) |
| **Question** | What does Axiom protect, from whom, through which entry points, and where does trust change? When may it be used for sensitive logins? |
| **Feeds** | [Process model](process-model.md), ADR-0004, the security sections of the architecture (Step 4), the testing strategy (Step 6) |
| **Builds on** | [ADR-0001](../adr/0001-scope-and-target.md) (R3 and the defaults), [ADR-0002](../adr/0002-implementation-language.md) (the `unsafe` policy), [ADR-0003](../adr/0003-gc-handle-and-rooting-pattern.md) (one heap per agent) |
| **Last updated** | 2026-10-21: Round 1's accepted findings |

Claims follow the [research conventions](../research/README.md): **sourced facts** cite a numbered source (§11), and **inference**, **hypothesis** and **background** are marked as such. Everything about Axiom itself is **planned**, not implemented.

---

## 1. Summary

- **What we protect, in order:** credentials and logged-in sessions; each site's data, including from other sites; the user's files and the rest of the Mac; the integrity of the browser's own interface.
- **Who we defend against:** malicious web content; a renderer that such content has compromised; an attacker on the network. A compromised renderer is an expected event, not an exotic one, so the design assumes it.
- **The central rule:** untrusted input is handled only where a compromise gains little. Code that parses web content runs in sandboxed, site-locked processes. The privileged browser process handles untrusted data only in safe Rust, and never parses web content itself.
- **Sensitive use is gated.** Security levels L0–L3 (§8) say what Axiom may be used for at each stage. Email, banking and accounts with write access wait for L3, whose checks V1–V9 must pass first. L3 also requires out-of-process iframes (the owner's amendment, 2026-10-17).

## 2. Scope and assumptions

**In scope:** the whole browser, as one user runs it on their own Mac (charter §3–§4): the browser process, its helper processes, its stored profile, and its network traffic.

**Assumptions,** which form the trusted computing base:

| ID | Assumption | Consequence if false |
|---|---|---|
| TA1 | The macOS kernel, the M1 hardware, and Apple's system services are trustworthy | Out of scope (NG1). The sandbox still reduces what an attacker in a renderer can reach of them |
| TA2 | The owner is the only user, and their account isn't already compromised by local malware | Out of scope (NG1): software running as the user can read the profile directly |
| TA3 | The browser process's code is trusted, but may have bugs. The design limits what untrusted input reaches it | — |
| TA4 | The toolchain and dependencies are trustworthy at the versions pinned in `Cargo.lock` | A residual risk (§9). Dependency policy (charter, principle 4) and sandboxing limit the damage |
| TA5 | Apple's libraries that parse complex formats (CoreText, ImageIO, VideoToolbox, the Metal shader compiler) are **not** trusted with untrusted input in a privileged process | They are used on web data only inside sandboxed processes (§5) |

**Why TA5.** These libraries are memory-unsafe code that parses attacker-controlled formats. Image decoders are a demonstrated attack surface: the 2023 WebP vulnerability was exploited in the wild [8]. Chromium's Rule of 2 forbids combining untrustworthy input, an unsafe implementation language and high privilege in one place [4] (sourced). Applying that rule to Apple's parsers is an **inference**.

## 3. Assets

| ID | Asset | Examples | Main threat |
|---|---|---|---|
| AS1 | **Credentials and sessions** | Cookies (session cookies above all), HTTP authentication, saved passwords if a password manager is ever added (not decided), WebAuthn credentials | Account takeover |
| AS2 | **Each site's stored data** | Cookies, `localStorage`, IndexedDB, Cache API, service workers (later), the HTTP cache, per-site permissions | Theft or tampering by another site |
| AS3 | **Cross-site data in memory** | Another site's documents, script heaps and fetched responses | Reads by a compromised renderer or by Spectre-class side channels |
| AS4 | **The user's files and the rest of the Mac** | Documents, SSH keys, other apps' data; running programs; persistence (login items, launch agents); camera, microphone, location, screen | A compromise that escapes the browser |
| AS5 | **Integrity of the browser interface** | Address bar, security indicators, permission and download prompts, file pickers | Spoofing that misleads the user about which site they are using |
| AS6 | **Network position** | Services on `localhost` and devices on the local network (router, printer, development servers) | Attacks launched from the browser into the local network |
| AS7 | **Transport security** | TLS sessions, certificate validation, HSTS state, DNS answers | Interception and tampering |
| AS8 | **Browsing history and the profile** | History, bookmarks, form data | Privacy loss |
| AS9 | **Availability** | Memory, CPU and battery (charter G4); the responsiveness of the browser | A page that makes the machine unusable |

## 4. Adversaries

| ID | Adversary | Capabilities | Goals |
|---|---|---|---|
| AD1 | **Web attacker** (malicious content) | Controls the content of one or more sites, including ads, iframes and compromised third-party scripts. Serves arbitrary HTML, CSS, script, fonts, images and media. Once Axiom runs JavaScript, it runs code **with timers**. Makes the browser send requests with the user's ambient credentials, within SameSite and CORS rules. Opens popups, navigates, frames other sites, starts downloads | Read another site's data (AS2, AS3); act as the user on another site; compromise the renderer (becoming AD2); phish the user (AS5) |
| AD2 | **Compromised renderer** | AD1 after exploiting a bug in renderer code. Candidates: `unsafe` Rust in the `gc`, `jit` or `platform` crates; a JIT logic bug; a system library (TA5). Runs arbitrary native code in that process, reads all of its memory, sends **any** IPC message in **any** order, and tries every system call, Mach service and IOKit class that the sandbox allows | Escape to the browser's privileges or to the OS (AS4); obtain other sites' credentials or data (AS1–AS3); spoof the interface (AS5) |
| AD3 | **Network attacker** | On the path: public Wi-Fi, a malicious router or ISP. Reads and modifies unencrypted traffic, spoofs DNS, attempts TLS interception and downgrade. Injecting into an `http:` page makes it AD1 for that origin. Off the path, DNS rebinding against local services | Read or alter traffic (AS7); steal session cookies (AS1); reach the local network (AS6) |
| AD4 | **A compromised helper other than a renderer** | AD2's capabilities inside the network, GPU or a utility process | Each process exposes only what it holds (SG10): traffic and cookies in the network process; rendered pixels in the GPU process |

**Why AD2 is a normal case, not an edge case.**
- Chromium's site isolation work treats compromised renderers as something that must be "explicitly included in a browser's security threat model" [2].
- Of the Chrome exploits seen in the wild from 2021 to 2023 that began with memory corruption in a renderer, 60% exploited V8 bugs [5].
- V8's engineers argue that a memory-safe implementation language does not prevent these bugs, because many are logic bugs in JIT compilers that emit unsafe machine code [5].

Each point is sourced. **Inference for Axiom:** writing the engine in Rust lowers the rate of renderer compromise, but does not remove it. The design must assume AD2.

**Not adversaries** (non-goals, §7): local malware running as the user; anyone with physical access to an unlocked Mac; kernel and firmware attacks; a malicious toolchain.

## 5. Attack surfaces

Each entry point for untrusted input, with the process that must handle it (the [process model](process-model.md) defines the processes) and the main mitigation.

| Surface | Untrusted input | Handled in | Main mitigations |
|---|---|---|---|
| HTML, CSS, URL and text-encoding parsers | Page content | Renderer | Safe Rust (ADR-0002 rule 5); fuzzing; conformance tests (html5lib-tests, WPT) |
| JavaScript engine: parser, bytecode compiler, interpreter, builtins, regular expressions, GC | Scripts | Renderer | Safe Rust; Pattern B GC with no `unsafe` (ADR-0003); test262; fuzzing |
| A later JIT and WebAssembly compiler | Scripts and modules | JIT renderer variant only | The JIT policy (ADR-0004); W^X; JIT-free mode |
| DOM bindings | Script calls | Renderer | Generated code; no `unsafe` |
| Image decoders | Image bytes | Renderer | Safe Rust decoders; any system decoder (ImageIO) only inside a sandbox |
| Web fonts: OpenType, WOFF, WOFF2 | Font bytes | Renderer (or the GPU process, if rasterization moves there) | Never parsed in the browser process (TA5) |
| Media (later) | Audio and video streams | A sandboxed media process | System codecs only behind a sandbox |
| HTTP, TLS, DNS, decompression | Network bytes | Network process | Safe Rust where we write it; a sandbox around the process |
| IPC endpoints of more privileged processes | **Every** message from a less privileged process | Browser, network and GPU processes | The IPC trust rules IR1–IR10 ([process model](process-model.md) §6) |
| GPU work from WebGL and WebGPU (later) | Shaders and commands | GPU process | Validation before the Metal compiler; never in the browser process |
| Display lists, accessibility trees and other renderer output | Renderer-generated data | Display lists: the GPU/compositor process from M2, the browser at M1 only (Round 1, RV1-02). Accessibility trees and other output: the browser | Safe Rust decoding and validation; renderers never draw outside their area |
| Layer commits from the GPU/compositor process (from M2) | Layer trees, IOSurface ports, scroll and animation nodes, hit-test regions | Browser | Bounded decoding in safe Rust; checks against the browser's own records (IR1, IR2) |
| Browser interface | Page-controlled titles, URLs, dialogs, fullscreen requests, permission requests, downloads, drag and drop, clipboard writes, external URL schemes | Browser | Spoof-resistant URL display; browser-drawn prompts; the browser's own user-activation tracking; downloads marked as quarantined (background); a prompt before launching another app |
| Local network and `localhost` | Requests from public sites | Network process | Private Network Access-style checks (Step 5) |
| Updates and dependencies | Upstream code | Build time | Pinned `Cargo.lock`; advisory monitoring (§9) |

## 6. Trust boundaries

A trust boundary is where data or control passes between parties that trust each other differently. These boundaries are logical; the [process model](process-model.md) maps them onto processes.

| ID | Boundary | What crosses it | Who decides policy |
|---|---|---|---|
| TB1 | Network ↔ browser | Every byte from the network | The network process (TLS validation, HSTS, mixed content) |
| TB2 | Web content ↔ the engine's host | Documents and scripts, which get no OS authority | The sandbox, enforced by the kernel |
| TB3 | Site ↔ site | One site's data and another site's code | The browser, using site locks; same-origin rules inside the engine |
| TB4 | Renderer ↔ browser | IPC messages | The browser, which treats each message as untrusted |
| TB5 | Browser ↔ user and OS | User intent (clicks, file choices) and OS authority (files, devices, Keychain) | The browser, acting only on trusted UI events |
| TB6 | Helpers ↔ browser | IPC from the network, GPU and utility processes | The browser; each helper is less trusted than the browser |
| TB7 | Engine ↔ system parsers | Untrusted bytes passed to CoreText, ImageIO, VideoToolbox or the Metal compiler | Placement: only sandboxed processes may make these calls (TA5) |

```
 Trust zones (higher = more trusted)

 Z0  macOS kernel, hardware, Apple system services        assumed trustworthy (TA1)
 ─────────────────────────────────────────────────────────────────────────────
 Z1  Browser process        user's authority: files, devices, Keychain, UI, policy
 ──────────── TB4 / TB6 (IPC, validated by the browser) ────────────────────────
 Z2  Network · GPU · utility processes    one OS capability each, no user files
 ─────────────────────────────────────────────────────────────────────────────
 Z3  Renderers              no OS authority; each locked to one site (TB3)
 ──────────── TB1 / TB2 ────────────────────────────────────────────────────────
 Z4  The web and the network                                       untrusted
```

## 7. Security goals and non-goals

### 7.1 Goals

Each goal names the adversary it addresses and the security level (§8) by which it must hold.

| ID | Goal | Against | Level |
|---|---|---|---|
| SG1 | **No file access from renderers.** A compromised renderer can't read or write any user file, run a program, or persist. Files the user picks for upload are read by the browser and passed as data | AD2 | L1 |
| SG2 | **Credentials stay with their site.** A renderer obtains only the cookies and storage of the site it is locked to. `HttpOnly` cookies never reach a renderer. No renderer holds the cookie jar | AD2 | L2 |
| SG3 | **No cross-site data in a renderer** unless the other site opts in (CORS). This defeats both AD2 and Spectre-class reads by AD1, because the data is never in the attacker's address space | AD1, AD2 | L2 |
| SG4 | **No direct network access from renderers.** Every request goes through the network process, which derives the initiator from the renderer's site lock rather than trusting a claimed origin. It enforces CORS, SameSite, mixed-content rules and response blocking in the style of ORB [7] | AD2 | L1 (lock checks from L2) |
| SG5 | **No direct device or interface access from renderers:** no camera, microphone, location, screen capture, clipboard reads or window-server connections. Every such use is mediated by the browser, with prompts tied to browser-tracked user activation. A renderer can't draw outside its content area | AD2 | L1 |
| SG6 | **IPC can't be used to escalate.** Privileged processes take security decisions from their own records, never from fields a less privileged process sent. A malformed message terminates its sender. Decoding is safe Rust | AD2, AD4 | L1 |
| SG7 | **Transport security.** TLS is validated against the system trust store. There is no silent downgrade, HSTS is honoured, and mixed content is blocked | AD3 | L1 |
| SG8 | **The web's own security model, implemented correctly** and tested with WPT: same-origin policy, CORS, CSP, SameSite (Lax by default), frame-ancestors and X-Frame-Options, COOP and COEP, sandboxed iframes, Referrer-Policy | AD1 | Each feature, when it ships |
| SG9 | **Renderers can't attack each other.** They can't name, signal or share memory with each other except through channels the browser brokers | AD2 | L1 |
| SG10 | **Helpers are compartments too.** A compromised network process exposes traffic and cookies, but not user files, the interface, or other secrets. A compromised GPU process exposes rendered pixels, but not cookies or files | AD4 | L2 |
| SG11 | **Containment.** A crashed, hung or memory-hungry renderer affects only its own tabs. The browser never blocks waiting for a child process. **As amended in Round 2** (RV2-13): nor does any process that serves several renderers, so one renderer can't stall another's loads or drawing ([process model](process-model.md) IR5; [IPC](../architecture/ipc.md) IP16) | AD1, AD2 | L1 |
| SG12 | **No executable memory where it isn't needed.** Only the JIT renderer variant can create executable memory, and that is enforced by the kernel through entitlements. Sensitive sites run without a JIT. A global switch turns every JIT off | AD2 | Before any JIT |

**Why SG3 needs process isolation.** Transient-execution attacks such as Spectre let script read data inside its own process without any bug in the browser. Site Isolation therefore keeps cross-site data out of an attacker's process instead of relying on software checks [2][3]. Chrome re-enabled `SharedArrayBuffer` and precise timers only once Site Isolation was on [3].

The web platform encodes the same rule:
- `SharedArrayBuffer` can be shared only in **cross-origin isolated** contexts. These are set up with COOP and COEP headers [6].
- Timers are coarsened to 100 µs, or to 5 µs in cross-origin isolated contexts [6].

### 7.2 Non-goals

| ID | Non-goal | Why |
|---|---|---|
| NG1 | Protection against malware already running as the user, or against a root or kernel compromise | Such software can read the profile and the screen directly (TA1, TA2) |
| NG2 | Physical attacks, such as an unlocked or stolen machine | FileVault and the OS cover these |
| NG3 | Anonymity and strong resistance to fingerprinting | A Tor-class design is a different product. Storage partitioning against cross-site tracking is in scope, as a standards decision |
| NG4 | Side channels other than cross-site transient execution: power, acoustic, or cross-process microarchitectural attacks | They need OS and hardware mitigations [3]. **As amended in Round 1** (RV1-19): rendering-timing attacks across frames, such as pixel stealing, are *not* excluded; §9 and the rendering pipeline's PL9 cover them |
| NG5 | Reputation services such as Safe Browsing | They need a cloud service, which ADR-0001 rules out. Phishing defence rests on a clear interface and on binding credentials to sites |
| NG6 | Denial-of-service resistance beyond containment | A page can always use CPU in its own process. Axiom contains it (SG11) and throttles background work |
| NG7 | Extensions, sync and multiple profiles | ADR-0001 |
| NG8 | Defending against the owner's own deliberate changes, such as disabling the sandbox in a debug build | Single trusted user (TA2) |

## 8. Security levels and the sensitive-use gate

ADR-0001 gates sensitive use "until the isolation milestone is verified". The levels below define that milestone. Step 6 assigns each level to a milestone.

| Level | Name | What must be true | Permitted use |
|---|---|---|---|
| **L0** | Prototype | Anything goes, including a single process and no sandbox | Local test pages and public documents. **Never credentials** |
| **L1** | Sandboxed | Web content is parsed and run only in renderer processes under a deny-by-default sandbox. A separate, sandboxed network process. The browser process parses no web content and, as amended in Round 1 (RV1-02), consumes no renderer display lists: the GPU/compositor process does. IPC follows IR1–IR10. SG1, SG4–SG7, SG9 and SG11 hold | General browsing **without logging in** |
| **L2** | Site-isolated | Renderers are locked to one site. Credentials and storage are mediated with lock checks. ORB-style response blocking is on. Third-party state is partitioned by top-level site. SG2, SG3 and SG10 hold | Logging in to accounts that are **not** sensitive |
| **L3** | Sensitive-use gate passed | **Out-of-process iframes:** every cross-site frame renders in a process locked to its own site (the owner's amendment, 2026-10-17). Checks V1–V9 pass, and their evidence is recorded | Email, banking, and accounts with write access |

**The gate's checks** (evidence for L3):

| # | Check |
|---|---|
| V1 | **Sandbox escape tests.** From inside each process type, the GPU/compositor process included, an automated suite tries the operations that type must not perform: reading user files, opening sockets, spawning programs, looking up Mach services outside its allowlist, and, from a renderer, opening IOKit user clients (Round 1, RV1-09). Every attempt is denied |
| V2 | **IPC fuzzing.** Every message decoder of every privileged endpoint is fuzzed, with coverage reported, and the campaign ends without open findings |
| V3 | **Lock enforcement tests.** A test renderer that lies about its origin is caught and terminated, for cookies, storage, navigation commits and requests |
| V4 | **Cross-site leak tests.** The relevant WPT suites (CORS, COOP and COEP, ORB, cookies) pass. A test confirms that cross-site no-CORS responses carrying HTML, JSON or XML never reach the renderer's memory |
| V5 | **An audit of the `unsafe` code** in `gc`, `jit` and `platform`, done after the last change to it |
| V6 | **A dependency audit:** no open advisories |
| V7 | **The JIT policy holds:** sensitive sites run without a JIT, and the global switch works |
| V8 | **A written threat-model review.** This document is re-read against the code as built, and every deviation is recorded |
| V9 | **Out-of-process iframe tests** (added with the owner's amendment, 2026-10-17). In tests, every cross-site frame runs in a renderer locked to its own site. A renderer that requests another site's frame, or that site's data, is terminated |

**Inference.** L3 is the bar at which a single developer can reasonably trust Axiom with high-value accounts. It is lower than a commercial browser's bar, because it lacks external audits and a vulnerability reward programme. The owner may set a higher bar.

## 9. Residual risks

| Risk | Note |
|---|---|
| **The sandbox relies on private macOS interfaces** | The deny-by-default Seatbelt profiles that Chromium, Firefox and WebKit use are applied through undocumented libsandbox functions ([macOS confinement note](../research/macos-process-confinement.md)). Apple could change them |
| **No automatic security updates** | Axiom is built locally. Fixes in dependencies arrive only when the owner rebuilds. A routine of advisory checks and rebuilds is needed (Step 6) |
| **System parsers inside renderers** | A bug in CoreText or ImageIO compromises a renderer (AD2). A sandbox escape still needs a second bug |
| **Spectre inside one site's process** | Accepted. Same-site data may share a process |
| **Cross-site iframes before out-of-process iframes exist** | At L1 and non-sensitive L2 only, a cross-site iframe may share its embedder's process. The [process model](process-model.md) §5 limits the exposure: partitioned third-party state, ORB, and no sensitive site rendered inside another site's process. L3 requires out-of-process iframes |
| **WebAuthn and passkeys** | **Hypothesis:** the platform authenticator needs an Apple entitlement that ad-hoc signing can't claim. Chrome's main executable carries `com.apple.developer.web-browser.public-key-credential` (observed with `codesign -d` on this machine). New open question OQ-16. **Round 1** (RV1-21): roaming keys over USB HID and the hybrid transport (a phone as the authenticator, as Chromium implements it) need no restricted entitlement, to be checked before M6 |
| **Rendering-timing side channels across frames** (Round 1, RV1-19) | GPU.zip (IEEE S&P 2024) stole pixels from cross-origin frames through the timing of filters drawn on the GPU; it needed the frame to load with cookies, SVG filters on frames, and GPU rendering. Axiom blocks third-party cookies (NW11), but its compositor applies a parent's effect nodes to child frames (PL6, PL9), and same-site cross-origin frames, or a Storage Access grant, still carry credentials. **Mitigation, from M5:** PL9 doesn't apply effects whose cost depends on pixel values to a cross-origin child frame's output, or applies them only in forms whose cost doesn't; V4 tests it ([RV1-19](../review/round-1/02-findings.md#rv1-19-rendering-timing-side-channels)) |

## 10. Open questions raised

- **OQ-16.** How can Axiom support WebAuthn and passkeys without a paid developer account? Candidates are roaming security keys over USB HID, or obtaining the entitlement; **Round 1** (RV1-21) added the hybrid transport (§9). Needed before L3, because high-value accounts increasingly use passkeys.
- Inputs to existing questions: OQ-4 (JIT policy), via SG12 and V7; OQ-5 (per-agent heaps), because an agent cluster must live in one process [6].

## 11. Sources

All web sources were checked on 2026-10-17.

1. [ADR-0001](../adr/0001-scope-and-target.md): R3 and the defaults.
2. Chromium, "Site Isolation" (chromium.org): compromised renderers "need to be explicitly included in a browser's security threat model"; Spectre reads renderer memory "even without bugs in Chrome"; CORB is "a best effort approach". https://www.chromium.org/Home/chromium-security/site-isolation/
3. C. Reis, A. Moshchuk, N. Oskov, "Site Isolation: Process Separation for Web Sites within the Browser", USENIX Security 2019, §5.2–5.3: same-process transient-execution attacks; Chrome re-enabled `SharedArrayBuffer` and precise timers with Site Isolation; a 9–13% total memory overhead. https://www.usenix.org/conference/usenixsecurity19/presentation/reis
4. Chromium, "The Rule Of 2": "Pick no more than 2 of: untrustworthy inputs; unsafe implementation language; and high privilege." https://chromium.googlesource.com/chromium/src/+/HEAD/docs/security/rule-of-2.md
5. V8, "The V8 Sandbox" (2024-04-04): of the in-the-wild Chrome exploits from 2021 to 2023 that began with renderer memory corruption, 60% were V8 vulnerabilities; many V8 bugs are logic bugs, including JIT bugs, that a memory-safe language would not prevent. https://v8.dev/blog/sandbox
6. WHATWG HTML: COOP, COEP and cross-origin isolation (§7.1.3–7.1.4); `SharedArrayBuffer` serialization requires the cross-origin isolated capability (§2.7.4); agent clusters (§8.1.2). W3C High Resolution Time (Editor's Draft, 2026-09-01) §4: 100 µs coarsening, or 5 µs when cross-origin isolated. https://html.spec.whatwg.org/multipage/ ; https://w3c.github.io/hr-time/
7. A. van Kesteren, "Opaque Response Blocking (ORB, aka CORB++)", being upstreamed to the Fetch Standard (PR #1442). https://github.com/annevk/orb
8. Isosceles, "The WebP 0day" (2023). https://blog.isosceles.com/the-webp-0day/
