# Engine/Embedder API and the Shell

| | |
|---|---|
| **Status** | **Binding** since ADR-0009's acceptance (2026-10-17, Gate B). Written at Step 4, CP9. Answers OQ-11, subject to the shell milestone's validation (EM11). **Amended by the architecture review's Round 1** (the owner's dispositions, 2026-10-21; [the review](../review/README.md) §5): EM9's diagnostics and reference-test path, for RV1-01 and RV1-15 (CP1). **Amended by Round 4** (the owner's dispositions, 2026-10-22; [the review](../review/README.md) §5.9): §8 for RV4-14 (CP7) |
| **Question** | Where is the boundary between the engine and the browser's shell? What does the API offer in each direction? How does a headless embedder use it for tests and WPT? Is the shell pure Rust, or a thin Swift layer (OQ-11)? |
| **Answers** | Step 4 question Q14 ([tracker](../research/step-4-architecture-proposal.md) §3): F4; OQ-11 |
| **Feeds** | ADR-0009; CP11 (overview and crate structure); Step 5 (the shell's subsystems: menus, the accessibility bridge); Step 6 (the testing strategy and WPT) |
| **Builds on** | [Event loop and threading](event-loop-and-threading.md) (EL6, EL24); [IPC](ipc.md) (IP2, IP7); [frame tree](frame-tree-and-navigation.md); [input and accessibility](input-and-accessibility.md); [S2 results](../research/s2-results.md) |
| **Last updated** | 2026-10-21: Round 1's accepted findings |

Everything here is **planned**; nothing is implemented. Rules are numbered EM1–EM12. Precedent is sourced (§2); the design is inference unless marked otherwise.

## Summary

- **The shell is an embedder of the engine** (EM1–EM3).
  - The shell is AppKit's main thread: windows, tabs, the address bar, menus, prompts.
  - The engine is the core and compositor threads, plus the helper processes.
  - They meet at a **message API** defined in the IDL like every other interface.
  - The engine never depends on AppKit or on the shell, a rule enforced at build time, like Chromium's "content doesn't depend on chrome at all".
- **Commands flow in, events and requests for decisions flow out** (EM4, EM5). The shell creates web views, navigates them and forwards input. The engine reports navigation, title, progress and crashes. It asks the shell to decide what only a user interface can: new windows, permissions, dialogs, downloads, file choices, authentication, context menus. WKWebView's navigation and UI delegates are the model.
- **Presentation, input, IME and accessibility cross the boundary as objects the shell hosts** (EM6). The compositor draws into a layer in the shell's view, and the view's event, text-input and accessibility entry points delegate to the engine's sinks and mirrors.
- **The browser's own UI is native, never web content** (EM8). No privileged web pages exist that a compromised renderer could aim at.
- **A headless embedder** (EM9) implements the same API without AppKit: offscreen surfaces, synthetic input and, in single-process mode, virtual time (Round 2, RV2-03), for WPT, reference tests and fuzzing.
- **OQ-11: pure Rust, with objc2** (EM11). S2 showed AppKit, Metal and CoreText working from Rust. Defining Objective-C classes, which IME and accessibility need, is the first thing the shell milestone validates. A Swift shell could sit behind the same message API if that fails, so the engine's architecture doesn't depend on the answer.

## 1. Binding inputs

| Input | Consequence here |
|---|---|
| **EL6:** AppKit's main thread runs the shell only; the core thread holds the engine's authority | EM1, EM3 |
| **PM6, IP2, IP7:** every boundary a message interface defined in the IDL, working in-process | EM2 |
| **AC6:** a deterministic, headless core | EM9 |
| **IN7, IN9:** IME and accessibility answered from mirrors on the main thread | EM6 |
| **Process model §3:** the browser holds profile data other than web data, mediates user files and permissions, and never parses web content | EM5, EM7, EM8 |
| **OQ-11; S2:** objc2 worked for a window, Metal and CoreText, but menus, IME, accessibility and bundles were untested, and no Objective-C class was defined ([S2 results](../research/s2-results.md) §3, §8) | EM11 |

## 2. Precedent

From documentation read on 2026-10-17 (Sources).

| Engine | Boundary | What crosses it |
|---|---|---|
| **Chromium** | `content` is "the core code needed to render a page using a multi-process sandboxed browser". Features such as extensions and autofill live in `chrome`, and "content doesn't depend on chrome at all", enforced by layering rules. `content_shell` is a "basic browser built on top of content" for developing and testing it [C1] | The Content API: `ContentClient` (in all processes), `ContentBrowserClient`, `ContentRendererClient`, `WebContentsObserver`, `NavigationThrottle` [C1] |
| **WebKit (WKWebView)** | An app embeds a web view and implements delegates | **Navigation delegate:** decide policy for navigation actions and responses; provisional start, redirect, commit, finish and failure; authentication challenges; downloads; and "the web view's content process was terminated" [A1]. **UI delegate:** "Creates a new web view" for new windows; JavaScript alert, confirm and prompt panels; file upload panels; context menus; media-capture permissions [A2] |
| **Servo** | The embedder talks to the constellation by messages: `NewWebView`, `CloseWebView`, `ForwardInputEvent`, `TickAnimation`; the constellation answers with its own messages. Its embedding documentation is "sparse" ([event loop](event-loop-and-threading.md) §3, sources S1–S2) | Messages in both directions |

**Observation** (inference). Each engine separates the engine from the product, and each test harness rides on the separation: `content_shell`, WKWebView test apps. Servo's boundary is already a message protocol, which is what Axiom's PM6 asks for.

## 3. The layers of the browser process

```
┌────────────── shell (main thread, AppKit) ──────────────┐
│ windows · tab strip · address bar · menus · prompts     │
│ web-view NSView: event sink, IME client, AX children,   │
│ presentation layer                                      │
└──────────────── embedder API (messages, IDL) ───────────┘
┌────────── engine (core thread, compositor thread) ──────┐
│ frame tree · navigation · process manager · policy ·    │
│ profile store · compositor                              │
└──────────────── IPC (Mach) ─────────────────────────────┘
            renderers · network process · GPU, utility
```

## 4. Design

### 4.1 The boundary

| # | Rule |
|---|---|
| EM1 | **The shell is an embedder.** It runs on AppKit's main thread and owns everything the user sees around web content: windows, the tab strip, the address bar, menus, prompts, the downloads list, settings. The engine runs on the core and compositor threads and in the helper processes (EL6) |
| EM2 | **The embedder API is a message interface,** defined in the IDL (IP7) and carried by the same channel types (IP2), between the shell's main thread and the engine's core. The shell is trusted, but typed messages make the boundary testable and replaceable (EM9, EM11) |
| EM3 | **The engine never depends on the shell.** Engine crates may not depend on AppKit or on any shell crate. The build enforces it, as Chromium's layering rules enforce "content doesn't depend on chrome" [C1] |

### 4.2 Commands, events and decisions

| # | Rule |
|---|---|
| EM4 | **Commands from the shell** (shell → engine): |

- **Web views:** create a web view (a tab, which is a top-level traversable) for a window region; close it; resize it; show or hide it; set its device scale.
- **Navigation:** load a URL; reload; stop; go back, forward or to a history index.
- **Settings and profile actions:** clear browsing data; change permissions; manage the sensitive-site list (SI5) and the switches that turn the JIT off globally or per site (J4).
- **Find in page,** zoom, and printing (later).

**Events to the shell** (engine → shell):
- navigation started, redirected, committed, finished or failed;
- URL, title and favicon changes;
- load progress;
- security state;
- a renderer crash or hang; **as amended in Round 2** (RV2-04), a helper's too, the network or the GPU/compositor process's, with what was lost and whether it was relaunched (IR10);
- audio playing, and fullscreen changes.

| # | Rule |
|---|---|
| EM5 | **Requests for decisions** (engine asks, shell answers). The engine asks the shell to decide what only a user interface can, after applying its own policy first. The table below lists them. WKWebView's two delegates are the model [A1, A2]. **Every request has a default, and the engine never waits indefinitely.** A request that is still pending when its tab closes is cancelled. **Each request's default is the table's last column** (**as amended in Round 2,** RV2-03): the engine takes it when the shell doesn't answer, and the headless embedder answers with it and logs it (EM9) |

| Request | Engine's part | Shell's part | Default, if the shell doesn't answer |
|---|---|---|---|
| New window (`window.open`, `target=_blank`) | Popup blocking from its activation records (IN8); the opener relationship | Open a tab or a window, or show that it was blocked | A new tab |
| Permission (camera, microphone, location, notifications) | Records and defaults; never granted to a renderer directly | Prompt; the decision is stored in the profile | Denied, and not stored |
| JavaScript dialog (alert, confirm, prompt) and `beforeunload` | Pauses the event loop (EL17) | A sheet attached to the tab | Dismissed: `confirm` answers false, `prompt` null; `beforeunload` lets the navigation proceed |
| Download | Classifies the response; writes the file and marks it quarantined | Location and confirmation; the downloads list | Cancelled |
| File chooser | Reads the chosen files for the renderer, which receives their bytes as data or a read-only memory entry, never a descriptor (SG1; the File API's SW16; **as amended in Round 2,** RV2-11) | `NSOpenPanel` | Cancelled: no files |
| HTTP authentication, certificate errors | Interstitial pages are internal documents (FT10) | Credentials prompt; the security indicator | Cancelled: the request fails, and the interstitial stays |
| Context menu | The hit-tested element's data, with URLs checked against its records (IN10) | The menu | None shown |
| External protocols (`mailto:`) | Requires user activation | Confirmation, then `NSWorkspace` | Refused |

### 4.3 What the shell hosts

| # | Rule |
|---|---|
| EM6 | **Presentation, input, IME and accessibility cross as hosted objects.** The shell's web-view `NSView` hosts a layer that the compositor draws into, committing explicit Core Animation transactions from its own thread (macOS runtime note item 11). The view's event methods forward to the compositor's input sink (IN1). Its `NSTextInputClient` methods read and write the IME mirror (IN7). Its accessibility children come from the engine's accessibility cache (IN9). These objects live on the main thread, but are owned and kept current by the engine |
| EM7 | **Profile data lives with the engine,** in the browser process: settings, history, bookmarks, permissions and the sensitive-site list ([process model](../security/process-model.md) §3). The core records history visits at commit (FT7). The shell reads and edits the data through the API. Web data (cookies, cache, storage) stays in the network process |
| EM8 | **The browser's own UI is native, never web content.** Settings, history, downloads and permission management are AppKit views in the shell. Internal pages that are documents, such as error and interstitial pages, are rendered without privileges, in renderers with internal locks (FT10). **There is no privileged web UI** for a compromised renderer to aim at. Chromium's privileged `chrome://` pages, by contrast, are web content in renderers (background) |

### 4.4 Tests

| # | Rule |
|---|---|
| EM9 | **A headless embedder implements the same API without AppKit.** It provides offscreen presentation (the compositor renders into an offscreen texture, read back for reference tests and screenshots), synthetic input injected at the compositor's sink, virtual time through the deterministic scheduler (EL24) in single-process mode (**as amended in Round 2,** RV2-03: conformance runs in real processes on the host clock, TS2), and logged answers to decision requests, which take EM5's defaults. It runs WPT, reference tests, crash tests and fuzzing harnesses (Step 6), on this Mac, without a window. `content_shell` plays this role for Chromium [C1].<br>**As amended in Round 1** (2026-10-21):<br>• **A script-free reference-test path** (RV1-01; [testing](../testing.md) TS2): given a URL, it loads it, waits natively for the load event and for pending images and fonts, writes a PNG of the offscreen texture, and exits, so wptrunner can run reference tests before script exists. WPT's host names resolve through a hosts file that only the test build accepts.<br>• **Diagnostics for one developer** (RV1-15), staged: **from M1,** test-only commands dump, as text in stable formats that tests can compare, the DOM, a node's computed style, the box and fragment trees, the display list and the network log; **from M2,** a crashed helper's report is moved into a local directory the owner controls, with the build identifier (IP18), as S9's script already does for test crashes; **from M3,** console messages reach the embedder's log and the test runners (JE27). A developer-tools protocol stays later (JE27).<br>Every dump and log is test-only or local: none leaves this Mac.<br>**Round 2** (RV2-08): the headless embedder answers WebDriver's Get Computed Role and Label from M4; tests of the platform mapping need AppKit, so they run against the shell's build (AX11) |
| EM10 | **Single-process mode uses the same API.** The shell can't tell whether the engine runs one process (single-process test mode) or many (every level from M1, as amended in Round 1, RV1-07). That is a property of the engine's process manager |

### 4.5 The shell's language and packaging

| # | Rule |
|---|---|
| EM11 | **OQ-11: a pure-Rust shell, using objc2.** |

- **For:**
  - S2 drove AppKit, Metal and CoreText from Rust with 19 contained `unsafe` sites.
  - The owner is fluent in Rust.
  - One toolchain keeps the build simple and fast (thresholds T1–T4).
  - The IME mirror and accessibility cache are Rust data, which a Swift shell would reach only across another FFI boundary.
- **Against, still unproven:** S2 defined no Objective-C class. IME and accessibility need an `NSView` subclass implementing `NSTextInputClient` and the `NSAccessibility` protocols, which objc2 provides through `define_class!`, untested here. Menus and an app bundle were also outside S2.
- **So the shell milestone starts by validating** an objc2-defined view class that implements text input and accessibility, plus a menu bar and a bundle built without Xcode.
- **If objc2 falls short,** a thin Swift shell sits behind the same message API. The IDL generator can emit Swift (IP8), and nothing in the engine changes. The decision is still recorded now, because CP11's crate structure depends on it.

| # | Rule |
|---|---|
| EM12 | **One application bundle** contains the shell and the helper executables (`renderer`, `renderer-jit` later, `network`, and `gpu` and `utility` later). They are signed with the hardened runtime, each with its own entitlements, by the stable local identity, with ad hoc as the fallback (**as amended in Round 1**, RV1-17; [S15 results](../research/s15-results.md)) ([process model](../security/process-model.md) §10; `tools/s4-sign.sh`). A script builds the bundle, without Xcode |

## 5. Staging

| Level | What changes |
|---|---|
| L0 | The shell and the engine's browser side run in one process; **as amended in Round 1** (RV1-07), renderers and the network process are sandboxed helpers from M1, and single-process mode is kept strictly for automated tests, fuzzing and Miri. The headless embedder's script-free test path and dumps exist from M1 (EM9) |
| L1–L3 | The engine's process manager launches helpers; the embedder API doesn't change (EM10) |

## 6. Alternatives considered

| | Alternative | Assessment |
|---|---|---|
| A | **The engine as a library of direct calls,** with no message boundary | Faster to start, but the headless embedder and a possible Swift shell would each need their own glue, and the core would drift toward AppKit. Rejected for EM2 |
| B | **The browser's UI in web content** (privileged internal pages) | Familiar from Chromium, but privileged pages give compromised renderers a target. Rejected for EM8 |
| C | **A Swift shell from the start** | Swift is AppKit's first language, but it adds a second toolchain and an FFI boundary in front of the IME mirror and accessibility cache, and the owner is fluent in Rust. Kept as the fallback (EM11) |
| D | **SwiftUI** | Even further from the engine's Rust data, and an added layer over AppKit. Rejected |

## 7. Criteria check

| Criterion | How this design meets it |
|---|---|
| AC1 | The embedder API is the same from L0 to L3 (EM10) |
| AC2 | The engine applies policy before asking the shell; the shell decides only what needs a person (EM5) |
| AC6 | The headless embedder (EM9) |
| AC9 | objc2 FFI stays in the shell's `platform` modules, under the `unsafe` policy |

## 8. Open questions

| # | Question | Where it is settled |
|---|---|---|
| 1 | OQ-11: confirm the pure-Rust shell | **Confirmed at Gate B** (2026-10-17, ADR-0009 accepted); validated at the start of the shell milestone (EM11) |
| 2 | The full list of settings, and how the sensitive-site list and the JIT switches are presented | The settings list approved at Gate B (accessibility §6.1); the sensitive-site list and the JIT switches at M5's plan (Round 4, RV4-14) |
| 3 | WPT's harness integration (`testharness.js`, `testdriver`) on the headless embedder | **Settled** at Step 6 ([testing](../testing.md) TS2), and amended in Round 1 (RV1-01): a script-free executor at M1 and M2, WebDriver from M3 |

## Sources

All checked on 2026-10-17.

- [C1] Chromium, "Content module" (`content/README.md`) — https://chromium.googlesource.com/chromium/src/+/HEAD/content/README.md
- [A1] Apple, `WKNavigationDelegate` (read in its JSON form) — https://developer.apple.com/documentation/webkit/wknavigationdelegate
- [A2] Apple, `WKUIDelegate` (read in its JSON form) — https://developer.apple.com/documentation/webkit/wkuidelegate
