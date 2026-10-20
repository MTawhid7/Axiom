# The Standards Baseline

| | |
|---|---|
| **Status** | **Proposed** (Step 6, CP4, 2026-10-20), with [ADR-0018](adr/0018-corpus-standards-and-testing.md), for the owner at Gate B; binding when it is accepted |
| **Question** | Which specifications does Axiom implement, at which dated versions, and how fully? Which suites check each, and what pass rate counts at each milestone? |
| **Answers** | Step 6 question Q2 ([tracker](research/step-6-standards-budgets-roadmap.md) §3); decision-map item X1; the tracker's handed-over question 3 (§2.2) |
| **Feeds** | Testing (CP5): the suites and how expectations are kept; the roadmap (CP8): what each milestone completes; Milestone 1 (CP9): its conformance criteria |
| **Builds on** | The [charter](charter.md)'s principle 5 and §8; Step 5's criterion SC5 and every subsystem document's §2; [ADR-0001](adr/0001-scope-and-target.md); the [corpus](corpus.md) and spike S8's census ([S8 results](research/s8-results.md)); [OQ-22](open-questions.md) |
| **Last updated** | 2026-10-20 |

Rules are numbered SB1–SB12. Precedent and sources are dated (§2). The design is inference unless marked otherwise.

## Summary

- **The baseline** is, for each feature area, the specification Axiom implements, pinned to a dated version, with one support class and the suites that check it (§5's matrix).
- **Pins differ by body** (SB1):
  - **WHATWG living standards** are pinned at a commit snapshot, with the latest Review Draft noted.
  - **W3C reports** are pinned at their dated version, or at a dated editor's draft where the report is stale and WPT tests the draft.
  - **ECMAScript** is pinned by edition plus finished proposals.
  - **Unicode** is pinned at the version the text stack implements, the same one across the engine.
  - **IETF specifications** are pinned by RFC, or by draft revision until the RFC is published.
- **The baseline moves only at a milestone's start,** every pin together with the suites' revisions, so a change in pass rates traces to the code or to the move, never to both (SB2).
- **Five support classes:**
  - to the specification;
  - approximate, with a reason;
  - staged to a milestone;
  - deferred until a decision;
  - not planned, with a reason.

  A staged API is not exposed until it works, so pages that detect features see the truth (SB3, SB4).
- **Four behaviours are approximate,** because no specification settles them (SB5):
  - `execCommand`;
  - automatic table layout;
  - `Error.prototype.stack`;
  - `Date.parse`'s legacy formats.
- **Priorities, in order** (SB6):
  1. what the corpus uses, measured by S8;
  2. Baseline status, from the W3C WebDX group's web-features project;
  3. the Interop 2026 focus areas.
- **The suites** (SB7–SB9):
  - WPT at a pinned revision;
  - html5lib-tests' tokenizer tests (its tree-construction tests now live in WPT);
  - test262;
  - Unicode's conformance files;
  - later, WebGPU's conformance suite and WebAssembly's specification tests.

  Expected results are recorded test by test, in wptrunner's metadata format, so every change in a result is visible.
- **Pass rates** (SB10–SB12):
  - A milestone that completes an area targets the median pass rate of Chrome, Firefox and Safari on its WPT directories, at the pinned revision.
  - A test that passed may not regress without a recorded reason.
  - §6 calibrates the targets on wpt.fyi's runs of 2026-10-19.

## 1. Binding inputs

| Input | Consequence here |
|---|---|
| **Charter principle 5:** keep "standards-compliant and approximate" distinct in every document | SB3–SB5 |
| **Charter §8:** milestone criteria draw on "pass rates on selected WPT directories, html5lib-tests and test262" | SB7–SB12 |
| **Charter §6:** DRM-protected media is a non-goal; bug-for-bug compatibility beyond the specifications is out, but "the specs' own quirks-mode behaviour **is** in scope", and behaviour outside the specifications is added "only when the target corpus needs it" | SB3's not-planned class; SB5 |
| **ADR-0001 R1:** the architecture reaches the full modern app web; milestones start with static documents | SB3's staged class: nothing in the modern web is excluded for being hard |
| **SC5** (Step 5): each subsystem names its dated specifications and its suites, keeps approximate behaviour apart, and leaves the baseline and pass rates to Step 6 | This document; §5 gathers every subsystem's §2 |
| **OQ-22** (the owner, 2026-10-20): "Defer planning XSLT until a concrete use case arises." | XSLT is deferred (SB3) |
| **The corpus** (CP2) and **S8's census** (CP3) | SB6's first priority; §5's corpus column |

## 2. Sources and precedent

**How the bodies publish:**
- **WHATWG.** Each normative change "must have support from implementers", and "normative changes to a WHATWG standard requires tests, usually submitted to" web-platform-tests [W1]. Each Living Standard links a "Snapshot as of this commit" [W1]. "Editors publish Review Drafts approximately every six months" as "the reference for patent review" [W2]. HTML's Review Drafts are dated January and July, the latest 2026-07, published 2026-07-17 [W3].
- **W3C.** The Process in force is dated 2025-08-18. A Working Draft "is suitable for gathering wide review"; a Candidate Recommendation Snapshot "corresponds to a Patent Review Draft"; a Candidate Recommendation Draft "is published to solicit review of intended changes" [P1]. Many specifications that engines ship sit at Working Draft for years: CSS Scoping 1's report is a First Public Working Draft of 2014, and its editor's draft is newer ([style](architecture/style.md) §2).
- **CSS.** *CSS Snapshot 2026*, a Group Note of 2026-06-22: "This profile includes only specifications that we consider stable" [C1]. It sorts modules into four groups:
  - the official definition, 24 modules, from CSS 2.1 to Counter Styles 3;
  - reliable Candidate Recommendations, such as Grid 1 and 2, Cascade 5 and Media Queries 4;
  - fairly stable modules with limited implementation experience, such as Display 3, Text 3 and View Transitions 1;
  - modules with rough interoperability, such as Selectors 4, Fonts 4, Web Animations, CSSOM and Nesting.

  It also lists features "safe to release" before Candidate Recommendation, for example `:has()`, `aspect-ratio`, `color-mix()` and everything in CSS Animations 1 and Transitions 1.
- **Ecma.** ECMA-262's 17th edition, *ECMAScript 2026*, June 2026, of which "the normative copy is the HTML version" [E1]. ECMA-402's 13th edition, June 2026 [E2]. Proposals finished for 2027 include Temporal ([the JavaScript engine](architecture/javascript-engine.md) §2).
- **Unicode.** Version 18.0.0, released 2026-09-16, adds 13,007 characters, for 172,808 [U1]. The consortium's list of versions shows it as released, while its version page still carried a beta-draft notice on 2026-10-20 [U2]. ICU4X 2.3.0 implements 17.0.0 ([text and fonts](architecture/text-and-fonts.md) §2).
- **IETF.** RFCs are fixed. The revision of cookies, `draft-ietf-httpbis-rfc6265bis-22`, is dated 2025-12-01, and was in the RFC Editor's queue when Step 5 read the IETF's datatracker ([networking](architecture/networking.md) §2).

**What the web relies on:**
- **Baseline,** defined by the W3C WebDX community group's web-features project [B1]:
  - **the core browser set:** Safari on iOS and macOS, Chrome on Android and desktop, Edge, Firefox on Android and desktop;
  - **"low" status:** "the initial interoperability of a feature across many browsers", once every current stable release in the set supports it;
  - **"high" status:** when "the feature's keystone date is on or before today's date minus 30 months".

  The project publishes its features with their status, version 3.43.0 of 2026-10-15 [B2]. Chrome counts each page's use of the same features (the `WebDXFeature` use counters, named for web-features' identifiers), which S8 read for the corpus.
- **Interop 2026,** run by Apple, Bocoup, Google, Igalia, Microsoft and Mozilla, "focused on technology that is already specified in web standards" [I1]. Its 20 focus areas:
  - container style queries; CSS anchor positioning; CSS `attr()`; CSS `contrast-color()`; CSS `zoom`;
  - custom highlights; dialogs and popovers; Fetch uploads and ranges; IndexedDB; JSPI for Wasm;
  - media pseudo-classes; the Navigation API; scoped custom element registries; scroll-driven animations; scroll snap;
  - CSS `shape()`; view transitions; web compat; WebRTC; WebTransport.

**How engines test:**
- **wpt.fyi** publishes runs at aligned WPT revisions [T1]:
  - 2026-10-19, at `0f26972422`: Chrome 155, Firefox 158 and Safari 27.0 (on macOS 27.0), with Servo 0.7.0 as a nightly;
  - 2026-10-20, at `a26422aab3`: Ladybird 1.0.

  §6 uses those runs.
- **wptrunner** keeps expectations in `.ini` files that mirror the test tree: an `expected` status per test and subtest, the first value typical and later ones known intermittents, with conditions on the platform. `wpt update-expectations` rewrites them from a run's report [T2].
- **New engines run WPT through WebDriver.** wptrunner's Ladybird product launches Ladybird's WebDriver server and runs testharness, reference and crash tests through WebDriver executors [T3]. Servo has its own product too.
- **html5lib-tests' tree-construction tests moved into WPT** on 2026-06-26. Its README now says they "are now solely maintained on web-platform-tests", in `html/syntax/parsing`. The repository keeps its tokenizer tests, which gained processing instructions on 2026-10-01 [T4].

## 3. Pinning and moving the baseline

| # | Rule |
|---|---|
| SB1 | **Each specification is pinned as its body publishes it:**<br>• **WHATWG living standards** (HTML, DOM, Fetch, URL, Encoding, Streams, Storage, Web IDL, MIME Sniffing, Console, Compatibility, Quirks Mode, File System): **a commit snapshot** for each, recorded in the milestone's plan, with the latest Review Draft noted beside it. Implementations and WPT follow the living standard, since every normative change arrives with its tests [W1], and a Review Draft can be six months behind [W2].<br>• **W3C reports:** the dated report with its maturity. Where the report is older than its editor's draft and WPT tests the draft, the editor's draft at a dated commit, with both recorded (for example CSS Scoping 1, §2).<br>• **CSS:** as W3C, with CSS Snapshot 2026's group noted as a measure of stability [C1].<br>• **Ecma:** ECMA-262 and ECMA-402 by edition (2026: the 17th and 13th editions), plus proposals that are finished (stage 4) for the next edition. test262 is pinned by commit.<br>• **Unicode:** the version Axiom's text stack implements, which ICU4X's release sets: 17.0.0 for ICU4X 2.3.0, while 18.0.0 is out [U1]. **One version across the engine:** segmentation, line breaking, shaping, IDNA, normalization, and JavaScript's identifiers and property escapes all move together, and a check fails the build if their data disagree.<br>• **IETF:** RFCs by number; an Internet-Draft by revision only while its RFC is pending, replaced by the RFC once published (cookies, §2).<br>• **Formats:** by edition (for example PNG's third edition, a W3C Recommendation of 2025-06-24, which S6 cited) |
| SB2 | **The baseline moves at a milestone's start, all at once:** the snapshots and report versions together with WPT's revision, test262's and html5lib-tests' commits, and Unicode's version. Expectations are regenerated in the same checkpoint, so a change in pass rates traces either to the move or to the code. Within a milestone, a pin moves only for a specification bug the milestone meets, recorded with its reason |

## 4. Support classes

| # | Rule |
|---|---|
| SB3 | **Every feature area has one support class,** and a feature that differs from its area has its own:<br>• **To the specification:** implemented as specified; the suites' expectations are targets (SB10).<br>• **Approximate:** no specification defines the behaviour precisely enough (SB5); measured, not targeted.<br>• **Staged:** implemented to the specification at a milestone the roadmap names (CP8).<br>• **Deferred:** no milestone until a decision or a use case: XSLT (OQ-22).<br>• **Not planned:** outside the charter's scope or outside the standards, with the reason (§5.3) |
| SB4 | **A staged API isn't exposed until it works.** An interface, property, CSS feature or media type is either absent, or present and implemented to its class. A stub that answers feature detection while doing nothing would mislead pages into code paths that then fail. Where `@supports`, `CSS.supports()`, `canPlayType()` or `isTypeSupported()` answer, they answer for what Axiom actually does |
| SB5 | **Approximate behaviour is listed here, with its reason, and nowhere else:**<br>• **`execCommand`** ([HTML and the DOM](architecture/html-and-dom.md) HD28): its draft "is incomplete and it is not expected that it will advance beyond draft status", and its features "are not implemented consistently or fully by user agents". Axiom follows the engines where they agree, and the draft elsewhere.<br>• **Automatic table layout** ([layout and paint](architecture/layout-and-paint.md) LY10): CSS 2.1 says that "UAs are not required to implement this algorithm", and CSS Table 3 "is not yet ready for implementation".<br>• **`Error.prototype.stack`** ([the JavaScript engine](architecture/javascript-engine.md) JE25): no specification; its format follows the engines.<br>• **`Date.parse`'s legacy formats** (JE22): implementation-defined beyond the specified format; the engines' common formats.<br>Code and tests for these carry an "approximate" mark. **Adding an entry** needs the evidence that no specification settles the behaviour, and is reported to the owner at the next gate |

## 5. The support matrix

| # | Rule |
|---|---|
| SB6 | **Priorities,** where a milestone must choose among areas or features:<br>1. **What the corpus's pages use,** from S8's census, and from inference for the pages measured by hand until their session.<br>2. **Baseline status:** "high" features first; "low" ones when the corpus uses them; features in only some engines only where the corpus needs them and a specification exists, and never as a page's only path.<br>3. **Interop 2026's focus areas,** as the engines' own priorities.<br>**Evidence:** of the 116 web-features that S8's five pages touch, 72 are Baseline "high", 19 "low" and 25 in only some engines. The pages use those 25 mostly as enhancements, since the same sites serve Safari and Firefox ([S8 results](research/s8-results.md) §3) |

**Columns:** the area; what is implemented and how it is pinned (SB1); its class (SB3); its suites; what the corpus needs, from S8's census for the five automated pages and inference for the rest ([corpus](corpus.md) §2). Each area's design is in the subsystem document named. **Staged** areas get their milestone from the roadmap (CP8).

### 5.1 Documents, style and layout

| Area | Specifications | Class | Suites | Corpus |
|---|---|---|---|---|
| Encoding | WHATWG Encoding | To the specification | WPT `encoding/` | Every page |
| URLs | WHATWG URL; UTS #46 for IDNA | To the specification | WPT `url/` (including its JSON test data) | Every page |
| HTML parsing and serialization | WHATWG HTML §13, with processing instructions parsed into nodes | To the specification ([HTML and the DOM](architecture/html-and-dom.md)) | WPT `html/syntax/parsing` (tree construction); html5lib-tests' tokenizer tests | Every page |
| The DOM | WHATWG DOM: nodes, events, ranges, traversal, mutation observers, `moveBefore()` | To the specification | WPT `dom/` | Every page |
| HTML elements and forms | WHATWG HTML §4, forms §4.10 | To the specification, staged by element group | WPT `html/semantics/` | Every page; forms for logins (CO8) |
| Custom elements and shadow DOM | HTML §4.13, scoped registries; DOM §4.8; declarative shadow DOM | Staged | WPT `custom-elements/`, `shadow-dom/` | MDN and VS Code (S8); YouTube and Reddit (inference) |
| Editing | Selection API, Input Events 2, EditContext; `execCommand` | Staged; `execCommand` approximate (SB5) | WPT `selection/`, `input-events/`, `editing/` | VS Code, Prism (inference) |
| Events and input | UI Events, Pointer Events, Touch Events where macOS supplies touch | Staged with script | WPT `uievents/`, `pointerevents/` | Every interactive page |
| XML | XML 1.0 (Fifth Edition), Namespaces 1.0 (Third Edition); HTML §14 | To the specification | WPT `html/the-xhtml-syntax/`, `dom/` tests on XML documents | SVG images (every page with icons) |
| XSLT | DOM §9 `XSLTProcessor`; `xml-stylesheet` | **Deferred** (OQ-22) | — | None on the automated pages (S8) |
| CSS syntax, cascade and selectors | CSS Syntax 3, Cascade 5 and 6, Selectors 4, Nesting 1, Scoping 1 (editor's draft), Shadow Parts 1, Namespaces 3 | To the specification ([style](architecture/style.md)) | WPT `css/css-syntax/`, `css/css-cascade/`, `css/selectors/`, `css/css-nesting/`, `css/css-shadow/` (scoping and Shadow Parts, ST36) | Every page; `:has()` on all five automated pages, nesting on VS Code and Wikipedia (S8) |
| CSS values and properties | Values 4 and 5, Custom Properties 1, Properties and Values API 1, CSSOM, the WHATWG Compatibility Standard (`-webkit-` aliases), the Quirks Mode Standard | To the specification | WPT `css/css-values/`, `css/css-variables/`, `css/css-properties-values-api/`, `css/cssom/`, `compat/`, `quirks/` | Every page |
| Media and container queries | Media Queries 4 and 5; Conditional Rules 5 (container queries) | To the specification | WPT `css/mediaqueries/`, `css/css-conditional/`, `css/css-contain/` | Container queries on VS Code (S8); media queries on every page (inference) |
| Block, inline and floats | CSS 2.1 (normal flow, floats, margin collapsing), Display 3, Box Model 4, Box Sizing 3 and 4, Inline Layout 3 | To the specification ([layout and paint](architecture/layout-and-paint.md)) | WPT `css/CSS2/`, `css/css-display/`, `css/css-box/`, `css/css-sizing/`, `css/css-inline/` | Every page |
| Flexbox and grid | Flexbox 1; Grid 1, 2 and 3 (grid lanes) | To the specification | WPT `css/css-flexbox/`, `css/css-grid/` | Both on all five automated pages (S8) |
| Tables | CSS 2.1 §17; CSS Table 3 | To the specification, except automatic layout: approximate (SB5) | WPT `css/css-tables/`, `css/CSS2/tables/` | Wikipedia (inference) |
| Positioning and overflow | Positioned Layout 3, Anchor Positioning 1, Overflow 3 and 4, Scroll Snap 1, Scroll Anchoring 1, Containment 2 | To the specification; anchor positioning staged | WPT `css/css-position/`, `css/css-anchor-position/`, `css/css-overflow/`, `css/css-scroll-snap/`, `css/css-scroll-anchoring/`, `css/css-contain/` | Every page |
| Multi-column and fragmentation | Multi-column 1, Fragmentation 3, Paged Media 3 | Staged | WPT `css/css-multicol/`, `css/css-break/`, `css/css-page/` | Printing; not seen on the automated pages (inference) |
| Text and writing modes | Text 3 and 4, Text Decoration 3 and 4, Writing Modes 4, Logical Properties 1, Ruby 1, Lists and Counters 3 | To the specification ([text and fonts](architecture/text-and-fonts.md)) | WPT `css/css-text/`, `css/css-text-decor/`, `css/css-writing-modes/`, `css/css-logical/`, `css/css-ruby/`, `css/css-lists/` | Every page |
| Fonts | Fonts 4, Font Loading 3; OpenType, WOFF 1.0 and WOFF2 | To the specification | WPT `css/css-fonts/`, `css/css-font-loading/` | Every page that loads web fonts (S8) |
| Unicode algorithms | UAX #9, #14, #15, #29, #31, #50; UTS #46, #51 | To the specification, at ICU4X's Unicode version (SB1) | Unicode's conformance files (`BidiCharacterTest.txt`, `LineBreakTest.txt`, `NormalizationTest.txt`, `GraphemeBreakTest.txt`, `WordBreakTest.txt`, `SentenceBreakTest.txt`, `IdnaTestV2.txt`); ICU4X's own tests | Every page |
| Painting | Backgrounds and Borders 3 and 4, Images 3 and 4, Color 4 and 5, Masking 1, Filter Effects 1 and 2, Compositing and Blending 1, Transforms 1 and 2, Basic User Interface 4 | To the specification ([graphics](architecture/graphics.md)) | WPT `css/css-backgrounds/`, `css/css-images/`, `css/css-color/`, `css/css-masking/`, `css/filter-effects/`, `css/compositing/`, `css/css-transforms/`, `css/css-ui/` | Every page |
| Animation | CSS Animations 1, Transitions 1, Web Animations 1, Scroll-driven Animations 1, View Transitions 1, Easing 1 | Staged with script | WPT `css/css-animations/`, `css/css-transitions/`, `web-animations/`, `scroll-animations/`, `css/css-view-transitions/`, `css/css-easing/` | CSS animations and transitions on Maps, VS Code and Wikipedia; Web Animations on VS Code (S8) |
| Geometry and observers | CSSOM View, Resize Observer, Intersection Observer, Geometry Interfaces 1 | Staged with script | WPT `css/cssom-view/`, `resize-observer/`, `intersection-observer/`, `css/geometry/` | Every automated page used an observer (S8) |
| SVG | SVG 2 (Candidate Recommendation Snapshot, 2026-10-06) | To the specification, staged in parts: SVG images first, then SVG in documents, then its DOM | WPT `svg/` | Icons on every page (inference) |
| MathML | MathML Core (Candidate Recommendation Snapshot, 2025-06-24) | Staged | WPT `mathml/` | Wikipedia (S8) |

### 5.2 Script, platform APIs, media and network

| Area | Specifications | Class | Suites | Corpus |
|---|---|---|---|---|
| JavaScript | ECMA-262, 17th edition (2026), and finished proposals for 2027, including Temporal; ECMA-402, 13th edition | To the specification, except `Error.prototype.stack` and `Date.parse`'s legacy formats: approximate (SB5) | test262 at a pinned commit | Every page but Wikipedia's text |
| Web IDL and bindings | WHATWG Web IDL | To the specification ([bindings](architecture/bindings.md)) | WPT `webidl/`; every API directory's `idlharness` tests | Every scripted page |
| Scripting and modules | HTML §8.1: scripts, module scripts, import maps, CSS and JSON modules | Staged with script | WPT `html/semantics/scripting-1/`, `import-maps/` | MDN, Three.js, VS Code: modules (S8) |
| Navigation and history | HTML §7: navigables, session history, `Location`; the Navigation API | Staged | WPT `html/browsers/`, `navigation-api/` | Every page; applications navigate in place (inference) |
| Workers | HTML §10: dedicated and shared workers; `BroadcastChannel`; Web Locks | Staged | WPT `workers/`, `webmessaging/`, `web-locks/` | Maps and VS Code (S8) |
| Storage | WHATWG Storage; Web Storage; IndexedDB 3; the Cache API; the WHATWG File System Standard (the origin private file system) | Staged ([storage and workers](architecture/storage-and-workers.md)) | WPT `storage/`, `webstorage/`, `IndexedDB/`, `fs/` | Maps and VS Code: IndexedDB (S8) |
| Service workers | Service Workers | Staged | WPT `service-workers/` | Maps and VS Code (S8) |
| Fetch and HTTP | WHATWG Fetch (with ORB, Fetch pull request #1442), MIME Sniffing; HTTP semantics, HTTP/1.1, HTTP/2, HTTP/3, caching; QUIC; TLS 1.3; HSTS; Happy Eyeballs; Extensible Priorities | To the specification ([networking](architecture/networking.md)) | WPT `fetch/`, `mimesniff/`, `xhr/`, `cors/`, `beacon/`; the Rust stack's own tests | Every page |
| Cookies and partitioning | RFC 6265 and its revision (`draft-ietf-httpbis-rfc6265bis-22`); Client-Side Storage Partitioning; the Storage Access API | To the specification; storage access staged with logins | WPT `cookies/`, `storage-access-api/` | Every page; logins (CO8) |
| Security policies | Content Security Policy 3, Referrer Policy, Mixed Content, `X-Frame-Options`, COOP and COEP, Permissions Policy | To the specification | WPT `content-security-policy/`, `referrer-policy/`, `mixed-content/`, `x-frame-options/`, `html/cross-origin-opener-policy/`, `html/cross-origin-embedder-policy/`, `permissions-policy/` | Every page |
| WebSockets | WebSocket (WHATWG), over HTTP/1.1 and HTTP/2 | Staged | WPT `websockets/` | Twitch's chat, Discord (inference) |
| WebTransport | WebTransport | Staged | WPT `webtransport/` | None on the automated pages (S8) |
| Accessibility | WAI-ARIA, Core-AAM 1.2, HTML-AAM, Accessible Name and Description 1.2 | To the specification ([accessibility](architecture/accessibility.md)) | WPT `wai-aria/`, `html-aam/`, `core-aam/`, `accname/` | Every page |
| Canvas 2D | HTML §4.12.5, `OffscreenCanvas` | Staged | WPT `html/canvas/` | MDN, Maps, Three.js, VS Code (S8) |
| Images | PNG (with APNG), JPEG, GIF, WebP, AVIF, JPEG XL, BMP, ICO; ICC profiles, CICP ([images and media](architecture/images-and-media.md)) | To the specification | WPT `html/semantics/embedded-content/the-img-element/`, `css/css-images/`; each format's conformance images | JPEG, PNG, GIF, SVG and WebP on the automated pages; no AVIF among them (S8) |
| Media | HTML media elements, Media Source Extensions 2, Web Audio 1.1, WebCodecs, WebVTT; the codecs the corpus needs | Staged | WPT `media-source/`, `webaudio/`, `webcodecs/`, `html/semantics/embedded-content/media-elements/`, `webvtt/` | YouTube, Twitch, Instagram (inference) |
| WebGL | WebGL 1 and 2 | Staged, with the GPU process ([graphics](architecture/graphics.md)) | Khronos's WebGL conformance tests; WPT `webgl/` | Maps (S8) |
| WebGPU | WebGPU, WGSL | Staged, with the GPU process | The WebGPU conformance test suite (CTS) | Three.js (S8); Maps probed it (S8) |
| WebAssembly | WebAssembly 2 and 3; its JavaScript and Web APIs; JSPI | Staged; how it is built is OQ-3 | WebAssembly's specification tests; WPT `wasm/` | Maps and VS Code (S8); Figma (inference) |
| Logins | Credential Management, WebAuthn (passkeys, OQ-16), FedCM | Staged with the login track (CP8) | WPT `credential-management/`, `webauthn/`, `fedcm/` | Logins (CO8) |
| Clipboard, fullscreen, notifications, permissions | Clipboard API, Fullscreen, Notifications, Permissions, Page Visibility, Screen Wake Lock | Staged, each when the corpus needs it | WPT `clipboard-apis/`, `fullscreen/`, `notifications/`, `permissions/`, `page-visibility/`, `screen-wake-lock/` | Discord, VS Code (inference) |
| WebRTC and media capture | WebRTC, Media Capture and Streams | Staged | WPT `webrtc/`, `mediacapture-streams/` | None on the automated pages (S8); Discord's voice channels would need them (inference) |

### 5.3 Not planned

| Feature | Reason |
|---|---|
| Encrypted Media Extensions and DRM | The charter's non-goal: access depends on licensing, not engineering |
| Plugins (NPAPI) | Obsolete; HTML lets a user agent support no plugins, which is the class "to the specification" for `<embed>` and `<object>` without them |
| Application Cache, Web SQL | Removed from HTML; Web SQL was never standardized |
| Device APIs implemented by one engine only: WebUSB, WebHID, Web Serial, Web Bluetooth, Web NFC; the File System Access API's pickers | No specification with implementations in more than one engine, and large attack surfaces. The origin private file system, a WHATWG standard, is in scope (§5.2). VS Code for the Web works without the pickers, with less access to local folders (inference, to be checked) |
| WebXR, Gamepad, Web MIDI, the Web Speech API | No use in the corpus, and hardware or services this Mac's single user doesn't need. Revisit on corpus evidence |
| Push API | Needs a push service run by a browser vendor. Notifications while a page is open are staged with the rest (§5.2) |
| Browser extensions, sync and accounts | Charter non-goals |

## 6. Pass rates

| # | Rule |
|---|---|
| SB7 | **The suites:** WPT at the pinned revision (SB2); html5lib-tests' tokenizer tests; test262; Unicode's conformance files; and, at their milestones, WebGPU's CTS, Khronos's WebGL tests and WebAssembly's specification tests. Each runs under the harness CP5 designs |
| SB8 | **Expected results are recorded test by test,** in wptrunner's metadata format [T2]: one `.ini` file for each test whose result isn't a plain pass, with subtests, known intermittent results and conditions. They are kept in the repository beside the code they describe. test262's and html5lib-tests' expectations are kept as lists of known failures in the same spirit. A changed expectation is reviewed like code, and a pass that becomes a failure needs a stated reason |
| SB9 | **Each milestone selects its directories** from §5's areas that it completes or starts. Milestone 1's selection is CP9's |
| SB10 | **The measure:** for a WPT directory, the share of its results that pass: subtests for testharness tests, and the test itself for reference and crash tests, as wpt.fyi counts them [T1]. Tests the milestone doesn't run at all count as failures |
| SB11 | **Targets:**<br>• An area **completed** at a milestone targets, on each of its directories, **the median of Chrome's, Firefox's and Safari's pass rates** on that directory, at the pinned WPT revision, rounded down to a whole percent. The median of the three is what the web can rely on: it discounts tests of features only one engine ships.<br>• An area **started** at a milestone may have a lower, interim target, stated with its reason in the milestone's plan.<br>• **Approximate** areas are measured and reported, never targeted.<br>• **No test that passed may regress** without its expectation being changed with a reason (SB8) |
| SB12 | **What Axiom claims.** An area is "to the specification" in Axiom's documents only when it meets its target and holds no approximate behaviour. Status documents report each area's class and its pass rates, never one overall figure |

**Calibration** (wpt.fyi's runs of 2026-10-19 at WPT `0f26972422`, and Ladybird's of 2026-10-20 at `a26422aab3` [T1]; computed from their summary files):

| Directory | Tests | Chrome 155 | Firefox 158 | Safari 27.0 | **Median, the target (SB11)** | Servo 0.7.0 | Ladybird 1.0 |
|---|---|---|---|---|---|---|---|
| `encoding/` | 1,345 | 100.0% | 100.0% | 100.0% | **100.0%** | 100.0% | 100.0% |
| `url/` | 77 | 94.3% | 89.3% | 99.9% | **94.3%** | 90.0% | 99.9% |
| `html/syntax/parsing/` | 245 | 99.2% | 96.3% | 96.0% | **96.3%** | 96.1% | 96.4% |
| `dom/` | 845 | 99.7% | 98.4% | 98.2% | **98.4%** | 98.0% | 98.4% |
| `html/semantics/` | 3,089 | 94.4% | 87.4% | 82.3% | **87.4%** | 64.1% | 83.7% |
| `custom-elements/` | 202 | 98.3% | 92.7% | 82.4% | **92.7%** | 84.6% | 91.8% |
| `shadow-dom/` | 359 | 97.7% | 98.2% | 98.2% | **98.2%** | 96.5% | 96.6% |
| `editing/` | 860 | 92.6% | 95.3% | 90.1% | **92.6%** | 79.5% | 89.6% |
| `css/css-syntax/` | 52 | 98.2% | 77.3% | 99.3% | **98.2%** | 74.5% | 92.8% |
| `css/css-cascade/` | 164 | 98.5% | 97.6% | 98.1% | **98.1%** | 77.8% | 95.8% |
| `css/selectors/` | 833 | 83.5% | 93.4% | 96.7% | **93.4%** | 65.5% | 96.6% |
| `css/css-values/` | 519 | 90.4% | 57.4% | 59.9% | **59.9%** | 55.2% | 60.9% |
| `css/cssom/` | 296 | 96.9% | 97.4% | 96.0% | **96.9%** | 84.6% | 89.9% |
| `css/css-conditional/` | 468 | 96.4% | 95.0% | 93.9% | **95.0%** | 72.7% | 93.6% |
| `css/CSS2/` | 6,364 | 99.9% | 98.9% | 98.9% | **98.9%** | 91.1% | 99.2% |
| `css/css-flexbox/` | 1,421 | 99.3% | 96.8% | 95.8% | **96.8%** | 68.7% | 88.2% |
| `css/css-grid/` | 2,516 | 91.0% | 90.2% | 94.0% | **91.0%** | 64.0% | 76.9% |
| `css/css-tables/` | 347 | 95.6% | 89.5% | 84.6% | (approximate: measured, not targeted) | 83.9% | 75.1% |
| `css/css-position/` | 391 | 99.7% | 91.4% | 90.3% | **91.4%** | 72.3% | 84.5% |
| `css/css-overflow/` | 932 | 92.2% | 65.9% | 55.1% | **65.9%** | 49.8% | 61.8% |
| `css/css-text/` | 1,982 | 88.1% | 87.5% | 78.0% | **87.5%** | 72.9% | 64.1% |
| `css/css-fonts/` | 561 | 96.1% | 95.5% | 95.5% | **95.5%** | 68.6% | 79.3% |
| `css/css-writing-modes/` | 1,237 | 93.8% | 93.8% | 72.6% | **93.8%** | 31.5% | 58.5% |
| `css/css-backgrounds/` | 875 | 95.1% | 95.6% | 96.4% | **95.6%** | 74.8% | 93.3% |
| `css/css-color/` | 380 | 89.4% | 90.4% | 92.7% | **90.4%** | 90.1% | 87.6% |
| `css/css-images/` | 510 | 99.1% | 97.5% | 97.6% | **97.6%** | 93.7% | 93.6% |
| `css/css-transforms/` | 931 | 99.8% | 98.1% | 97.8% | **98.1%** | 65.3% | 83.3% |
| `css/css-animations/` | 207 | 92.2% | 76.2% | 93.3% | **92.2%** | 49.6% | 71.2% |
| `web-animations/` | 207 | 95.3% | 95.8% | 95.9% | **95.8%** | 7.8% | 88.3% |
| `css/cssom-view/` | 246 | 85.2% | 86.6% | 71.5% | **85.2%** | 65.8% | 75.0% |
| `svg/` | 1,405 | 96.9% | 97.8% | 92.9% | **96.9%** | 38.1% | 65.6% |
| `mathml/` | 623 | 97.7% | 83.1% | 85.6% | **85.6%** | 23.9% | 45.6% |
| `webidl/` | 63 | 93.8% | 96.4% | 94.2% | **94.2%** | 95.1% | 99.3% |
| `html/semantics/scripting-1/` | 596 | 95.5% | 86.5% | 76.3% | **86.5%** | 83.0% | 84.3% |
| `html/browsers/` | 792 | 84.6% | 81.2% | 80.4% | **81.2%** | 63.6% | 72.1% |
| `workers/` | 336 | 92.5% | 97.5% | 96.6% | **96.6%** | 98.5% | 92.0% |
| `IndexedDB/` | 820 | 99.9% | 99.8% | 88.4% | **99.8%** | 71.5% | 95.4% |
| `service-workers/` | 334 | 92.0% | 90.3% | 91.8% | **91.8%** | 7.2% | 34.3% |
| `fetch/` | 957 | 94.0% | 90.7% | 87.0% | **90.7%** | 84.7% | 87.0% |
| `cookies/` | 89 | 96.7% | 93.6% | 80.5% | **93.6%** | 85.3% | 93.3% |
| `content-security-policy/` | 930 | 88.9% | 81.0% | 79.3% | **81.0%** | 77.2% | 68.5% |
| `websockets/` | 803 | 91.5% | 74.2% | 74.0% | **74.2%** | 73.1% | 69.3% |
| `wai-aria/` | 35 | 97.4% | 95.2% | 92.9% | **95.2%** | 44.2% | 92.7% |
| `html/canvas/` | 3,933 | 86.4% | 76.4% | 86.1% | **86.1%** | 67.9% | 69.9% |
| `media-source/` | 86 | 82.4% | 69.9% | 93.7% | **82.4%** | 3.9% | 64.8% |
| `wasm/` | 615 | 99.8% | 99.9% | 97.8% | **99.8%** | 99.6% | 99.6% |
| `storage/` | 56 | 97.5% | 33.0% | 32.6% | **33.0%** | 36.5% | 29.6% |
| `quirks/` | 51 | 99.9% | 99.9% | 99.9% | **99.9%** | 99.8% | 98.9% |

**What the calibration shows:**
- **"Tests"** counts the directory's test files in the runs. The figures are the share of results that pass, computed with `tools/wpt-pass-rates.py` from the five runs' summary files, downloaded into `target/`. The other directories in §5 are in the same computation.
- **The median discounts tests of one engine's features.** In `storage/`, Chrome passes 97.5% and the others about 33%: the directory tests features that only Chromium ships (inference, from the split). The median, 33.0%, is what the web can rely on. `css/css-values/` shows the same pattern, with Chrome at 90.4% and the others near 58%.
- **A new engine can reach the targets.** Ladybird, an engine written from scratch (background), meets or passes the shipping median on `html/syntax/parsing/` (96.4%), `dom/` (98.4%), `css/CSS2/` (99.2%), `css/selectors/` (96.6%) and `webidl/` (99.3%) (measured). So SB11's targets are demanding, and reachable.
- **Where both new engines lag most:** `service-workers/`, `svg/`, `mathml/` and `css/css-writing-modes/`, with `web-animations/` for Servo. Most are staged areas in §5, which reach their targets at their milestones.
- **WebGPU's conformance suite** isn't among wpt.fyi's runs (`webgpu/` shows no results), so it is run from its own repository at its milestone (SB7).

## 7. Staging

The roadmap (CP8) places each staged area at the milestone where its capability arrives. Two orders follow from the matrix (inference):
- **Documents before script.** Encoding, URLs, HTML parsing, the DOM, XML, style, block, inline and table layout, text, fonts, Unicode, painting and images make the documents tier, which needs no script (Wikipedia's article reads without it). Script brings Web IDL, scripting, events, observers, animation, canvas 2D, storage and workers.
- **The GPU process before WebGL and WebGPU; media before YouTube and Twitch; WebAssembly before Figma.** These match the corpus's tiers ([corpus](corpus.md) §8).

## 8. Open questions

1. **Pins and wpt.fyi.** wpt.fyi aligns its stable runs on one revision each day, so a milestone that pins that day's revision gets calibration for free. Whether to pin only revisions that wpt.fyi has aligned runs for is CP5's to settle.
2. **Unicode's lag.** ICU4X implements 17.0.0 while 18.0.0 is out. Waiting for ICU4X keeps the engine on one version (SB1). Whether a lag of one version is acceptable is the owner's call when a milestone pins it.
3. **The not-planned list** (§5.3) is inference from the corpus and the charter. The owner may move an entry, at any gate.

## Sources

All read on 2026-10-20.

- [W1] WHATWG, *Working Mode* — https://whatwg.org/working-mode
- [W2] WHATWG, *Workstream Policy*, "Review Drafts" — https://whatwg.org/workstream-policy ; *IPR Policy*, effective 2018-01-11 — https://whatwg.org/ipr-policy
- [W3] WHATWG, HTML's Review Drafts (2018-07 to 2026-07) — https://html.spec.whatwg.org/review-drafts/
- [P1] W3C, *Process Document*, 18 August 2025, §6.3.1, "Maturity Stages on the Recommendation Track" — https://www.w3.org/policies/process/20250818/
- [C1] W3C CSS Working Group, *CSS Snapshot 2026*, Group Note, 22 June 2026 — https://www.w3.org/TR/css-2026/
- [E1] Ecma International, ECMA-262, 17th edition, *ECMAScript 2026*, June 2026 — https://262.ecma-international.org/17.0/
- [E2] Ecma International, ECMA-402, 13th edition, June 2026 — https://402.ecma-international.org/13.0/
- [U1] Unicode, *Unicode 18.0.0*, release date 2026-09-16 — https://www.unicode.org/versions/Unicode18.0.0/
- [U2] Unicode, *Enumerated Versions* — https://www.unicode.org/versions/enumeratedversions.html
- [B1] web-features, `docs/baseline.md`, maintained by "The WebDX community group" — https://github.com/web-platform-dx/web-features/blob/main/docs/baseline.md
- [B2] web-features, release v3.43.0, 2026-10-15, `data.json` — https://github.com/web-platform-dx/web-features/releases/tag/v3.43.0
- [I1] Interop 2026, `2026/README.md` — https://github.com/web-platform-tests/interop/blob/main/2026/README.md
- [T1] wpt.fyi, its runs API and the runs' summary files — https://wpt.fyi/api/runs
- [T2] web-platform-tests, *Expectation Data* (wptrunner) — https://web-platform-tests.org/tools/wptrunner/docs/expectation.html
- [T3] web-platform-tests, `tools/wptrunner/wptrunner/browsers/ladybird.py` — https://github.com/web-platform-tests/wpt/blob/master/tools/wptrunner/wptrunner/browsers/ladybird.py
- [T4] html5lib-tests, commits `224991ec10db` (2026-06-26) and `c777c408b610` (2026-10-01) — https://github.com/html5lib/html5lib-tests/commits
