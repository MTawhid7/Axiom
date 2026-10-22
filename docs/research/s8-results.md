# S8 Results: the Corpus Census

| | |
|---|---|
| **Status** | **Complete** (2026-10-20). **In Chrome:** the five corpus pages whose sites' terms permit automated loads, three valid runs each (§3–§6), and the other eight signed in, by the owner's hand, three runs each (§7). **In Safari:** the ten pages that work logged out, by the owner's hand with the corrected tool, three runs each (§7.4), recorded at Step 6's Gate C. The first Safari session is superseded, since the tool's protocol kept earlier pages open. Not run: Chrome logged out by hand, and Safari signed in (§7.5). **The build** wasn't recorded (macOS 27.2) (Round 4, RV4-13) |
| **Question** | What does each corpus page use and cost in shipping engines on this Mac? Its features (S8-Q1), size (S8-Q2), memory (S8-Q3), and load milestones and cost without input (S8-Q4) |
| **Feeds** | The [corpus](../corpus.md)'s baselines; the [standards baseline](../standards.md)'s priorities (CP4); the [performance budgets](../performance.md) (CP6) |
| **Plan** | [Step 6 tracker](step-6-standards-budgets-roadmap.md) §10.1, authorized at Gate A (2026-10-20), as refined at CP2 to the sites' terms ([corpus](../corpus.md) CO3–CO6) |
| **Code** | `spikes/s8/` (Python, standard library only) and `tools/s8-measure.sh`, at `6ba5068` for the formal run and `df7b179` for the rerun; at `cbf8559` for the session by hand; `s8.py blank` and the corrected session by hand at `c914044`, which Safari's second session ran (at `84254cf`); `tools/s8-report.py` for the tables (§10) |
| **Last updated** | 2026-10-20 |

**Evidence labels** follow the [research conventions](README.md):
- **Measured** facts come from the census's records, in the runs named in §2.
- **Observed** facts were read on this Mac, naming the file or command.
- **Sourced** facts link to their source.
- **Inference** marks an explanation drawn from them.

MB means 10⁶ bytes, as B1 and S6 use it. Every figure is a median of three valid runs, with the range in brackets.

---

## Summary

- **Automated: five pages, three valid runs each,** in Chrome Canary 158.0.8112.0: Wikipedia's article on π, Google Maps, MDN's canvas tutorial, Three.js's WebGPU example and VS Code for the Web. Two runs had the page hidden and were replaced; the two sites' fifth loads are the corpus's limit (CO4).
- **Features (S8-Q1).** Chrome's own use counters name 86–316 platform features per page. Among the 116 features of the web-features project that the five pages touch:
  - 72 are Baseline "high", widely available;
  - 19 are "low", newly available;
  - 25 are in only some engines (§3).

  Wikipedia's article carries 126 MathML formulas and loads 164 SVG images. MDN builds its page from 98 custom elements with open shadow roots. Maps draws with WebGL and WebGL 2, runs WebAssembly, workers and a service worker, and probes WebGPU. Three.js uses WebGPU. VS Code uses WebAssembly, five workers and IndexedDB.
- **Size (S8-Q2).** From 24 requests and 0.84 MB (Three.js) to 2,449 requests and 9.0 MB (VS Code). Wikipedia's DOM has 12,828 elements and 45,815 nodes; the others' 456–2,695 elements.
- **Memory (S8-Q3).** All of Chrome's processes together, once the page had been steady for 30 s, cost above the same browser on about:blank, leaving out three component-extension renderers of Chrome's own that exit in between:
  - Maps 627 MB;
  - Three.js 470 MB;
  - VS Code 457 MB;
  - Wikipedia 319 MB;
  - MDN 145 MB.

  The GPU process carries much of it on Maps (342 MB) and Three.js (443 MB), against 34–38 MB on about:blank. A fresh Canary on about:blank costs 250–266 MB.
- **Time and energy (S8-Q4).** First contentful paint at 0.8–2.3 s. In the 30 s after the interaction, the static pages used 0.9–2.4 s of CPU across all processes. Three.js's simulation used 15.4 s, about half a core, at an energy impact of 47.6.
- **By hand, signed in** (§7), in Chrome Canary 158.0.8113.0, with the owner's secondary accounts. Each page's steady-state cost above a fresh Canary on about:blank, leaving out extension renderers:
  - Reddit 1,260 MB, Figma 1,111, YouTube 908;
  - Google Search 582, Instagram 578, Prism 494, Discord 427, Twitch 401.

  Runs driven by hand vary more than automated ones: Reddit's ran from 495 to 1,631 MB. Signed in, the content pages cost as much as the applications.
- **By hand, logged out, in Safari** (§7.4), in Safari 27.2 with the corrected tool: ten pages, three runs each. Each page's steady-state cost above the same fresh Safari on the census's list page:
  - Google Maps 1,880 MB, YouTube 1,624, VS Code 1,076;
  - Twitch 795, Reddit 736, Google Search 568, Three.js 433, Wikipedia 385, Instagram 278, MDN 181.

  The owner went beyond each page's listed interaction, with tasks such as MDN's live code demos and Street View in Maps. So these figures aren't like for like with Chrome's automated runs, whose interaction was scripted. Neither browser's costs bound the other's. Safari's figures don't move the ceilings, which the accepted rule takes from Chrome. A first Safari session, earlier that evening, is superseded: the tool's protocol kept earlier pages open.

## 1. What was built

- **A DevTools Protocol client** over Chrome's `--remote-debugging-pipe`: JSON messages ended by a NUL byte on file descriptors 3 and 4, the default of Chromium's pipe handler. No port is opened, so no other program can attach.
- **The automated run** of one page, in a fresh profile under `target/`, deleted afterwards:
  1. the processes sampled on about:blank after 10 s, the baseline;
  2. the page loaded, waiting for the load event and 2 s without network activity, at most 30 s;
  3. 30 s of the page's interaction (CO10);
  4. 30 s without input, with `top` sampling every process each second.

  `footprint` measures every Chrome process after each phase. GoogleUpdater, which Canary launches as a child, is left out, since it serves no page.
- **Features** from Chrome's use counters, read through `Browser.getHistograms` with `delta`, so each page's counts are its own. Nothing is injected into the page. The histograms are `Blink.UseCounter.Features`, `CSSProperties`, `AnimatedCSSProperties` and `WebDXFeatures`. The last names features by the web-features project's identifiers: "canvas-2d" is `kCanvas_2d`, as Chromium's `webdx_feature.mojom` defines [C1]. The names come from Chromium's mojom files at Canary's version.
- **Sizes** come from the Network domain's events, and from the DOM read in an isolated world, a script context the page can't see. **Timings** come from the page's Navigation Timing, Paint Timing and Largest Contentful Paint entries, read in the same isolated world.
- **Automation identifies itself.** It appends `AxiomCensus/0.1 (+https://github.com/MTawhid7/Axiom)` to the user agent, which a local server confirmed arrives in the `User-Agent` header, and sets `navigator.webdriver`. It reads each site's robots.txt first, and refuses a disallowed page (CO4). The five pages were allowed (§2).
- **A local test page,** served on 127.0.0.1, checks the method. Every feature it uses appeared among the counters: a custom element with an open shadow root, canvas 2D, WebGL, a worker, IndexedDB, two observers, Web Animations and `fetch`.

## 2. Method and environment

| | |
|---|---|
| **This Mac** | MacBook Air (M1, 8 GB), macOS 27.2; AC power, Low Power Mode off (`pmset`); the display scaled at 1,440 × 900 points (2,560 × 1,600 panel); dark appearance |
| **Browser** | Chrome Canary 158.0.8112.0, from its `Info.plist`; the same version in every run |
| **Flags** | `--enable-automation --use-mock-keychain --disable-background-networking --disable-sync --no-first-run --no-default-browser-check`, a window of 1,280 × 800 points at the top left, and the census's user agent. The mock keychain keeps the owner's keychain out of it |
| **Runs** | The formal run, 12:24–13:01, at `6ba5068`: 5 pages × 3 runs, each after a 60 s cool-down. The rerun, 13:02–13:09, at `df7b179`: one run each of Wikipedia and Three.js, replacing the two hidden runs. Raw records: `target/s8-results/20261020-122447-auto/` and `20261020-130253-auto/`, which git ignores |
| **Loads per site** | Wikipedia and Three.js: five (the quick check, three formal runs, the rerun), the limit. Maps, MDN and VS Code: four. Each site's robots.txt allowed its page: Maps by `Allow: /maps/search/`, Three.js by its absence (404), the others by no matching rule |
| **Load average** | 2.77 at the formal run's start. After each run 2.3–11.3, a figure that includes the browser's own work in the run just ended (inference). Before each rerun: 2.26 (Wikipedia) and 4.65 (Three.js). Memory free: 49–71% |
| **Window** | Visible in 15 of 17 runs. In the formal runs, Wikipedia's run 2 and Three.js's run 1 reported `document.visibilityState` as hidden. Wikipedia's run 2 had no first paint, its interaction failed (an input event unanswered after 10 s), and its GPU process held 49 MB against 118–127 MB. Both were left out, the tools were changed to check visibility at every phase, and the two pages were measured again. The window was never focused, since another app was active (observed: `document.hasFocus()` false) |

**What each phase measures:**
- **Loaded:** after the load event and 2 s of network quiet. That came at 4.4–14.4 s after navigation, never at the 30 s limit.
- **After interaction:** right after 30 s of the page's interaction.
- **After 30 s without input:** the window starts as the interaction ends, so it includes the page settling. For example, Wikipedia's GPU process used 0.9 s of CPU in it after the scrolling.

## 3. S8-Q1: features

| Page | Chrome features used | CSS properties | S8-Q1's groups (features matched) | web-features touched | Baseline high / low / not Baseline |
|---|---|---|---|---|---|
| Wikipedia | 169 | 173 | observers (3), image formats (1), MathML (2) | 39 | 30 / 5 / 4 |
| Google Maps | 316 | 197 | canvas 2D (6), WebGL (5), WebGL 2 (3), WebGPU (1), OffscreenCanvas (3), WebAssembly (2), workers (2), service workers (1), IndexedDB (2), observers (4), font loading (1) | 54 | 33 / 8 / 13 |
| MDN | 162 | 122 | custom elements (1), shadow DOM (3), canvas 2D (1), observers (3), font loading (1), JavaScript modules (3), dialog and popover (1) | 51 | 36 / 8 / 7 |
| Three.js | 86 | 100 | canvas 2D (2), WebGPU (5), observers (2), JavaScript modules (3) | 23 | 16 / 5 / 2 |
| VS Code for the Web | 268 | 175 | custom elements (1), canvas 2D (2), WebAssembly (2), workers (4), IndexedDB (3), observers (3), font loading (1), JavaScript modules (3) | 60 | 39 / 7 / 14 |

Baseline status is from web-features 3.43.0 (2026-10-15) [W1]. A group's count is the number of Chrome's features that matched it.

**The features that aren't Baseline "high"** (measured; status sourced):
- **Low, newly available (19):**
  - CSS: `backdrop-filter`, `light-dark`, `registered-custom-properties`, `relative-color`, `round-mod-rem`, `scope`, `scrollbar-color`, `scrollbar-gutter`, `scrollbar-width`, `text-wrap`, `text-wrap-balance`;
  - HTML: `details-content`, `fetch-priority`;
  - script: `float16array`, `top-level-await`;
  - other: `ext-texture-compression-rgtc` (WebGL), `http3`, `largest-contentful-paint`, `reporting`.
- **Not Baseline (25):**
  - CSS: `background-clip-text`, `box-decoration-break`, `interpolate-size`, `marker`, `text-spacing-trim`, `prefers-reduced-transparency`, `display-mode`;
  - editing and HTML: `edit-context` (VS Code), `dialog-closedby`, `hidden-until-found` (Wikipedia), `referencetarget`;
  - script and APIs: `beforeinstallprompt`, `beforeunload`, `canvas-2d-alpha`, `clear-site-data`, `intersection-observer-v2`, `keyboard-map`, `manifest`, `requestidlecallback`, `ua-client-hints`, `window-controls-overlay`;
  - GPU: `ext-texture-compression-bptc`, `ext-texture-norm16`, `webgl-multi-draw`, `webgpu` (Maps and Three.js).
- **A use counter counts any use, feature detection included.** These sites also serve Safari and Firefox, which lack many of the features above, so most are enhancements with fallbacks (inference). For example, VS Code for the Web works in Safari without EditContext.

**Formats** (measured, from responses' MIME types):
- **images:** JPEG, PNG, GIF, SVG and WebP; no AVIF on these five pages;
- **web fonts:** WOFF2 on Maps (5) and MDN (2); WOFF and TrueType on VS Code; none on Wikipedia;
- **HTTP/3:** Maps fetched 313 of its 352 responses over it.

## 4. S8-Q2: size

| Page | Requests | Transferred (MB) | Hosts | Script | Images | Elements | DOM depth | Nodes (Chrome) | Documents (Chrome) | Frames' processes |
|---|---|---|---|---|---|---|---|---|---|---|
| Wikipedia | 239 (231–242) | 3.74 (3.74–3.76) | 5 | 9, 0.46 MB | 191, 2.99 MB | 12,828 | 61 | 45,815 | 151 | — |
| Google Maps | 344 (341–352) | 9.66 (9.14–9.85) | 10 | 24, 1.51 MB | 91, 2.86 MB | 1,359 | 29 | 1,986 | 2 | — |
| MDN | 89 | 0.43 | 5 | 36, 0.13 MB | 20, 0.11 MB | 2,695 | 16 | 11,421 | 18 | 3 frames |
| Three.js | 24 | 0.84 | 1 | 21, 0.83 MB | 0 | 461 | 16 | 826 | 4 | — |
| VS Code for the Web | 2,449 (2,449–2,450) | 8.96 (8.86–9.60) | 8 | 12, 6.70 MB | 3 | 656 | 44 | 2,136 | 4 | 1 frame |

**Notes:**
- "Script" and "Images" give requests and transferred bytes. "Nodes" and "Documents" are Chrome's counts after 30 s without input (`Performance.getMetrics`).
- **Wikipedia's 151 documents and 150 frames** match its 164 SVG images: each SVG image is a document of its own in Chrome (inference from the counts). The page holds 126 MathML `<math>` elements, most likely the same formulas as the images (inference).
- **MDN's page** holds 98 custom elements, each with an open shadow root (counted in the isolated world). Its three frames are cross-site, so each runs in its own process.
- **VS Code's** 2,367 `fetch` and XHR requests are 97% of its total.
- **JavaScript heaps** (Chrome's `JSHeapUsedSize`): VS Code 66.9 MB; Maps 29.2 MB; Three.js 10.5 MB; Wikipedia 7.8 MB; MDN 4.4 MB.

## 5. S8-Q3: memory

All of Chrome's processes, in MB (`footprint`'s total, shared memory counted once):

| Page | Loaded | After interaction | After 30 s without input | On about:blank | After 30 s − blank, without extension renderers | Renderers, loaded | GPU, loaded | GPU, after 30 s | Browser, loaded |
|---|---|---|---|---|---|---|---|---|---|
| Wikipedia | 544 (539–544) | 552 (542–553) | 523 (516–544) | 255 (254–257) | 319 (315–340) | 242 (242–244) | 123 (118–127) | 170 (157–188) | 99 |
| Google Maps | 1,167 (997–1,190) | 900 (897–912) | 830 (826–845) | 255 (250–256) | 627 (625–641) | 452 (440–460) | 554 (377–595) | 342 (339–351) | 93 |
| MDN | 409 (402–423) | 357 (355–366) | 350 (345–358) | 257 (252–266) | 145 (141–147) | 150 (148–150) | 94 (91–100) | 99 (99–102) | 97 |
| Three.js | 751 (738–755) | 690 (682–694) | 674 (664–675) | 259 (242–259) | 470 (466–476) | 146 (129–147) | 456 (451–457) | 443 (442–444) | 88 |
| VS Code for the Web | 771 (767–772) | 821 (812–830) | 665 (651–673) | 253 (250–259) | 457 (449–472) | 504 (503–508) | 54 (54–55) | 138 (136–141) | 110 |

- **A fresh Canary on about:blank** cost 250–266 MB in every run, across these processes:
  - the browser (75–85 MB);
  - three renderers (57–74 MB together);
  - **three component-extension renderers of Chrome's own** (48–55 MB together);
  - the GPU process (34–38 MB);
  - the network process (13–15 MB);
  - the storage process (6–10 MB).

  The extension renderers were still running when the page had loaded, and **had exited by the steady state** in every run. So the cost of a page above blank leaves them out on both sides: compared with their footprints included, every page's steady-state cost would read about 50 MB lower. footprint's total counts shared memory once, and their own footprints are subtracted from it, an approximation.
- **The renderers** include those for cross-site frames: four on MDN, five on VS Code, three elsewhere.
- **The GPU process** grew with the page's drawing: from 34–38 MB on about:blank to 342 MB on Maps and 443 MB on Three.js at the steady state (measured).
- **Chrome's network process** held 13–16 MB on every page.
- **The steady state** ("after 30 s without input") is what CP6's budgets apply to (PF4). Maps released about 340 MB between loading and its steady state, mostly from the GPU process.

## 6. S8-Q4: load milestones and the time without input

| Page | First contentful paint (ms) | Largest contentful paint (ms) | Load event (ms) | Wait ended | CPU, 30 s without input (s) | Idle wakeups | Energy impact | Renderers' energy impact |
|---|---|---|---|---|---|---|---|---|
| Wikipedia | 1,636 (1,056–2,832) | 1,636 (1,056–2,832) | 3,483 (2,209–4,578) | quiet | 2.36 (2.17–2.44) | 65 (47–90) | 7.2 (6.6–7.4) | 1.3 (1.2–1.5) |
| Google Maps | 1,944 (1,264–4,332) | 4,064 (3,512–5,484) | 1,920 (1,224–5,808) | quiet | 1.13 (0.98–1.28) | 8 (6–9) | 3.4 (2.9–3.8) | 0.1 (0.1–0.2) |
| MDN | 812 (288–1,044) | 812 (288–1,044) | 735 (251–983) | quiet | 0.89 (0.83–1.16) | 8 (6–10) | 2.7 (2.5–3.5) | 0.0 (0.0–0.1) |
| Three.js | 1,100 (448–1,200) | 1,100 (448–1,200) | 1,671 (735–1,755) | quiet | 15.41 (14.91–15.51) | 165 (42–288)* | 47.6 (45.9–48.4) | 13.2 (12.6–13.8) |
| VS Code for the Web | 2,259 (1,919–3,012) | 2,276 (1,936–3,029) | 1,998 (1,652–2,718) | quiet | 2.41 (1.90–2.46) | 44 (37–51)* | 7.3 (5.5–7.4) | 2.0 (1.6–2.2) |

\* **One run each is left out of the wakeup column.** In Three.js's run 3 and VS Code's run 3, `top` reported no idle wakeup for any process while they used CPU, which an animating page can't do, so the counter failed in those runs (inference). Their CPU time and energy impact are kept.

- **Load times are over this Mac's own network,** unthrottled, and vary with it; they are context (CO11).
- **Chrome's browser process used 0.45–0.9 s of CPU** in every 30 s window, whatever the page. That figure includes whatever an automation session costs Chrome (inference).
- **Maps is quiet once loaded:** its renderers used 0.04–0.08 s of CPU in the 30 s, with 1–2 wakeups.
- **Three.js's simulation keeps the GPU process busy:** 10.3–10.7 s of CPU in 30 s, with the renderer at about 4.4 s.

## 7. The census by hand

**Run by the owner on 2026-10-20,** after Gate B (the owner's decision 4), with secondary accounts in the dedicated profile: "I have used my secondary accounts for these tests." The tools only sampled the browsers' processes (CO6). Safari's second session, with the corrected tool, followed that evening, and was recorded at Gate C (the owner's decision 5).

| | Chrome Canary, signed in | Safari, logged out: the second session (§7.4) | Safari, logged out: the first session, superseded |
|---|---|---|---|
| **Pages** | Google Search, YouTube, Instagram, Reddit, Twitch, Discord, Figma, Prism | The ten that work logged out: Google Search, YouTube, Instagram, Reddit, Wikipedia, Google Maps, Twitch, MDN, Three.js, VS Code for the Web | Google Search, YouTube, Instagram, Reddit, Twitch |
| **Runs** | Three of each, 18:17–18:58 | Three of each, 22:24–23:21, a round of the ten pages at a time, with Safari quit and restarted for every run | Three of each, 19:01–19:25 |
| **Browser** | Chrome Canary 158.0.8113.0, the next build after the automated runs' 158.0.8112.0, which it updated itself to in between. The dedicated profile, no debugging connection, no automation flags | Safari 27.2 (22625.2.8.11.9), a Private window, except in one run of Reddit (§7.4) | Safari 27.2 (22625.2.8.11.9), a Private window |
| **Records** | `target/s8-results/20261020-181706-hand-chrome-signed-in/` | `target/s8-results/20261020-222359-hand-safari-logged-out/` | `target/s8-results/20261020-190125-hand-safari-logged-out/`. A first start at 18:59 recorded only its environment |
| **Conditions** | AC power, Low Power Mode off; load average 1.63 at the start and 1.5–4.7 after the runs; memory free 61–73% | The same power; load average 4.04 at the start, above PF1's 3.0, and 1.3–3.0 after each run; memory free 64–75% | The same power; load average 1.86 at the start and 1.3–4.4 after the runs; memory free 43–72% |

### 7.1 The baseline

The census by hand recorded no baseline of its own. So a fresh Chrome Canary with the same flags was measured on about:blank afterwards: three runs, 19:53–20:00 (`s8.py blank`; `target/s8-results/20261020-195301-blank-chrome-fresh/`). It measured:
- **10 s after launch:** 297–301 MB, including five component-extension renderers (91–93 MB), so **207 MB without them** (206–210);
- **90 s after launch:** 199–204 MB, with no extension renderer left. The runs by hand reached their steady state a median of 90 s after launch (from each record's start and write times).

A page's cost subtracts the 207 MB, the definition §5 uses for the automated runs.

**The baseline's profile is fresh, not the dedicated one,** so each cost includes the dedicated profile's own state. Chrome's browser process held 101–152 MB at the steady state by hand, against 90–92 MB at 90 s in the fresh profile. How much of the difference is the profile and how much the page can't be separated (inference).

**A first attempt opened the dedicated profile** on about:blank, with no page loaded and no debugging connection: two runs, 19:46–19:51. CO14 says that profile is never opened by automation, so the attempt was stopped and its records (`20261020-194649-blank-chrome-signed-in/`) aren't used. Since then:
- `s8.py blank` opens only a fresh profile;
- the census by hand records each run's own baseline, on the list page, before the owner opens the corpus page.

### 7.2 Chrome, signed in: memory

All of Chrome's processes, in MB (footprint's total, shared memory counted once):

| Page | Tier | Loaded | After interaction | After 30 s without input | Renderers, loaded | GPU, after 30 s | Browser, loaded | Cost above the baseline, after 30 s |
|---|---|---|---|---|---|---|---|---|
| Google Search | Content | 562 (553–585) | 846 (528–1,292) | 789 (517–1,053) | 298 (296–302) | 202 (115–256) | 108 | **582** (310–846) |
| YouTube | Media | 1,000 (957–1,026) | 1,221 (1,044–1,430) | 1,115 (1,063–1,119) | 585 (584–596) | 236 (138–241) | 113 | **908** (856–912) |
| Instagram | Content | 555 (548–583) | 1,247 (1,109–1,468) | 785 (691–1,001) | 314 (308–328) | 388 (137–391) | 103 | **578** (484–794) |
| Reddit | Content | 820 (812–896) | 1,675 (1,089–2,174) | 1,467 (702–1,838) | 484 (474–489) | 311 (191–365) | 108 | **1,260** (495–1,631) |
| Twitch | Media | 845 (828–861) | 762 (619–1,010) | 608 (599–783) | 468 (463–478) | 119 (118–170) | 104 | **401** (391–576) |
| Discord | Applications | 795 (758–1,273) | 862 (632–1,259) | 634 (617–1,029) | 474 (469–772) | 120 (114–211) | 111 | **427** (410–822) |
| Figma | Applications | 1,882 (1,820–1,919) | 1,587 (1,367–1,855) | 1,319 (1,234–1,366) | 1,283 (1,222–1,297) | 291 (285–358) | 113 | **1,111** (1,027–1,159) |
| Prism | Applications | 778 (438–852) | 855 (812–923) | 715 (669–737) | 349 (213–398) | 241 (205–246) | 105 | **494** (462–508) |

- **Interaction by hand varies from run to run.** A feed loads as much as the hand scrolls: Reddit's steady state ran from 702 to 1,838 MB across its runs, Google's from 517 to 1,053 MB.
- **Signed-in content pages cost as much as the applications.** Reddit's median cost, 1,260 MB, is above Figma's 1,111 MB; Google Search's and Instagram's are above Discord's and Prism's.
- **The GPU process** held 119–388 MB at the steady state, against 28–35 MB on the fresh about:blank. It held the most on Instagram and Reddit, feeds of images and video (inference).
- **Renderers:** three to seven per page, counting cross-site frames' and the spare. Figma's held 1.3 GB when loaded, and Reddit's 1.5 GB after scrolling in its second run, each as the sum of five processes. The census by hand didn't record each process's own footprint; it now does, as the automated census did (B7, [performance](../performance.md) PF12).
- **Figma released about 560 MB** between loading and its steady state, much of it from its renderers.
- **A component-extension renderer appeared once,** in Prism's third run (36–40 MB), and is left out of the cost like the others.

### 7.3 Chrome, signed in: the 30 s without input

| Page | CPU time over 30 s (s) | Idle wakeups | Energy impact | Renderers' energy impact |
|---|---|---|---|---|
| Google Search | 3.03 (0.26–4.15) | 51 (13–190) | 9.4 (0.5–12.6) | 6.3 (0.2–6.3) |
| YouTube | 14.07 (13.43–14.41) | 233 (194–266) | 44.2 (42.4–45.2) | 20.5 (19.7–31.6) |
| Instagram | 2.34 (1.84–2.61) | 34 (31–160) | 6.9 (5.6–8.0) | 4.3 (4.0–4.7) |
| Reddit | 10.77 (3.06–23.00) | 97 (25–521) | 34.0 (9.3–71.5) | 22.4 (6.9–45.0) |
| Twitch | 13.02 (12.33–16.65) | 348 (154–538) | 41.0 (38.5–52.7) | 16.8 (14.4–26.3) |
| Discord | 1.25 (0.72–2.08) | 13 (7–25) | 3.6 (2.1–6.4) | 2.2 (0.8–4.2) |
| Figma | 4.38 (2.75–4.95) | 223 (152–333) | 13.6 (8.6–15.8) | 9.9 (6.2–12.6) |
| Prism | 9.98 (2.21–11.30) | 50 (42–56) | 31.0 (6.7–35.0) | 16.6 (4.1–20.0) |

- **YouTube and Twitch play video throughout,** so they are in a steady state rather than idle (CO9): about half a core each.
- **Prism and Reddit kept working without input,** about a third of a core at the median, with a wide spread on Reddit.
- **Discord was the quietest:** 1.25 s of CPU and 13 wakeups in 30 s.

### 7.4 Safari, logged out

**The second session, with the corrected tool,** 22:24–23:21, folded in at Gate C on the owner's decision 5. Safari was quit and restarted for every run, so each run's records hold one page. Each run's baseline is the census's list page, 10 s after a fresh Safari opened it in a Private window. All of Safari's processes, in MB (footprint's total, shared memory counted once):

| Page | Tier | Loaded | After interaction | After 30 s without input | Renderers, loaded | GPU, after 30 s | Cost above the baseline, after 30 s |
|---|---|---|---|---|---|---|---|
| Google Search | Content | 532 (451–594) | 864 (793–917) | 829 (790–859) | 289 (220–346) | 24 (20–26) | **568** (534–601) |
| YouTube | Media | 1,254 (1,194–1,408) | 2,131 (1,947–2,245) | 1,884 (1,790–1,906) | 966 (902–1,087) | 324 (320–327) | **1,624** (1,533–1,648) |
| Instagram | Content | 522 (506–597) | 603 (597–741) | 536 (536–586) | 289 (276–353) | 18 (18–20) | **278** (277–329) |
| Reddit | Content | 644 (379–660) | 1,082 (938–1,807) | 982 (836–1,325) | 390 (281–405) | 36 (33–76) | **736** (724–1,068) |
| Wikipedia | Documents | 504 (481–510) | 653 (610–750) | 646 (585–650) | 257 (240–259) | 36 (24–45) | **385** (327–390) |
| Google Maps | Media and maps | 1,362 (865–1,414) | 2,165 (1,459–2,518) | 2,137 (1,258–2,479) | 1,064 (611–1,116) | 48 (41–50) | **1,880** (998–2,219) |
| Twitch | Media | 1,045 (948–1,063) | 974 (971–1,005) | 1,053 (1,025–1,066) | 752 (670–770) | 103 (101–107) | **795** (766–806) |
| MDN | Documents | 378 (372–392) | 441 (437–507) | 440 (403–506) | 147 (143–150) | 28 (17–29) | **181** (145–247) |
| Three.js | Applications | 699 (689–701) | 699 (688–706) | 692 (687–710) | 431 (426–431) | 55 (55–70) | **433** (430–450) |
| VS Code for the Web | Applications | 936 (742–1,031) | 1,362 (1,012–1,400) | 1,336 (965–1,600) | 582 (508–684) | 29 (17–30) | **1,076** (708–1,341) |

- **The baseline** held 257–261 MB in 29 of the 30 runs: Safari's own process 203–207 MB, its GPU process 11 MB, its network process 6 MB, and two WebContent processes 36–38 MB.
- **The owner went beyond each page's listed interaction** (CO10): "for all sites, I ran some advanced browsing tasks such as running live code demo in mdn, street view in maps, etc." So these figures measure heavier use than the corpus's interactions and Chrome's automated runs, whose interaction was scripted (§2). Maps' renderers held 1.9 and 2.2 GB after 30 s without input in two of its runs, and 1.0 GB in the third.
- **One run of Reddit had no Private window:** "In the reddit case, I forgot to open the private window." The owner adds that this Safari is signed in to nothing and has no extension, so a regular window differs little from a Private one for the page. Only Reddit's third run differs in the record, so it is most likely that run (inference). There, Safari's own process held 45–61 MB, against 203–230 MB in the other 29 runs, and the baseline came to 100 MB. Its cost subtracts its own baseline, so it stays comparable: 737 MB, against 724 and 1,068 MB. **Inference, from that one run:** the Private window accounts for about 160 MB of Safari's own process, which cancels out of every cost.
- **Video keeps its memory on the GPU:** YouTube's GPU process held 320–327 MB after 30 s, and Twitch's 101–107 MB; the other pages' 18–55 MB at the median.
- **VS Code's data went to the network process.** Safari's network process grew from 6 MB to 233–241 MB with VS Code for the Web, against 7–22 MB on every other page. WebKit's network process hosts web storage, as Axiom's does (SW1), so this is most likely VS Code's IndexedDB and caches (inference). B4 bounds Axiom's network process only while idle ([performance](../performance.md) PF9).

**The 30 s without input:**

| Page | CPU time over 30 s (s) | Idle wakeups | Energy impact |
|---|---|---|---|
| Google Search | 0.23 (0.10–0.32) | 4 (1–4) | 0.6 (0.2–0.9) |
| YouTube | 12.69 (10.91–14.24) | 92 (73–182) | 39.4 (33.6–44.4) |
| Instagram | 2.23 (2.08–2.37) | 23 (14–24) | 6.9 (6.3–7.1) |
| Reddit | 8.30 (0.55–15.60) | 19 (9–77) | 1.6 and 48.4 in two runs. In the third, `top` gave the GPU process an impossible 29.9 million, a fault of its counter, left out; 24.9 without that process |
| Wikipedia | 3.81 (0.09–11.35) | 10 (0–13) | 11.7 (0.2–35.0) |
| Google Maps | 0.41 (0.32–0.43) | 4 (4–17) | 1.2 (0.9–1.2) |
| Twitch | 18.76 (17.77–18.80) | 59 (41–60) | 58.3 (55.4–58.3) |
| MDN | 6.82 (0.03–6.93) | 21 (1–24) | 21.0 (0.1–21.3) |
| Three.js | 15.95 (15.90–16.01) | 49 (44–50) | 49.6 (49.5–49.8) |
| VS Code for the Web | 1.09 (0.18–4.25) | 6 (0–32) | 3.2 (0.5–13.0) |

- **Twitch, Three.js and YouTube keep working,** about half a core each, as they do in Chrome (§6, §7.3). Three.js's simulation cost the same in both: 16.0 s of CPU here, 15.4 s in Chrome.
- **MDN kept working in two runs** (6.8 and 6.9 s), and Wikipedia in two (3.8 and 11.4 s), where Chrome's automated runs of the static pages used 0.9–2.4 s. Most likely the owner's tasks left work running, such as MDN's live demos (inference).
- **Google Maps idles cheaply in Safari:** 0.4 s of CPU in 30 s, despite its 1–2 GB.

**Beside Chrome** (inference; the conditions differ, so this is context, not a comparison of engines):
- **On the five automated pages,** Chrome's interaction was scripted, and Safari's by hand with the owner's advanced tasks. Safari's costs are higher on Maps (1,880 against 627 MB), VS Code (1,076 against 457), Wikipedia (385 against 319) and MDN (181 against 145), and lower on Three.js (433 against 470).
- **On the five content and media pages,** Chrome ran signed in and Safari logged out. Safari's costs are higher on YouTube (1,624 against 908 MB) and Twitch (795 against 401), about the same on Google Search (568 against 582), and lower on Reddit (736 against 1,260) and Instagram (278 against 578).
- **So neither browser bounds the other,** and a page's cost depends on what is done with it as much as on the engine.
- **The ceilings stay as the accepted rule gives them:** half of Chrome's cost, capped by tier ([performance](../performance.md) §4.1). Against Safari's costs, they come to 16% (Maps) to 54% (Instagram, Three.js).

**The first session, superseded** (19:01–19:25). After each page, the tool asked the owner to go back to the list in the same tab, and Safari kept the earlier pages' WebContent processes alive. So every run after the first counted earlier pages too: the renderers counted rose from 2 to 9, and the total from 0.74 to 3.8 GB. Only its first run measured one page: Google Search, at 738 MB across Safari's processes after 30 s without input. Its records stay under `target/`, and its figures aren't used. The tool was corrected before the second session:
- Safari is quit and restarted for each run;
- each run records its own baseline;
- Safari covers every page of the mode, the five automated pages included.

### 7.5 Not measured

- **Chrome logged out by hand:** Google Search, YouTube, Instagram, Reddit and Twitch, which CO10 measures beside signed in.
- **Safari signed in:** Discord, Figma and Prism, which need a sign-in, and the five content and media pages, which differ signed in. `python3 -I spikes/s8/s8.py hand --browser safari --mode signed-in` covers those eight.
- **The eight pages' features:** `chrome://histograms` wasn't saved, so they stay inference ([corpus](../corpus.md) §10, question 1).
- **Safari's remote automation** stayed off.

## 8. Implications for Axiom

These are inferences, for CP4 and CP6:
- **The documents tier needs more than documents.**
  - Wikipedia's article needs SVG images, 164 of them, most likely its formulas, and MathML.
  - MDN's needs custom elements and shadow DOM, 98 of each, plus cross-site frames for its live examples.

  So Milestone 1's static pipeline can read both pages' text, but the pages render fully only with SVG images, and MDN only with script (CP8, CP9).
- **The GPU process is a large share of the cost on GPU pages.** Chrome's grew by about 307 MB for Maps and 409 MB for Three.js. Charging GPU memory to the page it serves (CP6, PF4) makes that visible in Axiom's budgets.
- **SVG images as documents are costly in Chrome:** 150 documents on Wikipedia. Axiom's image path can rasterize an SVG image and keep its source, rather than keeping a live document for each (to be designed at the images milestone).
- **Most of what the pages use is widely available** (72 of 116 web-features). The features in only some engines are mostly enhancements (§3). That supports the standards baseline's order of priority (CP4, SB6).
- **HTTP/3 carries most of Maps' traffic.** Networking's plan to add HTTP/3 after HTTP/1.1 and HTTP/2 stands, and the corpus will measure its effect.
- **Signed-in feeds are the heaviest pages per tier** (§7.2). Reddit, Instagram and Google Search cost 578–1,260 MB in Chrome after scrolling, where the content tier's cap is 150 MB. So the content pages' ceilings are the tightest in the corpus, at 12–26% of Chrome's cost ([performance](../performance.md) §4.1).
- **Feeds keep their media on the GPU:** Instagram's and Reddit's GPU processes held 311–388 MB at the steady state. Charging GPU memory to the page (PF4), and dropping what has scrolled away, matter most on these pages (inference).
- **What is done with a page sets its cost as much as the engine does** (§7.4). With the owner's advanced tasks, Safari's costs exceeded Chrome's scripted ones on four of the five automated pages, up to three times Chrome's, on Maps. So a ceiling means something only with its interaction: each milestone's corpus check states the interaction it measures (CO10), and a check by hand records what the owner did (inference).
- **Web storage can make the network process large:** Safari's grew to 233–241 MB with VS Code for the Web (§7.4). Axiom keeps web storage in its network process too (SW1), where B4 bounds only the idle process. A budget for that process under storage load is for the storage milestone (inference).

## 9. Threats to validity

- **Live pages** change daily, and their content, experiments and advertising differ from day to day. Every figure is dated, and is context rather than a target (CO12).
- **This is not the browser the owner runs.** The automation flags differ from a user's Chrome: no background networking, a mock keychain, the automation infobar, `navigator.webdriver`. The window was never focused.
- **Visibility:** two of seventeen runs were hidden and left out (§2). The tools now check visibility at every phase.
- **Load:** 4.65 before the Three.js rerun, above the 3.0 that CP6's PF1 sets for formal runs. After-run loads include the browser's own work.
- **Use counters count feature detection as use,** and they miss features used only in a way Blink doesn't count.
- **The DOM counts** exclude closed shadow roots, and the documents of cross-site frames.
- **"Above about:blank"** includes Chrome's browser-process growth, a few MB. The baseline itself varies by up to 16 MB between runs. Leaving out the extension renderers subtracts their own footprints from a total that counts shared memory once, an approximation (§5).
- **`top`'s wakeup counter failed in two runs** (§6).
- **By hand** (§7):
  - the owner's interaction differs from run to run, so the spread is wide;
  - the baseline is a fresh profile's, measured after the session, so each cost includes the dedicated profile's own state (§7.1);
  - the runs used Canary 158.0.8113.0, the automated ones 158.0.8112.0;
  - one run's load average reached 4.68 (Figma's third), above PF1's 3.0.
- **Safari** (§7.4):
  - the owner's advanced tasks go beyond the corpus's listed interactions and Chrome's scripted ones, so the two browsers' figures aren't like for like;
  - one run of Reddit had no Private window, which changed Safari's own process by about 160 MB; the run's own baseline cancels it;
  - the session began at a load average of 4.04, above PF1's 3.0, and stayed at 1.3–3.0 after each run;
  - one reading of `top`'s energy counter was impossible (29.9 million) and is left out;
  - the first session is superseded, since the tool's protocol kept earlier pages open.

## 10. Reproducing these results

```sh
python3 -I spikes/s8/test_s8.py           # offline tests
python3 -I spikes/s8/s8.py check          # the method, on a local test page
python3 -I spikes/s8/s8.py robots         # each automated page's robots verdict
tools/s8-measure.sh                       # the formal census (AC power, Mac idle, window visible)
python3 -I tools/s8-report.py target/s8-results/<timestamp>-auto [...] \
    --web-features target/s8/web-features-3.43.0/data.json
```

`tools/s8-measure.sh --quick` checks the tools against the live pages, and counts towards each site's limit of five loads per census (CO4). The census by hand runs with `s8.py signin` and `s8.py hand` (§7). For the session of 2026-10-20, which recorded no baseline of its own, `s8.py blank` measures a fresh Canary on about:blank, and the report takes it with `--blank`:

```sh
python3 -I spikes/s8/s8.py blank          # three runs, about 6 minutes; no page loaded
python3 -I tools/s8-report.py target/s8-results/20261020-181706-hand-chrome-signed-in \
    --blank target/s8-results/20261020-195301-blank-chrome-fresh
```

Safari's second session records each run's own baseline, so its report needs no `--blank`:

```sh
python3 -I spikes/s8/s8.py hand --browser safari --mode logged-out   # by hand: ten pages, three runs, about an hour
python3 -I tools/s8-report.py target/s8-results/20261020-222359-hand-safari-logged-out
```

web-features' `data.json` is release v3.43.0's [W1], downloaded into `target/`.

## Sources

All read on 2026-10-20.

- [C1] Chromium, `third_party/blink/public/mojom/use_counter/metrics/webdx_feature.mojom`, `web_feature.mojom` and `css_property_id.mojom`, at tag 158.0.8112.0 — https://github.com/chromium/chromium/tree/158.0.8112.0/third_party/blink/public/mojom/use_counter/metrics
- [C2] Chromium, `content/browser/devtools/devtools_pipe_handler.cc`, at tag 158.0.8112.0 — https://github.com/chromium/chromium/blob/158.0.8112.0/content/browser/devtools/devtools_pipe_handler.cc
- [C3] Chromium, the DevTools Protocol's `Browser` domain (`getHistograms`), `third_party/blink/public/devtools_protocol/domains/Browser.pdl`, at tag 158.0.8112.0 — https://github.com/chromium/chromium/blob/158.0.8112.0/third_party/blink/public/devtools_protocol/domains/Browser.pdl
- [W1] web-features, release v3.43.0, 2026-10-15, `data.json` — https://github.com/web-platform-dx/web-features/releases/tag/v3.43.0
