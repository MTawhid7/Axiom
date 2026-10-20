# Status and Handoff Log

> **Read this first when resuming work.** Then check the documents it links to against the actual state of the repository (`git log`, `git status`, and the checkpoint tracker). Don't assume earlier work is still accurate.

## Current state (2026-10-20)

| Item | State |
|---|---|
| Phase | **Phase Zero: Step 6, standards, testing, budgets and the roadmap** (authorized 2026-10-20), **at Gate A:** CP1 ✅, the plan committed on branch `step-6`. Step 5 is done and merged (2026-10-20): ADRs 0010–0017 are accepted. Production engine code is **not** authorized |
| Repository | git, hosted at [github.com/MTawhid7/Axiom](https://github.com/MTawhid7/Axiom) (public; remote `origin`). Local folder: `~/Documents/Axiom`. **`main` includes Step 5** (merged with `--no-ff` on 2026-10-20) and is pushed. **Step 6 runs on branch `step-6`**, from `main` at `b4948ce`, local until the owner authorizes a push |
| Current checkpoint | **Step 6: CP1 ✅; Gate A,** the owner reviews the plan ([tracker](research/step-6-standards-budgets-roadmap.md)). Step 5: CP1–CP19 ✅ ([tracker](research/step-5-subsystems.md)) |
| Code | Spike code only, under `spikes/`: S1–S7, all complete. **S8, the corpus census,** is designed and awaits authorization. Tooling under `tools/`, now with `s6-tails.py` and `png-region-stats.py`; working files in `scratch/`, which git ignores. The repository is licensed "MIT OR Apache-2.0" (`LICENSE-MIT`, `LICENSE-APACHE`) |
| Verified capabilities | None at engine level. Spike results: [S1](research/s1-results.md), [S2](research/s2-results.md), [S3](research/s3-results.md), [S4](research/s4-results.md), [S5](research/s5-results.md), [S6](research/s6-results.md), [S7](research/s7-results.md). Observations of this Mac: [macOS note](research/macos-process-confinement.md) §9 |
| Accepted decisions | [ADR-0001](adr/0001-scope-and-target.md): scope, target, JavaScript strategy, security posture |
| | [ADR-0002](adr/0002-implementation-language.md): Rust (stable, 2024 edition), **final**, with build thresholds T1–T4 |
| | [ADR-0003](adr/0003-gc-handle-and-rooting-pattern.md): GC handle and rooting pattern, Pattern B (Nova style), with obligations C1–C5 |
| | [ADR-0004](adr/0004-process-model-and-sandbox.md): process model, sandbox and JIT policy (2026-10-17), validated by spike S4. The owner confirmed S4's refinements |
| | [Threat model](security/threat-model.md) (2026-10-17), amended: out-of-process iframes are required for L3 |
| | [Process model](security/process-model.md), binding with ADR-0004 (2026-10-17); refined at Step 5's Gate C (2026-10-20) in its change record: S5's and S7's metadata rules, ImageIO's place, and B1's scope (the core browser at 60 MB, tiles budgeted by their pixels) |
| | [ADR-0005](adr/0005-ipc-transport-and-interfaces.md): IPC, Mach transport, capabilities, IDL, `Untrusted<T>` (2026-10-17, Gate B) |
| | [ADR-0006](adr/0006-event-loop-and-threading.md): event loop and threading, with EL11 amended for read-only lending of DOM arenas (2026-10-17, Gate B) |
| | [ADR-0007](adr/0007-frame-tree-pipeline-and-input.md): frame tree, rendering pipeline, invalidation and input across frames (2026-10-17, Gate B); IV1 and IV2 amended for size container queries (2026-10-20, Step 5's Gate C) |
| | [ADR-0008](adr/0008-script-agents-and-heap.md): script, agents and heap. C1 is met by `SliceStack`, a safe root stack, so the `gc` crate has no `unsafe` (2026-10-18, Gate C); SH10 amended: the DOM's node arena is never compacted (2026-10-20, Step 5's Gate C) |
| | [ADR-0009](adr/0009-embedder-api-and-shell.md): engine/embedder API and the pure-Rust shell (2026-10-17, Gate B) |
| | [ADR-0010](adr/0010-dependency-and-supply-chain-policy.md): the dependency and supply-chain policy, with DP10's refinement (2026-10-18, Step 5's Gate B) |
| | [ADR-0011](adr/0011-text-fonts-and-unicode.md): text, fonts and Unicode; CoreText rasterizes system fonts (2026-10-20, Step 5's Gate C); `vello_cpu` adopted as the renderer's Rust rasterizer, subject to grading |
| | [ADR-0012](adr/0012-graphics-images-and-media.md): graphics, images and media; AVIF in the media utility process (2026-10-20, Step 5's Gate C). Colour tags confirmed by S6's second run; GR5's amendment deferred to the graphics milestone |
| | [ADR-0013](adr/0013-networking-storage-and-workers.md): networking, storage and workers (2026-10-19), the owner's Gate B condition met by S7; verification in the worker pool confirmed at Gate C |
| | [ADR-0014](adr/0014-accessibility.md): accessibility, with the settings list and the reserved key equivalents (2026-10-18, Step 5's Gate B) |
| | [ADR-0015](adr/0015-html-parsing-and-the-dom.md): HTML parsing and the DOM (2026-10-20, Step 5's Gate C); XSLT deferred (OQ-22) |
| | [ADR-0016](adr/0016-style-layout-and-paint.md): style, layout and paint, with `:visited` by partitioned history (2026-10-20, Step 5's Gate C) |
| | [ADR-0017](adr/0017-javascript-engine-and-bindings.md): the JavaScript engine and bindings (2026-10-20, Step 5's Gate C) |
| The corpus | 13 sites, set by the owner on 2026-10-20 ([OQ-2](open-questions.md); the [Step 6 tracker](research/step-6-standards-budgets-roadmap.md) §2.4), several of which need a login |

## Next steps: Gate A

**Work stops for the owner's Gate A decisions** on the [Step 6 plan](research/step-6-standards-budgets-roadmap.md), each with a recommendation:
1. **The plan:** its scope (§2.3), the readiness criteria RC1–RC10 (§5), the 11 checkpoints in two parts (§6), the three gates (§7), and the cadence: CP2–CP7 without pausing, then CP8–CP10. Recommended: approve.
2. **Spike S8, the corpus census** (§10.1), with read-only page loads of the 13 corpus URLs and everything they load. **Logged out,** automated in Chrome Canary with a temporary profile; **logged in,** in a dedicated profile you sign into by hand, where the spike automates nothing inside the page and only samples the browser's processes. Recommended: authorize. Its feature census and baselines feed CP4–CP6, and the logged-in mode is the only way to measure the full experience you asked for without code handling your credentials. Safari is measured by hand unless you enable its remote automation, which is optional.
3. **Which accounts.** Recommended: for S8, your own accounts in the dedicated profile, since the measurement is passive in a shipping browser. For Axiom, your own accounts wait for L3, as the threat model has it, while test accounts that hold nothing of value and are linked to nothing of yours count as non-sensitive, usable from L2. That reading of the threat model's §8 is yours to confirm.
4. **The two identifying URL parameters,** Prism's `u=` and Figma's `t=`. Recommended: keep the full URLs only in `scratch/`, which git ignores, for the census, and record them in the repository without those parameters. That means amending CP1's commit, which is still local, before `step-6` is first pushed.
5. **Push `step-6`** to `origin` and keep it pushed as checkpoints land. Recommended: yes, after item 4.

Informational: the roadmap's login track (CP8) plans what logging in needs in Axiom, from forms and cookies to sign-in pop-ups and passkeys (OQ-16).

## Session log

### 2026-10-20 — Session 13 (Gate C follow-ups; Step 6 kickoff)

**The owner's decisions after Gate C** (2026-10-20):
1. "Display P3 Color Pattern (S6-Q5): I've captured a screenshot of the three squares at the end of the run. Please analyze the image critically.." The second formal run: `target/s6-results/20261020-074751`.
2. "Real-World Site Corpus (OQ-2): Use the following 13 sites as Axiom's standard evaluation corpus for performance budgets, WPT suites, and memory baselines in Step 6", with the 13 URLs ([Step 6 tracker](research/step-6-standards-budgets-roadmap.md) §2.4).
3. "Memory Target B1 (OQ-23): Approved. In Step 6, budget visible window tile memory separately based on surface pixel dimensions (e.g. 25.2 MB for this Retina window), and maintain a 60 MB budget for the core browser process state."
4. "GPU Path Rasterization (OQ-24): Approved. Keep path rasterization in the renderer for now to preserve analytic edge quality. Defer the GR5 display list amendment to the graphics milestone."
5. "Renderer Rasterizer: Approved. Adopt vello_cpu as the primary Rust rasterizer for web fonts and paths, subject to crate dependency grading at the text milestone."
6. "Step 6 Authorization: Authorized. Proceed with creating branch step-6 from main and drafting the Phase Zero Step 6 plan (standards baseline, testing strategy, performance budgets, and Milestone 1 roadmap) for Gate A review."

During the session the owner added: "You can store important scratcpad codes and files in this project folder so that you can use them later even if the system wipes out the scratchpad." And: "Many of the websites that I have mentioned, requires log in. We need to create a mechanism to log in to get the full experience."

**S6's follow-up** ([S6 results](research/s6-results.md)):
- **The second formal run** reproduced the first: S6-Q1 and S6-Q3 pass at the median again. The tails were again in the session's first runs (225 and 127 of 600 typical updates over 2 ms in runs 1 and 2, 1 in run 3); one 10 s idle window recorded a package idle wakeup; S6-Q4's pixels were identical, and S6-Q5's footprints within 0.1 MB.
- **The Display P3 check,** measured in the owner's screenshot with the new `tools/png-region-stats.py`: the P3-tagged red is (255, 0, 0) in Display P3, and the sRGB-tagged red (234, 51, 35), exactly sRGB's red converted, computed independently (a plain 2.2 gamma would give (233, 54, 40)). The untagged surface is (255, 0, 0), unconverted. **S6-Q5's colour condition is met,** and every surface is tagged (GR6).
- **The display runs scaled,** 1,440 × 900 points in a 2,880 × 1,800 framebuffer on the 2,560 × 1,600 panel, so S6's window was not full screen; the results are corrected.

**Recorded:** ADRs 0011, 0012 and 0016; graphics GR2, GR5, GR6 and GR12; text TX12 and TX15; layout and paint; the process model's B1 and change record; the register; the open questions (OQ-2 and OQ-23 resolved, OQ-24 deferred); the decision map; the overview §2, §3, §5 and §8; the charter's §8.

**Tooling and working files,** as the owner allowed: `tools/s6-tails.py` and `tools/png-region-stats.py`, with `--help`; `scratch/` in `.gitignore`; CLAUDE.md's environment and layout sections. The screenshot is kept with its run, in `target/`.

**Step 6, CP1:** branch `step-6` from `main` at `b4948ce`, and [its plan](research/step-6-standards-budgets-roadmap.md): 7 questions; the 30 questions earlier steps handed to Step 6; readiness criteria RC1–RC10; 11 checkpoints; gates A–C; spike S8, the corpus census, designed for Gate A, with S9 only if needed.
- **Logging in,** as the owner asked: S8 gains a logged-in mode, in a dedicated profile the owner signs into by hand, where nothing is automated inside the page; the roadmap gains a login track (CP8).
- **Discord's terms** were read: they forbid scraping "by using any robot, spider, crawler, scraper" and "unauthorized software designed to modify the services", the reason the logged-in mode is passive.

**Not pushed:** `step-6` stays local until Gate A.

**Clean-up:** nothing to clean. No code ran but the analysis scripts and the five checks; no crash reports.

### 2026-10-20 — Session 12 (Gate C; spike S6 reported; Step 5 merged)

**Gate C, the owner's decisions** (2026-10-20), each as recommended:
1. "ADR-0015 (HTML Parsing and the DOM): Accepted as proposed."
2. "ADR-0008 Amendment (HD11): Accepted. Node slots in the arena never move; node identity remains stable."
3. "ADR-0016 (Style, Layout, and Paint): Accepted as proposed."
4. "ADR-0007 Amendment (ST25, ST31): Accepted. Nested lifecycle phases for size container queries are adopted."
5. ":visited Policy (ST10): Accepted. Partitioned history by top-level site is approved."
6. "ADR-0017 (JavaScript Engine and Bindings): Accepted as proposed."
7. "XSLT (OQ-22): Confirmed. Defer planning XSLT until a concrete use case arises."
8. "ADR-0011 (Text, Fonts, and Unicode): Accepted. I have visually inspected the S5 sheets and verified the Hebrew niqqud and Apple Color Emoji differences. CoreText is approved for system font rasterization under TX12."
9. "ADR-0012 (Graphics, Images, and Media): Accepted. The formal S6 benchmark run is complete (target/s6-results/20261019-201443/summary.txt) and easily meets all frame time thresholds. IM3's fallback (AVIF in the media utility process) is approved."
10. "NW6 TLS Verification: Confirmed. Certificate evaluation must run on worker pool threads to keep the network I/O thread non-blocking."
11. "Process Model: Approved. Record the S5 and S7 sandbox metadata rules in the process model change record."
12. "CP19 (Merge to main): Authorized. Please record S6's formal results, update the documents and indexes, and merge step-5 into main with --no-ff, followed by pushing main and step-5 to origin."

**CP11, spike S6, reported** ([results](research/s6-results.md)), from the owner's formal run on 2026-10-19 (`45bff95`, a clean tree, AC power with Low Power Mode off, the window visible in every run):
- **S6-Q1 and S6-Q3 pass at the median.** A typical update took 0.17–0.28 ms on the GPU and 0.67–1.84 ms on the compositor thread; moving layers took 0.19–0.40 ms a frame. S6-Q2: damage-limited tiles cost a spinner 47–49× less GPU time and 7–9× less energy impact than full redraws.
- **Checked against the criteria, not only the summary.** The thresholds hold at the median in every run, as the owner's words say, and the tails are recorded. Run 1 began on a busy Mac (load averages 103.80, 59.25 and 24.79, with no build to explain them): 192 of its 600 typical updates exceeded 2 ms on the compositor thread, against 0 and 2 in runs 2 and 3. In each run, 2–7 of 1,085 scrolling frames that only moved layers exceeded 1 ms.
- **S6-Q5's memory condition is not met as measured:** one window's compositor state, trimmed, leaves about 4 MB of B1 (OQ-23). The owner's words don't describe the Display P3 comparison, which S6-Q5's colour condition rests on. ADR-0012's acceptance is the owner's explicit decision, and is recorded with these results beside it.
- **S6-Q4:** GPU path filling pays in time (3.9–4.8× sooner) and memory (31–62× less), not in quality, so GR5's amendment goes to the owner (OQ-24). `vello_cpu` filled paths 2.1–6.8× faster than `zeno`.

**Recorded** (the CP19 documentation commit):
- the ADRs' statuses and decision histories: 0007, 0008, 0011, 0012, 0013 and 0015–0017;
- the designs, now binding, with IV1, IV2 and SH10 amended, and TX12, IM3, NW6, GR2, GR5, GR6, GR8 and GR12 updated;
- the process model's new change record: S5's and S7's metadata rules, and ImageIO in the media utility process;
- the register (CP3's to CP5's entries decided); the open questions (OQ-22 resolved; OQ-23 and OQ-24 raised); the decision map; the overview (§2, §3, §5, §8); S5's results' status; the tracker; the indexes.

**CP19:** `step-5` merged into `main` with `--no-ff`, and `main` and `step-5` pushed to `origin`.

**Clean-up:** nothing to clean. This session ran only analyses of the run's files and the five checks, and the formal run left no crash reports.

### 2026-10-19 — Session 11 (part 2: CP13–CP18; Gate C reached)

**Part 2 of Step 5, run without pausing as planned** (the owner's Gate B instruction: "Proceed with executing Spikes S5, S6, and S7 as planned"; the plan runs CP13–CP18 until Gate C):
- **CP13, [HTML and the DOM](architecture/html-and-dom.md)** (HD1–HD33, `4b81d95`): our own HTML and XML parsers; decoding straight into the DOM's 8- and 16-bit strings; a node arena of `u32` columns whose slots never move, which needs an amendment to ADR-0008 (HD11); one mutation path with fixed hooks.
- **CP14, [style](architecture/style.md)** (ST1–ST36, `3581d47`): our own CSS parser over 8- and 16-bit text; shared, grouped computed styles with a rule tree, a sharing cache and evictable caches; invalidation sets; container queries in nested lifecycles during layout, which needs an amendment to ADR-0007 (ST25, ST31); `:visited` by partitioned history.
- **CP15, [layout and paint](architecture/layout-and-paint.md)** (LY1–LY32, `8a5ede7`): a 1/64-pixel fixed-point layout unit; LayoutNG's contract; tables marked approximate; scroll nodes with snap areas, sticky constraints and compositor-moved scrollbar thumbs; paint into PL7's primitives.
- **CP16, [the JavaScript engine](architecture/javascript-engine.md)** (JE1–JE32, `48bc182`): heap-free compiled scripts; a register machine dispatched by a `match` loop; NaN-boxed values carrying the scope lifetime; `RegExp` interpreted with a linear-time fallback; `Intl` on ICU4X; Temporal of our own; evidence for OQ-3.
- **CP17, [WebIDL bindings](architecture/bindings.md)** (WB1–WB23, `04527fc`): our own generator over webref's curated IDL, emitting safe Rust; platform objects as heap entries, not wrappers; rooting only where a later conversion may collect; remote window proxies.
- **CP18:** [the overview, v1](architecture/overview.md); [ADR-0015](adr/0015-html-parsing-and-the-dom.md), [ADR-0016](adr/0016-style-layout-and-paint.md) and [ADR-0017](adr/0017-javascript-engine-and-bindings.md), Proposed; the decision map (P5–P7), the open questions (OQ-3's evidence; OQ-22, XSLT), the register (part 2 adds no runtime dependency) and the indexes.
- **Precedent** was read from primary sources at their latest commits on 2026-10-19. Notable current facts: Ladybird moved style (2026-08-13) and layout (2026-07-28) to Rust, put style and layout and paint on their own threads (2026-10-03), switched its DOM to wrappers built as host objects (2026-10-05), and gained an optimizing JIT; SVG 2 published a new Candidate Recommendation Snapshot (2026-10-06); Temporal is finished for ECMAScript 2027, and V8 builds it on `temporal_rs`.
- **Gate C reached:** the decisions are listed in the tracker's Gate C entry, each with a recommendation. No code was written, nothing was installed, and no crash reports or other side effects outside the repository were produced.

### 2026-10-19 — Session 10 (spike S7; ADR-0013 accepted)

**CP12, spike S7** ([results](research/s7-results.md); tracker §11), under the owner's Gate B authorization: "Spike S7 (Network Process & TLS): Authorized for questions S7-Q1 through S7-Q4 under spikes/s7/, including read-only outbound network requests to public test hosts."
- **Code:** `spikes/s7/net` (`s7-net`): a broker that compiles the network profile, and a sandboxed helper with modes for each question; its `unsafe` in `src/platform/` (Security, Network framework, DNS-SD, kqueue, Mach, sockets, libproc) under invariants N1–N9; 8 offline tests; `tools/s7-measure.sh`. Commits `4840d7d` and `13daa44`.
- **Network access:** read-only GETs to public test hosts and DNS queries through the system's resolver. No certificate was added to a keychain, and nothing on the Mac was reconfigured.
- **Formal run:** `4840d7d`, from a clean tree, on AC power with Low Power Mode off (`target/s7-results/20261019-071313/`); three runs, the Mac in normal use. Supplementary `startup` runs at `13daa44`.
- **S7-Q1: passes.** In the broker-compiled profile, rustls verified through `SecTrustEvaluateWithError` gave the expected verdict for every host it reached, the same as unsandboxed and as Network framework. The profile has eight rules; two metadata rules were found by failures the kernel didn't log (`/var`, for DNS-SD; the executable's directory, without which `SecPolicyCreateSSL` returns NULL). Every denial is explained, using reporting runs that log every attempted operation.
- **S7-Q2: reported; NW6 settled as designed.** rustls retired 2.4–3.3× fewer instructions per handshake than Network framework, 1.6× fewer per byte. rustls needs its `prefer-post-quantum` default feature (without it, Cloudflare answered every full handshake with a HelloRetryRequest). A process's first TLS use costs 29–43 ms of CPU, for AWS-LC's jitter entropy source.
- **S7-Q3: passes.** DNS-SD resolved nine queries on one shared connection, from kqueue, with one thread.
- **S7-Q4: passes.** One I/O thread drove rustls, `quinn-proto` (an HTTP/3 GET), DNS-SD, Mach messages received in the wait, a timer and a user event from one `kevent64` per turn, and woke only for real deadlines when idle.
- **ADR-0013 accepted** (2026-10-19), under the owner's Gate B words: "Pre-approved; will move to Accepted upon S7-Q1, S7-Q3, and S7-Q4 passing." The networking and storage designs are now binding; the register's CP6 and CP7 entries are decided.
- **For Gate C:** verification in the worker pool, off the I/O thread (NW6, because `SecTrustEvaluateWithError` blocks its caller and EL1 allows one place to block); noting the two metadata rules in the process model's network column; AWS-LC's jitter entropy, at the network milestone.
- **Clean-up:** no crash reports. One empty helper data directory in `target/release/`, left by an informal run cut short by `head`, was removed.

### 2026-10-18 — Session 9 (Gate B; spikes S5–S7)

**Gate B, the owner's decisions** (2026-10-18), each as recommended:
1. "ADR-0010 (Dependency and Supply-Chain Policy): Accepted as proposed, including the DP10 refinement."
2. "ADR-0011 (Text, Fonts, and Unicode): Pre-approved; will move to Accepted upon S5-Q1 and S5-Q3 passing, with TX12 settled by S5-Q2."
3. "ADR-0012 (Graphics, Images, and Media): Pre-approved; will move to Accepted upon S6-Q1, S6-Q3, and S6-Q5 passing, and S5-Q5 validating ImageIO inside the renderer sandbox."
4. "ADR-0013 (Networking, Storage, and Workers): Pre-approved; will move to Accepted upon S7-Q1, S7-Q3, and S7-Q4 passing."
5. "ADR-0014 (Accessibility): Accepted as proposed."
6. "Spike S5 (Text Stack & ImageIO Sandbox): Authorized for questions S5-Q1 through S5-Q5 under spikes/s5/, including the listed candidate crates and OFL/WPT test web fonts."
7. "Spike S6 (Metal Compositing): Authorized for questions S6-Q1 through S6-Q5 under spikes/s6/. The performance thresholds are confirmed: typical update ≤ 4 ms GPU / ≤ 2 ms compositor thread; scrolling/transforms ≤ 1 ms."
8. "Spike S7 (Network Process & TLS): Authorized for questions S7-Q1 through S7-Q4 under spikes/s7/, including read-only outbound network requests to public test hosts."
9. "Settings List: Approved as written in docs/architecture/accessibility.md §6.1."
10. "Reserved Key Equivalents: Approved as written in docs/architecture/accessibility.md §6.2 (Chromium reserved set, plus ⌘L, ⌘,, and Esc un-overridable even in fullscreen)."
11. "OQ-21 (Axiom License): Approved as 'MIT OR Apache-2.0'. Please add the standard LICENSE-MIT and LICENSE-APACHE files to the repository root."
12. "Proceed with executing Spikes S5, S6, and S7 as planned."

**Recorded** in ADRs 0010–0014's decision histories, the six design documents (the policy and the accessibility design are now binding), the register, the decision map, the open questions (OQ-21 resolved), the indexes, the tracker (§2, §6, §7, §10, §11) and this status.

**The license:**
- `LICENSE-APACHE` is the Apache Software Foundation's text, unchanged: fetched from apache.org on 2026-10-18, SHA-256 `cfc7749b…bc523d30`.
- `LICENSE-MIT` is the standard MIT text, word for word SPDX's, with "Copyright (c) 2026 Md. Tawhidul Islam".
- The workspace manifest declares `license = "MIT OR Apache-2.0"`, which every crate inherits. The root README states the dual license and the usual contribution terms.

**CP10, spike S5** ([results](research/s5-results.md); tracker §11):
- **Code:** three crates under `spikes/s5/` (`s5-rust-text`, with no `unsafe` and no Apple framework in its web-font executable; `s5-text`; `s5-image`), 40 tests, and `tools/s5-fetch.sh`, `tools/s5-images.sh`, `tools/s5-measure.sh`. Commits `f20e7f4` and `bad07a8`.
- **Formal runs:** `bad07a8` (primary, `target/s5-results/20261018-204642/`) and `f20e7f4` (reproduction), on AC power with Low Power Mode off, from clean trees; every deterministic output identical between them.
- **S5-Q1:** HarfRust chose CoreText's glyphs in all 432 runs; the Hebrew niqqud differ in position, in a font with no mark-positioning data. Awaits the owner's visual check.
- **S5-Q2:** the Rust rasterizers match CoreText above 20 px and at 2×; CoreText recommended for system fonts (TX12), pending the owner's comparison.
- **S5-Q3: passes.** 322 WPT fonts, 4,000 mutants, no crash, hang or sandbox denial.
- **S5-Q4:** B2 achievable; HarfRust shapes in 566 ns a glyph.
- **S5-Q5: missed its condition.** AVIF needs VideoToolbox's decoder service, which renderers may not reach; IM3's fallback (the media utility process) recommended for Gate C. JPEG keeps the safe decoder.
- **Findings recorded:** the PingFang stub and TX6's file-backed check; ImageIO's NULL-bundle crash (R1); CoreText's normal track against HarfRust's `trak` handling (TX10); `flate2` in `png`'s tree; ImageIO's non-deterministic JPEG pixels.
- **The owner's visual check:** published as a private page in this session, and in the results directory.
- **Clean-up** (outside the repository): seven crash reports left by informal S5 runs were deleted from `~/Library/Logs/DiagnosticReports/Retired`, after copies were made in the session's scratchpad. The formal runs' crash reports (six per run) were moved into their results directories by the script. Nothing else was left outside the repository.

### 2026-10-18 — Session 8 (Step 5 kickoff)

**The owner's decisions:**
1. "Step 4 is formally accepted."
2. "Step 5 Authorization: Authorized."
3. "Proceed with planning Phase Zero Step 5 (Subsystems) on a new branch `step-5`."
4. "Submit the Step 5 plan and checkpoint tracker first for Gate A review, defining the subsystem sequence, review gates, and identifying which areas may require targeted validation spikes."

**Done:**
- Branch `step-5` created from `main` at `c642ab3`. It is not pushed.
- **CP1:** the [Step 5 plan and tracker](research/step-5-subsystems.md):
  - 14 questions; the 21 questions earlier steps handed to Step 5, each assigned to a checkpoint;
  - the subsystem criteria SC1–SC10;
  - 19 checkpoints: the dependency policy and six platform-facing subsystems (text, graphics, images, networking, storage, accessibility); then the spikes; then the document and script engine (HTML and the DOM, style, layout and paint, the JavaScript engine, bindings), the overview and the merge;
  - three gates: A, the plan; B, part 1's ADRs and the spikes; C, part 2's ADRs and the merge;
  - three candidate spikes: S5 (text stack), S6 (compositing on Metal), S7 (the network process and TLS).
- The plan proposes a new decision-map item, X4 (dependency and supply-chain policy), for the owner to approve. Until then the map is unchanged except for Step 5's row.

**Verification:** the four standard checks and the link check pass, with the code unchanged.

**Gate A, the owner's decisions** (2026-10-18), each as recommended:
1. "Plan Approval: Approved as written." "The 19-checkpoint sequence, two-part structure, and subsystem criteria SC1–SC10 are approved." "Step 4 interfaces remain binding under SC1."
2. "Decision Map Addition: Approved. Add X4 (Dependency and Supply-Chain Policy) to docs/decision-map.md, to be decided at CP2 in ADR-0010."
3. "Candidate Spikes: Confirmed." S5, S6 and S7 "are approved as candidate spikes, with code authorization to be decided individually at Gate B."
4. "Cadence: Approved. Proceed through CP2–CP9 without pausing until Gate B."
5. "Push Branch: Authorized." `step-5` pushed to `origin`.

**Recorded** in the tracker (§2, §7, §11), the decision map (X4 added) and this status.

**CP2 (completed 2026-10-18):** the [dependency policy](technology/dependency-policy.md) (**Proposed**; DP1–DP23) and a living [register](technology/dependency-register.md):
- The core of charter G1 is Axiom's own. The infrastructure layer may be reused behind interfaces Axiom owns. Axiom never writes its own cryptography.
- Dependencies get soundness grades (Google's `ub-risk` scale) and groups after Chromium's (`trusted`, `sandboxed`, `build`, `test`). A reused parser of untrusted input must be pure Rust at `ub-risk-1` or better, or it counts as memory-unsafe code under the Rule of 2.
- The supply chain: `cargo-vet` audits with Firefox's imports, `cargo-deny`, a license allow-list, minimal features, T1–T4.
- **Found:** the repository has no license file, so Axiom's license is unchosen. It is raised for Gate B.

**CP3 (completed 2026-10-18):** [text, fonts and Unicode](architecture/text-and-fonts.md) (**Proposed**; TX1–TX21), and spike S5's design ([tracker](research/step-5-subsystems.md) §10.1):
- ICU4X is the one Unicode source. Encodings are Axiom's own, because Google grades `encoding_rs` at `ub-risk-3`.
- CoreText's database and fallback serve system fonts. HarfRust shapes every font. Web fonts take a Rust-only path, so no C parser sees them. Glyphs are rasterized in the renderer, with CoreText or a Rust rasterizer for system fonts, as S5 decides.
- The dependency policy's DP10 is refined: `ub-risk-2` across the tree in sandboxed helpers, `ub-risk-1` in the browser. Applying CP2's stricter rule would have excluded every Rust font stack while admitting CoreText.

**CP4 (completed 2026-10-18):** [graphics](architecture/graphics.md) (**Proposed**; GR1–GR14), and spike S6's design ([tracker](research/step-5-subsystems.md) §10.2):
- Metal directly, through `platform`, with no portability layer in the browser.
- The window server composites retained CoreAnimation layers, and Axiom draws damaged tiles into IOSurfaces, as Firefox does on macOS, where it measured large power savings.
- Our own pipelines draw the display items. Paths are rasterized in the renderer until S6 reports; if GPU filling pays, an amendment adding path items to PL7 goes to the owner.
- Display P3 at 8 bits by default. Compositor animations: the transform family and opacity, then `filter`, `backdrop-filter` and scroll-driven timelines.

**CP5 (completed 2026-10-18):** [images and media](architecture/images-and-media.md) (**Proposed**; IM1–IM13):
- Images decode in the renderer: safe Rust for PNG, JPEG, GIF, WebP and JPEG XL, and our own BMP and ICO decoders. `zune-jpeg` is graded `ub-risk-3`, so JPEG uses a safe decoder.
- AVIF goes to ImageIO in the renderer, under DP10's exception, accepted by ADR-0012; S5 gains a question (S5-Q5) to test ImageIO in the sandbox.
- Pixels decode at display size and are discarded when not visible. `moxcms` manages color.
- Media runs in a sandboxed utility process, with VideoToolbox, IOSurface frames and audio output.

**CP6 (completed 2026-10-18):** [networking](architecture/networking.md) (**Proposed**; NW1–NW18), and spike S7's design ([tracker](research/step-5-subsystems.md) §10.3):
- Protocols are state machines driven from one wait on the I/O thread: no tokio.
- One URL parser of our own in every process; in the browser, only canonical ASCII URLs from other processes, which settles CP3's IDNA question.
- HTTP/1.1, HTTP/2 and HTTP/3 framing are our own (`httparse` is graded `ub-risk-4`). QUIC is `quinn-proto`. TLS is rustls with `aws-lc-rs`, under DP10's exception, and Security verifies certificates.
- DNS through DNS-SD. Caches, connections, sessions and cookies are partitioned.
- Fetch's policy is enforced from bound endpoints: CORS, ORB, CORP and COEP, mixed content, HSTS, and Local Network Access.

**CP7 (completed 2026-10-18):** [storage and workers](architecture/storage-and-workers.md) (**Proposed**; SW1–SW13):
- All web storage lives in the network process, keyed by origin and top-level site.
- One engine: `redb` if its audit agrees, with SQLite under DP10's exception as the fallback; crash tests decide. The cache and the cookie jar share the engine.
- `localStorage` is cached in the renderer. IndexedDB values arrive serialized, so the network process never parses script values.
- Service workers are intercepted inside Fetch. Shared and service workers run in a renderer locked to their site, launched when none exists.

**CP8 (completed 2026-10-18):** [accessibility](architecture/accessibility.md) (**Proposed**; AX1–AX12):
- Renderers push trees built under HTML-AAM and Core-AAM, once an assistive technology first asks. The browser answers from its cache.
- Text markers, public in SDK 27, carry text navigation, with boundaries computed in the renderer. The bridge is our own.
- Two questions for the owner at Gate B: the settings list, and reserved key equivalents.

**CP9 (completed 2026-10-18):** ADRs [0010](adr/0010-dependency-and-supply-chain-policy.md)–[0014](adr/0014-accessibility.md) Proposed:
- the dependency policy;
- text, fonts and Unicode, pending S5;
- graphics, images and media, pending S6 and S5-Q5;
- networking, storage and workers, pending S7;
- accessibility.

The decision map has I1–I5 Proposed. The open questions gain OQ-21 (Axiom's license), and OQ-7 notes that the graphics design needs no Xcode. **Gate B** is listed under "Next steps".

### 2026-10-17 to 2026-10-18 — Session 7 (Gate B; spike S3; Gate C; S3 addendum; merge)

**Gate B, the owner's decisions** (each as recommended):
1. ADR-0005 (IPC), ADR-0006 (event loop and threading), ADR-0007 (frame tree, pipeline, input) and ADR-0009 (embedder API and pure-Rust shell): **accepted**. OQ-11 and OQ-17 are resolved.
2. ADR-0008 (script, agents, heap): "Pre-approved; will move to Accepted upon S3-Q1 and S3-Q2 passing."
3. **Spike S3** is "Authorized for the full scope (questions S3-Q1 through S3-Q5 under spikes/s3/)". S3-Q2's threshold is "Confirmed at ≤10% of runtime spent rooting".
4. **OQ-19:** "Amend criterion AC5 and rule EL11 to allow read-only scoped lending of DOM arenas to helper threads during borrow-checker-frozen rendering stages. Keep the actual parallel style/layout implementation deferred until profiling benchmarks justify it."
5. The S4 test fix from Session 6 is approved.

**Recorded** in the ADRs' decision histories, the design documents (now binding where their ADR is accepted), AC5 in the tracker, EL11, PL10, the decision map, the open questions, the indexes and this status.

**CP13 (spike S3a, completed 2026-10-18)** ([tracker](research/step-4-architecture-proposal.md) §11):
- **Crates:** `s3-gc` (S1's heap with NaN-boxed values, three root stacks and a barrier hook; its only `unsafe` is 3 blocks in `src/raw/`), `s3-vm` (a register-machine interpreter with callback builtins, four dispatch techniques and a reference evaluator) and `s3-bench` (probes, S1's DOM ported to `s3-gc`, node layouts). Scripts `tools/s3-bench.sh`, `s3-profile.sh` and `s3-ceremony.sh`; `samply-summary.py` gains `--share`.
- **Correctness:** every workload and 34 edge-case programs match the reference evaluator on every technique, stack and pacing. Counted builds show bytecode performs zero root operations. A deliberately unrooted value fails the oracle. Miri finds no undefined behaviour under Stacked and Tree Borrows, on `s3-gc` and on the VM driving C1's stack with a collection at every allocation.
- **Refinements:** a third, safe root stack (`CellStack`), to test whether C1 needs `unsafe` at all; the stack borrowed instead of `Rc`-shared; QoS left at the command-line default, as S1's was, because raising it needs FFI.
- **Preliminary, not formal:** frames and reads meet C1's 1 ns target, but under S1's method a push costs about 2.0 ns on every stack. The method's barrier forces each push to reload the stack length the previous one stored (calibrated: 2.18 ns for any escaped counter; **corrected at CP16:** the cost depends on how the code around the push compiles). A supplementary back-to-back push probe was added. CP14 measures formally.

**CP14 (spike S3b, completed 2026-10-18):** formal runs from the clean tree at `05185cb` ([tracker](research/step-4-architecture-proposal.md) §11).
- **A first run was discarded:** on battery, Low Power Mode slowed every timing about 1.64 times. After the owner plugged in the charger, the full set ran on AC power. The repeat covered S3's suites with counts and nightly tail calls, S1's dispatch and traverse suites for Pattern A, the profiles, and the ceremony count.
- **A tool fix:** `samply-summary.py --share` skipped inlined frames without a source path (`11be3a7`).
- **Headlines:**
  - C1 reads in 0.51 ns, but pushes in 2.03 ns by S1's method (0.56 ns back to back).
  - Rooting is 0.9–1.1% of W3's time with C1's stack.
  - `match` is the fastest dispatch on stable Rust; nightly `become` is 5–22% faster than it.
  - Struct-of-arrays link columns traverse faster than both of S1's patterns.
  - Setters are free without a barrier; card marking adds about 3%.

**CP15 (spike S3c, completed 2026-10-18):** the [S3 results](research/s3-results.md), ADR-0008 updated (still Proposed), script-and-heap (SH7–SH9, SH13), open questions (OQ-14 answered), decision map and indexes. Also: `RawStack`'s lazy commitment verified (an 8 MiB stack with 1,000 roots adds 32 KiB to the footprint), and an unverified comment removed. **Gate C** is listed under "Next steps".

**Before Gate C, the owner asked** whether `RawStack`'s `unsafe` was worth its 94 ns on S1's dispatch case, and shared a second opinion that recommended `CellStack`.
- **The answer was no.** The architecture admits `gc`'s raw module "only if C1 needs it", and `CellStack` meets C1 exactly where `RawStack` does; recommending `RawStack` had given one benchmark too much weight.
- **A probe outside the repository** then found a safe contiguous stack (`SliceStack`) that compiles to `RawStack`'s push. With its length first, it halved S1's push and made dispatch 19% faster (informal).
- **It also showed that CP15's explanation of the 2 ns push was wrong.**

**Gate C, the owner's decisions** (2026-10-18):
1. "S3 Addendum Checkpoint: Authorized."
2. "C1 Implementation: Adopt `SliceStack` (Safe Contiguous Stack)." "The `gc` crate remains strictly `#![forbid(unsafe_code)]`." "Keep `RawStack` archived in spikes/s3/ as benchmark history."
3. "We will formally accept ADR-0008 and merge `step-4` into `main` (with a subsequent push to origin) immediately upon completion of the formal addendum measurements."

**CP16 (the S3 addendum, completed 2026-10-18)** ([tracker](research/step-4-architecture-proposal.md) §11):
- **Code** (`17775c4`):
  - `SliceStack` and its field-order control in `s3-gc`, with no `unsafe`; `RawStack` stays, archived.
  - Every stack in the heap, root-stack and oracle tests: 197 tests pass, and Miri passes on `s3-gc` under both borrow models.
  - The new stacks in the benchmarks and profiles, and a new `s3-probe footprint` command.
- **Formal runs** at `17775c4`, on AC power. The Mac was in use, so absolute times were noisier than CP14's. The comparisons rest on ratios within each run, which reproduce CP14's.
- **Results:**
  - `SliceStack` reads in 0.53 ns and has no `unsafe`.
  - By S1's method a push costs 2.12 ns, as on every stack: **the informal 1.02 ns did not reproduce.** The push's own code is the same in both binaries; the harness's code differs. So S1's method measures the loads and stores around a push as much as the push.
  - **S3-Q1 still fails as written.** The corrected explanation is in the [S3 results](research/s3-results.md) §3.2.
  - In S1's dispatch case `SliceStack` is the fastest stack: in each run, 13–17% faster than `RawStack` and 14–20% faster than its control.
  - Rooting takes 0.8–1.0% of W3, and the counts are unchanged.
- **Documents:**
  - The S3 results, ADR-0008, and ADR-0003's decision history.
  - Script and heap, now binding; the overview, where `gc` has no `unsafe`.
  - The decision map, and the open questions: OQ-5, OQ-14 and OQ-13's design part resolved, and OQ-5's row repaired.
  - The indexes and the spikes README.
- **Disclosed:** S1's suite and the profiles ran with uncommitted Markdown changes in the tree; the code was `17775c4`.

**Gate C passed** (2026-10-18): the measurements were complete, so ADR-0008 is Accepted. **CP17:** `step-4` merged into `main` (`--no-ff`), and `main` pushed, as the owner instructed.

### 2026-10-17 — Session 6 (Step 4 kickoff)

**The owner's decisions:**
1. Push `main`: authorized.
2. ADR-0004's three S4 refinements are confirmed: browser-compiled profile bytecode, the required explicit `(deny dynamic-code-generation)`, and `single-jit` on `renderer-jit`.
3. **Step 4 is authorized** as design work on a new branch `step-4`: "Submit the checkpoint plan and tracker first, and request separate go-ahead for Spike S3's code when its design is ready."

**Done:**
- `main` pushed to `origin` at `da79e69`.
- Branch `step-4` created from `main`.
- The confirmation recorded in ADR-0004's decision history.
- **CP1:** the [Step 4 plan and tracker](research/step-4-architecture-proposal.md): 16 questions, 14 deliverables, the architecture criteria AC1–AC10, 16 checkpoints and three review gates. It proposes adding "HTML parsing and DOM" and "JavaScript engine and bindings" to Step 5, whose list doesn't name them.

**Verified** (SDK 27 headers): `CVDisplayLink` is deprecated since macOS 15, in favour of AppKit's `displayLink(target:selector:)` on `NSView`, `NSWindow` and `NSScreen`. This bears on frame timing in renderers, which have no AppKit (CP2).

**Verification:** every relative link in `docs/` resolves. The four standard checks pass, with the code unchanged.

**Gate A (owner, 2026-10-17):**
- The plan is approved as written: scope, criteria AC1–AC10, the 16 checkpoints.
- Step 5's list in the decision map gains "HTML parsing and DOM" and "JavaScript engine and bindings" (done).
- Cadence: CP2–CP12 run without pausing; Gate B, after CP12, covers ADRs 0005–0009 and S3's code together.
- `step-4` push authorized; pushed to `origin`.

**CLAUDE.md and a link checker** (the owner's request, then "proceed with your recommendation"):
- `CLAUDE.md` at the repository root holds the working instructions: session start, authorization and review gates, communication (every decision presented with a recommendation and rationale), documentation conventions, verification, code rules, git workflow, environment. It stays in the public repository.
- `tools/check-doc-links.py` (standard-library Python) checks every relative Markdown link and anchor in the repository. It is now the fifth check before every commit.

**CP12:** ADRs [0005](adr/0005-ipc-transport-and-interfaces.md)–[0009](adr/0009-embedder-api-and-shell.md) Proposed. Decision map (F4, F5, P1–P4), open questions (OQ-5, 11, 13, 14 and 17 updated; OQ-19 and OQ-20 new) and indexes updated. **Gate B** is listed under "Next steps".

**CP11:** [architecture overview v0](architecture/overview.md) (Proposed): the system on one page, the crate structure and its `unsafe` map, five end-to-end scenarios, the criteria check (AC1–AC10 met), the complete-model table, and staging in which L3 changes only process selection.

**CP10:** [script, agents and heap](architecture/script-and-heap.md) (Proposed): one heap per agent; copies, transfers and shared buffers between agents; script only in tasks and microtasks; the root set; C1–C5 resolved by design, with C1 left to spike S3, **whose design is in the [tracker](research/step-4-architecture-proposal.md) §10** for Gate B.

**CP9:** [embedder API and shell](architecture/embedder-api.md) (Proposed): the shell as an embedder over a message API; commands, events and requests for decisions; hosted presentation, IME and accessibility objects; native browser UI only; a headless embedder for WPT; OQ-11 proposed as pure Rust with objc2, validated first at the shell milestone.

**CP8:** [input and accessibility](architecture/input-and-accessibility.md) (Proposed): input enters at the compositor and is routed by hit testing; gestures latch; scrolling chains across frames in the compositor; focus and capture are browser records; IME from a main-thread mirror (Firefox's model); activation from routed input; accessibility as a browser-side cache.

**CP7:** [invalidation](architecture/invalidation.md) (Proposed): a compiler-checked lifecycle per document; two entry points, with no synchronous flush across agents; reuse at every stage; compositor-driven scrolling and animation with display ports; a test oracle comparing incremental updates with rebuilds from scratch.

**CP6:** [rendering pipeline](architecture/rendering-pipeline.md) (Proposed): five owned stages; immutable fragments; per-frame display lists of validated primitives over property trees; glyphs rasterized in the renderer; compositing by token; one 8-bit or 16-bit string representation; style and layout on the event loop's thread within AC5, with lendable DOM arenas. Amending AC5 for parallel stages is a Gate B question.

**CP5:** [frame tree, navigation and loading](architecture/frame-tree-and-navigation.md) (Proposed): the core thread's authoritative frame tree; skeleton replicas and remote window proxies in renderers; every operation between agents routed through the browser, even within one process, so out-of-process iframes change only process selection; browser-driven navigation and session history; loader and storage endpoints bound at commit; V9 tests specified.

**CP4:** [IPC](architecture/ipc.md) (Proposed): Mach messages as the only transport between processes (OQ-17); endpoints as browser-issued capabilities; an IDL with a safe-Rust generator; `Untrusted<T>` validation after WebKit; synchronous requests only toward more privilege; bounded queues, with a browser that never blocks.

**Housekeeping:** an intermittent test failure (about one run in 75) was traced to an unsound assertion in S4's `this_process_has_a_plausible_footprint`: the kernel's lifetime-maximum footprint can trail the current footprint by a page. The assertion was fixed; afterwards there were no failures in 150 runs.

**CP3:** the [event loop and threading model](architecture/event-loop-and-threading.md) (Proposed): one wait per thread; a browser **core** thread separate from AppKit's main thread; one thread and heap per event loop in renderers; input-first task selection; frames offered by the browser's compositor only to event loops that asked; throttling from Chrome's tested levels, applied per agent cluster.

**CP2:** the [macOS runtime note](research/macos-runtime-facilities.md): run loops, frame timing, QoS, surfaces, IME and accessibility on macOS 27, with browser precedent. The browser must own the frame clock, since `CADisplayLink` comes only from AppKit objects; Mach messages can carry QoS (an input to OQ-17).

### 2026-10-17 — Session 5 (Step 3)

**Owner input:** "Continue implementation". Read with status.md, this was taken as authorization for Step 3's design work. Spikes need their own authorization, so no code was written.

**Done** (branch `step-3`, CP1–CP5; [tracker](research/step-3-security-and-process-model.md)):
- **CP1:** the plan and checkpoint tracker.
- **CP2:** the [threat model](security/threat-model.md): assumptions, assets, adversaries, attack surfaces, trust boundaries, goals SG1–SG12, non-goals, security levels L0–L3 and the gate's checks V1–V8.
- **CP3:** the [macOS confinement note](research/macos-process-confinement.md), with mechanisms, browser precedent, observations of this Mac, and the design of spike S4.
- **CP4:** the [process model](security/process-model.md): five process types, a capability matrix, site isolation SI1–SI7, IPC trust rules IR1–IR10, memory targets B1–B7, staging.
- **CP5:** [ADR-0004](adr/0004-process-model-and-sandbox.md) (Proposed), including the JIT policy J1–J7 for OQ-4. Decision map, open questions (OQ-16 to OQ-18 added), indexes and this status updated.

**Verified on this Mac** (read-only; nothing compiled or installed):
- The macOS 27 SDK **kills processes that use the named sandbox profiles** (`sandbox_init(3)`).
- Chrome Canary 158, built against SDK 27.0, still applies custom Seatbelt profiles through `sandbox_init_with_parameters` and `sandbox_apply` (`nm -u`). Firefox uses `sandbox_init_with_parameters`.
- WebKit ships deny-by-default profiles for its WebContent, GPU and Networking processes. Its content profile denies the window server and font server, and its two no-JIT content variants deny `dynamic-code-generation`.
- App Sandbox's base profile allows 170 global Mach services.
- Multiple WebKit XPC instances rely on the private `_MultipleInstances` key.
- The SDK contains BrowserEngineKit with macOS 14.3 availability, but Apple documents it for iOS, iPadOS and Mac Catalyst only, with entitlements by request.
- A minimal process has a 1.03 MB footprint.

**Corrected during review:** WebKit's XPC services carry no hardened-runtime flag. They are platform binaries with library validation only.

**Verification:** the four standard checks pass, unchanged from Step 2b. Every relative link in `docs/` resolves.

**The owner's review** (2026-10-17), recorded in ADR-0004's decision history:
- **Guiding principle:** no calendar deadline. No architectural corners are cut and no weaker security boundaries are accepted for convenience; staged features rest on an architecture designed for the complete model from day one. Added to the charter as principle 7.
- **Accepted:**
  - the threat model;
  - levels L0–L3 and checks V1–V8, with **out-of-process iframes mandatory for L3** (new check V9);
  - the JIT policy J1–J7, with global and per-site switches;
  - SI1–SI7 as staged, with SI3 only at L1 and non-sensitive L2;
  - memory targets B1–B7 as initial baselines.
- **ADR-0004 stays Proposed** until S4-Q1 and S4-Q2 pass, then moves to Accepted.
- **Spike S4 authorized** (S4a, S4b, S4c).
- **Git:** `step-3` pushed now; it merges into `main` with `--no-ff` after S4.

**Spike S4** (CP6–CP8; [S4 results](research/s4-results.md)):
- **S4-Q1 and S4-Q2 pass.** A Rust helper built against SDK 27.0 and signed ad hoc with the hardened runtime enters a custom deny-by-default profile by three methods: `sandbox_init_with_parameters`; compile + apply; bytecode compiled by the broker. All 18 probes behave as specified, and the control run allows all 18. A *named* profile kills the helper (SIGKILL, namespace `SANDBOX`).
- **S4-Q3.** With broker-compiled bytecode, a sandboxed helper starts as fast and as small as an unsandboxed one: 2.1 ms, 1.14 MiB. Compiling in the helper costs about 6 ms and 2.8 MiB. Mach round trips are 24% faster than a socket pair (3.75 against 4.92 µs). 1 MiB transfers are comparable once socket buffers are enlarged.
- **S4-Q4.** `(deny default)` does not stop JIT; only an explicit `(deny dynamic-code-generation)` does. The one-region `MAP_JIT` limit applies only with `single-jit`. Ad-hoc signing honours `allow-jit` and `single-jit`.
- **S4-Q5.** CoreText renders pixel-identically with the window server and font server denied, given read access to system fonts and ICU's data. Without either, it degrades silently.
- **Code:** `spikes/s4/sandbox`, `spikes/s4/text`, `tools/s4-sign.sh`, `tools/s4-measure.sh`. The four standard checks pass (115 tests, 5 ignored).

**ADR-0004 Accepted** (2026-10-17), per the owner's instruction, with S4's refinements recorded:
- D2: the browser compiles profiles;
- J3: the explicit JIT deny is required;
- J5: `renderer-jit` carries `single-jit`.

**Merged** `step-3` into `main` with `--no-ff` (CP9).

**Housekeeping.** The named-profile runs left crash reports in `~/Library/Logs/DiagnosticReports`. One is archived with the formal results, and all were removed from that folder. The check is opt-in from now on.

### 2026-10-17 — Session 4 (renamed to Axiom)

**Owner input:** the project is renamed from Arbor to **Axiom**, including its folder on disk. The owner created the GitHub repository <https://github.com/MTawhid7/Axiom>.

**Done**
- Every reference in the repository now uses the new name: the docs, the workspace manifest, the S2 window's title, drawn text and tests, and the build-measurement scripts, whose log variables are now `AXIOM_RUSTC_LOG` and `AXIOM_LINK_LOG`.
- Earlier commits are unchanged, so their files still use the old name.
- The project folder moved from `~/Documents/Arbor` to `~/Documents/Axiom`.
- The GitHub repository is the remote `origin`. `main` and `step-2b` are pushed to it.

**Verified:** the four standard checks pass, and the S2 smoke test passes with the new text.

### 2026-10-17 — Session 3 (CP7)

**Owner input:** "continue" with CP7, after the CP3–CP6 progress report.

**CP7a completed** (benchmark harness):
- `s1-bench`, with reference-validated inputs, six verified suites, and the `s1-probe` binary.
- Measurement scripts in `tools/`: `s1-bench.sh`, `s1-memory.sh`, `s1-profile.sh`, `samply-summary.py`, `s1-ceremony.sh`.
- `samply` 0.13.1 installed, as planned. Building it was itself measured: 281 crates in 83 s, with a 2.0 GiB aggregate compiler peak.
- The shakedown run found five methodology problems, all fixed before any reported number:
  - an invalid traversal case;
  - unrealistic allocation order;
  - clock ramp-up;
  - samply's line attribution;
  - profile scoping.
- Profiles also showed incidental allocations in both patterns' dispatch. A symmetric fix went into both DOM crates; the oracle tests and Miri (`s1-a-dom`) were re-run green; before and after numbers are in the tracker's §8.

**CP7b and CP7c completed** (measurements and results):
- Formal runs, at commit `d5d8272` with a clean tree:
  - runtime suites, 3 runs;
  - memory and instructions, 3 runs;
  - profiles;
  - builds (dev and release).
- Rooting unit costs measured with the new `s1-probe roots`.
- [S1 results](research/s1-results.md) written. Its recommendation for ADR-0003 is Pattern B (Nova style), conditional on C1–C5, with medium confidence.
- New open questions OQ-13 to OQ-15.
- Design facts checked against current sources: gc-arena (intrusive list, incremental, barriers), piccolo (stackless because of gc-arena) and Nova (`stack_refs` is a `RefCell<Vec>`).

**CP8a completed** (S2 window):
- `s2-window` draws CoreText through a Metal pipeline whose shaders are compiled from source at run time, in an AppKit window, with no Xcode.
- 19 `unsafe` blocks, all in `src/platform/`, under P1–P6.
- The smoke oracle matches the predicted ink pixel for pixel.
- Six tests run under `cargo test` without a window.
- The Command Line Tools have no offline Metal compiler.

**CP8b and CP8c completed** (S2 measurements and results):
- Startup: first frame at 174 ms; idle: 0.0% CPU and no wakeups.
- Build: 6.0 s clean with trimmed bindings, against 29 s with objc2's defaults; the whole workspace builds clean in 8.1 s.
- [S2 results](research/s2-results.md) written, with the proposed build thresholds T1–T4 for ADR-0002 revisit condition 3.

**CP9 synthesis written:**
- ADR-0002 recommends final acceptance, with a Step 2b evidence table.
- ADR-0003 is proposed: Pattern B, with obligations C1–C5.
- The language evaluation has dated Step 2b notes.
- The decision map, open questions and indexes are updated.

**The owner's decisions (2026-10-17)** close Step 2b:
1. S1-Q2: Pattern B's ceremony is acceptable.
2. ADR-0002 is **accepted as final**.
3. ADR-0003 is **accepted as proposed**: Pattern B, with obligations C1–C5.
4. Thresholds T1–T4 are accepted (OQ-12 resolved).
5. S3 is a **required** benchmark, run in Step 4 (OQ-15 resolved).
6. The S2 smoke test and the owner's visual check are satisfactory.

**Recorded and merged.** The decisions are recorded in ADR-0002, ADR-0003, the open questions, the decision map, the indexes and the tracker. `step-2b` was merged into `main` with a merge commit (`--no-ff`), which keeps Step 2b's history together.

**Verified**
- `samply` works on this machine without Xcode, including line-level attribution through `dsymutil` and `atos` (Command Line Tools).
- The system allocator is Apple's "xzone" malloc. Some of its paths call `mach_absolute_time`, so allocation in hot loops is relatively costly here.


### 2026-10-16 — Session 2 (Step 2b kickoff)

**Owner inputs**
- Step 2b (S1 and S2) authorized.
- Rust provisionally accepted.
- Owner is fluent in Rust (OQ-6).
- No deadline or fixed budget (OQ-1).
- git requested (OQ-8).
- Efficiency (memory, CPU, power) is a primary goal (charter G4).
- CLI-only measurement tools.

**What changed**
- `git init`; baseline docs committed on `main` (`a223fe2`).
- Branch `step-2b` created.
- Plan and checkpoint tracker written: [research/step-2b-validation-spikes.md](research/step-2b-validation-spikes.md).
- ADR-0002 marked provisionally accepted; evaluation §15.3 gained rule 7 (macro-generated `unsafe`).
- Charter, open questions and decision map updated.

**CP1 completed** (foundation):
- workspace with resolver 3 and the 2024 edition;
- toolchain pinned to 1.94.1;
- workspace lints (pedantic, plus unsafe hygiene);
- `tools/check-unsafe-policy.sh`, with a self-test;
- `tools/measure-build.sh`, with the `rustc` and linker RSS wrappers;
- the `s1-scenarios` model types, RNG, digests and counters, with 17 tests.

Verification results and the baseline build numbers are in the tracker's §8.

**Owner decision (2026-10-16):** keep exactly the two canonical patterns.
- Pattern A is gc-arena: raw pointers and mark–sweep.
- Pattern B is Nova: `u32` indices and mark–compact.
- There is no third variant. The storage confound is handled in the analysis (§4.4).

**CP2 completed** (workload model and oracle):
- the `reference` interpreter;
- four seeded workload generators at three sizes, each producing its expected outcome;
- 10 golden tests;
- a gate of 100 seeds per workload;
- a coverage gate.

Results, workload volumes and the pressure bound for CP4 and CP6 are in the tracker's §8.

**CP3 completed** (Pattern A heap):
- `s1-a-gc`: a canonical gc-arena design. All `unsafe` is under `src/raw/`, governed by invariants I1–I6.
- `s1-a-gc-derive`.
- The first external dependencies: `syn`, `quote` and `proc-macro2`, justified in `Cargo.toml`.
- Verification:
  - 12 tests plus compile-time assertions;
  - 6 compile-fail doctests, each with a verified reason and a pinned error code;
  - Miri clean under both Stacked Borrows and Tree Borrows with strict provenance, with leak checking on.
- Clean build in 2.6 s, with a peak rustc RSS of 252 MiB (`syn`).
- One-time setup: the Miri component was added to the nightly toolchain.

**CP4 completed** (Pattern A DOM):
- `s1-a-dom` is `forbid(unsafe_code)`. Dispatch is a resumable state machine that lives in the arena root.
- It matches the reference exactly on:
  - the 10 edge cases;
  - 100 Small seeds × 4 workloads;
  - Medium seeds;

  both at default pacing and when collecting at every boundary.
- Pressure peak: 2,211 live nodes, against a bound of 2,314 and about 131k without mid-dispatch collection.
- Miri is clean (Stacked Borrows and Tree Borrows).
- `s1_scenarios::edge_cases` was added.
- **S1-Q1 for Pattern A: yes.** The only `unsafe` in the expanded code comes from the two audited macro templates (rule 7).

**CP5 completed** (Pattern B heap):
- `s1-b-gc`: a canonical Nova design with **zero `unsafe`** (`forbid`), including its `heap!` macro.
- It provides `u32` handles bound to `GcScope`/`NoGcScope`, RAII root frames, typed arenas, and a mark–compact collector with rank-based forwarding.
- Verification:
  - 9 tests, including the reachability oracle under compaction;
  - debug detection of stale handles and of missed marks, both proven by tests;
  - 2 compile-fail doctests (E0502, E0597);
  - Miri clean.
- Clean build in 1.1 s.

**CP6 completed** (Pattern B DOM):
- `s1-b-dom` has zero `unsafe` across its whole stack. Dispatch is an ordinary function with rooted frames.
- It matches the reference everywhere, at both pacings.
- A ≡ B on every workload.
- The debug run with epoch checks found no stale handles.
- Pressure peak: 2,940 live nodes, against a tight bound of 2,940 (vs about 131k without mid-dispatch collection).
- Miri is clean.
- **S1-Q1 for Pattern B: yes, unconditionally.**
- A preliminary ceremony comparison and the build cost of each stack are in the tracker's §8.

**Verified**
- Experiment: `#![forbid(unsafe_code)]` doesn't catch `unsafe impl` generated by external derive or `macro_rules!` macros, but does catch hand-written ones (rustc 1.94.1).
- Toolchains present:
  - 1.94.1 with clippy, rustfmt and rust-src;
  - nightly 2026-04-08, without Miri (to be added at CP3).
- `samply` was not installed (installed in Session 3, CP7a).

### 2026-10-16 — Session 1 (Phase Zero, Steps 0–2a)

**What changed**
- Created the `docs/` foundation:
  - README, charter, decision map, open questions, this status log;
  - research conventions;
  - the ADR template, ADR-0001 and ADR-0002;
  - the language evaluation.

**Verified**
- Target machine facts, read from the system:
  - MacBook Air M1 (`MacBookAir10,1`): 4 performance + 4 efficiency cores, 8 GB RAM;
  - macOS 27.2;
  - Rust 1.94.1, Swift 6.4, Apple clang 21;
  - Command Line Tools only.

**Decisions made**
- ADR-0001 Accepted.
- ADR-0002 Proposed. It was provisionally accepted in Session 2.

**Verification gaps carried forward**
- Boa's 2026 test262 numbers come from a secondary source.
- The Miri limitations are stated as background; re-check the Miri README before relying on them.
- The Fuzzilli integration path for a Rust engine is a hypothesis.
