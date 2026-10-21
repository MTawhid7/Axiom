# Review Round 2: Findings

| | |
|---|---|
| **Status** | **Round 2, for the owner's disposition** (2026-10-21). Fifteen findings: no Critical, no High, ten Medium, five Low. They stay as written, as the round's record; the owner's words in the [disposition log](../README.md#5-the-disposition-log) will govern where they differ |
| **Question** | Where are the subsystems Round 1 deferred wrong, inconsistent, outdated or incomplete, and what did Round 1's own changes leave behind? |
| **Covers** | The baseline on `architecture-review` at `ec7c6b3` (2026-10-21): Round 1's amendments ([verification](02-verification.md)), and in depth the [accessibility](../../architecture/accessibility.md) design (ADR-0014), [storage and workers](../../architecture/storage-and-workers.md) (ADR-0013), [style](../../architecture/style.md) (ST1–ST36) and [layout](../../architecture/layout-and-paint.md) (LY1–LY32), the [embedder API](../../architecture/embedder-api.md) (EM1–EM12) and [IPC](../../architecture/ipc.md) (IP13–IP22), as the owner commissioned |
| **Summary** | [Risk matrix](01-risk-matrix.md) · [Verification of Round 1](02-verification.md) · [Actions](04-actions.md) |
| **Last updated** | 2026-10-21 |

**How to read it.** As in [Round 1](../round-1/02-findings.md), each finding gives:
- its severity, its area, and the milestone it blocks (whose plan shouldn't start until the owner has decided it);
- the claim it challenges, quoted from the baseline with its rule ID;
- the evidence, labelled as the [research conventions](../../research/README.md) require: **sourced**, with a numbered source below; **observed**, on this Mac or in the repository, naming the file or command; **measured**, by a spike, named; **inference**, **hypothesis** or **background**;
- the failure mode, a counter-proposal, the cost of making the change later instead of now, and the rules it would amend.

Severities are defined in the [review process](../README.md#3-findings-and-severities). Every external source was read on 2026-10-21. Quotations from sources were checked against files downloaded with `curl`, not only against an automated summary; the one exception is marked **secondary**. SDK headers are those of the Command Line Tools' `MacOSX27.0.sdk`.

---

## RV2-01. The entry documents predate Round 1

**Medium** · Program · Blocks M1 (the owner's decision under MA10, and every session, start from these documents) · Challenges the [handover](../../handover-phase-0.md)'s status; the [decision map](../../decision-map.md)

**The claim.**
- The handover is "the starting point of the independent architecture review", "Updated when a review round changes the baseline".
- The project's instructions ([`CLAUDE.md`](../../../CLAUDE.md)) have every session read the status first, then the decision map.

**Evidence.**
- **Observed: Round 1 changed the handover only for RV1-20,** its three mentions of Encrypted Client Hello. It still describes the baseline the owner's dispositions replaced:

  | The handover | Says | Round 1 made it |
  |---|---|---|
  | §4.14, §6, §9 | M1 is "one process, every interface"; "L0 is one unsandboxed process"; "M2 splits renderers and the network process" | Sandboxed renderer and network processes from M1 (RV1-07) |
  | §4.14, §9 | `css/CSS2`'s "5,848 reference tests in scope at ≥ 97%" | An interim target over the 5,652 script-free tests, the full one at M3 (RV1-01) |
  | §6, §7, §11 item 6 | The browser composites "until a GPU process exists"; the GPU process at M9; "Until the GPU process arrives at M9, it validates every renderer's display lists" | The GPU/compositor process from M2 (RV1-02) |
  | §4.6 | "stop-the-world mark–compact for now, then incremental marking in idle periods and a generational nursery, as corpus measurements demand" | The end-state fixed before M3 (RV1-04) |
  | §7, §9, §10 | The 150 MB cap "Kept as the baseline hypothesis for review" | A slope and a plateau on a normalized workload, 150 MB a stretch target; OQ-25 answered (RV1-05) |
  | §7, §9 | WebAssembly decided at "M8's gate" | Its architecture before M3, with S14 (RV1-13) |
  | §9 | Passkeys: "Roaming security keys over USB HID" | The hybrid transport added (RV1-21) |

- **Observed: the decision map** records Round 1 in I2 (the GPU process) and I6 (WebAssembly) only:
  - F2 (the collector's end-state, RV1-04), F3 (RV1-02, RV1-07, RV1-09, RV1-17's signing identity, and D2's exception after S10), P7 (the slot width left open, RV1-06; Temporal and BigInt reclassified, RV1-08), X1 and X2 (RV1-01, RV1-11), X3 (RV1-05) and X4 (DP1 (g) and DP2, RV1-08) carry no note;
  - its "Review" row still reads "**Next**".
- **Observed: the harm is bounded.** The handover says that where it "seems to differ" from its source, "the source governs", and the binding documents carry the changes ([verification](02-verification.md)). But the documents a reader meets first are the stale ones.

**Failure mode.** A person reviewing independently, a later round or a new session takes the old baseline as current: it reviews a topology that no longer exists, or plans M1 as one process. The owner's decision to start M1 would rest on a summary that contradicts what the owner approved.

**Counter-proposal.**
1. **Update the handover** for Rounds 1 and 2: each row above, marked "as amended in Round 1 (RV1-nn)" with a link to the disposition log, and §11's flagged areas marked where Round 1 answered them.
2. **Add Round 1's notes to the decision map's** F2, F3, P7 and X1–X4, and bring its Review row up to date.
3. **Add a step to the review's process** ([README](../README.md#4-how-a-round-runs) §4): a round's changes include the entry documents, checked by searching them for each finding's old wording.

**Cost now and later.** Now, one checkpoint of text. Later, a review or a milestone plan built on a superseded baseline.

**Amends.** The handover; the decision map; the review's README §4.

---

## RV2-02. Statements Round 1's changes left stale

**Low** · Documents · Blocks M1 (a sweep before the plan) · Challenges the statements below

**Evidence.** Round 1's sixth checkpoint swept for stale statements, and [verification](02-verification.md) found these left. **Observed,** each in the file named:

| Finding | Where | Says | Now |
|---|---|---|---|
| RV1-02 | [Graphics](../../architecture/graphics.md) §3 | "Axiom's compositor lives in the browser until a GPU process exists, which constrains GR5"; "Axiom's compositor runs in the unsandboxed browser for now" | The GPU/compositor process from M2 |
| RV1-02 | Graphics §9, alternative A | wgpu would put a large `unsafe` tree "in the most privileged process" | True when written; the compositor has moved since |
| RV1-02 | The [rendering pipeline](../../architecture/rendering-pipeline.md)'s "Builds on" | "(the browser composites; the Rule of 2)" | As above |
| RV1-02 | [Layout and paint](../../architecture/layout-and-paint.md#7-security) §7 | Paint's primitives, "which the browser validates again before drawing (PL9)" | The GPU process validates them from M2 |
| RV1-02 | [OQ-24](../../open-questions.md)'s evidence | "Path items would bring renderer-supplied geometry into the browser process" | They would reach the sandboxed GPU process, which weakens the objection behind the deferral (inference); it should be weighed again at its milestone |
| RV1-04 | [OQ-13](../../open-questions.md)'s resolution | "Incremental marking in idle periods and a generational nursery come later, in the order corpus measurements demand"; "a corpus page that breaks either triggers SH10's incremental marking" | SH10's end-state, fixed before M3 |
| RV1-05 | The [status](../../status.md)'s ADR-0019 row; [performance](../../performance.md) §9, question 1 | "the content tier's 150 MB cap kept at Gate C as the baseline hypothesis for review"; "the owner kept the cap at Gate C as the baseline hypothesis for review" | OQ-25 answered: a slope and a plateau (§4.1) |
| RV1-07 | The [requirements](../../requirements.md) RQ24, RQ29 | "M1 (one process), M2 (each process)"; "M1 (in-process channels, generated code), M2 (across processes)" | Real processes from M1 |
| RV1-07 | The [roadmap](../../roadmap.md)'s summary | "L0 is one process with every channel, record and replica" | As above |
| RV1-07 | [Script and heap](../../architecture/script-and-heap.md) §4, L0 row | "Agents and heaps are per event-loop thread inside one process" | As above |
| RV1-07 | [EL24](../../architecture/event-loop-and-threading.md); [EM10](../../architecture/embedder-api.md) | "Single-process mode (L0) runs the production threads in one process"; "whether the engine runs one process (L0) or many (L1–L3)" | Single-process mode is a test mode; L0 runs several processes |
| RV1-08 | The [JavaScript engine](../../architecture/javascript-engine.md) §3 | "DP1 (g) makes builtins Axiom's own, so Axiom uses it as a reference instead (JE22)" | `temporal_rs` graded before M3, reusable behind Axiom's interface |
| RV1-09 | [Images and media](../../architecture/images-and-media.md) §1 | "display lists carry decoded pixels in shared memory or IOSurfaces" | Memory entries only |
| RV1-13 | The [overview](../../architecture/overview.md) §5.4 | "I6, WebAssembly, deliberately open and placed at M8's gate (MS9)" | Its architecture before M3 |
| RV1-17 | [EM12](../../architecture/embedder-api.md); [ADR-0009](../../adr/0009-embedder-api-and-shell.md)'s decision 7 | "signed ad hoc with the hardened runtime"; "One ad-hoc-signed bundle" | The stable local identity, ad hoc the fallback (MA10) |
| RV1-21 | The [threat model](../../security/threat-model.md) §10 | OQ-16's "Candidates are roaming security keys over USB HID, or obtaining the entitlement" | Its §9 row adds the hybrid transport |
| RV1-13, RV1-21 | OQ-3; OQ-16 | "use it.."; "(MS9).." | Typing errors |

**Counter-proposal.** Correct each, with "as amended in Round 1" where the old text is the record (ADR-0009's decision 7 by a history row, as ADRs are amended). Extend the sweep's searches to the phrases above.

**Amends.** The documents listed.

---

## RV2-03. Which process configuration runs the conformance suites

**Medium** · Testing · Blocks M1 · Challenges EM5, EM9, EL24, TS2, TS8, MA4, MA8

**The claims.**
- [EM9](../../architecture/embedder-api.md): the headless embedder provides "virtual time through the deterministic scheduler (EL24)", and "runs WPT, reference tests, crash tests and fuzzing harnesses".
- [EL24](../../architecture/event-loop-and-threading.md): "one deterministic scheduler with a **virtual clock** drives every event loop, including those of other 'processes', on one thread in a defined order".
- [TS8](../../testing.md): "every test runs on EL24's virtual clock"; [MA8](../../milestone-1.md): "Every test that involves time uses EL24's clock".
- RV1-07, in the owner's words: "Retain single-process mode strictly for rapid automated testing, fuzzing, and Miri"; MA1 builds "the browser with sandboxed renderer and network processes".
- TS2: wptrunner "launches `axiom-headless` once per screenshot".

**Evidence.**
- **Observed: virtual time exists only in one process.** EL24's scheduler drives every loop "on one thread", which only single-process mode allows. Real renderer and network processes run their own threads on the host clock.
- **Observed: no document says which configuration runs WPT, the corpus copies, or MA4's gate.**
- **Inference: each answer fails one rule.**
  - **In one process,** M1's conformance figures never cross a process boundary, so the shared memory, profiles and crashes that RV1-07 brought forward are exercised only by the helpers' own tests.
  - **In real processes,** EL24's clock can't drive the helpers, so tests that involve time run on host time, against TS8 and MA8.
- **Observed: EM5's defaults are unstated.** "Every request has a default, and the engine never waits indefinitely", but its table gives the engine's policy and the shell's interface, not the default each request takes. EM9's "logged answers to decision requests" need those defaults to be reproducible.

**Failure mode.** M1's gate measures a configuration the owner doesn't run, or the determinism rule breaks silently, and flaky or unrepresentative numbers reach the gate.

**Counter-proposal.**
1. **A configuration rule in TS2 and TS8,** for the owner to choose (inference):
   - **conformance in real processes, under their profiles,** on host time: reference tests wait for the load event natively and rarely depend on time, which MA8's repeated runs check;
   - **time-dependent tests in single-process mode** on the virtual clock: timers and animations, and the collector's pacing from M3;
   - **a cross-check** that runs the reference-test selection in both configurations at each milestone's gate, and explains the differences, as TS2 already does for its two executors at M3.

   The alternative, a virtual clock across processes, with the browser as the time authority and every helper's thread loop asking it, costs a message per wait and more design (inference).
2. **EM5's defaults listed per request:** deny permissions; cancel downloads and file choosers; dismiss dialogs, as WebDriver's default user-prompt handler does (background).

**Cost now and later.** Now, a rule and a list. Later, gate figures measured again, or a flaky suite debugged at the end of M1.

**Amends.** TS2, TS8; MA4, MA8; EM5, EM9; EL24's wording.

---

## RV2-04. When a shared helper dies or hangs

**Medium** · Robustness, architecture · Blocks M1 (the network process), M2 (the GPU process), M3 (`sessionStorage`), M8 (Web Locks) · Challenges IR10, GR13, EM4, PF19, SW3, SW10, SW13

**The claims.**
- [IR10](../../security/process-model.md): "When a child dies, the browser discards everything that child reported and recovers from its own state: a crash page in the tab, and a reload on request."
- [GR13](../../architecture/graphics.md): "If the GPU/compositor process dies (from M2), the browser relaunches it and keeps each window's last layer contents meanwhile; renderers resend their display lists and resources (IR10)."
- [EM4](../../architecture/embedder-api.md)'s events to the shell: "a renderer crash or hang".
- [PF19](../../performance.md): "The browser pings each renderer's control thread once a second."
- [SW10](../../architecture/storage-and-workers.md): "Web Locks are arbitrated in the network process, by storage key, next to the data they guard."
- SW3: `sessionStorage` is "held in the network process's memory, and saved with the tab's session history for session restore (FT9)".

**Evidence.**
- **Observed: IR10's recovery fits renderers only.** A crash page in a tab answers a renderer's death. GR13 designs the GPU process's, but IR10 and EM4 don't reflect it.
- **Observed: nothing designs the network process's death,** although it exists from M1 (RV1-07) and holds the cookie jar, the HTTP cache, every site's web storage, `sessionStorage` in memory, service-worker registrations and Web Locks. SW13 kills it in crash tests, but only to check the stores' consistency, not what its clients do.
- **Observed: no watchdog for helpers.** PF19 pings renderers only. A hung network process stalls every tab's loads; a hung GPU process, RV1-02's own case, freezes every window's content; nothing detects either.
- **Sourced: Chromium restarts its network service.** "If the network service crashes, it gets restarted in a new utility process. The goal is for the failure to be mostly recoverable." Endpoints bound before the crash "become disconnected, and will no longer continue to work", so their users keep reconnection logic [C4].
- **Sourced: Chromium arbitrates Web Locks in the browser process.** `LockManager` lives in `content/browser/locks/`: "One instance of this exists per StoragePartition, and services multiple child processes/origins" [C3].
- **Inference: Web Locks fail silently across a restart.** The API can't tell a holder that its lock was lost. If the arbiter is relaunched empty, every holder still believes it holds its lock, and the new arbiter grants the same locks again. Mutual exclusion between tabs, the API's whole purpose, fails without an error.
- **Inference: `sessionStorage`** dies with the process's memory, unless the browser's copy for session restore is current.

**Failure mode.** At M1, a network process crash leaves every tab with dead loader endpoints and no defined recovery. Later, a crash duplicates locks and loses `sessionStorage`; a GPU or network hang freezes everything with no hang page.

**Counter-proposal.**
1. **IR10 by process type:**
   - **renderer:** as now;
   - **network process:** the browser relaunches it under the same profile and data directory, and rebinds the renderers' loader and storage endpoints (FT11's endpoints, issued again); requests in flight fail as network errors; `sessionStorage` is restored from the browser's copy; SW13's crash tests extend to what the clients see;
   - **GPU process:** GR13, referenced from IR10.
2. **Web Locks arbitrated in the browser,** as Chromium does. The locks guard no data, so "next to the data" buys nothing, and the browser outlives the helpers.
3. **PF19's watchdog for every helper:** the browser pings the network and GPU processes' control threads, and past a deadline ends and relaunches them, as for a death.
4. **EM4's events:** a helper's crash or hang, with what was lost.

**Cost now and later.** Now, a rule per process type and one placement. Later, a lock arbiter moved between processes, and a recovery protocol retrofitted to every endpoint renderers hold.

**Amends.** IR10 and the process model's change record; GR13; EM4; PF19; SW3, SW10, SW13; the histories of ADR-0004 and ADR-0013.

---

## RV2-05. Two protocols the GPU process needs from M2

**Medium** · Performance, graphics · Blocks M2 · Challenges PF4; GR8, GR12; PL8; IPC's open question 4

**The claims.**
- [PF4](../../performance.md): a page costs "the GPU and compositor memory allocated on its behalf, charged to its renderer: atlases, image textures, WebGL and WebGPU allocations, its share of tiles beyond the window's"; "This answers the IPC design's open question 4."
- [GR8](../../architecture/graphics.md): frames that drew newly exposed tiles took "up to 8.6 ms when the surface pool had to grow".

**Evidence.**
- **Measured, S10:** "The tiles are charged to the GPU process, which creates them"; a renderer's memory entries "stay charged to the renderer, which allocated them"; a blit "adds 64.6 MiB to the GPU process"; and macOS offers "no API to move the charge" for memory a renderer allocates ([S10 results](../../research/s10-results.md) §4, §7).
- **Inference: PF4's charge has no mechanism.** From M2 the GPU process allocates the atlases, any blitted image textures and every tile, so a page's share is visible only if the GPU process accounts for it by page and reports it. No document designs that, and without it no budget check can run on a page: neither PF5's misses nor B3's slope and plateau.
- **Observed: IPC's open question 4,** "Do browser-allocated surfaces get charged to the renderer they serve?", still names "CP6" where it is settled, while PF4 says it answers it; S10 answered it the other way for tiles.
- **Observed: reusing tiles across processes is undesigned.** From M2 the GPU process draws tiles that the browser's CoreAnimation tree displays. It may draw into a spare only once CoreAnimation and the window server no longer read it. S10 never attached a window, so CoreAnimation never held a surface the GPU process wanted back (S10 §9, threat 2). The [runtime note](../../research/macos-runtime-facilities.md)'s `IOSurfaceIsInUse` is a use count across processes; port rights the browser keeps after `IOSurfaceLookupFromMachPort` could hold it up (inference).

**Failure mode.** From M2, budgets can't be checked per page. And the GPU process either draws into a surface still on screen, or never reuses one and allocates on the slow path S6 measured.

**Counter-proposal.**
1. **Accounting by page in the GPU process:** each allocation tagged with the renderer and frame it serves; a report on the browser's request, through the control interface; PF4's figure computed as the renderer's footprint plus that share. `IOSurfaceSetOwnershipIdentity`, the runtime note's hypothesis for moving the charge itself, tested at M2 as the alternative.
2. **A protocol for reusing surfaces:** the browser drops each port right once it has looked the surface up; the GPU process checks `IOSurfaceIsInUse` before drawing into a spare, and keeps a bounded pool allocated ahead (GR8). Tested at M2 with a window attached.
3. **IPC's open question 4 closed** with S10's answer.

**Cost now and later.** Now, two protocols on paper. Later, budgets that can't be enforced, or a visual defect found on the owner's screen.

**Amends.** PF4; GR8, GR12; PL8; IPC §8, question 4; the roadmap's M2.

---

## RV2-06. Accessibility geometry after the GPU process

**Medium** · Accessibility · Blocks M4 (the accessibility tree) · Challenges AX2, AX4; PL15 and PL16 as amended in Round 1

**The claims.**
- [AX2](../../architecture/accessibility.md): each node carries "its geometry in frame coordinates, with the property-tree node it sits in, so that the browser can place it on screen after scrolling or zooming without asking"; for text, "runs with offsets and their word, sentence and line boundaries".
- AX4: "The browser answers every `NSAccessibility` query from its cache, on the main thread, without waiting on a renderer".
- [PL15](../../architecture/rendering-pipeline.md), as amended in Round 1: "the GPU process holds the full trees; each layer commit carries the browser the scroll and animation nodes its layers need".

**Evidence.**
- **Observed: a regression from RV1-02.** Before Round 1 the browser held the full property trees, so it could place any node. From M2 it holds only the nodes its layers need, so a node under a transform, clip or effect without a layer of its own can't be placed "without asking".
- **Sourced: macOS's hit test is synchronous, and only the renderer is accurate.** Chromium, 2016: "Native accessibility APIs all have some sort of 'hit test' function. Unfortunately on Mac, Windows, and Linux, they're all synchronous APIs. This is problematic for Chrome because only the render process can do an accurate hit test." Its answer: "every time we get a hit test request we send an asynchronous message to do an accurate hit test. While waiting, we return an approximate hit test result instead" [C9]. AX4 has no hit-test policy.
- **Sourced: text ranges need positions within runs.** Chromium exposed inline text boxes in 2013 so that "the embedder can compute the bounding rectangle for an arbitrary text range, and determine the word that any given character offset belongs to" [C10]. AX2's runs carry offsets and boundaries, not positions within a run, so the bounds of a text-marker range can't be computed from the cache. (Background: VoiceOver draws its cursor around such ranges.)

**Failure mode.** VoiceOver's cursor lands in the wrong place under transforms, and text ranges have no bounds; or the browser waits on a renderer, against AX4 and IR5.

**Counter-proposal.**
1. **Geometry relative to the nearest node the browser holds:** the renderer maps each node's bounds into the space of the nearest scroll or animation node that has a layer, as PL16 tags hit-test regions with their scroll node, so the browser applies only offsets and transforms it has.
2. **Advances per grapheme cluster in text runs,** sent only while an assistive technology is active (AX7) and bounded by AX2's limits.
3. **A hit-test policy in AX4:** answer from cached bounds at once, ask the renderer asynchronously for an accurate answer, and use it for the next query in that area, as Chromium does.

**Cost now and later.** Now, three paragraphs. Later, a change to AX2's format in the IDL and to the renderer's serializer after M4's tree exists.

**Amends.** AX2, AX4; PL16's note; ADR-0014's history.

---

## RV2-07. Accessibility enabled by any client

**Medium** · Accessibility, performance · Blocks M4 · Challenges AX7, AX12

**The claims.**
- [AX7](../../architecture/accessibility.md): "Enabled on first use, as WebKit enables its cache only after an assistive technology's first query"; "The cost is paid only by users of assistive technologies."
- AX12: "With no assistive technology, there is no tree and no cache (AX7)."

**Evidence.**
- **Sourced: many clients of the accessibility API aren't assistive technologies.** Chrome: "Approximately 5-10% of users have accessibility code enabled, often due to tools like password managers and antivirus software that use platform accessibility APIs." An experiment, "Auto Disable Accessibility (turning off accessibility when it is not used)", showed the cost to be "quite high" [C8].
- **Sourced: Chromium refined its detection on macOS in 2024.** Sonoma's changed signals caused "Chrome to activate screen reader support when it's not necessary (which consumes memory)". The change "Uses KVO to watch specifically for VoiceOver state changes", and when "we receive an accessibilityRole request, and VoiceOver is not active", turns on only a basic mode, "ui::kAXModeBasic (kNativeAPIs + kWebContents)" [C7].
- **Observed: AppKit offers the signal.** `NSWorkspace`'s `voiceOverEnabled` and `switchControlEnabled` are each "observable through KVO" (`NSAccessibility.h`).
- **Inference:** a password manager or a window-management utility on the owner's Mac would make every visible document build and push a full tree (AX7's "The first query tells every renderer with a visible document to build its tree"): the cost AX12 says doesn't exist.

**Counter-proposal.**
1. **Enablement in tiers:** full trees, text markers and notifications while VoiceOver or Switch Control runs (observed by KVO); for any other client, a basic tree for the document it queries, built on demand.
2. **Disabled again** after a period with no queries, the trees discarded.
3. **AX12 corrected,** and each tier's memory measured at the shell milestone, where AX7 already measures the cache.

**Cost now and later.** Now, AX7's wording and an observer. Later, memory and energy spent for every user of such a tool, found after the tree's format and the cache exist.

**Amends.** AX7, AX12; ADR-0014's history.

---

## RV2-08. Accessibility tests that can't run as written

**Medium** · Accessibility, testing · Blocks M4 · Challenges AX11, TS2, EM9

**The claim.** [AX11](../../architecture/accessibility.md): "WPT's `accname`, `html-aam` and `core-aam` directories, through the headless embedder, which exposes each document's computed roles and names (EM9)".

**Evidence.**
- **Observed: the role and name tests call WebDriver commands TS2 doesn't list.** At WPT `0f26972422`, `wai-aria/scripts/aria-utils.js` calls `test_driver.get_computed_role` and `test_driver.get_computed_label`, which `resources/testdriver.js` documents as matching WebDriver's "Get Computed Role" and "Get Computed Label" commands [T1]. [TS2](../../testing.md)'s subset is "sessions, navigation, execute script (including asynchronous), window rectangles, screenshots, actions, element click and send keys, cookies, and permissions through WebDriver's extension commands".
- **Observed: the platform-mapping tests query macOS itself.** `core-aam/aamtests` and `html-aam/aamtests` run under pytest, beside a WebDriver session. `support/fixtures_a11y_api.py` takes the browser's process ID from the session's capabilities, `goog:processID`, `moz:processID` or `safari:processID`, and `support/axapi_wrapper.py` opens it with `AXUIElementCreateApplication`, through PyObjC [T1].
- **Observed: they need what the headless embedder lacks.**
  - EM9 is "the same API without AppKit", so there is no `NSAccessibility` tree to query.
  - The querying process must be trusted: `AXIsProcessTrusted` "Returns whether the current process is a trusted accessibility client", and messaging functions can return `kAXErrorAPIDisabled` (`AXUIElement.h`). (Background: the user grants that trust in System Settings.)

**Failure mode.** AX11's suites can't run at M4, and the platform mapping, the part only Axiom's own bridge implements (AX8), goes untested.

**Counter-proposal.**
1. **TS2's WebDriver subset gains Get Computed Role and Get Computed Label** at M4, answered from the renderer's tree.
2. **The `aamtests` run against the shell's build,** or a test embedder with the AppKit bridge: an `axiom:processID` capability, a matching entry in the fixture's `pid_from` carried with wptrunner's `axiom` product, and the runner's Accessibility permission granted by the owner's hand, like S15's privacy decisions.
3. **AX11 says which suite runs where.**

**Cost now and later.** Now, a list and a paragraph. Later, a test path designed at M4's gate.

**Amends.** AX11; TS2; EM9; ADR-0014's history.

---

## RV2-09. Accessibility notifications, announcements and inverted colours

**Low** · Accessibility · Blocks M4 · Challenges AX4's notifications; the accessibility design's [§9](../../architecture/accessibility.md#9-staging-and-the-complete-model); AX10

**Evidence.**
- **Observed: macOS defines eight notifications for web content** in `AXWebConstants.h`: `AXActiveElementChanged`, `AXCurrentStateChanged`, `AXExpandedChanged`, `AXInvalidStatusChanged`, `AXLayoutComplete`, `AXLiveRegionChanged`, `AXLiveRegionCreated` and `AXLoadComplete`. AX4 lists "value, selection, focus, layout and live-region changes".
- **Observed: live regions are both in and out.** AX4 posts live-region changes, while §9 puts "live regions in full" under "Later". The [corpus](../../corpus.md#2-the-sites)'s Twitch row is a "live stream and chat"; a chat log is the typical live region (inference), and Twitch is M7's target.
- **Observed: `ariaNotify`.** WPT holds eleven manual tests under `core-aam/arianotify` at the pinned revision [T1]. AppKit's `NSAccessibilityAnnouncementRequestedNotification` (`NSAccessibilityConstants.h`) is the likely mapping (inference).
- **Observed: AX10 omits inverted colours.** AppKit exposes `accessibilityDisplayShouldInvertColors` (`NSAccessibility.h`); Media Queries 5's `inverted-colors` feature would read it (background).

**Counter-proposal.** AX4 lists the eight notifications and what posts each. Live regions (`aria-live`, `aria-atomic`, `aria-relevant`, `aria-busy`) and `ariaNotify` are staged no later than M7, for Twitch's chat. AX10 adds `inverted-colors`.

**Amends.** AX4, AX10; the accessibility design's §9.

---

## RV2-10. The storage key lacks the ancestor chain bit

**Medium** · Security, storage · Blocks M3 (Web Storage, the first data stored under the key) · Challenges SW1, SW9, SW10; SI3 (a)

**The claims.**
- [SW1](../../architecture/storage-and-workers.md): "The storage key is the origin and the top-level site, the partitioned key of Client-Side Storage Partitioning. So a cross-site frame's storage is keyed by both sites (SI3 (a))".
- [SI3](../../security/process-model.md) (a): "a cross-site frame's cookies and storage are keyed by the top-level site and the frame's site, so it gets no unpartitioned credentials".

**Evidence.**
- **Inference: the A→B→A case.** A page on site A embeds site B, which embeds A again. The inner frame's origin lies in the top-level site, so SW1 gives it A's first-party key: it reads and writes A's own storage, although B chose its URL and controls its embedding.
- **Sourced: Chromium partitions that frame.** Its storage key "additionally contains a top-level site and an ancestor chain bit". "A corner-case is a first-party origin embedded in a third-party origin, such as https://a.com embedded in https://b.com in https://a.com. The inner `a.com` frame can be controlled by `b.com`, and is thus considered third-party. The ancestor chain bit tracks this status" [C1].
- **Sourced: the network partition key is a separate question, and NW8 matches Chromium.** Chromium's network anonymization key has "a->b->a => is_cross_site = false" [C2]; Fetch leaves the second key open: "The second key is intentionally a little vague as the finer points are still evolving" [W1]. [NW8](../../architecture/networking.md)'s top-level site and frame site give the same answer, so this finding leaves NW8 and NW10 alone. Whether NW11's partitioned cookies need the bit is to be verified at M2.
- **Sourced: what is keyed by storage key follows it.** Chromium's `blob:` URL registry associates each URL "with the `storage_key`, and most subsequent URL lookup attempts will require a matching StorageKey to succeed" [C5].
- **Observed: SW9 and SW10 leave keys unstated.** Shared workers' key isn't given; `BroadcastChannel` is "routed through the browser, like any operation between agents (FT6)", with no key.

**Failure mode.** A cross-site embedder gets a frame of the victim site, with its first-party storage, under its control: the confused-deputy shape that partitioning exists to remove (inference). Changing the key after M3 means migrating stored data.

**Counter-proposal.**
1. **SW1's key becomes the origin, the top-level site and an ancestor chain bit,** set when any frame between the document and the top level is cross-site to the top level.
2. **Everything scoped by storage key uses the whole key:** shared workers (SW9), `BroadcastChannel` and Web Locks (SW10), `blob:` URLs (RV2-11), the origin private file system (SW11) and quotas (RV2-12).
3. **NW8 unchanged,** with a note saying why the two keys differ.
4. **An A→B→A case** in SW13's isolation tests.

**Cost now and later.** Now, a field in a key that no data uses yet. Later, a migration of every store, and a security fix to the key.

**Amends.** SW1, SW9, SW10, SW11, SW13; SI3 (a)'s wording; NW8's note; ADR-0013's history.

---

## RV2-11. The File API has no design

**Medium** · Standards, security · Blocks M3 · Challenges the [standards](../../standards.md#52-script-platform-apis-media-and-network)' §5.2; the frame tree's special documents; SG1, EM5, IP15, SW11

**Evidence.**
- **Observed: no document designs `Blob`, `File`, `FileReader` or `blob:` URLs.** A search of `docs/` for them (and for "File API") finds only the [frame tree](../../architecture/frame-tree-and-navigation.md#4-design)'s special documents, which "derive their origin and process from the browser's records of the initiator or creator: `about:blank`, `about:srcdoc`, `data:`, `blob:`". That rule needs a store of `blob:` URLs and their creators that no document places. The standards' §5.2 has no row for the File API.
- **Inference: they arrive with script.** M3 brings `fetch`, canvas 2D (whose `toBlob` makes a `Blob`) and MDN's live examples.
- **Observed: three rules disagree about the user's files.**
  - [SG1](../../security/threat-model.md): "Files the user picks for upload are read by the browser and passed as data".
  - [EM5](../../architecture/embedder-api.md)'s file chooser: "Gives the renderer read access only to the chosen files, through a capability (IP15)".
  - [IP15](../../architecture/ipc.md): "Interfaces that face renderers can't declare `file` handles, so a renderer never receives a descriptor to a user's file".
  - And SW11 gives a worker "a descriptor for one file" of the origin private file system, "where its interface allows a descriptor (IP15)", which IP15 as written never allows on a renderer's interface.
- **Sourced: Chromium keeps the `blob:` URL registry outside the renderer,** keyed by storage key [C5]. Its 2024 intent partitions `blob:` URL access by storage key, with navigations by frame origin and `noopener` on some cross-site blob navigations, and says Firefox and Safari "already behave similarly" [C6, **secondary**; whether it shipped wasn't verified].

**Failure mode.** M3 meets `Blob` with no place for large blob data, no owner for the URL store the frame tree already relies on, and three contradictory rules for the user's files.

**Counter-proposal.**
1. **A File API section in the storage design,** before M3:
   - where blob data lives: small blobs in the renderer, large ones in the network process under its budget (inference, to be weighed against holding them in the renderer);
   - the `blob:` URL store in the browser or the network process, keyed by the whole storage key (RV2-10), revoked with its document;
   - navigation to `blob:` URLs as the frame tree's special documents describe.
2. **One rule for the user's files:** SG1's. The browser reads the chosen files and passes their data, or a read-only memory entry, so IP15 stands and EM5's wording follows it. IP15 states SW11's exception: a `file` handle only on the worker interface of the origin private file system, for files in the network process's data directory.
3. **A row in the standards' §5.2** for the File API, staged with script.

**Cost now and later.** Now, a design section and three corrected sentences. Later, a store and a security rule designed while M3's examples fail.

**Amends.** The storage design; the standards §5.2; the frame tree's §4; SG1, EM5, IP15, SW11; ADR-0013's history.

---

## RV2-12. Stale statements in storage

**Low** · Storage · Blocks M3 · Challenges the storage design's open question 2 and SW3; SG10; PF24

**Evidence.** **Observed:**
- The storage design's [open question 2](../../architecture/storage-and-workers.md#10-open-questions), "Quotas, area limits and the eviction policy's thresholds", is still settled in "Step 6", and SW3 says "Each area has a size limit, set in Step 6". PF24 set them: 5 MiB per area, and quotas per site.
- [SG10](../../security/threat-model.md): "A compromised network process exposes traffic and cookies". It also holds every site's web storage, databases and caches (SW1), so the statement understates what is exposed.
- PF24's quotas are "per site", while SW1's buckets belong to a storage key (origin and top-level site). The rule for keys that share a site isn't stated (inference: a total per site across its keys, since eviction is already "by site").

**Counter-proposal.** Close open question 2 with PF24; correct SW3 and SG10; state quotas per key within a total per site.

**Amends.** The storage design §5 and §10; SG10; PF24.

---

## RV2-13. Never blocking, beyond the browser

**Medium** · Security, IPC · Blocks M1 (the network process) and M2 (the GPU process) · Challenges IR5, IP16, PF21; SG11; EL1

**The claims.**
- [IR5](../../security/process-model.md): "The browser never blocks on a less privileged process."
- [IP16](../../architecture/ipc.md): "The browser sends to children without blocking", with a zero send timeout; "A child that floods the browser stalls only itself: its sends wait or time out against the browser's queue limit. The browser drains its ports fairly, round-robin across children".
- [PF21](../../performance.md): "The browser's backlog for a child whose queue is full is at most 1,024 messages or 8 MiB, past which the child is treated as hung".
- [SG11](../../security/threat-model.md): "A crashed, hung or memory-hungry renderer affects only its own tabs."

**Evidence.**
- **Observed: the rules name only the browser.** From M1 the network process serves every renderer, and from M2 the GPU process does. Nothing says that they send without blocking, keep a bounded backlog per renderer, drain fairly, or report a renderer that stops receiving.
- **Inference: one renderer could stall the rest.** A compromised or hung renderer that stops receiving fills its port's queue (PF21's limits). If the network process's I/O thread then blocks on a send to it, every tab's loads stop; if the GPU process blocks, every window's content stops. SG11's containment holds at L1 only if no shared helper can block on a renderer.
- **Observed: IP16's "its sends wait" contradicts [EL1](../../architecture/event-loop-and-threading.md):** "One wait per thread. Every engine thread is a thread loop: it blocks in exactly one place". A thread waiting on a full queue blocks in a second place.
- **Observed: Mach has the notification such a design needs.** `MACH_NOTIFY_SEND_POSSIBLE` (`mach/notify.h`) tells a sender when a full queue has room again.

**Failure mode.** One misbehaving renderer freezes loading or drawing for every site, at L1, where SG11 claims containment.

**Counter-proposal.**
1. **IR5 and IP16 cover every process that serves more than one less privileged peer:** the network process, the GPU process and, from M7, the media utility process. Sends have a zero timeout; each peer gets a bounded backlog, sized in PF21 per helper; past it, the peer is reported to the browser as hung and its endpoints are closed; draining is fair and bounded across peers.
2. **Children's sends follow EL1:** a zero timeout and a bounded local backlog, drained when the send-possible notification arrives; past it, the child stops producing. It never blocks.
3. **A test:** one renderer stops receiving while another loads and draws; at M1 for the network process, at M2 for the GPU process.

**Cost now and later.** Now, two rules and a test. Later, a send policy changed in every helper once their protocols exist.

**Amends.** IR5 and the process model's change record; IP16; PF21; SG11's note; ADR-0005's history.

---

## RV2-14. Contradictions in IPC's text

**Low** · IPC · Blocks M1 (IPC is built there) · Challenges IP7, IP13, IR6, PF21; IPC's §4.12 and §8

**Evidence.** **Observed:**
- **Timeouts:** [IP13](../../architecture/ipc.md): "Every synchronous request has a timeout. If it expires, the operation fails as the platform specifies for an unavailable resource." [PF21](../../performance.md): "Synchronous requests (cookies, storage, synchronous XHR) have no timeout of their own: their answer is bounded by PF19, since the caller's thread is the one being watched."
- **The target:** [IR6](../../security/process-model.md): "Synchronous calls from a child to the browser exist only where the web platform requires synchronous semantics, such as reading `document.cookie`." IP13: "These go to the network process's storage or I/O thread (EL8), never to the browser's core."
- **A synchronous request that waits on a peer:** IP13 lists synchronous `XMLHttpRequest`. In a document a service worker controls, the answer comes from the worker, which runs in a renderer (SW9), so the wait isn't "toward more privilege" alone; PF19's watchdog over the worker's renderer bounds it (inference). The rule doesn't say so.
- **`iosurface` on renderers' interfaces:** IP7 lists `iosurface` among the parameter types, and nothing bars it from interfaces that face renderers, as IP15 bars `file`. RV1-09 says renderers never hold IOSurfaces.
- **Missing families** in [§4.12](../../architecture/ipc.md#412-initial-interface-families): the network process with the browser (launching service workers, SW9; clearing data, EM4; permission prompts), and the media utility process with the GPU process (M7's video layers).
- **Open questions:** [§8](../../architecture/ipc.md#8-open-questions)'s question 3 is still settled in "Step 6" (PF21 answered it); question 4 in "CP6" (RV2-05).

**Counter-proposal.** IP13 takes PF21's rule; IR6 names the network process; IP13 adds the service-worker case; IP7 bars `iosurface` from renderers' interfaces, checked by the generator; §4.12 gains the two families; §8 closes questions 3 and 4.

**Amends.** IP7, IP13, §4.12, §8; IR6.

---

## RV2-15. Style hooks from Interop 2026

**Low** · Style, layout · Blocks M4 (style's second stage) · Challenges style's complete model ([§8](../../architecture/style.md#8-staging-and-the-complete-model)); ST4, ST12, ST19, ST25; IV1

**The claim.** Style §8: "every CSS module fits these rules without architectural change."

**Evidence.**
- **Sourced: Interop 2026's focus areas include CSS anchor positioning, `attr()` and `zoom`** (the [standards](../../standards.md#2-sources-and-precedent) §2, citing Interop 2026's list).
- **Sourced: anchor positioning adds a cascade origin, applied during layout.**
  - "All of the properties in a `@position-try` are applied to the box as part of the Position Fallback Origin, a new cascade origin that lies between the Author Origin and the Animation Origin."
  - Applying an option: "With new styles inserted into the cascade in the position fallback origin, resolve the cascade, and perform enough layout to determine el's used styles."
  - The results "are applied to the element via interleaving, so they affect computed values (and can trigger transitions/etc) even though they depend on layout and used values" [W2].

  ST12's origins don't include it, and ST25's nested lifecycle covers size containers only. Layout stages anchor positioning "Later".
- **Sourced: `zoom` scales lengths, multiplied down the tree.** "The effective zoom of an element is the product of its computed value of zoom and all flat tree ancestors' computed values of zoom", and "Unlike transform, scaling the zoom property affects layout" [W3]. ST4 absolutizes lengths "to CSS pixels in floating point", with no zoom.
- **Sourced: `attr()` makes values depend on attributes.** "The attr() function substitutes the value of an attribute on an element into a property, similar to how the var() function substitutes a custom property value" [W4]. ST19 records "for each class, ID, attribute name and state it mentions" in selectors; a declaration's `attr()` makes an attribute matter that no selector names.
- **Inference: each fits the architecture,** as the claim intends: a new origin is a field of ST12's precomputed key; interleaving generalizes ST25's nested lifecycle and IV1's amendment; effective zoom is an inherited factor applied in ST4's computed stage; `attr()` adds dependencies to ST19's map from the declaration's side. None needs new architecture, but each changes a rule the claim says new modules don't touch.

**Counter-proposal.** Name the four hooks in style's §8 and layout's §8, so the claim can be checked: the position fallback origin in ST12, interleaving under ST25 and IV1, effective zoom in ST4, `attr()` in ST19. Stage `zoom` and `attr()` with style's second stage (M4), and anchor positioning when a corpus page or a milestone's selection needs it.

**Amends.** Style §8, ST4, ST12, ST19, ST25; layout §8; ADR-0016's history.

---

## What holds, by area

Credited so that later rounds don't reopen them. Each was checked against current engines on 2026-10-21, through the designs' own precedent sections and the sources above.

| Area | What holds |
|---|---|
| **Accessibility** | Renderers push trees and the browser answers AppKit synchronously from its cache, as Chromium does and AccessKit's schema follows; text markers; Axiom's own bridge, since AccessKit's macOS adapter lacks "rich text or hypertext" (AX8); segmentation in the renderer; frames stitched by token; an assistive technology's press routed as input, so it counts as user activation (AX4, IN8) |
| **Storage and workers** | All web storage in the network process, segregated by key (SW1, SW2); keys derived from endpoints, never named by a renderer (IR2); IndexedDB's values opaque to the network process (SW4); `localStorage` cached in the renderer, as Chromium's; `redb` against SQLite decided by crash tests (SW13); workers never share a process across sites (SW9) |
| **Style** | Typed value stages (ST4); the rule tree, shared computed styles and the sharing cache; Blink's invalidation sets (ST19) and Stylo's `:has()` dependencies (ST10); ST25's nested lifecycle for container queries, as Blink's; `:visited` by partitioned history; correctness without the caches, checked by IV14 |
| **Layout** | LayoutNG's contract of immutable fragments from constraints (LY6); 1/64-pixel fixed point, as Blink, WebKit and Ladybird use; incremental layout checked against a rebuild (IV14); bounded recursion with explicit stacks and saturating quantities (§7); hit testing only within a renderer's own frames (LY25) |
| **Embedder API** | A message API, as Chromium's content layer and WKWebView's delegates are; native interface rather than privileged web pages; the headless embedder, with RV1-01's script-free path and RV1-15's dumps; the pure-Rust shell, with Swift as a fallback |
| **IPC** | Mach as transport (S4); one IDL with generated decoders and `Untrusted<T>`, as WebKit's; messages always encoded, even in one process (IP2); each sender checked against the audit token pinned for its channel (IP19), as Chromium pins them; synchronous requests only toward more privilege; bounded queues; every decoder fuzzed |

---

## Sources

All read on 2026-10-21. Repository documents are linked where they are cited, and observations name their files or commands. Chromium sources were downloaded from the `main` branch.

- [C1] Chromium, `third_party/blink/public/common/storage_key/storage_key.h` (© 2021) — https://github.com/chromium/chromium/blob/main/third_party/blink/public/common/storage_key/storage_key.h
- [C2] Chromium, `net/base/network_anonymization_key.h` (© 2022) — https://github.com/chromium/chromium/blob/main/net/base/network_anonymization_key.h
- [C3] Chromium, `content/browser/locks/lock_manager.h` (© 2017) — https://github.com/chromium/chromium/blob/main/content/browser/locks/lock_manager.h
- [C4] Chromium, `services/network/README.md`, "What happens if the network service crashes?" — https://github.com/chromium/chromium/blob/main/services/network/README.md
- [C5] Chromium, `storage/browser/blob/blob_url_registry.h` (© 2020) — https://github.com/chromium/chromium/blob/main/storage/browser/blob/blob_url_registry.h
- [C6] **Secondary.** blink-dev, "Implement and Ship: Blob URL Partitioning: Fetching/Navigation", 2024-10-29, read through a search engine's summary, not in full — https://groups.google.com/a/chromium.org/g/blink-dev/c/erVBugcYwRc/m/tt0zqNIjAAAJ
- [C7] Chromium commit `34d5b59a67da`, "[Mac A11y Sonoma] Refine a11y detection and enablement on macOS Sonoma", 2024-01-23 (main@{#1250961}) — https://chromium.googlesource.com/chromium/src/+/34d5b59a67da7b0ea1a3eb90835467eb63e7262e
- [C8] Chrome for Developers, "Improving the performance of Chromium accessibility", last updated 2024-08-20 — https://developer.chrome.com/blog/chromium-accessibility-performance
- [C9] Chromium code review 2393123002, "Implement caching asynchronous accessibility hit testing", committed 2016-10-11 — https://codereview.chromium.org/2393123002
- [C10] Chromium code review 23983002, "Expose InlineTextBoxes in the accessibility tree", created 2013-09-05, committed 2013-10-19 (Blink r160020) — https://codereview.chromium.org/23983002
- [T1] web-platform-tests at `0f26972422`: `wai-aria/scripts/aria-utils.js`; `resources/testdriver.js`; `core-aam/aamtests/support/fixtures_a11y_api.py` and `axapi_wrapper.py`; the `core-aam` and `core-aam/arianotify` directories, listed through GitHub's API — https://github.com/web-platform-tests/wpt/tree/0f26972422b15f587e2afbd38dbb6ea2397fded0
- [W1] WHATWG, Fetch, Living Standard, last updated 2026-10-19, §2.7 "Network partition keys" — https://fetch.spec.whatwg.org/#network-partition-keys
- [W2] W3C, CSS Anchor Positioning Module Level 1, Editor's Draft, 2026-09-06, §6.4 and §6.5.2 — https://drafts.csswg.org/css-anchor-position-1/
- [W3] W3C, CSS Viewport Module Level 1, Editor's Draft, 2026-08-29 (`zoom`) — https://drafts.csswg.org/css-viewport/
- [W4] W3C, CSS Values and Units Module Level 5, Editor's Draft, 2026-09-04 (`attr()`) — https://drafts.csswg.org/css-values-5/
- **Local:** the Command Line Tools' `MacOSX27.0.sdk`: `AppKit.framework/Headers/NSAccessibility.h` and `NSAccessibilityConstants.h`; `HIServices.framework/Headers/AXWebConstants.h`, `AXUIElement.h` and `AXError.h`; `usr/include/mach/notify.h`. The downloaded copies of the sources above are kept under `scratch/review-r2/` (not committed)
