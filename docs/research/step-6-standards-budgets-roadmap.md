# Step 6 — Standards, Testing, Budgets and the Roadmap: Plan and Checkpoint Tracker

| | |
|---|---|
| **Status** | **CP1–CP7 ✅** (2026-10-20). **Gate B passed** (2026-10-20): ADRs 0018 and 0019 accepted, with the final B1–B7; spike S9 authorized; CP8–CP10 to run without pausing, to Gate C. **Gate A passed** (2026-10-20). Spike S8 is complete in Chrome; its Safari runs need repeating (§11) |
| **Authorized** | 2026-10-20, by the owner: "Step 6 Authorization: Authorized. Proceed with creating branch step-6 from main and drafting the Phase Zero Step 6 plan (standards baseline, testing strategy, performance budgets, and Milestone 1 roadmap) for Gate A review." Design documents only; each spike needs its own authorization. **Spike S8 authorized at Gate A** (2026-10-20), for the questions it names (§10.1) |
| **Branch** | `step-6`, from `main` at `b4948ce` (the Step 5 merge). Pushed to `origin` as the owner authorized at Gate A, after CP1's commit was amended (§11), and kept pushed as checkpoints land |
| **Feeds** | Decision-map items X1–X3; the roadmap and Milestone 1's acceptance criteria (charter §8); the final design review of Phase Zero; the questions earlier steps handed to Step 6 (§2.2) |
| **Last updated** | 2026-10-20 |

> **Resuming in a new session?**
> 1. Read §6 to find the first checkpoint that isn't ✅.
> 2. Check the repository against §11, the results log.
> 3. Continue from that checkpoint's deliverables (§6.1).

---

## 1. Purpose

Phase Zero's last step turns the accepted architecture into work that can be built and judged:
- **a standards baseline** (X1): which specifications Axiom implements, at which dated versions, how fully, and which suites check them;
- **a testing strategy** (X2): oracles, harnesses, differential testing, fuzzing, the security checks that gate L3, and the routine that keeps a locally built browser patched;
- **performance budgets** (X3): how performance is measured on this Mac, and the numbers each process and subsystem must meet, including the limits and deadlines that Step 4's and Step 5's designs left open;
- **a roadmap:** milestones from a static-document engine at L0 to the sensitive-use gate at L3 and the app web, ordered and gated but not dated (OQ-1), each with its security level, corpus targets and decision points;
- **Milestone 1,** defined precisely, with measurable acceptance criteria.

**The owner's corpus grounds all five** (OQ-2, resolved 2026-10-20; §2.4). It names the features that matter to the standards baseline, the pages tests and benchmarks run on, the baselines budgets are judged against, and the order in which sites become usable.

**Step 6 ends Phase Zero** ([decision map](../decision-map.md) §5; [charter](../charter.md) §8). Its last gate is a design review: the owner approves the architecture proposal as a whole, as the [overview](../architecture/overview.md)'s status anticipates, with the roadmap and Milestone 1's acceptance criteria. Implementation then needs the owner's authorization, milestone by milestone.

**The level of design:** the decisions that frame every milestone: what is measured, how, and against which numbers; what is tested, and by which oracle; which milestone delivers what, at which security level. Test cases, benchmark code and harness code are written in the milestones.

## 2. Owner input and scope (2026-10-20)

| Input | Effect |
|---|---|
| Gate C: "CP19 (Merge to main): Authorized. Please record S6's formal results, update the documents and indexes, and merge step-5 into main with --no-ff, followed by pushing main and step-5 to origin." | Step 5 is closed: merged into `main` at `b4948ce` and pushed ([Step 5 tracker](step-5-subsystems.md), CP19) |
| "The latest run result is in target/s6-results/20261020-074751." "Display P3 Color Pattern (S6-Q5): I've captured a screenshot of the three squares at the end of the run. Please analyze the image critically." | S6's second formal run is recorded, and the screenshot is measured: CoreAnimation honours the tags, so S6-Q5's colour condition is met; untagged surfaces aren't converted, so every surface is tagged ([S6 results](s6-results.md) §7). Recorded in ADR-0012 and graphics GR6 |
| "Real-World Site Corpus (OQ-2): Use the following 13 sites as Axiom's standard evaluation corpus for performance budgets, WPT suites, and memory baselines in Step 6" (the list in §2.4) | OQ-2 resolved; §2.4; CP2 and spike S8 (§10.1) |
| "Memory Target B1 (OQ-23): Approved. In Step 6, budget visible window tile memory separately based on surface pixel dimensions (e.g. 25.2 MB for this Retina window), and maintain a 60 MB budget for the core browser process state." | OQ-23 resolved; the [process model](../security/process-model.md)'s B1 and change record; CP6 sets the tile budget |
| "GPU Path Rasterization (OQ-24): Approved. Keep path rasterization in the renderer for now to preserve analytic edge quality. Defer the GR5 display list amendment to the graphics milestone." | OQ-24 deferred; graphics GR5; CP8 places it at the graphics milestone |
| "Renderer Rasterizer: Approved. Adopt vello_cpu as the primary Rust rasterizer for web fonts and paths, subject to crate dependency grading at the text milestone." | Text TX12, the [register](../technology/dependency-register.md) and ADR-0011; CP8 places the grading at the text milestone |
| "Step 6 Authorization: Authorized. Proceed with creating branch step-6 from main and drafting the Phase Zero Step 6 plan (standards baseline, testing strategy, performance budgets, and Milestone 1 roadmap) for Gate A review." | This plan, on branch `step-6`, created from `main` at `b4948ce`. CP1 ends at Gate A (§7) |
| "You can store important scratcpad codes and files in this project folder so that you can use them later even if the system wipes out the scratchpad." | Reusable scripts go in `tools/` (`s6-tails.py` and `png-region-stats.py` so far); other working files in `scratch/`, which git ignores; CLAUDE.md records it |
| "Many of the websites that I have mentioned, requires log in. We need to create a mechanism to log in to get the full experience." | Two mechanisms, both planned here (§2.4): **for measurement now,** S8 gains a logged-in mode, in a dedicated browser profile the owner signs into by hand (§10.1); **in Axiom,** the roadmap gains a login track to L2 and L3, within the security levels (CP8). Gate A decides which accounts are used (§7) |
| **Gate A (2026-10-20):** "Step 6 Plan Approval: Approved as written. The scope, readiness criteria RC1–RC10, 11-checkpoint sequence, and review gates are approved. Proceed through CP2–CP7 without pausing until Gate B." | The plan stands as written (§3–§10). CP2–CP7 run without pausing |
| "Spike S8 (Corpus Census): Authorized. Proceed with measuring the 13 corpus sites across Chrome Canary and Safari. The two-mode design (automated logged-out mode; passive, non-automated logged-in mode in a dedicated scratch profile) is approved." | S8 runs at CP3 (§10.1). CP2 found that eight sites' terms forbid automated access even logged out, so S8 measures those by hand, in the approved passive mode ([corpus](../corpus.md) §3; §10.1 here) |
| "Account Policy: Approved as recommended: For Spike S8 measurements in shipping browsers, use my regular accounts via the dedicated local profile. For Axiom engine testing, sensitive personal accounts will wait for Level 3 (L3) verification, while non-sensitive test accounts may be used starting at Level 2 (L2)." | [Corpus](../corpus.md) CO7 and CO8; the roadmap's login track (CP8) |
| "URL Parameter Scrubbing: Approved. Please amend the local CP1 commit to remove the identifying parameters (Prism's u= and Figma's t=) from the committed repository documents, while preserving the full URLs in scratch/ for Spike S8." | CP1's commit amended before the first push (`e86ea98` became `a73ccd9`); the full URLs stay in `scratch/corpus-urls.txt` ([corpus](../corpus.md) CO13) |
| "Push Branch: Authorized. Push branch step-6 to origin after amending the commit, and keep it pushed as checkpoints land." | `step-6` pushed on 2026-10-20 at `a73ccd9`, and kept pushed |
| **Gate B (2026-10-20):** "ADR-0018 (Corpus, Standards Baseline, and Testing Strategy): Accepted as proposed." | [ADR-0018](../adr/0018-corpus-standards-and-testing.md) accepted: the [corpus](../corpus.md), the [standards baseline](../standards.md) and the [testing strategy](../testing.md) are binding |
| "ADR-0019 (Performance Methodology and Budgets): Accepted as proposed, including the B1-B7 process model amendment (B1 tile scaling formula and B3 per-page memory ceilings)." | [ADR-0019](../adr/0019-performance-methodology-and-budgets.md) accepted: the [performance budgets](../performance.md) are binding, and the [process model](../security/process-model.md)'s §7 and change record carry the final B1–B7 |
| "Automation Boundary: Confirmed. Automating only the 5 permitted pages and reserving the other 8 for manual testing was the correct decision to respect site terms." | [Corpus](../corpus.md) CO3 stands as written |
| "Spike S8 Manual Session: Completed. I have executed the manual testing sessions across both Chrome Canary and Safari. The formal results are recorded on disk: Chrome Canary (Signed-in, 8 sites): target/s8-results/20261020-181706-hand-chrome-signed-in/ Safari (Logged-out, 5 sites): target/s8-results/20261020-190125-hand-safari-logged-out/ Please integrate these baselines into docs/corpus.md, docs/performance.md, and s8-results.md to calibrate the final B3 ceilings for all 13 sites." With it: "I have used my secondary accounts for these tests," and Prism's URL for that account | [S8 results](s8-results.md) §7; the [corpus](../corpus.md)'s baselines (§5) and CO7; every page's ceiling in the [performance budgets](../performance.md) §4.1. Prism's new URL differs from the first only in `u=`, which stays out of the repository (CO13); `scratch/corpus-urls.txt` holds it. The Safari runs need repeating: the tool kept earlier pages open in Safari (§11) |
| "Spike S9 (Testing and Fuzzing Tooling): Authorized. Proceed with validating cargo-fuzz, AddressSanitizer/ThreadSanitizer, and wptrunner on this Mac." | S9 runs before CP8 ([testing](../testing.md) §9; §10 here) |
| "Not-Planned List: Approved as written in docs/standards.md §5.3 (DRM, plugins, Web SQL, AppCache, WebXR, Web MIDI, etc.)." | [Standards](../standards.md) §5.3 |
| "Cadence: Approved. Proceed through CP8–CP10 (Roadmap, Milestone 1 specification, and Phase Zero final synthesis) without pausing up to Gate C." | CP8–CP10 run without pausing, to Gate C |

### 2.1 Binding inputs

Every one of these is accepted, so Step 6 plans within them:
- **[ADR-0001](../adr/0001-scope-and-target.md):**
  - the full modern app web must stay reachable, and milestones begin with static documents;
  - our own JavaScript engine;
  - sensitive logins, gated until the isolation milestone is verified;
  - DRM-protected media is a non-goal;
  - performance budgets are measured on the target M1 with 8 GB.
- **[ADR-0002](../adr/0002-implementation-language.md):** Rust, the `unsafe` policy, and the build thresholds T1–T4.
- **ADRs [0003](../adr/0003-gc-handle-and-rooting-pattern.md)–[0009](../adr/0009-embedder-api-and-shell.md):**
  - the process model, its security levels L0–L3 and the checks V1–V9 that gate L3 ([threat model](../security/threat-model.md) §8);
  - memory targets B1–B7, with B1 revised (§2) and the final values left to Step 6 (ADR-0004, D6);
  - the JIT policy J1–J7;
  - IPC, the event loop, the pipeline and its invalidation lifecycle, input, and the embedder API with its headless embedder.
- **ADRs [0010](../adr/0010-dependency-and-supply-chain-policy.md)–[0017](../adr/0017-javascript-engine-and-bindings.md):** the dependency policy, whose advisory routine Step 6 sets, and the subsystem designs. Each names its specifications and suites, its costs and its staging (Step 5's SC5, SC6, SC9).
- **The [overview](../architecture/overview.md) v1:** the crates, the end-to-end scenarios, and the staging from L0 to L3.
- **The [charter](../charter.md):** goals G1–G5; principle 7, no architectural corners cut; and Phase Zero's success criteria (§8).

### 2.2 Questions earlier steps handed to Step 6

| # | Question | Raised in | Checkpoint |
|---|---|---|---|
| 1 | The target-site corpus: per tier, the sites the owner uses (resolved: §2.4; at CP2, the [corpus](../corpus.md)) | [OQ-2](../open-questions.md); [charter](../charter.md) §8 | CP2 |
| 2 | **Logging in to the corpus's sites,** for their full experience (the owner, §2): how logged-in pages are measured now, in shipping browsers, without code handling credentials; and how Axiom reaches them: credentials, cookies and sessions, sign-in redirects and pop-ups, passkeys. Under the threat model, accounts with write access are sensitive and wait for L3 | The owner, 2026-10-20; [threat model](../security/threat-model.md) §8; [OQ-16](../open-questions.md); §2.4 here | CP2, CP3; CP8 |
| 3 | The standards baseline and support matrix; each subsystem's specifications and suites, whose pass rates Step 6 sets; the approximate behaviours to classify (`execCommand`, automatic table layout, `Error.prototype.stack`, `Date.parse`'s legacy formats) | [Decision map](../decision-map.md) X1; [Step 5 tracker](step-5-subsystems.md) §5, SC5; [overview](../architecture/overview.md) §5.2 | CP4 |
| 4 | The testing and fuzzing strategy | [Decision map](../decision-map.md) X2 | CP5 |
| 5 | WPT's harness on the headless embedder (`testharness.js`, `testdriver`); reference tests, crash tests and fuzzing harnesses | [Embedder API](../architecture/embedder-api.md) open question 3; [ADR-0009](../adr/0009-embedder-api-and-shell.md) | CP5 |
| 6 | Fuzzing every parser the designs name: HTML and XML, differentially against html5ever and xml5ever; CSS; layout with generated documents; JavaScript's parser, bytecode compiler and `RegExp`; image decoders; URLs, HTTP/1.1, HPACK, QPACK, cookies and decompression | [HTML and the DOM](../architecture/html-and-dom.md) HD33; [style](../architecture/style.md) ST36; [layout and paint](../architecture/layout-and-paint.md) LY32; [the JavaScript engine](../architecture/javascript-engine.md) JE32; [images and media](../architecture/images-and-media.md) IM10; [networking](../architecture/networking.md) | CP5 |
| 7 | Test-only differential references, under DP2: html5ever, xml5ever, Taffy, `regress`, `temporal_rs`, `num-bigint` | [Layout and paint](../architecture/layout-and-paint.md) open question 3; [register](../technology/dependency-register.md) | CP5 |
| 8 | How the L3 gate's checks V1–V9 are built: sandbox escape suites, IPC fuzzing, lock enforcement, cross-site leak suites, the `unsafe` audit, the dependency audit, the JIT policy, the threat-model review, out-of-process iframe tests | [Threat model](../security/threat-model.md) §8 | CP5; CP8 places them |
| 9 | The routine of advisory checks and rebuilds for a browser built locally | [ADR-0004](../adr/0004-process-model-and-sandbox.md), "Step 6 sets"; [threat model](../security/threat-model.md) §9; [dependency policy](../technology/dependency-policy.md) | CP5 |
| 10 | How checks that depend on the macOS version (sandbox rules, CoreText, ImageIO, Security, CoreAnimation) repeat after updates | [Step 5 tracker](step-5-subsystems.md) §9 | CP5 |
| 11 | Limits for hostile documents: recursion depth, boxes per formatting context, path complexity, image dimensions | [Layout and paint](../architecture/layout-and-paint.md) open question 5; [images and media](../architecture/images-and-media.md) IM10 | CP5, CP6 |
| 12 | Benchmarks kept from the spikes: S1's dispatch case, and S6's compositor traces | [Bindings](../architecture/bindings.md) WB17, WB23; [S3 results](s3-results.md) §9; [S6 results](s6-results.md) | CP5, CP6 |
| 13 | The performance methodology and budgets | [Decision map](../decision-map.md) X3 | CP6 |
| 14 | Final memory budgets B1–B7: B1's tile budget by surface pixels (OQ-23), B5's renderer limit, B7's per-renderer cap | [ADR-0004](../adr/0004-process-model-and-sandbox.md), D6; [process model](../security/process-model.md) §7 | CP6 |
| 15 | The collector's pause budget | [OQ-13](../open-questions.md); [script and heap](../architecture/script-and-heap.md) SH10 | CP6 |
| 16 | The event loop's starvation bounds, timer leeway, hang deadlines, hidden-page throttling and per-process thread cap; Apple's idle-QoS target as a design check | [Event loop and threading](../architecture/event-loop-and-threading.md) open question 1; [ADR-0006](../adr/0006-event-loop-and-threading.md) | CP6 |
| 17 | Deadlines for listeners that may block scrolling and for imprecise hit tests; display-port margins | [Invalidation](../architecture/invalidation.md) open question 1; [input and accessibility](../architecture/input-and-accessibility.md) open question 1 | CP6 |
| 18 | IPC timeouts for synchronous requests, backlogs and queue limits | [IPC](../architecture/ipc.md) open question 3 | CP6 |
| 19 | Per-frame overhead, the back-forward cache's memory, and how `beforeunload` is bounded across processes | [Frame tree and navigation](../architecture/frame-tree-and-navigation.md) open questions 2 and 4 | CP6 |
| 20 | The HTML parser's time budget and read-ahead bound; the node arena's density; a parser thread, by profiling | [HTML and the DOM](../architecture/html-and-dom.md) HD5, HD11, open question 3 | CP6 |
| 21 | Style's caches, the user-agent stylesheet's parse, style and layout memory against B3; parallel style and layout, by profiling | [Style](../architecture/style.md) open questions 3, 4, 6; [layout and paint](../architecture/layout-and-paint.md) open question 4 | CP6 |
| 22 | The engine's thresholds (ropes and slices, BigInt algorithms) and memory against B3; the generated bindings' size and build time against T1–T4 and B2 | [The JavaScript engine](../architecture/javascript-engine.md) open question 3; [bindings](../architecture/bindings.md) open question 1 | CP6 |
| 23 | Storage quotas, area limits and eviction thresholds; image decoder limits | [Storage and workers](../architecture/storage-and-workers.md) open question 2; [images and media](../architecture/images-and-media.md) open question 4 | CP6 |
| 24 | Tiles budgeted by surface pixels, with this Mac's scaled display mode | [S6 results](s6-results.md) §8, items 6 and 9; [graphics](../architecture/graphics.md) GR12 | CP6 |
| 25 | Which milestone reaches each security level; the milestone for out-of-process iframes, mandatory before L3 | [ADR-0004](../adr/0004-process-model-and-sandbox.md), "Step 6 sets"; [threat model](../security/threat-model.md) §8; [OQ-18](../open-questions.md) | CP8 |
| 26 | Decision points to place: WebAssembly ([OQ-3](../open-questions.md)), a JIT (J1), the GPU process, passkeys ([OQ-16](../open-questions.md)), Xcode ([OQ-7](../open-questions.md)), QoS overrides ([OQ-20](../open-questions.md)), DNS over HTTPS | [ADR-0004](../adr/0004-process-model-and-sandbox.md); [networking](../architecture/networking.md) open question 3; the open questions | CP8 |
| 27 | Decisions deferred to milestones: GR5's amendment (OQ-24) and surfaces allocated ahead (graphics); `vello_cpu`'s and `smallvec`'s grades (text); the storage engine (SW2); demuxers, and media instances per site (media); AWS-LC's jitter entropy (network); objc2's text-input and accessibility classes (shell, EM11) | The designs' open questions; [S6 results](s6-results.md) §8; [S7 results](s7-results.md) | CP8 |
| 28 | The assumption that the owner is the only human developer | [OQ-9](../open-questions.md) | CP8 |
| 29 | Milestone 1's measurable acceptance criteria | [Charter](../charter.md) §8 | CP9 |
| 30 | Approval of the architecture proposal as a whole; the planned `requirements.md`; Phase Zero's success criteria | [Overview](../architecture/overview.md) status; [docs index](../README.md); [charter](../charter.md) §8 | CP10 |

### 2.3 Scope

**In scope:** the corpus, the three cross-cutting decisions X1–X3, the roadmap, Milestone 1, requirements traceability, the overview's revision, and the spikes the owner authorizes.

**Out of scope.** Each item names where it is decided.
- **Production code, a project skeleton and dependencies:** none in Phase Zero. Milestone 1 starts only when the owner authorizes it, after Gate C; CP9 lists what that authorization covers.
- **The decisions the roadmap places but doesn't take:** WebAssembly (OQ-3), a JIT (J1), GR5's amendment (OQ-24), the storage engine (SW2) and the other milestone decisions in §2.2, item 27. Each is taken at its milestone, on its evidence.
- **Calendar dates.** The project has no deadline and no hourly budget (OQ-1); the roadmap orders and gates milestones, and each milestone is planned in checkpoints when it starts.
- **Changes to accepted ADRs:** an amendment for the owner, recorded in the ADR it changes. The final budgets are the owner's under ADR-0004, D6, so CP6 proposes them and CP7 puts them to the owner.

**Evidence without code,** as in earlier steps: specifications, WPT's published results (wpt.fyi), test262, engines' documentation and sources, published benchmark suites, and observations of this Mac (tools present, the display mode). Measurements come only from authorized spikes.

### 2.4 The corpus

**Superseded at CP2 by the [corpus](../corpus.md),** which classifies each site, records its terms on automation, and defines how it is measured. This section keeps the plan's first classification.

As the owner gave it, with a first classification, except that two URL parameters that may identify the owner are left out (below). **The tiers and needs are inference,** from background knowledge of these sites, to be checked by CP2 and spike S8. "Earliest level" applies the [threat model](../security/threat-model.md)'s §8: L1 permits browsing without logging in, L2 logging in to accounts that aren't sensitive, and L3 accounts with write access.

| # | Site (owner's label) | URL, as given | Tier | Needs beyond a static document (inference) | Login | Earliest level |
|---|---|---|---|---|---|---|
| 1 | Google (search results) | https://www.google.com/search?q=Premier+League | Content | Script-built result modules (live scores, panels), web fonts, images | No | L1 |
| 2 | YouTube (video player and comments) | https://www.youtube.com/watch?v=2Zhv7SjAKeg | Media | Custom elements and Shadow DOM, Media Source Extensions video, lazily loaded comments | No | L1 |
| 3 | Instagram (media feed) | https://www.instagram.com/leomessi | Content | A script-rendered feed of images and video; a login wall after some scrolling | Partly | L1 logged out; L3 logged in |
| 4 | Reddit (community feed) | https://www.reddit.com/r/RedditGames/ | Content | Web components, infinite scrolling; interactive posts embedded in frames | No | L1 |
| 5 | Wikipedia (article layout and math) | https://en.wikipedia.org/wiki/Pi | Document | Mathematical formulas, tables, images, references; little script | No | L0 |
| 6 | Google Maps (interactive map tiles) | https://www.google.com/maps/search/Times+Square,+New+York+City,+USA | Media and maps | WebGL map rendering, heavy script | No | L1, with WebGL's GPU process (L2) |
| 7 | Twitch (live stream and chat) | https://www.twitch.tv/eslcs | Media | Live video through Media Source Extensions; chat over WebSockets | No | L1 |
| 8 | Discord (web client and channels) | https://discord.com/channels/662267976984297473/999550150705954856 | Application | A script application over WebSockets, with local storage and IndexedDB | Yes | L3, or L2 with a test account |
| 9 | MDN Web Docs (Canvas API tutorial) | https://developer.mozilla.org/en-US/docs/Web/API/Canvas_API/Tutorial/Pixel_manipulation_with_canvas | Document | Live examples in frames, drawing with canvas 2D (`getImageData`, `putImageData`) | No | L0 |
| 10 | Three.js (WebGPU compute particles) | https://threejs.org/examples/webgpu_compute_particles.html | Application | WebGPU compute and rendering; JavaScript modules | No | L2, with WebGPU's GPU process |
| 11 | Figma (design editor canvas) | https://www.figma.com/make/uulSQtKZ5FbRgWPRc0EmVo/Doodle-Fonts--Community-?p=f (`t=` left out) | Application | WebAssembly and GPU-rendered canvases; possibly a login | Possibly | L3, or L2 with a test account |
| 12 | VS Code Online (browser IDE) | https://vscode.dev/ | Application | A large script application: workers, a service worker, IndexedDB, the File System Access API | No | L1 |
| 13 | Prism (document editing) | https://prism.openai.com/?pg=1&m=MIT-thesis-template%2FMIT-Thesis.tex (`u=` left out) | Application | A script editor for LaTeX documents with a rendered preview, and collaboration | Yes | L3, or L2 with a test account |

**What this suggests** (inference, for CP2 and CP8):
- **Two sites are documents** (Wikipedia and MDN); three are content sites, three media or maps, and five applications. So the first milestones can target only the documents tier, and the rest become targets as their capabilities arrive.
- **GPU features gate three sites:** WebGL for Google Maps and Figma, WebGPU for Three.js and possibly Figma. Each needs the GPU process (GR10, GR14).
- **WebAssembly** gates Figma at least, so OQ-3 must be decided before Figma can be a target.
- **Media** (Media Source Extensions, and the media utility process) gates YouTube and Twitch, and Instagram's video.
- **Logins** gate Discord and Prism, and Figma and Instagram in part; on YouTube, Reddit, Twitch and Google a login adds posting, chat and personalization. **The owner wants the logged-in experience** (§2), which needs two mechanisms:
  - **Measuring logged-in pages now,** in shipping browsers: a dedicated browser profile, kept in `scratch/`, which the owner signs into by hand, so no code handles a credential. In it the spike automates nothing inside the page; it only samples the browser's processes, because sites' terms restrict automated access to their services (Discord's forbid "scraping our services without our written consent, including by using any robot, spider, crawler, scraper" [D1]) and automation in a signed-in page could act for the owner (§10.1).
  - **Logging in with Axiom:** a track in the roadmap (CP8). Under the threat model, accounts with write access are sensitive, so the owner's own accounts wait for L3: out-of-process iframes and the checks V1–V9. Test accounts holding nothing of value, and linked to nothing of the owner's, would be non-sensitive, and usable from L2. That reading of the threat model's §8 is a decision for the owner (§7).
- **Two URLs carried parameters that may identify the owner** (inference): Prism's `u=`, which looks like an account or project identifier, and Figma's `t=`, a share token. The repository is public, so they are left out of it, and the full URLs are kept for S8 in `scratch/corpus-urls.txt`, which git ignores. The owner approved this at Gate A (2026-10-20), and CP1's commit was amended before `step-6` was first pushed, so they are in none of its history.
- **The pages change daily,** with new content, experiments and advertising. CP2 decides how they are captured for repeatable measurement. Captures of third-party content stay on this Mac, under `target/` or `scratch/`, and are never committed.
- **This Mac's display runs scaled** at 1,440 × 900 points, so a full-screen window is 2,880 × 1,800 pixels ([S6 results](s6-results.md) §2). Every measurement states its window size.

## 3. Questions

| ID | Question | Answered at |
|---|---|---|
| Q1 | What does each corpus site need, and cost? Its tier, the platform features it uses, its logins and security level, and the milestone where it becomes an acceptance target. How is it measured repeatably, and what are its baselines in shipping engines on this Mac: memory, document and resource sizes, load and interaction timings, idle cost? | CP2, CP3 |
| Q2 | Which specifications does Axiom implement, at which dated versions, and how fully: to the specification, approximately with a stated reason, staged to a milestone, or not planned? Which suites check each, and what pass rate counts at each milestone? | CP4 |
| Q3 | How is Axiom tested? The harnesses (WPT on the headless embedder, reference tests, html5lib-tests, test262), differential oracles, fuzzing of every parser and IPC decoder, determinism under virtual time, performance regression tests, the checks V1–V9, and the routine of advisory checks and rebuilds. Where do tests run? | CP5 |
| Q4 | How is performance measured on this Mac, and what are the budgets: memory (B1–B7 final, with the tile budget), latency and frame times, throughput, energy and idle, build (T1–T4), and the limits and deadlines the designs left open? How are budgets enforced? | CP6 |
| Q5 | Which milestones lead from L0 to L3 and the app web? Each one's scope, security level, corpus targets and acceptance criteria, and where each decision point and deferred decision falls. How do implementation milestones run? | CP8 |
| Q6 | What exactly is Milestone 1? Its scope, the corpus pages it must render, its measurable acceptance criteria, and what starting it requires the owner to authorize | CP9 |
| Q7 | Does every requirement trace from the charter and the ADRs to a milestone that verifies it? Does the architecture proposal hold as a whole, and are Phase Zero's success criteria met? | CP10 |

## 4. Deliverables

Planned documents are named in code font until they exist.

| # | Deliverable | Checkpoint |
|---|---|---|
| 1 | This plan and tracker | CP1 |
| 2 | [`docs/corpus.md`](../corpus.md): the corpus, each site's tier, needs, logins and level, the measurement protocol and the capture of pages; rules CO | CP2; baselines added at CP3 |
| 3 | Spike S8, if authorized: code in `spikes/s8/`, a script in `tools/`, results in `docs/research/s8-results.md` | CP3 |
| 4 | `docs/standards.md` (X1): the baseline, the support matrix, the suites and their expectations; rules SB | CP4 |
| 5 | `docs/testing.md` (X2): harnesses, oracles, fuzzing, the security checks, the advisory routine; rules TS. Spike S9's design, only if CP5 needs one | CP5 |
| 6 | `docs/performance.md` (X3): the methodology, the budgets, the limits and deadlines, and how each is enforced; rules PF | CP6 |
| 7 | ADRs for part 1, Proposed. Tentatively: ADR-0018, the standards baseline and testing strategy (X1, X2); ADR-0019, performance methodology and budgets (X3), with the final B1–B7 as an amendment to the process model for the owner. CP7 settles the grouping | CP7 |
| 8 | `docs/roadmap.md`: the milestones, their security levels, corpus targets and decision points, and how implementation milestones run; rules MS | CP8 |
| 9 | `docs/milestone-1.md`: Milestone 1's scope and measurable acceptance criteria, and what starting it requires; rules MA | CP9 |
| 10 | `docs/requirements.md`, planned since Step 1: each requirement traced from its source to the milestone that verifies it; entries RQ. The [overview](../architecture/overview.md), v2. ADRs for part 2, Proposed: tentatively ADR-0020, the roadmap and its security levels, and ADR-0021, Milestone 1 | CP10 |
| 11 | Updates to the decision map, the open questions, the charter's §8, the indexes, status and the spikes README | CP7, CP10 |

**Each Step 6 document** has the structure of earlier design documents: status, question, what it feeds and builds on; a summary; binding inputs; sources and precedent, dated; the design as numbered rules; staging across milestones; open questions and sources. Each becomes binding when its ADR is accepted.

## 5. Readiness criteria

Step 6's outputs are judged against these criteria. CP10 checks every one and records the evidence in the overview.

| # | Criterion | From |
|---|---|---|
| RC1 | **Measurable.** Every budget and acceptance criterion is a number with its method of measurement on this Mac: the tool, the conditions (AC power, Low Power Mode off, an otherwise idle Mac, cool-downs, the window size and display mode) and the statistic (the median, with tails reported) | Charter G4 and §8; S1–S7's methods |
| RC2 | **Traceable.** Every budget, baseline and criterion traces to evidence (a spike, the corpus census, a dated source), or is a labelled hypothesis that names the milestone that will test it | Charter G5, principle 5; SC10 |
| RC3 | **Grounded in the corpus.** Every corpus site has a tier, its needs, its logins and security level, and the milestone where it becomes an acceptance target. Budgets and conformance priorities cite the corpus | OQ-2 |
| RC4 | **Standards first.** Each specification Axiom implements is named with a dated version and a support class (to the specification; approximate, with a reason; staged; not planned), and each class has its suites and expectations | Charter principle 5; SC5 |
| RC5 | **Security-gated.** Each milestone names its security level. Credentials wait for L2 and sensitive accounts for L3, whose checks V1–V9 are planned as tests | ADR-0001 R3; threat model §8; ADR-0004 |
| RC6 | **Staged, never cut.** The roadmap stages implementation within the accepted ADRs. A milestone that needs an architectural change raises an amendment for the owner | Charter principle 7; SC1 |
| RC7 | **Efficiency is budgeted.** Memory, CPU, energy and idle cost each have budgets, consistent with B1–B7 as revised, on the 8 GB M1, including tiles by surface pixels | Charter G4; ADR-0004 D6; OQ-23 |
| RC8 | **Testable at every boundary.** Every parser and IPC decoder is fuzzed; output is deterministic under virtual time; incremental results are checked against rebuilds; dependencies are checked against advisories on a routine | ADR-0002 rule 5; IV14; EL24; threat model §9 |
| RC9 | **Paced by checkpoints, not dates.** Milestones are ordered and gated, not scheduled, and each splits into checkpoints that leave the workspace clean and working | OQ-1; charter §7 |
| RC10 | **Phase Zero is complete.** At the final gate the architecture proposal and roadmap are put to the owner, every foundational decision has an accepted ADR, the corpus is defined, and Milestone 1's measurable acceptance criteria are written | Charter §8 |

## 6. Sequence and checkpoints

**The order: the corpus, then evidence, then the three cross-cutting decisions, then the roadmap.**
1. **The corpus first (CP2).** Every later checkpoint uses it: its features set the standards' priorities, its pages carry the tests and benchmarks, and its tiers order the roadmap.
2. **The corpus census next (CP3),** if the owner authorizes spike S8, because its feature census and baselines feed CP4–CP6. Without it, those checkpoints rest on sources and spike results alone, and say so.
3. **Standards, then testing, then budgets (CP4–CP6).** Tests check the standards, and budgets are enforced by benchmarks and tests, so each consumes the one before.
4. **Synthesis and Gate B (CP7).**
5. **The roadmap (CP8), then Milestone 1 (CP9),** which needs the roadmap's first milestone and the budgets, then the final synthesis and design review (CP10), then the merge (CP11).

**Every checkpoint ends with:**
- every relative link resolving, checked by `tools/check-doc-links.py`. Planned documents are named in code font, not linked, until they exist;
- the four standard checks green. Only spike checkpoints change code;
- a commit on `step-6`, and an update to this tracker.

| CP | Scope | Status |
|---|---|---|
| **CP1** | This plan; branch `step-6`; the owner's decisions recorded | ✅ 2026-10-20 (§11). **Gate A passed** 2026-10-20: approved as written |
| | **Part 1: evidence and the cross-cutting decisions** | |
| CP2 | The corpus: each site's tier, needs, logins and level; the measurement protocol; capturing pages | ✅ 2026-10-20 (§11; [corpus](../corpus.md)). Eight sites' terms forbid automated access, so S8 measures them by hand |
| CP3 | Spike S8: the corpus census, if authorized | ✅ 2026-10-20 (§11; [results](s8-results.md)). The automated part: five pages, three valid runs each. The session by hand (eight pages, and Safari) awaits the owner at Gate B |
| CP4 | The standards baseline and support matrix (X1) | ✅ 2026-10-20 (§11; [standards baseline](../standards.md)) |
| CP5 | The testing and fuzzing strategy (X2) | ✅ 2026-10-20 (§11; [testing strategy](../testing.md)). Spike S9 designed for Gate B |
| CP6 | Performance methodology and budgets (X3) | ✅ 2026-10-20 (§11; [performance budgets](../performance.md)). The final B1–B7 and the corpus pages' ceilings for the owner at Gate B |
| CP7 | Synthesis of part 1: ADRs (Proposed); the final budgets for the owner; decision map, open questions, indexes, status. **Gate B** | ✅ 2026-10-20 (§11; ADRs [0018](../adr/0018-corpus-standards-and-testing.md) and [0019](../adr/0019-performance-methodology-and-budgets.md) proposed). **Gate B passed** 2026-10-20: both ADRs accepted |
| (Gate B) | The owner's decisions recorded; S8's session by hand folded in, with every page's ceiling; spike S9, authorized at Gate B | ✅ 2026-10-20 (§11). S9 ([results](s9-results.md)): Q1 and Q3 pass; Q2 fails with the installed nightly. The pass-rate tool corrected, and the standards' calibration recomputed |
| | **Part 2: the roadmap** | |
| CP8 | The roadmap: milestones, security levels, corpus targets, decision points | ✅ 2026-10-20 (§11; [roadmap](../roadmap.md)) |
| CP9 | Milestone 1: scope and measurable acceptance criteria | ✅ 2026-10-20 (§11; [Milestone 1](../milestone-1.md)) |
| CP10 | Requirements traceability; the overview v2 and the criteria checks; ADRs for part 2 (Proposed); decision map, open questions, indexes, status. **Gate C: the final design review** | Not started |
| CP11 | Merge `step-6` into `main` (`--no-ff`), when the owner instructs | Not started |

### 6.1 What each checkpoint decides

**CP2. The corpus (Q1):**
- **Each site:** its tier, the features it uses, its logins and earliest security level, and the milestone where it becomes a target. Checked against sources where they settle it, and by S8 where they don't.
- **What is measured on each page:** the page as loaded, idle afterwards, and a scripted interaction per site: scrolling a feed, playing a video, typing in an editor, panning the map.
- **Repeatability:** live pages vary daily, so CP2 decides between repeated live runs, saved copies for the documents, and a recording proxy for applications, as Chromium's Web Page Replay does (to be read). Captures stay on this Mac and are never committed.
- **Logins:** what the logged-in experience adds on each site, and which account each uses: the owner's own, or a test account. In shipping browsers, logged-in pages are measured in a dedicated profile the owner signs into by hand, passively (§10.1). In Axiom, the owner's own accounts wait for L3; test accounts holding nothing of value count as non-sensitive and serve from L2, if the owner agrees at Gate A.
- **Reference browsers:** Safari (WebKit) and Chrome Canary (Chromium), both installed on this Mac (observed in `/Applications`), with their versions recorded.
- **The window:** one stated size for measurements, with the scaled display mode recorded.

**CP3. Spike S8, the corpus census** (§10.1), if authorized at Gate A. Results in `docs/research/s8-results.md`; baselines added to the [corpus](../corpus.md).

**CP4. The standards baseline and support matrix (X1; Q2):**
- **Versions:** how each body's specifications are pinned (WHATWG living standards by review draft or commit date, W3C reports by date, ECMA-262 and ECMA-402 by edition, Unicode by version, IETF RFCs by number), and how often the baseline moves.
- **Support classes** for every feature area: to the specification; approximate, with the reason (`execCommand`, automatic table layout, `Error.prototype.stack`, `Date.parse`'s legacy formats); staged to a milestone; not planned (XSLT deferred, OQ-22; DRM, a non-goal).
- **Suites:** the WPT directories each subsystem document named, html5lib-tests, test262, css-parsing-tests, Unicode's conformance files, and later WebGPU's and WebAssembly's suites. How expectations are recorded per test, and how regressions show.
- **Priorities:** S8's feature census ranks the areas; wpt.fyi's results for shipping engines, dated, show what a directory's full pass rate looks like.

**CP5. The testing and fuzzing strategy (X2; Q3):**
- **Harnesses:** WPT on the headless embedder, with `testdriver` mapped onto the embedder API (EM9); reference tests; html5lib-tests; test262; crash tests. Determinism through EL24's virtual time.
- **Oracles:** the test-only differential references (§2.2, item 7); IV14's comparison of incremental updates with rebuilds; ST22's cache-free style path.
- **Fuzzing:** targets (every parser and IPC decoder), the tool (for example `cargo-fuzz` with libFuzzer on nightly Rust, to be checked on this Mac), seed corpora from the suites and the site corpus, coverage, and how crashes are triaged and kept as tests.
- **The checks V1–V9:** how each is built, from S4's, S5's and S7's sandbox probes to IPC fuzzing and the cross-site leak suites, and when each runs.
- **Performance regression tests:** the benchmarks kept from the spikes, and corpus benchmarks with the budgets as thresholds.
- **The advisory and rebuild routine:** RustSec advisories through `cargo-deny`, audits through `cargo-vet` (DP rules), and macOS updates, after which the sandbox-rule tests and the other checks that depend on the OS run again; its cadence.
- **Where tests run:** this Mac, and whether a hosted continuous-integration service adds anything for a public repository with one developer.

**CP6. Performance methodology and budgets (X3; Q4):**
- **Methodology,** from S1–S7: release builds; AC power with Low Power Mode off; an otherwise idle Mac, with the load average recorded; cool-downs; three runs; medians with tails reported; the window's visibility and size. Tools: `samply`, `footprint`, `vmmap`, `top`'s energy impact, `proc_pid_rusage`, GPU timestamps and `/usr/bin/time -l`. Instruments only after OQ-7.
- **Memory:** B1's 60 MB for the core browser and a tile budget by surface pixels (OQ-23); B2–B7's final values; per-subsystem memory (the DOM, style, layout, the JavaScript heap, caches) against B3 for each corpus tier, from S8's baselines.
- **Time:** input to paint, scrolling and frame times (S6's thresholds), page-load milestones per tier, and parse, style and layout throughput.
- **Energy and idle:** no wakeups while idle (AC3, PM8), and `top`'s energy impact for animation and video.
- **Build:** T1–T4, with the bindings' generated code included.
- **Limits and deadlines:** the values the designs left open (§2.2, items 15–24).
- **Enforcement:** each budget becomes a benchmark or test with a threshold, checked at every milestone.

**CP7. Synthesis of part 1:**
- ADRs for X1–X3, Proposed; the final B1–B7, proposed as an amendment to the process model for the owner.
- The decision map, the open questions, the indexes and status.
- **Gate B** (§7).

**CP8. The roadmap (Q5):**
- **Milestones** from L0 to L3 and the app web, each with its scope (subsystems and features, from the overview's staging and the designs' staging sections), its security level, its corpus targets (CP2), and the shape of its acceptance criteria.
- **Decision points and deferred decisions,** each placed (§2.2, items 26 and 27). Out-of-process iframes come before L3 (OQ-18).
- **The login track** (§2.2, item 2): what logging in needs from each subsystem, and the milestone of each step. Forms and password fields (HD27); the cookie jar, with partitioning (NW11); storage (SW); redirects and pop-ups for single sign-on, with their opener rules (FT); whether the browser keeps passwords, in the Keychain the process model allows it; passkeys (OQ-16); and the bot checks an unfamiliar browser meets. Test accounts from L2, if the owner agrees; the owner's own accounts at L3.
- **How implementation milestones run:** a branch per milestone, checkpoints, gates, the checks and the budgets.
- No calendar dates (OQ-1).

**CP9. Milestone 1 (Q6):**
- **Scope.** Hypothesis, to be settled by CP8: the static-document pipeline at L0. That is HTML, the DOM, CSS, layout, text, images, paint, the compositor and a shell window, loading local files and public documents.
- **Corpus pages:** from the documents tier.
- **Acceptance criteria,** each measurable: pass rates on named suites, corpus checks (the page renders and reads correctly, links navigate, no crash or hang), budgets, and L0's rule of no credentials.
- **What starting it requires,** for the owner to authorize: a project skeleton, the first dependencies adopted after their grades (ICU4X, HarfRust and `vello_cpu` among them), and the tooling.

**CP10. The final synthesis:**
- `docs/requirements.md`, the traceability of every requirement.
- The overview v2: the budgets and the roadmap's staging, and the checks of AC1–AC10, SC1–SC10 and RC1–RC10.
- The charter's Phase Zero criteria, checked.
- ADRs for part 2, Proposed; the decision map, the open questions, the indexes and status.
- **Gate C** (§7).

**CP11. Merge** `step-6` into `main` with `--no-ff`, when the owner instructs.

## 7. Review gates

Between gates, checkpoints run without pausing, as in Steps 2b to 5: CP2–CP7, then CP8–CP10.

| Gate | After | The owner decides |
|---|---|---|
| A | CP1 | **This plan:** its scope (§2.3), the readiness criteria (§5), the sequence and checkpoints (§6), these gates and the cadence. **Spike S8** (§10.1), with its logged-in mode. **The accounts:** the owner's own or test accounts, for measurement now and for Axiom's levels (§2.4). **The corpus's two identifying parameters** (§2.4). **Pushing** `step-6` to `origin` |
| B | CP7 | **The ADRs for part 1** (Proposed), and **the final memory budgets** as an amendment to the process model. Spike S9, if CP5 proposes one. Anything S8 found that changes the corpus. **S8's session by hand,** which needs the owner, with the boundary CP2 drew for automation ([corpus](../corpus.md) §3) and the corpus's open questions 1 and 2 (Chrome's feature counters saved by hand; Safari's remote automation) |
| C | CP10 | **The final design review:** the ADRs for part 2 (the roadmap, Milestone 1); the overview v2, as the architecture proposal as a whole; the requirements' traceability; Phase Zero's completion. Then merging `step-6` into `main`. Milestone 1's implementation is a separate authorization |

## 8. Method

- **Sources** follow the [research conventions](README.md): primary first, each dated, every claim labelled.
- **Standards:** the WHATWG's and W3C's publication policies, the specifications the subsystem documents name, test262, and WPT with its published results (wpt.fyi). The Interop project's focus areas, as a signal of what engines prioritize.
- **Testing precedent:** how Chromium, Firefox, WebKit, Servo and Ladybird run WPT and their own suites; how they fuzz (OSS-Fuzz, ClusterFuzz); how Rust projects fuzz (`cargo-fuzz`, `libfuzzer-sys`).
- **Performance precedent:** published suites (Speedometer, MotionMark, JetStream) and how engines use them; page-load metrics and their definitions; Apple's guidance on energy; Chromium's Web Page Replay for repeatable page loads.
- **Roadmap precedent:** how Servo and Ladybird staged their engines, and what Chromium's and Firefox's multi-process and site-isolation retrofits teach about ordering.
- **This Mac:** the tools present, the display mode, the browsers installed. Measurements only in authorized spikes.
- **The corpus:** aggregate measurements only. No page content, screenshots, cookies or credentials are committed, and nothing from a logged-in page beyond aggregate figures.

## 9. Risks to this plan

| Risk | Mitigation |
|---|---|
| **The corpus is mostly applications** (§2.4), far ahead of the first milestones | Tiers: each site becomes a target at the milestone its capabilities arrive, and early milestones use the documents tier. Budgets for later tiers are hypotheses, labelled, until a milestone measures them |
| Budgets set before an engine exists are guesses | Each traces to a measurement (S1–S8) or a source, or is labelled a hypothesis with the milestone that tests it (RC2). Milestones revisit them on evidence, with the owner |
| Live pages change, block automation, or ask for consent or logins | CP2's capture decision; three runs; logged-out pages and test accounts; every run records the date and the browser's version |
| WPT is too large to run whole on an 8 GB laptop | CP4 selects directories per milestone from the corpus's needs; CP5 decides how subsets run and how expectations are kept |
| Reference browsers' numbers vary with their versions | Versions recorded; baselines treated as context for budgets, never as targets to copy |
| The roadmap grows into a schedule | Milestones are ordered and gated, not dated (RC9). Each is planned in checkpoints when it starts |
| A decision deferred to a milestone turns out to need an architectural change | RC6: an amendment for the owner. The designs already name their revisit conditions |
| S8 touches the owner's accounts or settings | Logged-out runs use a temporary profile under `target/`, deleted after each run. Logged-in runs use a dedicated profile in `scratch/`, which the owner signs into by hand and can delete; there the spike automates nothing in the page. No credential in code; Safari's automation setting changed only by the owner, if at all (§10.1) |
| Automating sites breaches their terms, or trips their bot checks | Automation only on public pages, logged out, a few loads per page, with nothing collected but aggregate figures; none in signed-in pages, whose terms can forbid it ([D1]). A page that answers with a check or a consent screen is recorded as such, not worked around. **At CP2 this risk was found real:** eight sites' terms forbid automated access even logged out, so automation loads only the five pages whose terms and robots files permit it ([corpus](../corpus.md) CO3, CO4) |

## 10. Candidate spikes

One spike is designed now, so that its evidence can reach CP4–CP6; a second only if CP5 needs one. **Nothing runs without the owner's authorization.**

| Spike | Area | Question | Why sources can't settle it | What it decides |
|---|---|---|---|---|
| **S8** | The corpus census (designed here, §10.1) | What do the owner's 13 pages use and cost in two shipping engines on this Mac? | The pages are specific, change over time, and are measured nowhere for this machine. Feature use and memory per page need measuring | The corpus's tiers and needs (CP2); the standards' priorities (CP4); memory and time baselines (CP6) |
| (S9) | Test tooling on this Mac (**designed at CP5**, [testing](../testing.md) §9; **authorized at Gate B**, 2026-10-20; **run** 2026-10-20, [results](s9-results.md)) | Does fuzzing with `cargo-fuzz` and libFuzzer run on nightly Rust for this Mac's target, with coverage? Do the sanitizers work? Does wptrunner run here within memory? | Tool support for `aarch64-apple-darwin` changes with toolchains; trying it is the only check | X2's fuzzing tools; WPT's cost on 8 GB |

**No spike is planned for these areas:** the standards baseline (sources and wpt.fyi settle it); the roadmap and Milestone 1 (they assemble decisions already taken); and performance methodology, which S1–S7 already exercised.

### 10.1 Spike S8: the corpus census (designed at CP1)

**Purpose.** Measure, in shipping engines on this Mac, what each corpus page uses and costs, so that the corpus's tiers, the standards' priorities and the budgets rest on measurements of these pages rather than on background knowledge.

| ID | Question | Passes if |
|---|---|---|
| S8-Q1 | **Features.** Which platform features does each page use? Custom elements and Shadow DOM; WebGL and WebGPU contexts; WebAssembly compilation; workers and service workers; IndexedDB and other storage; WebSockets; media (Media Source Extensions, and the codecs requested); canvas 2D; Web Animations; the observers; web fonts (count and formats); image formats; frames | Report. Each feature is detected by an instrumented call or a DOM count that the results describe |
| S8-Q2 | **Size.** How large is each page: elements and documents (frames), DOM depth, stylesheets and rules, script, font and image bytes, requests by type? | Report |
| S8-Q3 | **Memory.** What does each page's content process cost: its footprint after loading, after the scripted interaction, and after 30 s idle, in each engine? | Report, with three runs per page and their spread |
| S8-Q4 | **Time and energy.** When do the page's load milestones arrive, and what does its idle cost (CPU, wakeups and energy impact over 30 s)? | Report |

**Two modes,** because the owner wants the logged-in experience measured too (§2), and signed-in pages must not be automated:

| | Logged out | Logged in |
|---|---|---|
| Pages | The corpus URLs that work without an account | The pages that need one, or that a login changes (§2.4) |
| Browser profile | Fresh, in `target/`, deleted after each run | Dedicated and kept, in `scratch/`, which git ignores; the owner signs in by hand once, and can delete it at any time |
| Who drives the page | The spike: it loads the page and runs the scripted interaction | The owner, by hand: open the page, then the interaction CP2 defines for it |
| What the spike does | Loads, instruments and measures (S8-Q1 to S8-Q4) | Samples only the browser's processes, from outside: memory, CPU, wakeups and energy (S8-Q3, S8-Q4). It injects nothing into the page and reads nothing from it |
| Credentials | None used | Typed by the owner into the browser; never seen, stored or read by code |

**Method:**
- **Chrome Canary,** driven through the Chrome DevTools Protocol in the logged-out mode. CDP's pipe transport (`--remote-debugging-pipe`) needs no WebSocket client, so a Python script with the standard library alone can drive it (to be checked at the start of CP3). Instrumentation registered before each page's scripts counts feature use; the DOM and Resource Timing give the sizes. In the logged-in mode it runs with no debugging port at all, as an ordinary browser.
- **Safari,** for memory and time. Its automation needs the owner to enable remote automation (Develop → Allow Remote Automation, or `safaridriver --enable` with an administrator's password), a setting on the owner's Mac that the spike never changes itself. Without it, Safari is measured in the logged-in style: the owner opens each page, and the spike samples its content processes.
- **Measurements:** `footprint` and `vmmap` on each engine's content processes; `top -stats power`; `proc_pid_rusage` where it applies.
- **Conditions:** AC power, Low Power Mode off, the Mac otherwise idle, cool-downs, three runs per page, a stated window size, and the browsers' versions recorded.
- **Network access:** read-only page loads of the 13 corpus URLs, and of everything those pages load, including third-party resources; in the logged-in mode, whatever the owner's own browsing loads. That is outward-facing, so the owner's authorization must cover it explicitly. Nothing on the Mac is reconfigured by the spike.
- **What is kept:** aggregate figures in the results; raw traces under `target/s8-results/`, which git ignores. No page content, screenshots, cookies or credentials are committed, and nothing from a signed-in page beyond process measurements.
- **Code:** `spikes/s8/` (Python, standard library only) and `tools/s8-measure.sh`. No new dependency; no `unsafe`, since no Rust crate is needed.

**Refined at CP2 (2026-10-20),** within the questions the owner authorized at Gate A:
- **Automation loads only the five pages whose terms and robots files permit it:** Wikipedia, Google Maps, MDN, Three.js and VS Code for the Web ([corpus](../corpus.md) CO3). Its loads identify themselves, stay few and spaced, and never sign in (CO4).
- **The other eight are measured in the passive mode the owner approved** (CO6): by hand, with process sampling only. Each is measured logged out in a temporary profile where the site works logged out, and signed in in the dedicated profile. That needs the owner's hands, so the session by hand is put to the owner at Gate B, with S8's automated results. It is the part of "measuring the 13 corpus sites across Chrome Canary and Safari" that automation may not do.
- **Features from the browser's own counters,** instead of the instrumentation first planned. Chrome counts the platform features each page uses (Blink's use counters, the source of chromestatus.com's usage figures), in histograms that the DevTools Protocol's `Browser.getHistograms` reads. So nothing is injected into the page. The features' names come from Chromium's `web_feature.mojom` at the Canary's version. To be checked at the start of CP3.
- **Sizes** come from the DOM, read in an isolated world, and from the Network domain's events. Nothing runs in the page's own world.
- **Safari:** through WebDriver for the five automated pages if the owner enables its remote automation; otherwise by hand, like the rest.

**Outputs:** `docs/research/s8-results.md`; the [corpus](../corpus.md)'s baselines, logged out and logged in; inputs to CP4 and CP6.

## 11. Results log

### CP1 — completed 2026-10-20

- Branch `step-6` created from `main` at `b4948ce`.
- **The owner's decisions recorded** (§2):
  - **S6's second formal run** (`target/s6-results/20261020-074751/`, at `b4948ce`) reproduced the first: S6-Q1 and S6-Q3 pass at the median again, with the tails again in the session's first runs; S6-Q4's pixels and S6-Q5's footprints match. One idle window recorded a package idle wakeup.
  - **The Display P3 check,** from the owner's screenshot, measured with the new `tools/png-region-stats.py`: the P3-tagged red is (255, 0, 0) in Display P3 and the sRGB-tagged red (234, 51, 35), exactly sRGB's red converted, so CoreAnimation honours both tags; the untagged surface is not converted, so every surface is tagged. S6-Q5's colour condition is met.
  - **The display runs scaled,** 1,440 × 900 points in a 2,880 × 1,800 framebuffer, so S6's window was not full screen; the S6 results are corrected.
  - **B1** revised (OQ-23), **GR5's amendment** deferred to the graphics milestone (OQ-24), **`vello_cpu`** adopted, and **the corpus** set (OQ-2). Recorded in the [S6 results](s6-results.md), ADRs 0011, 0012 and 0016, [graphics](../architecture/graphics.md), [text and fonts](../architecture/text-and-fonts.md), [layout and paint](../architecture/layout-and-paint.md), the [process model](../security/process-model.md)'s B1 and change record, the [register](../technology/dependency-register.md), the [open questions](../open-questions.md), the [decision map](../decision-map.md), the [overview](../architecture/overview.md) §8 and the [charter](../charter.md) §8.
- **Tooling:** `tools/s6-tails.py` (frames over S6's thresholds, from the traces) and `tools/png-region-stats.py` (pixel values and colour profile of a PNG), committed with `--help`; `scratch/` added to `.gitignore` for working files; CLAUDE.md records where files worth keeping go.
- **This plan written:** 7 questions, the 30 questions earlier steps handed to Step 6, the readiness criteria RC1–RC10, 11 checkpoints in two parts, three gates, and spike S8 designed for Gate A, with S9 only if CP5 needs it.
- **Logging in,** as the owner asked during CP1: S8 gains a logged-in mode, in a dedicated profile the owner signs into by hand, where nothing is automated inside the page; the roadmap gains a login track (CP8). Discord's terms were read for the reason ([D1]).
- [Status](../status.md), the [decision map](../decision-map.md) (Step 6 in progress) and the indexes ([docs](../README.md), [research](README.md)) updated.
- The four standard checks and the link check pass.
- **Amended after Gate A, before the first push** (2026-10-20, the owner's Gate A decision 4): Prism's `u=` and Figma's `t=` removed from §2.4.

**Gate A.** The owner reviews this plan. The decisions, with recommendations, are in [status](../status.md), "Next steps".

### Gate A — passed 2026-10-20

The owner's decisions, each as recommended, quoted in full in §2:
- **The plan:** "Approved as written." The cadence: "Proceed through CP2–CP7 without pausing until Gate B."
- **Spike S8:** "Authorized", across Chrome Canary and Safari, with the two-mode design.
- **The accounts:** "Approved as recommended". S8 uses the owner's regular accounts in the dedicated local profile. In Axiom, sensitive personal accounts wait for L3, and test accounts that aren't sensitive may be used from L2.
- **The URL parameters:** CP1's commit was amended before the first push. `e86ea98` became `a73ccd9`; the only other change is a line in CP1's log. Checked: neither value appears in any commit on `step-6` that isn't on `main`, in diffs or messages. The original commit survives only in this Mac's reflog, and was never pushed. The full URLs stay in `scratch/corpus-urls.txt`.
- **The branch:** `step-6` pushed to `origin` at `a73ccd9`, and kept pushed.

CP2 starts.

### CP2 — completed 2026-10-20

[The corpus](../corpus.md) written (**Draft**; rules CO1–CO14). Main decisions:
- **The sites and tiers** (CO1, CO2): two documents, three content sites, three media or map sites, five applications. Each has its needs (inference until measured), what a login adds, and its earliest level.
- **Every site's terms of use read** (corpus §3), with robots.txt matched as RFC 9309 specifies:
  - **Eight sites forbid automated access even logged out:** Google Search (its terms turn on robots.txt, which disallows `/search`), YouTube, Instagram, Reddit, Twitch, Discord, Figma, and Prism under OpenAI's terms (a secondary source, since openai.com refused retrieval).
  - **Five permit occasional automated loads:**
    - Wikipedia, identified and within Wikimedia's Robot and User-Agent policies;
    - Google Maps, whose robots file allows `/maps/search/`, with nothing copied;
    - MDN;
    - Three.js, which publishes no terms;
    - VS Code for the Web.
  - **So S8 automates five pages and measures eight by hand** (§10.1, refined). This narrows the automated mode the owner approved, in the direction of doing less, because the plan cited only Discord's terms. It is put to the owner at Gate B.
- **Automated loads** identify themselves, check robots.txt, load each site at most five times and never sign in (CO4).
- **The accounts** (CO7, CO8), as the owner decided at Gate A, with what makes a test account non-sensitive.
- **The protocol** (CO9–CO11):
  - three phases: loaded, interaction, then 30 s without input;
  - an interaction for each site;
  - a 1,280 × 800-point window;
  - Safari 27.2 and Chrome Canary 158.0.8112.0, observed.
- **Repeatability** (CO12): live baselines. Copies only of Wikipedia's and MDN's pages and of the Three.js example, made by the milestone that needs them. No captures of sites whose terms forbid copying.
- **Privacy and the dedicated profile** (CO13, CO14).
- **Precedent:** Chromium's system-health benchmark, whose 141 desktop stories replay from Web Page Replay archives dated by year.

The `CO` prefix was checked unused, as were the plan's other prefixes. Every relative link resolves; the four standard checks pass.

### CP3 — completed 2026-10-20

**Spike S8, the corpus census,** built and run ([results](s8-results.md)):
- **The tools:** `spikes/s8/`, Python with the standard library only; `tools/s8-measure.sh`; `tools/s8-report.py`. Committed at `6ba5068`, and at `df7b179` after the formal run showed that visibility must be checked at every phase.
  - A DevTools Protocol client over Chrome's pipe, so no port is opened.
  - Features from Chrome's own use counters, so nothing is injected into the pages. They include the WebDX counters, which name web-features' identifiers.
  - Sizes from the Network domain and the DOM read in an isolated world.
  - Processes sampled with `footprint` and `top`.
  - Automated loads identify themselves in the user agent, and are refused where robots.txt disallows them (CO4).
- **The method checked** on a local test page: every feature it uses appeared among the counters.
- **The census:** the five pages automation may load, three valid runs each, in Chrome Canary 158.0.8112.0.
  - Two runs had the page hidden: a page that isn't composited and takes no input. They were left out, and the two pages measured again, which used Wikipedia's and Three.js's fifth and last loads (CO4).
  - In two runs `top`'s wakeup counter failed. Their wakeup figures are left out, and their CPU and energy figures kept.
- **Results:**
  - **Features:** the pages touch 116 of web-features' features, 72 Baseline high, 19 low and 25 in only some engines.
  - **Size:** Wikipedia's article holds 126 MathML formulas and loads 164 SVG images. MDN's page holds 98 custom elements with open shadow roots.
  - **Memory:** each page's steady-state cost in Chrome, above about:blank: Maps 627 MB, Three.js 470, VS Code 457, Wikipedia 319, MDN 145, with the GPU process carrying much of Maps' and Three.js's.
  - **Time:** first contentful paint at 0.8–2.3 s.
  - **The corpus** gains these baselines (§5).
- **Not done here: the session by hand.**
  - The eight pages automation may not load, signed in and logged out.
  - Safari for all thirteen pages. `s8.py safari-status` found Safari's remote automation off (observed).

  It is put to the owner at Gate B.
- **Clean-up:**
  - The Safari check made `safaridriver` launch Safari. A macOS agent it started, `nsattributedstringagent`, left a report of a non-fatal guard fault in `~/Library/Logs/DiagnosticReports`; it was removed. Safari and its WebKit GPU helper, both launched by the check, were ended.
  - No other report appeared.
  - Profiles were deleted after each run.
- **Research data for CP4,** downloaded into `target/`, which git ignores: web-features 3.43.0's `data.json`, and wpt.fyi's run summaries for Chrome, Firefox, Safari, Servo and Ladybird.

Every relative link resolves; the four standard checks and S8's own tests pass.

### CP4 — completed 2026-10-20

[The standards baseline](../standards.md) written (**Draft**; rules SB1–SB12). Main decisions:
- **Pinning by body** (SB1, SB2):
  - WHATWG living standards at a commit snapshot per milestone, with the January or July Review Draft noted;
  - W3C reports at their dated version, or a dated editor's draft where WPT tests it;
  - ECMA-262's 17th and ECMA-402's 13th editions, plus finished proposals;
  - Unicode at ICU4X's version, one across the engine: 17.0.0 for ICU4X 2.3.0, with 18.0.0 released on 2026-09-16;
  - RFCs by number.

  Everything moves together at a milestone's start.
- **Five support classes** (SB3): to the specification, approximate, staged, deferred, not planned. A staged API isn't exposed until it works (SB4). **Four approximate behaviours,** each with its reason (SB5).
- **Priorities** (SB6): the corpus (S8: of 116 web-features the automated pages touch, 72 Baseline high, 19 low, 25 in only some engines), then Baseline, then Interop 2026's 20 focus areas.
- **A support matrix** of about 50 areas, gathered from every subsystem's §2, each with its class, suites and corpus need. A not-planned list: DRM, plugins, Web SQL and AppCache, single-engine device APIs, and others without corpus use.
- **Suites and expectations** (SB7–SB9): wptrunner's metadata, test by test. html5lib-tests' tree-construction tests moved into WPT on 2026-06-26.
- **Targets** (SB10–SB12): a completed area targets the median of Chrome, Firefox and Safari on its directories at the pinned revision. Passing tests don't regress without a reason. Calibrated on wpt.fyi's runs of 2026-10-19 (WPT `0f26972422`) for about 120 directories, with `tools/wpt-pass-rates.py`, new. Ladybird meets the median on `html/syntax/parsing/`, `dom/` and `css/CSS2/`. **Corrected after S9** (2026-10-20, §11): the tool had left reference tests out. Recomputed, Ladybird meets the median on `html/syntax/parsing/` and not on `css/CSS2/`.

Sources read for it: the WHATWG's working mode and policies, the W3C Process, CSS Snapshot 2026, ECMA-262 and ECMA-402, Unicode 18.0, web-features' Baseline definition and its 3.43.0 data, Interop 2026, wpt.fyi's API and runs, and wptrunner's documentation.

Every relative link resolves; the four standard checks pass.

### CP5 — completed 2026-10-20

[The testing strategy](../testing.md) written (**Draft**; rules TS1–TS18). Main decisions:
- **Eight layers of tests,** each with a home and a trigger (TS1).
- **WPT through a WebDriver server in the headless embedder,** compiled only into the test embedder (TS2). wptrunner's Ladybird product is the precedent; it settles the embedder API's open question 3. Reference tests come from EM9's offscreen texture (TS3).
- **Axiom's own runners** for html5lib-tests' tokenizer tests, test262 (with `$262` in full) and Unicode's conformance files (TS4–TS6).
- **Oracles:** the test-only references (html5ever, xml5ever, Taffy, `regress`, `temporal_rs`, `num-bigint`), and the self-oracles IV14, ST22 and HD33 (TS7). **Determinism** on EL24's virtual clock, with the environment injected (TS8).
- **Fuzzing** of every parser and IPC decoder with cargo-fuzz, libFuzzer and AddressSanitizer; ThreadSanitizer for the threads; Miri for `unsafe` (TS9–TS11). Rust's sanitizers on this target: AddressSanitizer and ThreadSanitizer only. libFuzzer is maintained for bug fixes only.
- **The corpus checks** follow the corpus's rules: automated where CO3 permits, by hand elsewhere (TS12). **Benchmarks** kept from the spikes, with the budgets as thresholds (TS13). **V1–V9**, each built as a test (TS14).
- **The routine:** weekly advisory checks, and rebuilds when a crate that parses untrusted input is affected. The OS-dependent checks run again after every macOS update (TS15, TS16).
- **This Mac is authoritative.** GitHub's hosted runners (M1, 3 CPUs, 7 GB, macOS 26 at newest) are offered for correctness only, at Milestone 1 (TS17). WPT runs by directory within 8 GB (TS18).
- **Spike S9 designed** (§9 of the document), for Gate B: cargo-fuzz with AddressSanitizer, ThreadSanitizer, and wptrunner on this Mac.

Found along the way:
- html5lib-tests' tree-construction tests now live only in WPT (2026-06-26).
- `cargo-deny` 0.20.2 is already installed on this Mac, though not configured for Axiom; `cargo-vet` and `cargo-fuzz` aren't.

Every relative link resolves; the four standard checks pass.

### CP6 — completed 2026-10-20

[The performance budgets](../performance.md) written (**Draft**; rules PF1–PF24). Main decisions:
- **The method** (PF1–PF5):
  - S1–S8's conditions made rules: a load average below 3.0 before each run, and visibility checked at every sample;
  - the tools;
  - reference engines as frozen context;
  - **a page's cost includes the GPU and compositor memory allocated for it,** charged to its renderer, which settles the IPC design's open question 4;
  - every budget enforced at the gates.
- **The final B1–B7,** for the owner as an amendment to the process model (PF6–PF12):
  - B1: 60 MB for the core, plus tiles at 6.2 bytes per window pixel, from S6's 6.14;
  - B2: 10 MB, unchanged;
  - B3: a 50 MB median over the documents and content tiers, **with a ceiling for every corpus page**, half of Chrome's measured cost capped by tier: Wikipedia 100 MB, MDN 70, Maps 300, Three.js 235, VS Code 225, and the tier's cap for the eight pages measured by hand until their session;
  - B4: 25 MB; B5: 16; B6: 1 GB for a session of document and content pages; B7: 2 GiB.
- **Time** (PF13–PF15): S6's frame budgets; no engine task over 50 ms (the Long Tasks threshold); the collector's pauses at 4 ms while interacting and 50 ms when idle, the trigger for SH10's incremental marking.
- **Energy** (PF16, PF17): no idle wakeups on a static page; Chrome's throttling of hidden pages' timers; energy impact no higher than Safari's. **Build** (PF18): T1–T4, with the bindings counted.
- **Limits and deadlines** (PF19–PF24):
  - hangs: an offer to stop at 5 s, the end at 30 s;
  - starvation: 33 ms for rendering, 50 ms for tasks;
  - scrolling: 400 ms for listeners that may block it, and one prepainted tile row, from S6's row cost;
  - IPC: queues of 1,024, 16 and 5, and a backlog of 1,024 messages or 8 MiB;
  - navigation: two cached documents per tab, and 1 s for `beforeunload`;
  - the parser: 4 ms and 30 ms slices;
  - storage: quotas from free space, 5 MiB for Web Storage;
  - images: their limits.

  Each is labelled sourced, measured or hypothesis.

Sources read for it: Chromium's, WebKit's and Firefox's hang, throttling, scrolling, parsing and image constants; V8's string thresholds; the Long Tasks draft; the Storage Standard; MDN's account of quotas; Mach's limits in the SDK headers.

Every relative link resolves; the four standard checks pass.

### CP7 — completed 2026-10-20

Part 1 put together:
- **[ADR-0018](../adr/0018-corpus-standards-and-testing.md), Proposed:** the corpus (CO1–CO14), the standards baseline (SB1–SB12) and the testing strategy (TS1–TS18), decided together because each uses the others: X1 and X2.
- **[ADR-0019](../adr/0019-performance-methodology-and-budgets.md), Proposed:** the performance budgets (PF1–PF24), X3, with **the final B1–B7 as an amendment to the process model.** B1 is 60 MB for the core plus tiles at 6.2 bytes per window pixel. B3 keeps its 50 MB median and gains a ceiling for each corpus page. The others are unchanged. The process model itself changes only when the owner accepts it.
- **The plan's grouping settled:** three documents under one ADR, and the budgets under another. The corpus belongs with the standards and testing, since it sets their priorities and their corpus checks.
- **Updated:**
  - the four documents' status (Proposed, with their ADRs);
  - the decision map (X1–X3 proposed; Step 6 at Gate B);
  - OQ-13 (the pause budget proposed);
  - the docs index;
  - status, with Gate B's decisions.

Every relative link resolves; the four standard checks pass.

**Gate B.** The owner reviews the part 1 ADRs and the final budgets. The decisions, with recommendations, are in [status](../status.md), "Next steps".

### Gate B — passed 2026-10-20

The owner's decisions, quoted in full in §2:
- **[ADR-0018](../adr/0018-corpus-standards-and-testing.md) accepted** as proposed: the corpus, the standards baseline and the testing strategy are binding. **The automation boundary confirmed;** **the not-planned list approved** as written.
- **[ADR-0019](../adr/0019-performance-methodology-and-budgets.md) accepted** as proposed, with the B1–B7 amendment. The [process model](../security/process-model.md)'s §7 now holds the final budgets, and its change record the amendment, with the owner's words.
- **S8's session by hand,** run by the owner with secondary accounts: Chrome Canary signed in (eight pages) and Safari logged out (five pages). Folded into the [S8 results](s8-results.md) §7, the [corpus](../corpus.md) §5 and the [performance budgets](../performance.md) §4.1:
  - **The baseline.** The session recorded none, so a fresh Canary was measured on about:blank afterwards (`s8.py blank`, new): 207 MB without its extension renderers. **A first attempt opened the dedicated profile** (two runs on about:blank, with no page and no debugging connection). CO14 says that profile is never opened by automation, so the attempt was stopped and its records aren't used, and the tool now opens only a fresh profile.
  - **Chrome's costs, signed in:** Reddit 1,260 MB, Figma 1,111, YouTube 908, Google Search 582, Instagram 578, Prism 494, Discord 427, Twitch 401.
  - **Every page's ceiling, by the accepted rule:** Google Search, Instagram and Reddit 150 MB; YouTube and Figma 300 MB; Twitch 200; Discord 210; Prism 245. The content cap binds at 12–26% of Chrome's cost. ADR-0019 names that among its conditions for revisiting, so it goes to the owner at Gate C.
  - **Safari's runs need repeating.** The tool asked the owner to go back to the list in the same tab after each page, and Safari kept the earlier pages alive, so later runs counted them. Only the first run measures one page. **The tool is corrected:**
    - Safari quits between pages;
    - each run records its own baseline;
    - by hand, Safari covers every page of the mode.
  - **Not measured:** Chrome logged out by hand; Safari on the other eight pages; the eight pages' features (`chrome://histograms` wasn't saved).
  - **Prism's URL for the secondary account** is kept only in `scratch/corpus-urls.txt`, beside the first.
- **Spike S9 authorized;** it runs before CP8.
- **The cadence:** CP8–CP10 without pausing, to Gate C.
- **Clean-up:** two crash reports that Chrome Canary left at 17:32–17:33, during the owner's `s8.py signin` (non-fatal guard faults; parent `python3`), were removed from `~/Library/Logs/DiagnosticReports`. A third, at 17:10, came from macOS's Software Update settings, not from S8, and was left.

Every relative link resolves; the four standard checks and S8's tests pass.

### S9 — completed 2026-10-20

**Spike S9,** the test tools on this Mac ([results](s9-results.md); `spikes/s9/`, `tools/s9-measure.sh`, committed at `3485d3f`):
- **S9-Q1 passes.** cargo-fuzz 0.13.2, on nightly with AddressSanitizer, found the planted bug in a safe decoder in about a second, twice. It ran the checked decoder 10 minutes with no finding, twice: 80.0 and 59.2 million inputs. Coverage reached 93.25% of regions. The standard library wasn't rebuilt with the sanitizer, despite cargo-fuzz's stated default, and a fuzzer held about 0.5 GB.
- **S9-Q2 fails with the installed nightly** (2026-04-08). Its ThreadSanitizer runtime crashes before `main` on macOS 27.2: dyld calls `dispatch_once` while the runtime starts, and the runtime's interceptor dereferences null. Apple clang 21's ThreadSanitizer reported a planted race in C correctly. Milestone 1 pins a nightly whose runtime works.
- **S9-Q3 passes.** wptrunner ran `dom/nodes`, 435 tests, against Chrome Canary through ChromeDriver, headless, in 140 s, at a 1,152 MB peak summed over its processes. It passed 99.6%, against wpt.fyi's Chrome at 99.5%.
- **The first fuzzing run** found the Mac on battery with Low Power Mode on, so the fuzzing step was repeated on AC power.
- **Clean-up:** ThreadSanitizer's crashes left four reports in `~/Library/Logs/DiagnosticReports`, removed; the script now moves such reports into its results, where the C control's report went too.

**A correction to CP4's calibration.** Computing Milestone 1's target on `css/CSS2`'s reference tests showed that `tools/wpt-pass-rates.py` had left reference and crash tests out: wpt.fyi's summaries give them no subtest counts, only a status. The tool now counts each such test once, by its status, as SB10 defines the measure. [The standards baseline](../standards.md) §6 was recomputed:
- 42 of its 48 rows changed;
- `css/CSS2/`'s median fell from 98.9% to 98.1%;
- Ladybird meets the median on `html/syntax/parsing/` but not on `css/CSS2/` (93.8%).

[ADR-0018](../adr/0018-corpus-standards-and-testing.md)'s decision history records the correction; SB10 and SB11 are unchanged. The tool also gained `--manifest` and `--types`.

Every relative link resolves; the four standard checks pass.

### CP8 — completed 2026-10-20

[The roadmap](../roadmap.md) written (**Proposed**, with ADR-0020 at CP10; rules MS1–MS12). Main decisions:
- **Nine milestones, ordered and gated, never dated** (MS1, MS5):
  - M1 static documents (L0);
  - M2 the sandboxed engine, M3 script, M4 the modern document (L1);
  - M5 site isolation and logins (L2);
  - M6 the sensitive-use gate, with out-of-process iframes and V1–V9 (L3);
  - M7 media, M8 applications, M9 the GPU and WebAssembly, each keeping L3.
- **Security before surface:** the sandbox at M2, site locks at M5, L3 at M6, before media codecs, the GPU process and WebAssembly arrive (MS2). **No script from the web before L1** (MS3). **Every interface from M1** (MS4).
- **Corpus targets** by milestone (MS6): the documents at M1, MDN in full at M3, Wikipedia in full and the content sites logged out at M4, signed in with test accounts at M5, with the owner's accounts at M6; YouTube and Twitch at M7; VS Code, Discord and Prism at M8; Maps, Three.js and Figma at M9.
- **Every milestone's criteria take one shape** (MS7): conformance, corpus checks, budgets, the level's checks, fuzzing, determinism.
- **The login track** (MS8). M5's sites set no limit on a person's accounts (Google, Instagram and Reddit, read on 2026-10-20), so each test account is made by hand in the owner's name, with accurate details, and only signs in and reads. That answers the [corpus](../corpus.md)'s open question 3.
- **Decision points placed** (MS9): WebAssembly at M8's gate; a JIT not before L2, at M8's gate; passkeys before M6; Xcode at M1's plan; QoS overrides at M2; DNS over HTTPS at M5; Step 5's deferred decisions at their milestones.
- **How a milestone runs** (MS10): the owner's authorization, a branch, a plan with gates, checkpoints, dependencies adopted after grading with the owner's approval, the routine from M1.
- **Precedent:** Chromium's and Firefox's site-isolation retrofits, and Ladybird's sandboxes before its alpha, read on 2026-10-20.
- **Found for M1:** WPT's testharness tests are themselves script, so M1 runs reference and crash tests, and html5lib's tree-construction tests from WPT's `.dat` files in its own runner.

Every relative link resolves; the four standard checks pass.

### CP9 — completed 2026-10-20

[Milestone 1](../milestone-1.md) written (**Proposed**, with ADR-0021 at CP10; rules MA1–MA12):
- **Scope** (MA1, MA2): the static-document engine at L0, with CSS 2.1's visual formatting model without paged media. One process with every interface of the later levels. No script, cookies, storage, web fonts, SVG, flexbox, grid or separate processes.
- **The baseline pinned at its start** (MA3); candidates WPT `0f26972422` and html5lib-tests `c777c408`.
- **Conformance without script** (MA4), because WPT's testharness tests are script:
  - html5lib's tokenizer tests, 100%;
  - the 1,973 tree-construction tests in WPT's `.dat` files, at least 96%;
  - `css/CSS2`'s 5,848 reference tests in scope (all but `tables/` and the inline-SVG files), at least 97%: the median of Chrome 98.2%, Firefox 97.8% and Safari 96.4% on those tests, rounded down (SB11);
  - Unicode's line-breaking and bidi files, 100%.

  Neither new engine meets the CSS target yet: Servo 93.7%, Ladybird 91.2%.
- **The corpus** (MA5): Wikipedia and MDN from dated copies and live; text laid out in order without overlaps, measured; the owner's checklist against Safari; links navigate; 10 loads with no crash or hang.
- **Budgets** (MA6): the idle process at most 95 MB, being B1's core, the window's tiles and B2 in one process; Wikipedia at most 100 MB and MDN at most 70 MB above it; S6's frame budgets; no task over 50 ms; no idle wakeups; pauses within 4 ms; T1–T4.
- **Security** (MA7): no script engine linked; no credentials, by construction, with password forms refused below L2; every parser and decoder fuzzed at least an hour; `unsafe` only where allowed; dependencies registered and graded. **Determinism** (MA8).
- **Starting it** (MA10): the owner authorizes the branch, the skeleton under `crates/`, the dependencies listed, the pinned data and copies, the tools, a nightly whose ThreadSanitizer works, and the live loads.
- **Computed for it:** a WPT manifest of the sparse checkout gave each test's type; the corrected `tools/wpt-pass-rates.py` gave the medians over reference tests.

Every relative link resolves; the four standard checks pass.

## Sources

- [D1] Discord, *Terms of Service*, effective 29 September 2025 (last updated 29 August 2025), "Restrictions on your use of Discord's services": "scraping our services without our written consent, including by using any robot, spider, crawler, scraper", and "using any unauthorized software designed to modify the services". Checked 2026-10-20 — https://discord.com/terms
- The other sites' terms, their robots files, RFC 9309 and the precedent for repeatable page loads are cited in the [corpus](../corpus.md)'s sources (CP2).
