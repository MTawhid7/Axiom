# The Roadmap

| | |
|---|---|
| **Status** | **Binding** as the baseline under review, since [ADR-0020](adr/0020-roadmap-and-security-levels.md) was accepted (2026-10-20, Step 6's Gate C). Written at Step 6, CP8. No milestone starts before the independent architecture review on branch `architecture-review`; a change it brings returns to the owner under MS12. **Amended by the review's Round 1** (the owner's dispositions, 2026-10-21; [the review](review/README.md) §5): §4.1 for RV1-01 and RV1-15 (CP1); MS3–MS5, §4.1, §7 and §10 for the GPU/compositor process at M2 and real processes from M1 (RV1-02, RV1-07; CP2); MS5 and §4.1's M1, M3 and M5 for cross-site agents from M1 (RV1-03; CP3); MS5, §4.1, §5 and §7 for corpus capabilities, WebAssembly's and the JIT's timing, site compatibility, the Low findings and JPEG (RV1-12–RV1-14, RV1-19–RV1-22; CP5) |
| **Question** | Which milestones lead from a static-document engine at L0 to the sensitive-use gate at L3 and the app web? What does each build, at which security level, against which corpus pages? Where does each decision point fall, and how does an implementation milestone run? |
| **Answers** | Step 6 question Q5 ([tracker](research/step-6-standards-budgets-roadmap.md) §3); the tracker's handed-over questions 2 and 25–28 (§2.2); the [corpus](corpus.md)'s open question 3 |
| **Feeds** | Milestone 1's definition (CP9); requirements traceability (CP10); the [overview](architecture/overview.md) v2's staging (CP10) |
| **Builds on** | The [overview](architecture/overview.md) §6 (L0–L3); the [threat model](security/threat-model.md) §8 (levels and V1–V9); every design's staging section; the [corpus](corpus.md) (CO1–CO14); the [standards baseline](standards.md) (SB1–SB12); the [testing strategy](testing.md) (TS1–TS18); the [performance budgets](performance.md) (PF1–PF24); the [dependency policy](technology/dependency-policy.md); the [open questions](open-questions.md) |
| **Last updated** | 2026-10-21: Round 1's accepted findings |

Rules are numbered MS1–MS12. **The order is inference** from the accepted designs and the corpus's needs, unless a rule cites a decision. Precedent is sourced (§2). Everything here is **planned**; nothing is implemented.

## Summary

- **Nine milestones, ordered and gated, never dated** (MS1, MS5):

  | # | Milestone | Level | First corpus targets |
  |---|---|---|---|
  | M1 | Static documents | L0 | Wikipedia, MDN: readable, without script |
  | M2 | The sandboxed engine | L1 | The same, sandboxed |
  | M3 | Script | L1 | MDN in full |
  | M4 | The modern document | L1 | Wikipedia in full; Google Search, Instagram, Reddit, logged out |
  | M5 | Site isolation and logins | L2 | The content sites, signed in with test accounts |
  | M6 | The sensitive-use gate | L3 | The content sites with the owner's accounts |
  | M7 | Media | L3 | YouTube, Twitch |
  | M8 | Applications | L3 | VS Code for the Web, Discord, Prism |
  | M9 | The GPU and WebAssembly | L3 | Google Maps, Three.js, Figma |

- **Security climbs before the attack surface widens** (MS2): the sandbox at M2, site locks at M5, out-of-process iframes and V1–V9 at M6. Media codecs, the GPU process and WebAssembly arrive after L3, and each of those milestones keeps L3's checks green for what it adds.
- **No script from the web before L1** (MS3), so the first engine that runs web script runs it sandboxed.
- **Every interface exists from M1** (MS4): L0 is one process with every channel, record and replica, so L1–L3 change process selection, not interfaces (AC1).
- **Every milestone's criteria take one shape** (MS7): conformance targets, corpus checks, budgets, the level's security checks, fuzzing of what it adds, and no regressions.
- **The login track** (MS8): cookies and storage at L1, without logins; test accounts at L2 (M5); the owner's own accounts at L3 (M6). The three sites M5 targets set no limit on a person's accounts, but require accurate details and forbid automated creation (§6).
- **Every decision point has a milestone** (MS9): WebAssembly before M9; a JIT not before L2, on the applications' evidence; passkeys before M6; and the deferred decisions of Step 5.
- **Each milestone runs like a Phase Zero step** (MS10): the owner's authorization, its own branch, a plan with gates, checkpoints, the checks, and dependencies adopted only after grading, with the owner's approval.

## 1. Binding inputs

| Input | Consequence here |
|---|---|
| **ADR-0001:** milestones begin with static documents; the full modern app web stays reachable; sensitive logins wait for the isolation milestone; budgets on the target M1 | M1 is static documents (MS5); the app web is M8–M9; L3 is M6 |
| **The threat model §8:** L0 never handles credentials; L1 is browsing without logging in; L2 is logging in to accounts that aren't sensitive; L3 needs out-of-process iframes and V1–V9. "Step 6 assigns each level to a milestone" | MS2, MS5, MS8 |
| **OQ-18** (the owner, 2026-10-17): out-of-process iframes are mandatory before L3; SI3 only at L1 and non-sensitive L2. "The milestone is set in the Step 6 roadmap" | M6 (MS5) |
| **OQ-1:** no deadline and no hourly budget; work is paced by checkpoints | MS1, MS11 |
| **The account policy** (Gate A, decision 3): "sensitive personal accounts will wait for Level 3 (L3) verification, while non-sensitive test accounts may be used starting at Level 2 (L2)" | MS8 |
| **The corpus** (CO1, CO2, CO8): a page becomes a target at the first milestone with every capability it needs, at a level that permits its use; CP8 checks each L2 site's rules on additional accounts | MS6, §6 |
| **The overview §6:** what each level adds; every design's staging section: what each subsystem builds first, next and later | MS4, MS5 |
| **J1** ([ADR-0004](adr/0004-process-model-and-sandbox.md) D7): no JIT before L2, and only on evidence | MS9 |
| **The standards, testing and performance documents:** targets (SB10–SB12), the test layers (TS1–TS18), budgets as gate benchmarks (PF5, TS13) | MS7 |
| **Charter principle 7:** stage the implementation, never the architecture | MS4, MS12 |

## 2. Precedent

- **Chromium enabled Site Isolation by default "for all sites in Chrome 67 on Windows, Mac, Linux, and Chrome OS"** (2018, from its linked post), and calls it "a major architecture change for Chrome", with "increased memory overhead" [C1]. The USENIX paper measured that overhead at 9–13% of total memory ([threat model](security/threat-model.md) source 3).
- **Firefox's Site Isolation, Project Fission, "has been a massive multi-year project"**; in May 2021 it was still in testing on Nightly and Beta [F1].
- **Ladybird, a new engine, targets an alpha "in 2026" on Linux and macOS** [L1]. Its 2026 newsletters report separate processes added before that alpha: "The compositor also runs in its own process now" (May), and "Helper processes now run inside real sandboxes on Linux and macOS, using seccomp/Landlock on Linux and Seatbelt profiles on macOS" (June) [L2], [L3].
- **What that teaches** (inference): splitting an engine into processes and isolating sites took the established engines years of retrofitting, because their interfaces assumed one process. Axiom's interfaces carry processes and frames from the first line (AC1), so L1 to L3 change process selection, not code paths. And a new engine can put sandboxing early, as Ladybird did before its alpha.

## 3. Principles of the order

| # | Rule |
|---|---|
| MS1 | **Ordered and gated, never dated.** Milestones run in the order of §4, each after the one before has passed its gate. None has a date or an estimate (OQ-1). Each is planned in checkpoints when it starts (MS10) |
| MS2 | **Each milestone has one security level, and is used only as that level permits** (threat model §8): L0, local test pages and public documents, never credentials; L1, browsing without logging in; L2, logins to accounts that aren't sensitive; L3, the owner's own accounts. **A milestone after a level's gate keeps that level:** before its gate it runs the level's checks again for everything it adds or changes, as V5 already requires of the `unsafe` audit |
| MS3 | **No script from the web before L1.** Until renderers' sandboxes are verified (M2: V1's escape suite and V2's IPC fuzzing, L1's parts), Axiom runs no script from web content; at L0 it parses and renders documents without running their script. **Why:** at L0 nothing has yet shown that the renderer's sandbox holds, so a bug in the engine that runs web script might run with the owner's own rights. **As amended in Round 1** (RV1-07), M1's renderers are already sandboxed processes; L0 still runs no web script. Parsing HTML, CSS and images in safe Rust (rule 5) is the smaller risk L0 accepts. Script on local test content (test262, WPT) is allowed from M3 on, which is already L1 |
| MS4 | **Every interface from the first milestone.** **As amended in Round 1** (RV1-07; the owner: "Run real, sandboxed renderer and network processes from Milestone 1 using the profiles validated in Spikes S4, S5, and S7." "Retain single-process mode strictly for rapid automated testing, fuzzing, and Miri."): M1 runs the browser with real, sandboxed renderer and network processes, under the profiles S4, S5 and S7 wrote, over Mach channels; every thread role, channel, record and replica exists (overview §6, AC1). The compositor runs as a browser thread behind its interface until M2 moves it into the GPU/compositor process (RV1-02). Single-process mode is kept strictly for automated tests, fuzzing and Miri. A later milestone moves work across processes without changing an interface; one that would need to change one raises an amendment for the owner (RC6, MS12) |

## 4. The milestones

**MS5. The milestones, in order:**

| # | Milestone | Level | Builds (from the designs' staging sections) | Corpus targets (MS6) | Decision points (MS9) |
|---|---|---|---|---|---|
| **M1** | **Static documents** | L0 | The browser with sandboxed renderer and network processes (RV1-07), the shell and the headless embedder; HTML and the DOM without script; style; CSS 2.1's visual formatting model, tables included; text with system fonts; images; paint and the compositor; URLs, HTTP/1.1 and HTTP/2 over rustls | Wikipedia and MDN, readable, from local copies and live | Xcode (OQ-7); `vello_cpu`'s and `smallvec`'s grades; surfaces allocated ahead; AWS-LC's jitter entropy; objc2's classes (EM11) |
| **M2** | **The sandboxed engine** | L1 | **The GPU/compositor process** (RV1-02); audit tokens; V1's and V2's L1 parts; the browser's authority over frames, navigation and history; crashes contained; the disk cache, the cookie jar and CORS | The same pages, sandboxed | QoS overrides (OQ-20) |
| **M3** | **Script** | L1 | The JavaScript engine and the bindings' first stage; the event loop's tasks, microtasks and timers; script elements and modules; `fetch`; Web Storage and `document.cookie`; canvas 2D | MDN in full: its live examples, in cross-site frames | The storage engine (SW2); **WebAssembly's architecture** (OQ-3), with S14 (RV1-13) |
| **M4** | **The modern document** | L1 | Flexbox, grid and the rest of layout's second stage; SVG; style's second stage (scoping, `:has()`, container queries, animations); Shadow DOM and custom elements; web fonts; the accessibility tree; HTTP/3; **WebSockets**, and dedicated workers if the census addendum shows the content pages use them (RV1-12) | Wikipedia in full; Google Search, Instagram (without video), Reddit (without its interactive posts in frames), logged out | GR5's amendment (OQ-24) |
| **M5** | **Site isolation and logins** | L2 | Site locks in process selection and endpoints (cross-site frames are their own agents from M1: RV1-03); ORB, COEP and CORP; IndexedDB, the Cache API and quotas; the login track's L2 steps; V3 and V4; **downloads, find in page and zoom** (RV1-22) | Google Search, Instagram, Reddit, signed in with test accounts | DNS over HTTPS with Encrypted Client Hello (RV1-20); **a JIT (J1), decided at its gate** on M3's and M4's measurements (RV1-13) |
| **M6** | **The sensitive-use gate** | L3 | Out-of-process iframes: every cross-site frame in a renderer locked to its site; V1–V9 complete, with their evidence recorded; the login track's L3 steps; **session restore** (RV1-22) | The content sites with the owner's own accounts | Passkeys (OQ-16), before it starts, with the hybrid transport among the candidates (RV1-21); password saving |
| **M7** | **Media** | L3 | The media utility process: demuxers, VideoToolbox and AudioToolbox, video layers, audio, Media Source Extensions; AVIF through ImageIO in it (IM3) | YouTube, Twitch | Demuxers; media instances per site |
| **M8** | **Applications** | L3 | Dedicated, shared and service workers; `SharedArrayBuffer` and `Atomics` with cross-origin isolation; the engine's second stage (Temporal, off-thread compilation); EditContext | VS Code for the Web, Discord, Prism | WebAssembly's implementation (OQ-3), for M9 |
| **M9** | **The GPU and WebAssembly** | L3 | WebGL and WebGPU, in a process M9's plan places, never the browser (the GPU/compositor process exists since M2); WebAssembly, as OQ-3 decides | Google Maps, Three.js, Figma | — |

### 4.1 What each milestone contains

**M1, static documents (L0).** CP9 defines it precisely, in [Milestone 1](milestone-1.md). In outline:
- **The browser with sandboxed renderer and network processes** (MS4; Round 1, RV1-07), under S4's, S5's and S7's profiles, with the compositor as a browser thread behind its interface; the AppKit shell (window, address field, back and forward) and the objc2 classes for text input and accessibility validated (EM11); the headless embedder with its script-free reference-test path and its text dumps of the DOM, styles, boxes and fragments, the display list and the network log (EM9, TS2; Round 1's RV1-01 and RV1-15).
- **Cross-site frames are separate agents from M1,** each with its own event loop and heap, behind remote window proxies once script exists (FT10; Round 1, RV1-03).
- **HTML and the DOM:** the first document milestone's row ([HTML and the DOM](architecture/html-and-dom.md) §8), without script: the parser with encoding sniffing and fragments; nodes, attributes and the mutation algorithms with their hooks; event dispatch for routed input; ranges and selection; forms submitted as navigations.
- **Style:** the first document milestone's row ([style](architecture/style.md) §8).
- **Layout and paint: CSS 2.1's visual formatting model, without paged media.** That is the first document milestone's row ([layout](architecture/layout-and-paint.md) §8), **plus tables** (approximate, LY10), lists and generated content, every CSS 2.1 border style, and outlines. Tables are brought forward from the next stage because both documents lay out tables, such as Wikipedia's infoboxes (inference); the rest completes CSS 2.1, whose reference tests are M1's CSS target ([Milestone 1](milestone-1.md) MA4).
- **Text:** the first text milestone's row: system fonts through CoreText, HarfRust, ICU4X, the atlas, the encodings ([text](architecture/text-and-fonts.md) §8).
- **Images:** PNG, JPEG, GIF and WebP ([images](architecture/images-and-media.md) §8). **Graphics:** the first rendering milestone's row ([graphics](architecture/graphics.md) §8).
- **Networking:** URLs, `file:`, HTTP/1.1 and HTTP/2 over rustls with the system verifying certificates (S7), DNS-SD, an in-memory cache, mixed content, HSTS and Referrer-Policy. No cookies and no disk cache until M2.
- **Tests without script.** WPT's testharness tests are themselves script, so M1 runs WPT's reference and crash tests only, through a script-free executor, one process per test (TS2), run end to end at M1's second checkpoint. Its `css/CSS2` target is interim, over the 5,652 tests in scope that need no script (Round 1, RV1-01). html5lib's tokenizer tests, and the 1,973 tree-construction tests WPT keeps as `.dat` files beside their script wrappers, run in Axiom's own Rust runners (TS4); Unicode's conformance files run through ICU4X (TS6). The testharness directories become targets from M3.
- **Not in M1:** script (MS3), cookies, storage, web fonts, SVG, flexbox and grid, the GPU/compositor process (M2).

**M2, the sandboxed engine (L1).** **As amended in Round 1:** renderers and the network process are already processes under browser-compiled profiles from M1 (RV1-07). M2 hardens them, and builds the GPU/compositor process (RV1-02; the owner: "Formally design the sandboxed GPU/compositor process topology now (Renderer → GPU Process → Browser CALayer tree)." "Build the separate GPU/compositor process at Milestone 2; split Budget B1 accordingly." "Ensure Level 3 (L3) security certification at Milestone 6 is performed on the final multi-process topology."):
- **the GPU/compositor process:** validation, property trees and tile rasterization move out of the browser, which keeps the layer tree, scrolling, animation and input (process model §3, note 1), with S10's measurements behind it;
- Mach channels pin audit tokens; IP's validators and violation handling are complete;
- the browser's frame tree, navigation and session history are the authority (FT);
- crashes are contained (overview scenario 5);
- the spare renderer, B5's limit and discarding under memory pressure;
- the network's first stage completes: the partitioned disk cache and cookie jar, CORS;
- **every IPC decoder is fuzzed with coverage,** and V1's escape suite runs in each process type: L1's part of V1 and V2;
- **crash reports:** a crashed helper's report is moved into a local directory the owner controls, with the build identifier (EM9; Round 1, RV1-15).

**M3, script (L1).** The first script milestone's rows of the [JavaScript engine](architecture/javascript-engine.md) §8 and the [bindings](architecture/bindings.md) §8. With them:
- the event loop's task sources, microtasks and timers;
- DOM APIs and listeners from script; `document.write`; script elements, modules and import maps;
- `fetch` and `XMLHttpRequest`;
- `localStorage`, `sessionStorage` and `document.cookie`: the first storage milestone ([storage](architecture/storage-and-workers.md) §8), whose crash tests decide SW2;
- canvas 2D in the renderer;
- test262 (TS5). The interpreter's measurements on corpus scripts start the evidence J1 asks for;
- **remote window proxies** (WB20) in the bindings' first stage, with a test from M3 on: script in a cross-site frame that reaches for its parent's DOM fails, in every process configuration, V9's check applied from the first script milestone (RV1-03);
- **the test-only WebDriver server,** which runs WPT's testharness tests and the `css/CSS2` tests with script, for **the full `css/CSS2` target** over all 5,848 tests in scope (TS2; Round 1, RV1-01);
- **console messages** to the embedder's log and the test runners (JE27; RV1-15).

**M4, the modern document (L1).** The second stage of style, layout and the DOM: flexbox and grid, multi-column, sticky positioning, vertical writing modes, containment, the observers, SVG, clip paths, masks and filters, form controls and scrollbars; Shadow DOM's scoping, `:has()`, container queries, CSS animations and transitions; Shadow DOM and custom elements, the preload scanner, the sanitizer and the XML parser. Then:
- web fonts through the Rust path (TX7);
- `Intl` on ICU4X and BigInt;
- HTTP/3 over `quinn-proto`;
- **WebSockets** over HTTP/1.1 and HTTP/2, moved from M8 (Round 1, RV1-12): small next to M4's scope, and the corpus's chat pages (Twitch, Discord) depend on them; **dedicated workers** too, if the census addendum's use counters show the content pages use them;
- the accessibility tree and cache (AX).

**M5, site isolation and logins (L2).** Site locks join process selection and every endpoint (IP10; [networking](architecture/networking.md) §8's L2 row). Cross-site frames remain their own agents and event loops in their embedder's process (SI3, non-sensitive sites only), as they have been since M1, reached through remote window proxies (WB20; Round 1, RV1-03). Then:
- ORB, Local Network Access, COEP and CORP in full;
- IndexedDB, the Cache API, quotas and eviction (PF24), behind lock checks;
- the login track's L2 steps (§6);
- V3's lock-enforcement tests and V4's cross-site leak suites, with a pixel-stealing test (RV1-19);
- **the daily features the owner needs before L3** (Round 1, RV1-22): downloads, which the browser writes and quarantines (process model §3), PDFs among them, opened in Preview; find in page; zoom;
- **Encrypted Client Hello with DNS over HTTPS,** planned at M5's plan (RV1-20).

**M6, the sensitive-use gate (L3).** Process selection puts every cross-site frame in a renderer locked to its own site ([frame tree](architecture/frame-tree-and-navigation.md) §5's L3 row; IPC's frame-host channels). **V1–V9 pass, with their evidence recorded** (TS14):
- the escape suites (V1);
- IPC fuzzing (V2), lock enforcement (V3), cross-site leaks (V4);
- the `unsafe` audit (V5) and the dependency audit (V6);
- the JIT policy (V7): it holds trivially while no JIT exists, and its switch is tested;
- the threat-model review (V8) and the out-of-process iframe tests (V9).

The owner's own accounts arrive (§6), with **session restore** (RV1-22).

**M7, media (L3).** The media milestone's row ([images and media](architecture/images-and-media.md) §8): the media utility process with demuxers, VideoToolbox and AudioToolbox, video layers, audio output and Media Source Extensions; and AVIF through ImageIO there (IM3). It keeps L3: V1, V2 and V5 cover the new process, its endpoints and its `platform` code before the gate (MS2).

**M8, applications (L3).** Dedicated, shared and service workers (EL §4.6; SW), `SharedArrayBuffer` and `Atomics` under cross-origin isolation, Web Locks and the origin private file system, EditContext, Temporal and off-thread compilation, the bytecode cache. **VS Code for the Web uses WebAssembly** (S8); if its corpus interaction needs it, its target moves to M9, decided at M8's plan.

**M9, the GPU and WebAssembly (L3).** WebGL and WebGPU ([graphics](architecture/graphics.md) §8), in a process M9's plan places: the compositor's own, or, as recommended, one of their own, since the compositor's process holds every site's pixels (GR10). The GPU/compositor process exists since M2 (Round 1, RV1-02). WebAssembly, as OQ-3 decides, through JE31's interface. It keeps L3 for WebGL's and WebGPU's new surface (MS2).

**After M9:** the designs' remaining "later" rows (printing, anchor positioning, view transitions, scroll-driven animations, vertical text, colour fonts, JPEG XL, WebTransport and others), each in a milestone planned when it starts, ordered by the corpus, then Baseline, then Interop (SB6).

## 5. Corpus targets

**MS6. A page becomes an acceptance target** at the first milestone that has every capability it needs, at a level that permits its use (CO2). From then on its checks run at every gate: renders and is readable, links navigate, no crashes or hangs (charter §8; TS12), and its ceiling (performance §4.1). Pages measured by hand stay by hand (CO6).

| Page | Tier | Target from | Level, and how it's used |
|---|---|---|---|
| Wikipedia | Documents | M1, readable, with formulas as their alternative text; **in full at M4** (formulas as SVG images) | L0, logged out |
| MDN | Documents | M1, readable, without its live examples; **in full at M3** | L0, logged out |
| Google Search | Content | M4 | L1 logged out; M5 test account (L2); M6 the owner's (L3) |
| Instagram | Content | M4, the logged-out view, **without its feed's video until M7** (RV1-12) | As Google Search |
| Reddit | Content | M4, **without its interactive posts in frames,** which may need WebGL or WebAssembly (M9; inference, RV1-12) | As Google Search |
| YouTube | Media | M7 | L3: logged out, and the owner's account |
| Twitch | Media | M7, its chat over WebSockets from M4 (RV1-12) | As YouTube |
| VS Code for the Web | Applications | M8, or M9 if its interaction needs WebAssembly | L3, logged out |
| Discord | Applications | M8 | L3, the owner's account |
| Prism | Applications | M8 | L3, the owner's account |
| Google Maps | Media and maps | M9 (WebGL; S8 saw it use WebAssembly) | L3, logged out |
| Three.js | Applications | M9 (WebGPU) | L3 |
| Figma | Applications | M9 (WebAssembly; WebGL or WebGPU) | L3, the owner's account |

**Re-placed in Round 1** (RV1-12): MS6 required every capability a target needs, and three targets came before theirs: Twitch's chat needed WebSockets (M8) at M7, Instagram's feed video (M7) at M4, and Reddit's interactive posts (M9) at M4. WebSockets move to M4, and the targets name what they exclude. **The census addendum by hand,** authorized before M4, saves Chrome's use counters for the eight pages measured by hand ([corpus](corpus.md) §10); the targets are placed again on its evidence, by MS6, before M4's plan.

## 6. The login track

**MS8. What each milestone adds to logging in:**

| Milestone | Level | What logging in gains |
|---|---|---|
| M2–M3 | L1 | The partitioned cookie jar (M2), then `document.cookie` and Web Storage (M3). **No credentials:** L1 is browsing without logging in |
| M4 | L1 | Form controls rendered, password fields among them (HD27); no autofill |
| **M5** | **L2** | **Test accounts:** password fields work; the cookie jar and storage sit behind lock checks; sign-in redirects and pop-ups follow FT's opener rules; the sites' own bot checks are rendered and passed by hand, never worked around (CO4) |
| **M6** | **L3** | **The owner's own accounts.** Passkeys as OQ-16 decides; whether the browser saves passwords, in the Keychain the process model allows, decided at M6's plan |

**Test accounts at L2** (the corpus's open question 3). M5 targets Google Search, Instagram and Reddit; their terms, read on 2026-10-20, set no limit on how many accounts a person holds:
- **Google** [T1]: no clause on additional accounts. It forbids "creating fake accounts or content" among fraudulent or deceptive uses, and "hiding or misrepresenting who you are in order to violate these terms".
- **Instagram** [T2]: no clause on additional accounts. "You don't have to disclose your identity on Instagram", but "you must provide us with accurate and up-to-date information (including registration information)", "you may not impersonate someone or something you aren't", and it forbids "creating accounts ... in an automated way without our express permission".
- **Reddit** [T3]: no clause on additional accounts. "You will not license, sell, or transfer your Account without our prior written approval"; the Reddit Rules ask users to "Participate authentically", without "content manipulation".

**So each test account is** (CO8 with these terms; the reading is inference, by a non-lawyer):
- created by hand by the owner, in the owner's own name, with accurate details;
- used only to sign in and read, never to post, vote or manipulate;
- holding nothing of value and linked to nothing of the owner's (CO8).

The secondary accounts the owner used for S8's session by hand may serve, if they meet CO8. That is the owner's choice at M5's plan.

## 7. Decision points and deferred decisions

**MS9. Each decision is taken at its milestone, by the owner, on its evidence.** A decision that changes an accepted ADR is an amendment (MS12).

| Decision | Where it stands | Taken at | Evidence it needs |
|---|---|---|---|
| **WebAssembly** (OQ-3): our own engine or an embedded one | Open; JE31 lists what Axiom's engine must offer | **Its architecture before M3** (Round 1, RV1-13): own or embedded, WasmGC's objects as arena kinds, Wasm frames in the register file, tiering under J6; its implementation for M9 | Spike S14 (authorized before M3): an interpreter against Cranelift here, and WasmGC's support; JE31's interface; the corpus's use (S8: Google Maps and VS Code use it; Figma needs it) |
| **A JIT** (J1) | Policy accepted (J1–J7); whether to build one is open | Not before L2 (J1); **at M5's gate** (Round 1, RV1-13), the first L2 milestone, before the script-heavy content pages are used signed in | The interpreter on the content pages' scripts, at M3 and M4; S3's dispatch results |
| **The GPU/compositor process** | **Designed in Round 1** (RV1-02; process model §3, note 1) | Built at M2 | Spike S10: the hop, validation at display rate, zero-copy sampling (authorized 2026-10-21) |
| **Where WebGL and WebGPU run** | Open; recommended: a process of their own, apart from the compositor's (GR10) | M9's plan | The memory cost of a second Metal process; WebGL's and WebGPU's needs |
| **Passkeys** (OQ-16) | Open | Before M6 starts | Whether a restricted entitlement can be had; roaming security keys over USB HID; **the hybrid transport** (a phone as the authenticator), which needs no restricted entitlement (Round 1, RV1-21), checked for feasibility with CoreBluetooth under the signing identity (RV1-17) |
| **Xcode** (OQ-7) | Open; no design needs it (GR14) | M1's plan | Whether profiling or Metal debugging needs Instruments |
| **QoS overrides** (OQ-20) | Open | M1, the first milestone with Mach messages between processes (Round 1, RV1-07) | A measurement of a receiving thread in `kevent64` |
| **DNS over HTTPS**, **with Encrypted Client Hello** (Round 1, RV1-20) | Later ([networking](architecture/networking.md) §8); rustls has had experimental client-side ECH since 2024-06 | M5's plan | The owner's privacy needs once logins begin; ECH's keys come from DNS records |
| **The user-agent string** (SB13; Round 1, RV1-14) | Policy set: Safari-compatible, with an Axiom token | M1's plan, before its live loads | The Safari shipping on this Mac then |
| **JPEG's decoder** (IM2; Round 1, RV1-22) | `jpeg-decoder` is in maintenance mode | M1's plan | Its grade (DP8), or a decoder of Axiom's own, which adds scope (RV1-08) |
| **GR5's amendment** (OQ-24) and **surfaces allocated ahead** | Deferred to the graphics milestone (the owner, 2026-10-20); S6 §8 | Surfaces at M1, the first rendering milestone; GR5 at M4, with path-heavy SVG | A GPU filler of analytic quality, measured on the same contents |
| **`vello_cpu`'s and `smallvec`'s grades** | Adopted subject to grading (the owner, 2026-10-20); `smallvec`'s grade open | M1 | DP8's grades, on the versions adopted |
| **The storage engine** (SW2): `redb` or SQLite | Open | M3 | The crash tests of the first storage milestone |
| **Demuxers, and media instances per site** | Open | M7 | The media milestone's measurements |
| **AWS-LC's jitter entropy** | Open (S7) | M1, when TLS arrives | The network design's decision record |
| **objc2's text-input and accessibility classes** (EM11) | To be validated first | M1 | The shell's tests |
| **The workload** (OQ-9) | Assumed: one human developer | Recorded here (MS11) | — |

## 8. How a milestone runs

| # | Rule |
|---|---|
| MS10 | **Each milestone runs as a Phase Zero step did:**<br>• **The owner authorizes it**, naming what it may add: code, crates, tools. Milestone 1's list is CP9's.<br>• **Its own branch,** `milestone-N`, from `main`. Its plan is CP1, put to the owner at its first gate; later gates take ADR amendments, dependency adoptions and the final review.<br>• **Checkpoints** each leave the workspace clean and working, with the five checks green, the suites the milestone runs, and a tracker updated.<br>• **Dependencies** are adopted only after the register records them, with their class, grade and audit (DP6, DP8, DP15, DP16), and with the owner's approval in the milestone's plan.<br>• **The routine runs from M1:** weekly advisory checks (TS15), and the macOS-dependent checks again after every macOS update (TS16).<br>• **The merge** into `main` is `--no-ff`, when the owner says |
| MS7 | **Each milestone's acceptance criteria take one shape, written as numbers in its plan:**<br>• **conformance:** the directories it completes at their target, the shipping median (SB10–SB12), and no passing test regressing;<br>• **corpus checks** for its targets (MS6; TS12);<br>• **budgets** as benchmarks (PF5; TS13), including its pages' ceilings;<br>• **its level's security checks** (MS2; TS14);<br>• **fuzzing** of every parser and decoder it adds (TS9), with no open findings;<br>• **determinism** under the test scheduler (TS8) |
| MS11 | **One developer.** The roadmap assumes the owner is the only human developer, working with an assistant (OQ-9). There is no external audit, which is why L3's bar is the threat model's own (§8). Milestones are split into checkpoints small enough to leave the workspace clean (RC9) |

## 9. Revisions

| # | Rule |
|---|---|
| MS12 | **The roadmap changes only with the owner,** at a gate: reordering milestones, moving a corpus page or a decision point, or splitting a milestone. A milestone that needs an architectural change raises an amendment to the ADR it changes (RC6). A missed budget returns to the owner as a revision (PF5) |

## 10. Alternatives considered

- **The sensitive-use gate last,** after media and applications. The owner's accounts would wait longest, and media, GPU and WebAssembly would arrive before the level whose checks they must meet. Rejected: the gate comes as soon as its prerequisites exist (M6), and later milestones keep it.
- **Script before the sandbox:** M2 as script, at L0. It shows interactive pages sooner, but runs web script in an unsandboxed process (MS3). Rejected.
- **One process at M1,** with every process split at M2. The baseline chose it, because moving networking into its own process in the first milestone "mixes two risks: the first pipeline and the first process split". **Rejected in Round 1** (RV1-07, 2026-10-21): in one process, the shared memory, crash recovery, silent sandbox denials and per-process budgets that M2 depends on go untested through the longest milestone, so process-specific defects would surface together at M2. M1 now runs real, sandboxed renderer and network processes (MS4).
- **The GPU process at M9,** with the first web-supplied GPU work. **Rejected in Round 1** (RV1-02): the browser would consume display lists through L3's gate, and L3 would be certified on a topology M9 replaces. The GPU/compositor process arrives at M2.
- **Flexbox and grid in M1.** Wikipedia uses both (S8). They are the next stage of the layout design, and M1 is already the whole static pipeline. Tables are brought forward instead, since the documents' content depends on them more than their chrome does (inference). M1's corpus check is set for readability (CP9).
- **Dated milestones.** Rejected by OQ-1.

## 11. Open questions

1. **Corpus pages measured by hand** stay by hand (CO6): their corpus checks at M4–M9 need the owner's session for each gate. Whether a page's checks may sample fewer runs than a census is settled in each milestone's plan.
2. **VS Code for the Web and WebAssembly:** M8 or M9 (§4.1), at M8's plan.
3. **Google Maps without WebAssembly:** if Maps runs with WebGL alone, its target could move to an earlier GPU milestone. Checked when the GPU process is planned.

## Sources

All read on 2026-10-20.
- [C1] Chromium, "Site Isolation" (undated page; its newest item is from June 2025): "Site Isolation was enabled by default for all sites in Chrome 67 on Windows, Mac, Linux, and Chrome OS"; "Site Isolation represents a major architecture change for Chrome, so there are some tradeoffs when enabling it, such as increased memory overhead." The year, 2018, is from the post it links, dated July 2018 — https://www.chromium.org/Home/chromium-security/site-isolation/
- [F1] Mozilla Hacks, "Introducing Firefox's new Site Isolation Security Architecture", 2021-05-18: "Site Isolation (Project Fission), has been a massive multi-year project"; "We are currently testing Site Isolation on desktop browsers Nightly and Beta with a subset of users" — https://hacks.mozilla.org/2021/05/introducing-firefox-new-site-isolation-security-architecture/
- [L1] Ladybird, home page (© 2024–2026): "Target: Alpha 2026 (Linux & macOS)" — https://ladybird.org/
- [L2] Ladybird, newsletter, 2026-05-31: "The compositor also runs in its own process now" — https://ladybird.org/newsletter/2026-05-31
- [L3] Ladybird, newsletter, 2026-06-30: "Helper processes now run inside real sandboxes on Linux and macOS, using seccomp/Landlock on Linux and Seatbelt profiles on macOS"; "Sandboxing is now on by default" — https://ladybird.org/newsletter/2026-06-30
- [T1] Google, *Terms of Service*, effective 2026-07-30 (the country version served here), "Don't abuse our services" — https://policies.google.com/terms
- [T2] Instagram, *Terms of Use*, effective 2025-01-01, §4.1 and §4.2 (read through facebook.com/help/instagram/581066165581870, since the help page renders by script) — https://help.instagram.com/581066165581870
- [T3] Reddit, *User Agreement*, effective 2026-07-01, §4; *Reddit Rules*, Rule 2 — https://redditinc.com/policies/user-agreement ; https://redditinc.com/policies/reddit-rules
