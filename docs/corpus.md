# The Corpus

| | |
|---|---|
| **Status** | **Binding** since [ADR-0018](adr/0018-corpus-standards-and-testing.md) was accepted (2026-10-20, Step 6's Gate B), with the automation boundary (§3) confirmed by the owner. Written at Step 6, CP2. **Baselines from spike S8** (§5): all thirteen pages in Chrome Canary, the five automated logged out, the eight others signed in by the owner's hand. Safari's runs need repeating ([S8 results](research/s8-results.md) §7.4) |
| **Question** | Which pages is Axiom judged against? What does each need, and from which security level can it be used? How is each measured, repeatably and within its site's terms? |
| **Answers** | Step 6 question Q1 ([tracker](research/step-6-standards-budgets-roadmap.md) §3), with S8; the tracker's handed-over questions 1 and 2 (§2.2) |
| **Feeds** | Spike S8 (CP3); the standards baseline (CP4); testing (CP5); budgets (CP6); the roadmap's corpus targets and login track (CP8); Milestone 1 (CP9) |
| **Builds on** | [OQ-2](open-questions.md), resolved by the owner (2026-10-20); the owner's Gate A decisions on S8, accounts and URL parameters (tracker §11); the [threat model](security/threat-model.md) §8; [ADR-0001](adr/0001-scope-and-target.md); the [charter](charter.md) §8 |
| **Last updated** | 2026-10-20 |

Rules are numbered CO1–CO14. Each site's terms and robots file are quoted and dated (§3). Each site's needs are **inference** from background knowledge until S8 or a session by hand measures them, and are marked as such.

## Summary

- **The corpus is the owner's 13 pages** in four tiers: two documents, three content sites, three media or map sites, and five applications (CO1, CO2).
- **Each site's terms and robots file decide whether automation may load it** (CO3). Eight forbid automated access, logged out as well as logged in: Google Search, YouTube, Instagram, Reddit, Twitch, Discord, Figma and Prism. Five permit occasional automated loads, with care: Wikipedia, Google Maps, MDN, Three.js and VS Code for the Web. **This narrows S8's automated mode,** which the plan applied to every page that works logged out (§9).
- **Automated loads identify themselves,** check robots.txt first, stay few and spaced, and never sign in (CO4).
- **Pages whose sites forbid automation are measured by hand:** the owner drives a shipping browser, and tools sample its processes from outside (CO6). All eight of those sites have logins, so this is also how the logged-in experience is measured, as the owner asked.
- **Accounts, as the owner decided at Gate A:** the owner's regular accounts, in a dedicated local profile, for S8; in Axiom, the owner's personal accounts wait for L3, and test accounts that aren't sensitive may be used from L2 (CO7, CO8). For the session by hand of 2026-10-20, the owner chose secondary accounts.
- **Each page is measured loaded, during a defined interaction, and afterwards with no input,** in a 1,280 × 800-point window, in Safari and Chrome Canary (CO9–CO11).
- **Repeatability:** baselines are live, dated and run three times. Saved copies are made only where the terms and licences allow it: the two documents and Three.js's example. The other sites are never captured (CO12).
- **Nothing from a page is committed** but aggregate figures. Profiles that hold signed-in sessions stay in `scratch/` (CO13, CO14).

## 1. Binding inputs

| Input | Consequence here |
|---|---|
| **OQ-2** (the owner, 2026-10-20): "Use the following 13 sites as Axiom's standard evaluation corpus for performance budgets, WPT suites, and memory baselines in Step 6" | CO1: the corpus, changed only by the owner |
| **Gate A, decision 2:** "The two-mode design (automated logged-out mode; passive, non-automated logged-in mode in a dedicated scratch profile) is approved." | CO5, CO6 |
| **Gate A, decision 3:** "For Spike S8 measurements in shipping browsers, use my regular accounts via the dedicated local profile." "For Axiom engine testing, sensitive personal accounts will wait for Level 3 (L3) verification, while non-sensitive test accounts may be used starting at Level 2 (L2)." | CO7, CO8 |
| **Gate A, decision 4:** the identifying parameters removed from the repository, "while preserving the full URLs in scratch/ for Spike S8" | CO13 |
| **Threat model §8:** L0 never handles credentials; L1 permits browsing without logging in; L2, accounts that aren't sensitive; L3, "Email, banking, and accounts with write access" | §2's levels; CO8 |
| **ADR-0001:** performance budgets are measured on the target M1 with 8 GB; sensitive logins are gated until the isolation milestone is verified | CO8–CO11 |
| **Charter §8:** the target-site corpus is defined; target-corpus checks: "content renders and is readable, links navigate, and there are no crashes or hangs" | CO1; the milestones' corpus checks (CP8, CP9) |
| **The plan's method (§8):** aggregate measurements only; no page content, screenshots, cookies or credentials committed | CO13 |

## 2. The sites

| # | Site (the owner's label) | URL | Tier | Needs beyond a static document (inference until measured) | Login, and what it adds (inference) | Earliest level | Loaded by (§3) |
|---|---|---|---|---|---|---|---|
| 1 | Google (search results) | https://www.google.com/search?q=Premier+League | Content | Script-built result modules (live scores, panels), web fonts, images | Optional: personalized results and history | L1; L2 with a test account; L3 with the owner's | Hand |
| 2 | YouTube (video player and comments) | https://www.youtube.com/watch?v=2Zhv7SjAKeg | Media | Custom elements, video through Media Source Extensions, comments loaded lazily | Optional: recommendations, subscriptions, posting comments | L1; L2 test; L3 own | Hand |
| 3 | Instagram (media feed) | https://www.instagram.com/leomessi | Content | A feed of images and video built by script; a login wall after some scrolling | Partly required: the whole feed, stories | L1 (limited); L2 test; L3 own | Hand |
| 4 | Reddit (community feed) | https://www.reddit.com/r/RedditGames/ | Content | Web components, infinite scrolling, interactive posts in frames | Optional: voting, posting, joined communities | L1; L2 test; L3 own | Hand |
| 5 | Wikipedia (article layout and math) | https://en.wikipedia.org/wiki/Pi | Document | Mathematical formulas, tables, images, references; little script | Not needed (editing only) | L0 | Automation |
| 6 | Google Maps (interactive map tiles) | https://www.google.com/maps/search/Times+Square,+New+York+City,+USA | Media and maps | WebGL map rendering, heavy script | Optional: saved places | L1, with WebGL's GPU process | Automation |
| 7 | Twitch (live stream and chat) | https://www.twitch.tv/eslcs | Media | Live video through Media Source Extensions; chat over WebSockets | Optional: chatting, following | L1; L2 test; L3 own | Hand |
| 8 | Discord (web client and channels) | https://discord.com/channels/662267976984297473/999550150705954856 | Application | A script application over WebSockets, with local storage and IndexedDB | Required: logged out, the page asks the visitor to sign in (background) | L2 test; L3 own | Hand, signed in |
| 9 | MDN Web Docs (Canvas API tutorial) | https://developer.mozilla.org/en-US/docs/Web/API/Canvas_API/Tutorial/Pixel_manipulation_with_canvas | Document | Live examples in frames, drawing with canvas 2D (`getImageData`, `putImageData`) | Not needed | L0 | Automation |
| 10 | Three.js (WebGPU compute particles) | https://threejs.org/examples/webgpu_compute_particles.html | Application | WebGPU compute and rendering; JavaScript modules | None | L2, with WebGPU's GPU process | Automation |
| 11 | Figma (design editor canvas) | https://www.figma.com/make/uulSQtKZ5FbRgWPRc0EmVo/Doodle-Fonts--Community-?p=f (`t=` left out, CO13) | Application | WebAssembly and canvases rendered on the GPU | Probably required to open the file | L2 test; L3 own | Hand, signed in |
| 12 | VS Code for the Web (browser IDE) | https://vscode.dev/ | Application | A large script application: workers, a service worker, IndexedDB, the File System Access API | Optional: settings sync | L1 | Automation |
| 13 | Prism (document editing) | https://prism.openai.com/?pg=1&m=MIT-thesis-template%2FMIT-Thesis.tex (`u=` left out, CO13) | Application | An editor for LaTeX documents with a rendered preview, and collaboration | Required | L2 test; L3 own | Hand, signed in |

**CO1. The corpus.** These 13 pages, as the owner gave them, except for the two parameters kept out of the repository (CO13). Only the owner changes the corpus. If a page moves or disappears, the owner chooses its replacement, and the change is recorded here with its date.

**CO2. Tiers.** Each tier names what a site needs beyond the one before:
- **Documents** (Wikipedia, MDN): the content is in the HTML and readable without script; script adds interactive parts.
- **Content sites** (Google Search, Instagram, Reddit): pages that script builds or extends, with feeds, modules and frames.
- **Media and maps** (YouTube, Twitch, Google Maps): streamed video through Media Source Extensions, or maps drawn with WebGL.
- **Applications** (Discord, Three.js, Figma, VS Code for the Web, Prism): long-lived script applications with workers, storage and WebSockets, and in some, WebAssembly or WebGPU.

A site becomes an acceptance target at the first milestone that has every capability it needs, at a security level that permits its use (CP8). The levels above apply the threat model's §8: a site that needs a login waits for L2 with a test account, and for L3 with the owner's own (CO8).

## 3. Terms of use and automation

The plan's design for S8 loaded every page that works logged out by automation, and kept automation out of signed-in pages because sites' terms can forbid it ([D1]). **Reading each site's terms for CP2 showed that eight of the thirteen forbid automated access whether or not the visitor is signed in.** The table records what each says, and what robots.txt says about the corpus page.

| Site | Terms, and their date | On automated access | robots.txt for the corpus page (observed 2026-10-20, matched as RFC 9309 specifies) | Loaded by |
|---|---|---|---|---|
| Google Search | Google Terms of Service, effective 2026-07-30 [T2] | Forbids "using automated means to access content from any of our services in violation of the machine-readable instructions on our web pages (for example, robots.txt files that disallow crawling, training, or other activities)" | `Disallow: /search` for every user agent | Hand |
| YouTube | YouTube Terms of Service, effective 2022-01-05 [T1] | Users may not "access the Service using any automated means (such as robots, botnets or scrapers)", except "in the case of public search engines, in accordance with YouTube's robots.txt file", or "with YouTube's prior written permission" | No rule matches `/watch`, but the exception covers only public search engines | Hand |
| Instagram | Instagram Terms of Use, effective 2025-01-01, §4.2 [T5] | "This includes creating accounts or accessing or collecting information in an automated way without our express permission", "regardless of whether such automated access or collection is undertaken while logged in to an Instagram account." | Not served as a robots file (an HTML page) | Hand |
| Reddit | Reddit User Agreement, revised 2026-05-26, effective 2026-07-01 [T6] | Users may not "Access, search, or collect data from the Services by any means (automated or otherwise) except as permitted in these Terms or in a separate agreement with Reddit (we conditionally grant permission to crawl the Services in accordance with the parameters set forth in our robots.txt file, but scraping the Services without Reddit's prior written consent is prohibited)" | `User-agent: *` `Disallow: /` | Hand |
| Wikipedia | Wikimedia Terms of Use, effective 2023-06-07, §4 [T7]; the User-Agent Policy and the Robot policy [T7] | The terms forbid "automated uses of the Project Websites that are abusive or disruptive of the services, violate acceptable usage policies where available, or have not been approved by the Wikimedia community". The Robot policy, an acceptable usage policy, permits reading: "do not emulate a browser - do not store cookies or execute javascript, unless you're not crawling the sites at high volume (so, more than 5 requests per second)". The User-Agent Policy asks bots to identify themselves, with contact details: "bot-like behavior with a browser's user agent will be assumed malicious" | Allowed (no rule matches `/wiki/Pi`) | Automation, identified (CO4) |
| Google Maps | Google Terms of Service [T2], and the Google Maps/Google Earth Additional Terms of Service, last modified 2026-01-27 [T4] | Google's clause, above, turns on robots.txt. The Maps terms forbid users to "copy the content" and to "mass download or create bulk feeds of the content" | `Allow: /maps/search/` for every user agent | Automation, nothing copied (CO4) |
| Twitch | Twitch Terms of Service, "Last modified on 08/12/2026", §10 [T8] | Users may not "access any website, server, software application, or other computer resource owned, used, and/or licensed by Twitch, including but not limited to the Twitch Services, by means of any robot, spider, scraper, crawler, or other automated means for any purpose" | No rule matches `/eslcs`, but the terms forbid automation | Hand |
| Discord | Discord Terms of Service, effective 2025-09-29 [D1] | Forbids "scraping our services without our written consent, including by using any robot, spider, crawler, scraper", and "using any unauthorized software designed to modify the services" | `Disallow: /channels/` | Hand, signed in |
| MDN | Mozilla's Websites & Communications Terms of Use, effective 2026-09-16, and its Acceptable Use Policy [T9] | No clause on automated access. The Acceptable Use Policy forbids "any activity that interferes with or disrupts Mozilla's services or products" | Allowed (no rule matches the page) | Automation |
| Three.js | None published: threejs.org links no terms (observed). The library and its examples are MIT-licensed [L1] | — | No robots file (HTTP 404), which RFC 9309 treats as permitting access to every resource | Automation |
| Figma | Figma Acceptable Use Policy, last updated 2026-10-07, §2 [T10] | Users may not "scrape, data mine, systematically extract, collect, aggregate, or compile substantial quantities of Figma content or otherwise access Figma or Figma content programmatically", "except as expressly permitted by Figma"; nor "modify, or use anything that modifies, Figma's code or code execution" | `Disallow: /make/*` | Hand, signed in |
| VS Code for the Web | Microsoft Terms of Use, last updated 2022-02-07, linked from vscode.dev (observed in its HTML) [T11] | No clause forbids automated access. Users may not "use the Services in any manner that could damage, disable, overburden, or impair any Microsoft server" | No robots file: the path returns the application's HTML | Automation |
| Prism | OpenAI Terms of Use, "What you cannot do" [T12] | Forbids users to "Automatically or programmatically extract data or Output" (secondary: openai.com refused retrieval, HTTP 403; quoted from the version effective 2024-12-11) | No robots file (HTTP 404) | Hand, signed in |

**CO3. Which pages automation may load.** A corpus page is loaded by automation only if both hold:
- its site's terms don't forbid automated access, read for this table; and
- robots.txt allows its path to every user agent, matched as RFC 9309 specifies ("The most specific match found MUST be used", and a missing file permits access) [R1]. A census refuses to automate a page whose robots file disallows it.

Otherwise the page is measured by hand (CO6). **The reading of each site's terms is inference,** by a non-lawyer, on the dated text above. When it is unclear, the page goes to the by-hand side. Terms and robots files are read again before each census, and before each milestone's corpus checks; a change moves a page between the two sides, and is recorded here.

**CO4. How automated loads behave:**
- **They identify themselves.** The browser's own user agent, with `AxiomCensus/0.1 (+https://github.com/MTawhid7/Axiom)` appended. That is Wikimedia's User-Agent Policy, applied to every site. The token names the public repository, not a person.
- **They stay few and spaced:**
  - one automated page load at a time;
  - at least 60 seconds between loads of the same site;
  - at most five loads of each site per census, including the census tool's own checks.

  That is far below the Robot policy's limit for clients that run script: no crawling, at most a few page views.
- **They never sign in.** Logged-out pages only; no account, cookie or credential is ever given to an automated browser.
- **They work nothing around.** A consent screen, bot check or login wall is recorded as what loaded, and the run moves on.
- **They keep nothing from the page:** no content is copied out or stored, beyond the browser's own cache in a temporary profile deleted after the run. Only aggregate figures are kept (CO13). That respects the Maps terms' bans on copying and mass downloading.

**CO5. What automated runs measure.** Every S8 question:
- features used (S8-Q1), from the browser's own counters rather than script injected into the page (§10.1 of the tracker, as refined at CP2);
- sizes (S8-Q2);
- memory (S8-Q3);
- load milestones and the cost after loading (S8-Q4).

The runs use Chrome Canary through the DevTools Protocol, and Safari through WebDriver if the owner enables its remote automation, which only the owner may change (§10, question 2).

**CO6. Measurement by hand.** For a page that automation may not load, or that needs a sign-in:
- **The owner drives** a shipping browser by hand: opens the page from a local list of the corpus links, and performs its interaction (CO10). A census tool prompts each step.
- **Tools only sample the browser's processes from outside:** footprint, CPU time, idle wakeups and energy impact (S8-Q3, S8-Q4). Nothing is injected into the page or read from it.
- **The browser has no debugging connection.** It runs as an ordinary browser, so no code could act in a signed-in session.
- **Features and sizes aren't measured,** so for these pages they stay inference. §10, question 1, gives a by-hand way to record Chrome's own feature counters.

## 4. Logins and accounts

**CO7. Accounts for measurement in shipping browsers** (the owner's Gate A decision 3):
- **The owner's regular accounts,** used in a dedicated local profile kept in `scratch/` (CO14).
- **The owner signs in by hand,** once per site. Credentials are typed into the browser by the owner and are never seen, stored or read by code.
- **Logged-in pages are measured only by hand** (CO6), whatever the site's terms say about automation.
- **In practice,** the session by hand of 2026-10-20 used secondary accounts, the owner's choice: "I have used my secondary accounts for these tests."

**CO8. Accounts in Axiom** (the owner's Gate A decision 3, under the threat model's §8):
- **No credential before L2.** L0 never handles credentials, and L1 is for browsing without logging in.
- **From L2: test accounts that aren't sensitive.** Such an account:
  - holds nothing of value;
  - is linked to nothing of the owner's: no payment method, no real contacts or personal content, and no sign-in to other services;
  - has a password used nowhere else;
  - can be deleted without loss.

  The owner creates each one by hand, under its site's terms. Some sites limit the accounts a person may hold, which CP8 checks for each site the L2 milestone targets.
- **From L3: the owner's personal accounts.** Every login in this corpus is a personal account with write access: posting, messaging and editing. So each waits for L3, the sensitive-use gate.
- **Credentials never appear in code, tests, logs or the repository.** Axiom's own login tests use local test servers (CP5). On live sites, the owner signs in by hand.

What a login adds to each site is in §2's table. For CP8's login track, the capabilities it needs are:
- form controls, password fields among them (HD27);
- cookies with partitioning (NW11);
- storage (SW);
- redirects and pop-ups for single sign-on (FT);
- possibly passkeys (OQ-16).

## 5. The measurement protocol

**CO9. Phases.** Each measured page goes through three phases:
1. **Load.**
   - **Automated runs:** navigate, then wait until the load event has fired and the network has been quiet for 2 seconds, or until 30 seconds have passed, whichever comes first. The run records which ended the wait.
   - **By hand:** the owner confirms when the page looks complete.

   The browser's processes are sampled: **loaded**.
2. **Interaction:** the page's interaction (CO10), for 30 seconds. Then the processes are sampled again: **after interaction**.
3. **No input:** 30 seconds without input. CPU time, idle wakeups and energy impact are measured over this window, then the processes are sampled: **after 30 s without input**. Pages that keep playing video or animation are in a steady state rather than idle, and are reported as such.

A page's cost is the footprint of the browser processes that serve it. The content processes that render it are its own cost. The browser, GPU and network processes are reported beside them. Each run starts the browser fresh, with only the corpus page open.

**CO10. Each page's interaction:**

| # | Site | Interaction (30 s) | Driven by |
|---|---|---|---|
| 1 | Google Search | Scroll the results to the end, and back | The owner |
| 2 | YouTube | Play the video; then scroll down until comments load | The owner |
| 3 | Instagram | Scroll the profile's grid five screens; open one post and close it | The owner, signed in |
| 4 | Reddit | Scroll the feed five screens | The owner |
| 5 | Wikipedia | Scroll the article to the end at a steady pace, and back | Automation (wheel events) |
| 6 | Google Maps | Pan the map with three drags; zoom in two steps and out two | Automation |
| 7 | Twitch | Watch the stream with the chat open | The owner |
| 8 | Discord | Scroll the channel's history up five screens, then back to the latest messages | The owner, signed in |
| 9 | MDN | Scroll the article to the end, and back; the live examples run | Automation |
| 10 | Three.js | The simulation runs; the pointer moves across the canvas | Automation |
| 11 | Figma | Pan and zoom the canvas | The owner, signed in |
| 12 | VS Code for the Web | Open a new untitled file from the command palette and type twenty lines | Automation |
| 13 | Prism | Type a paragraph in the editor and wait for the preview to update | The owner, signed in |

Sites whose experience differs logged in are measured both ways where both are allowed: logged out by hand in a temporary profile, and signed in in the dedicated profile. That applies to Google Search, YouTube, Instagram, Reddit and Twitch. Discord, Figma and Prism are measured signed in only, since logged out each asks the visitor to sign in (background for Discord; inference for Figma and Prism, to be seen in the session by hand).

**CO11. Conditions:**
- **This Mac:** AC power, Low Power Mode off and the Mac otherwise idle, checked with `pmset` and the load average, which is recorded.
- **Repetition:** a cool-down between runs, and three runs of each page in each browser and mode, with the median and spread reported.
- **The window:** 1,280 × 800 points at the top left of the screen. That is 2,560 × 1,600 pixels in the display's current scaled mode (1,440 × 900 points in a 2,880 × 1,800 framebuffer, [S6 results](research/s6-results.md) §2), and each run records the mode.
- **Browsers:** Safari 27.2 (22625.2.8.11.6), for WebKit, and Chrome Canary 158.0.8112.0, for Chromium (observed 2026-10-20, from each application's `Info.plist`). Each run records the versions, and Canary updates itself daily: the session by hand that evening ran Chrome Canary 158.0.8113.0 and Safari 27.2 (22625.2.8.11.9). Microsoft Edge Canary, also installed, would only repeat Chromium.
- **Profiles:**
  - automated runs use a fresh temporary profile under `target/`, deleted after each run;
  - pages measured by hand logged out use a fresh temporary profile too;
  - signed-in pages use the dedicated profile (CO14).
- **The network** is this Mac's own, unthrottled, and varies, so load times are context rather than targets.

### Baselines, from S8 (CP3)

Chrome Canary 158.0.8112.0, 2026-10-20: the medians of three valid runs ([S8 results](research/s8-results.md)). They are context for the budgets, not targets (CO12).

| Page | First contentful paint (ms) | Requests | Transferred (MB) | Elements | All Chrome's processes, 30 s after the interaction (MB) | The page's cost above about:blank (MB) |
|---|---|---|---|---|---|---|
| Wikipedia | 1,636 | 239 | 3.74 | 12,828 | 523 | 319 |
| Google Maps | 1,944 | 344 | 9.66 | 1,359 | 830 | 627 |
| MDN | 812 | 89 | 0.43 | 2,695 | 350 | 145 |
| Three.js | 1,100 | 24 | 0.84 | 461 | 674 | 470 |
| VS Code for the Web | 2,259 | 2,449 | 8.96 | 656 | 665 | 457 |

The page's cost leaves out three component-extension renderers of Chrome's own, which run on about:blank and exit before the steady state (S8 results §5).

### Baselines by hand, from S8 (Gate B)

Chrome Canary 158.0.8113.0, signed in, 2026-10-20: the medians of three runs by the owner ([S8 results](research/s8-results.md) §7). The cost is above a fresh Canary on about:blank measured after the session, so it includes the dedicated profile's own state (§7.1 of the results).

| Page | All Chrome's processes, 30 s after the interaction (MB) | The page's cost above about:blank (MB) | CPU in those 30 s (s) |
|---|---|---|---|
| Google Search | 789 | 582 | 3.0 |
| YouTube | 1,115 | 908 | 14.1 |
| Instagram | 785 | 578 | 2.3 |
| Reddit | 1,467 | 1,260 | 10.8 |
| Twitch | 608 | 401 | 13.0 |
| Discord | 634 | 427 | 1.3 |
| Figma | 1,319 | 1,111 | 4.4 |
| Prism | 715 | 494 | 10.0 |

Interaction by hand varies, so the spread is wide: Reddit's cost ran from 495 to 1,631 MB. **Safari** was measured logged out on five of these pages, but the census tool kept the earlier pages open, so only one run measures one page (Google Search: 738 MB across Safari's processes after 30 s without input). The tool is corrected; repeating Safari, Chrome logged out by hand, and Safari on the other eight pages need the owner (§7.4–§7.5 of the results).

## 6. Repeatability and captures

**CO12. Live baselines; captures only where permitted.**
- **Baselines in shipping engines** are live, dated, versioned and run three times, with their spread (CO11). They are context for budgets, never targets to copy, as the plan's risk table has it.
- **Repeatable copies** let a milestone measure the same bytes in Axiom and a reference engine. They are made only where the terms and licences allow automated loading and keeping a copy:
  - the documents tier: Wikipedia's article and MDN's page, whose text both sites publish under Creative Commons licences;
  - Three.js's example, from the MIT-licensed project at a pinned release.

  The first milestone that needs them makes them; CP9 names what that authorization covers. They are kept on this Mac, in `scratch/` or `target/`, dated, and never committed.
- **No captures elsewhere.** Several sites forbid downloading or copying their content: YouTube's terms, Twitch's "downloading (except page caching)", and the Maps terms. So recording proxies of the kind Chromium uses ([P1], [P2]) are not used for them. Those pages are checked live: by automation where §3 permits it, by hand otherwise. **Their numbers vary with the day,** so they are reported with their dates and spread, and budgets that rest on them stay hypotheses until a milestone measures them.

## 7. Privacy and the profiles

**CO13. What is kept and where:**
- **The repository holds aggregate figures only:** counts, sizes, footprints and times. No page content, screenshot, cookie, credential or resource URL beyond the corpus pages themselves.
- **Signed-in pages yield process measurements only** (CO6).
- **Raw traces** stay under `target/s8-results/`, which git ignores.
- **The two identifying URL parameters** (Prism's `u=`, Figma's `t=`) are kept out of the repository, as the owner decided at Gate A. The full URLs live in `scratch/corpus-urls.txt`, which git ignores. Tools that need them read that file.

**CO14. The dedicated profile:**
- **It lives in `scratch/s8/`,** which git ignores. It holds live signed-in sessions for the owner's regular accounts. Chrome encrypts cookies at rest with a key in the owner's login keychain, its normal behaviour on macOS (background).
- **It is never committed, copied or shared,** and never opened by automation or a debugging connection.
- **The owner may delete it at any time.** Deleting it signs those sessions out of this profile only.
- **Safari** keeps no profile in a folder. There the owner uses Safari as they choose, for example a Safari profile created by hand. The tools only sample Safari's processes.

## 8. Targets and staging

| Tier | Sites | Capabilities that gate them (inference, for CP8) |
|---|---|---|
| Documents | Wikipedia, MDN | The static-document pipeline. Then script for MDN's live examples and canvas 2D, and math layout for Wikipedia's formulas (MathML, or the images Wikipedia substitutes, to be measured by S8) |
| Content | Google Search, Instagram, Reddit | Script, the DOM's dynamic parts, web components, frames; logins from L2 |
| Media and maps | YouTube, Twitch, Google Maps | Media Source Extensions and the media utility process; WebGL and the GPU process for Maps |
| Applications | Discord, Three.js, Figma, VS Code for the Web, Prism | Workers, storage, WebSockets; WebGPU for Three.js; WebAssembly for Figma (OQ-3); logins |

The roadmap (CP8) places each site at the first milestone and level where it can be an acceptance target. Milestone 1 (CP9) can target only documents. Earlier milestones use local test pages and the suites (CP4, CP5).

## 9. Alternatives considered

- **Automate every page that works logged out,** as the plan's design for S8 first had it. Rejected: it would breach the terms of eight sites (§3), which the owner's approval was not asked to cover, since the plan cited only Discord's terms. The by-hand mode, already approved for signed-in pages, covers them.
- **Ask the sites for written permission,** as YouTube's and Reddit's terms allow. Not pursued for a personal project: by-hand measurement answers S8's questions about memory and time. To revisit if a milestone needs automated tests of those sites.
- **Record and replay every site,** as Chromium's system-health benchmark does with Web Page Replay archives, dated by year [P1], [P2]. Rejected for sites whose terms forbid downloading or copying. Adopted, in effect, for the pages whose terms and licences permit it (CO12).
- **A browser extension that measures pages.** Rejected: an extension modifies pages, which Discord's terms ("unauthorized software designed to modify the services") and Figma's (§2(n)) forbid. For automated pages, the browser's own counters measure features without touching the page.

## 10. Open questions

1. **Features of the eight pages measured by hand.** S8 can't instrument them. **Optional, by hand:** after each page, the owner saves Chrome's own `chrome://histograms/Blink.UseCounter` page from the census profile. That page counts the features the browser saw used, records the browser's own statistics, and holds no page content. Not saved in the session of 2026-10-20, so these pages' features stay inference.
2. **Safari for the five automated pages.** Safari's automation needs its remote automation enabled (Develop → Allow Remote Automation, or `safaridriver --enable` with an administrator's password), a setting only the owner may change. It stayed off at Gate B, so Safari is measured by hand for every page, as the corrected census tool now does.
3. **Test accounts at L2:** answered at CP8 in the [roadmap](roadmap.md) §6. M5 targets Google Search, Instagram and Reddit, whose terms set no limit on a person's accounts; each test account is made by hand, in the owner's own name, with accurate details, and used only to sign in and read.
4. **When to read terms again:** before each census and each milestone's corpus checks (CO3). Whether a changed clause should stop a census automatically, as a changed robots file does, is CP5's to decide.

## Sources

All read on 2026-10-20.

- [T1] YouTube, *Terms of Service*, dated and effective 5 January 2022, "Permissions and Restrictions" — https://www.youtube.com/static?template=terms
- [T2] Google, *Terms of Service*, effective 30 July 2026, "Don't abuse our services"; and robots.txt — https://policies.google.com/terms ; https://www.google.com/robots.txt
- [T4] Google, *Google Maps/Google Earth Additional Terms of Service*, last modified 27 January 2026, §2 — https://www.google.com/help/terms_maps/
- [T5] Instagram, *Terms of Use*, effective 1 January 2025, §4.2, "How you can't use Instagram" (read through facebook.com/help/instagram/581066165581870, since the help page renders by script) — https://help.instagram.com/581066165581870
- [T6] Reddit, *User Agreement*, last revised 26 May 2026, effective 1 July 2026; and robots.txt — https://redditinc.com/policies/user-agreement ; https://www.reddit.com/robots.txt
- [T7] Wikimedia Foundation, *Terms of Use*, effective 7 June 2023, §4 — https://foundation.wikimedia.org/wiki/Policy:Terms_of_Use ; *User-Agent Policy* (last edited 27 March 2026) — https://foundation.wikimedia.org/wiki/Policy:Wikimedia_Foundation_User-Agent_Policy ; *Robot policy* (last edited 16 March 2026) — https://wikitech.wikimedia.org/wiki/Robot_policy
- [T8] Twitch, *Terms of Service*, "Last modified on 08/12/2026", §10, "Prohibited Conduct", item xi, and §7, "License", item (e) — https://legal.twitch.com/en/legal/terms-of-service/
- [D1] Discord, *Terms of Service*, effective 29 September 2025, "Restrictions on your use of Discord's services" — https://discord.com/terms
- [T9] Mozilla, *Websites & Communications Terms of Use*, effective 16 September 2026; *Acceptable Use Policy* (undated); MDN's robots.txt — https://www.mozilla.org/en-US/about/legal/terms/mozilla/ ; https://www.mozilla.org/en-US/about/legal/acceptable-use/ ; https://developer.mozilla.org/robots.txt
- [T10] Figma, *Acceptable Use Policy*, last updated 7 October 2026, §2(j) and §2(n); *Terms of Service*, effective 11 March 2026; robots.txt — https://www.figma.com/aup/ ; https://www.figma.com/legal/tos/ ; https://www.figma.com/robots.txt
- [T11] Microsoft, *Terms of Use*, last updated 7 February 2022 — https://www.microsoft.com/en-us/legal/terms-of-use
- [T12] OpenAI, *Terms of Use*, "What you cannot do" (openai.com answered HTTP 403 on 2026-10-20). Secondary: the version effective 11 December 2024, as mirrored by Wind River — https://openai.com/policies/terms-of-use/ ; https://open.windriver.com/info/uni-license-list/licenses/openai-tou-20241211.html
- [R1] M. Koster, G. Illyes, H. Zeller, L. Sassman, *Robots Exclusion Protocol*, RFC 9309, September 2022, §2.2.2 and §2.3.1.3 — https://www.rfc-editor.org/rfc/rfc9309
- [L1] three.js, `LICENSE` (MIT) — https://github.com/mrdoob/three.js/blob/dev/LICENSE
- [P1] Chromium, `tools/perf/page_sets/system_health/`: `loading_stories.py` [2026-08-17], `browsing_stories.py` and `system_health_story.py` [2026-10-08]; and `tools/perf/page_sets/data/system_health_desktop.json` [2025-06-23], which maps 141 desktop stories to Web Page Replay archives. Dates in brackets are each file's last commit, from GitHub's API — https://github.com/chromium/chromium/tree/main/tools/perf/page_sets/system_health
- [P2] Catapult, Web Page Replay, `web_page_replay_go/README.md` [2025-09-10]: "WprGo is currently used in Telemetry for Chrome benchmarking purposes"; replay is reached through `--host-resolver-rules`, with its certificates accepted through `--ignore-certificate-errors-spki-list` — https://github.com/catapult-project/catapult/blob/main/web_page_replay_go/README.md
