# S8 Results: the Corpus Census

| | |
|---|---|
| **Status** | **The automated part is complete** (2026-10-20): the five corpus pages whose sites' terms permit automated loads, in Chrome Canary, three valid runs each (§3–§6). **The part by hand awaits the owner** (§7): the other eight pages, logged out and signed in, and Safari for all thirteen, since Safari's remote automation is off (observed) |
| **Question** | What does each corpus page use and cost in shipping engines on this Mac? Its features (S8-Q1), size (S8-Q2), memory (S8-Q3), and load milestones and cost without input (S8-Q4) |
| **Feeds** | The [corpus](../corpus.md)'s baselines; the [standards baseline](../standards.md)'s priorities (CP4); the [performance budgets](../performance.md) (CP6) |
| **Plan** | [Step 6 tracker](step-6-standards-budgets-roadmap.md) §10.1, authorized at Gate A (2026-10-20), as refined at CP2 to the sites' terms ([corpus](../corpus.md) CO3–CO6) |
| **Code** | `spikes/s8/` (Python, standard library only) and `tools/s8-measure.sh`, at `6ba5068` for the formal run and `df7b179` for the rerun; `tools/s8-report.py` for the tables (§10) |
| **Last updated** | 2026-10-20 |

**Evidence labels** follow the [research conventions](README.md):
- **Measured** facts come from the census's records, in the runs named in §2.
- **Observed** facts were read on this Mac, naming the file or command.
- **Sourced** facts link to their source.
- **Inference** marks an explanation drawn from them.

MB means 10⁶ bytes, as B1 and S6 use it. Every figure is a median of three valid runs, with the range in brackets.

---

## Summary

- **Five pages, three valid runs each,** in Chrome Canary 158.0.8112.0: Wikipedia's article on π, Google Maps, MDN's canvas tutorial, Three.js's WebGPU example and VS Code for the Web. Two runs had the page hidden and were replaced; the two sites' fifth loads are the corpus's limit (CO4).
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

## 7. The census by hand (pending)

Not run; it needs the owner (CO6), and is put to the owner at Gate B:
- **The eight pages automation may not load:** Google Search, YouTube, Instagram, Reddit, Twitch, Discord, Figma and Prism. In Chrome Canary they are measured signed in, in the dedicated profile (`s8.py signin`, then `s8.py hand --browser chrome --mode signed-in`). The five of them that work logged out are also measured logged out (`--mode logged-out`).
- **Safari, all thirteen pages, by hand.** `s8.py safari-status` found its remote automation off: "You must enable 'Allow remote automation' in the Developer section of Safari Settings to control Safari via WebDriver" (observed, 2026-10-20). If the owner turns it on, the five automated pages can run in Safari through WebDriver instead.
- **Optional:** saving `chrome://histograms/Blink.UseCounter` by hand after each page, to record the features of the pages measured by hand ([corpus](../corpus.md) §10, question 1).

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

## 9. Threats to validity

- **Live pages** change daily, and their content, experiments and advertising differ from day to day. Every figure is dated, and is context rather than a target (CO12).
- **This is not the browser the owner runs.** The automation flags differ from a user's Chrome: no background networking, a mock keychain, the automation infobar, `navigator.webdriver`. The window was never focused.
- **Visibility:** two of seventeen runs were hidden and left out (§2). The tools now check visibility at every phase.
- **Load:** 4.65 before the Three.js rerun, above the 3.0 that CP6's PF1 sets for formal runs. After-run loads include the browser's own work.
- **Use counters count feature detection as use,** and they miss features used only in a way Blink doesn't count.
- **The DOM counts** exclude closed shadow roots, and the documents of cross-site frames.
- **"Above about:blank"** includes Chrome's browser-process growth, a few MB. The baseline itself varies by up to 16 MB between runs. Leaving out the extension renderers subtracts their own footprints from a total that counts shared memory once, an approximation (§5).
- **`top`'s wakeup counter failed in two runs** (§6).
- **Only Chrome, and only five pages.** Safari and the other eight pages wait for the session by hand (§7).

## 10. Reproducing these results

```sh
python3 -I spikes/s8/test_s8.py           # offline tests
python3 -I spikes/s8/s8.py check          # the method, on a local test page
python3 -I spikes/s8/s8.py robots         # each automated page's robots verdict
tools/s8-measure.sh                       # the formal census (AC power, Mac idle, window visible)
python3 -I tools/s8-report.py target/s8-results/<timestamp>-auto [...] \
    --web-features target/s8/web-features-3.43.0/data.json
```

`tools/s8-measure.sh --quick` checks the tools against the live pages, and counts towards each site's limit of five loads per census (CO4). The census by hand runs with `s8.py signin` and `s8.py hand` (§7). web-features' `data.json` is release v3.43.0's [W1], downloaded into `target/`.

## Sources

All read on 2026-10-20.

- [C1] Chromium, `third_party/blink/public/mojom/use_counter/metrics/webdx_feature.mojom`, `web_feature.mojom` and `css_property_id.mojom`, at tag 158.0.8112.0 — https://github.com/chromium/chromium/tree/158.0.8112.0/third_party/blink/public/mojom/use_counter/metrics
- [C2] Chromium, `content/browser/devtools/devtools_pipe_handler.cc`, at tag 158.0.8112.0 — https://github.com/chromium/chromium/blob/158.0.8112.0/content/browser/devtools/devtools_pipe_handler.cc
- [C3] Chromium, the DevTools Protocol's `Browser` domain (`getHistograms`), `third_party/blink/public/devtools_protocol/domains/Browser.pdl`, at tag 158.0.8112.0 — https://github.com/chromium/chromium/blob/158.0.8112.0/third_party/blink/public/devtools_protocol/domains/Browser.pdl
- [W1] web-features, release v3.43.0, 2026-10-15, `data.json` — https://github.com/web-platform-dx/web-features/releases/tag/v3.43.0
