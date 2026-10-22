# Review Round 4: Round 3 Verified

| | |
|---|---|
| **Status** | **Round 4, disposed** (2026-10-22): the owner accepted all seventeen findings as proposed, with the choices the [disposition log](../README.md#59-round-4s-dispositions) records. It stays as written, as the round's record; where the owner's words differ, they govern |
| **Question** | Which of Round 3's claims hold when tested, against the repository, primary sources and measurements on this Mac, and which don't? |
| **Covers** | Round 3, Gemini's external review ([`01-gemini-review.md`](../round-3/01-gemini-review.md), dated 2026-10-21), every claim in it, at the baseline on `architecture-review` at `e71803b` |
| **Summary** | [Risk matrix](01-risk-matrix.md) · [Findings](06-findings.md) · [Actions](07-actions.md) |
| **Last updated** | 2026-10-22 |

**How to read it.** Round 3 didn't number its findings. This round gives each claim an ID, `RV3-01` to `RV3-18`, in the order of Round 3's document, and names the section it comes from. Each entry quotes the claim, says what the baseline holds, tests the claim, gives a verdict, and names the Round 4 finding that carries any change. Evidence is labelled as the [research conventions](../../research/README.md) require. **Measured** facts come from spike S16, authorized by the owner for this round ([S16 results](../../research/s16-results.md)); **observed** facts were read on this Mac or in the repository; sources were downloaded with `curl` on 2026-10-22, at the revisions [§3](#3-sources) names.

**Verdicts:**
- **Confirmed:** the claim holds as stated, or in substance;
- **Partly confirmed:** part of it holds, and part doesn't, as the entry says;
- **Refuted:** it doesn't hold;
- **Already covered:** it holds, but the baseline already says so.

---

## 1. Summary

| ID | Round 3's section | The claim, in short | Verdict | Carried by |
|---|---|---|---|---|
| RV3-01 | 1.A | The browser's IOSurface lookups move IOKit's attack surface into the browser; neither WebKit nor Chromium passes the UI process surfaces | **Partly confirmed:** WebKit does exactly what Axiom does; Chromium's default doesn't. A real exposure remains: values the GPU process attaches | [RV4-07](06-findings.md#rv4-07-what-the-browser-takes-from-the-gpu-processs-surfaces) |
| RV3-02 | 1.B | Rights in dropped messages leak, and at 60 Hz exhaust 65,536 entries | **Partly confirmed:** the leak is real and measured; the rate, the number and the error codes are wrong | [RV4-05](06-findings.md#rv4-05-rights-in-messages-that-are-dropped-rejected-or-returned) |
| RV3-03 | 1.C | A renderer's death re-attributes its pages to the GPU process, which then dies over B1b | **Refuted as stated; a different flaw confirmed:** nothing is re-attributed, but the memory stays resident and charged to no one until the compositor unmaps it | [RV4-04](06-findings.md#rv4-04-a-dead-renderers-memory-charged-to-no-one) |
| RV3-04 | 1.D | `IOSurfaceIsInUse` is a lock count that races the GPU's timeline | **Partly confirmed:** it is the use count both engines rely on; the writer's half of the protocol is missing | [RV4-10](06-findings.md#rv4-10-the-tile-protocols-writer-side) |
| RV3-05 | 2.A | M1's ≥ 97% CSS target is unviable | **Partly confirmed:** the figures are right; "unviable" isn't shown, but the target asks for the long tail first. **Superseded** by a larger problem: the tests can't run without an XML parser | [RV4-09](06-findings.md#rv4-09-m1s-css-target-asks-for-the-long-tail-first), [RV4-01](06-findings.md#rv4-01-m1s-css-gate-needs-an-xml-parser-m1-doesnt-build) |
| RV3-06 | 2.B.1 | `tools/wpt-script-deps.py` never reads reference files | **Confirmed for the tool; no effect on the count:** no reference uses script | Fixed in this round (the tool) |
| RV3-07 | 2.B.2 | The script-free harness must wait for web fonts | **Already covered** (MA1, TS2) | — |
| RV3-08 | 2.B.3 | Antialiasing differences fail tests without fuzzy tolerances | **Refuted:** a reference test compares two renderings by the same engine | — |
| RV3-09 | 2.B.4 | Dynamic pseudo-class tests can't run without script | **Refuted:** none of the 5,652 uses them | — |
| RV3-10 | 3.A | No test exercises real processes concurrently | **Partly confirmed:** MA7 tests kills, stalls and containment; nothing stresses timing | [RV4-11](06-findings.md#rv4-11-channels-in-one-process-and-stress-in-many) |
| RV3-11 | 3.B | In-memory queues don't behave as Mach's | **Partly confirmed:** IP2 promises "the same API" but not the same limits and failures | [RV4-11](06-findings.md#rv4-11-channels-in-one-process-and-stress-in-many) |
| RV3-12 | 4.A | A hung lock holder deadlocks other tabs, since PF19 watches only the control thread; remedy: revoke the lock | **Confirmed** for the detection; **refuted** for the remedy | [RV4-03](06-findings.md#rv4-03-a-hung-event-loop-goes-unnoticed) |
| RV3-13 | 4.B | One ancestor bit lets B and C share A's partition | **Refuted as a flaw:** at parity with Chromium, and no new channel | — |
| RV3-14 | 5.A | Writing every legacy encoding before M1 is scope creep | **Confirmed in substance** | [RV4-08](06-findings.md#rv4-08-m1s-scope-against-value), [RV4-02](06-findings.md#rv4-02-the-dependency-policy-ranks-mature-rust-below-c) |
| RV3-15 | 5.B | M1 is monolithic; split it into M1-A (one process, `file:` only), M1-B and M1-C | **Partly confirmed:** M1 is large, but MA11 already stages it; M1-A as proposed can't run WPT and reverses RV1-07 | [RV4-08](06-findings.md#rv4-08-m1s-scope-against-value) |
| RV3-16 | Additional 1 | A global bootstrap name lets any process reach the browser; the engines inherit ports instead | **Refuted** for the precedent and the threat; a hardening is worth taking | [RV4-12](06-findings.md#rv4-12-the-rendezvous-checks-a-pid-not-a-process) |
| RV3-17 | Additional 2 | IP13's synchronous requests are bounded by a watchdog that doesn't watch their thread | **Confirmed** | [RV4-03](06-findings.md#rv4-03-a-hung-event-loop-goes-unnoticed) |
| RV3-18 | Verdict | "Blocked by findings"; merge after five amendments | **Agreed that the merge should wait,** for other reasons: an XML parser M1 lacks, and a dependency policy that blocks M1's text stack | [Risk matrix](01-risk-matrix.md) |

**Round 3's errors of fact,** besides the verdicts above, each checked:
- `posix_spawnattr_setmacpolicyinfo_np` passes MAC policy data (the sandbox's), not ports; `mach_ports_register` is not how either engine hands a helper its ports (RV3-16).
- "65,536 entries" is not XNU's ceiling: macOS's kernels allow 305,834, and a process that reaches it is ended, not given `KERN_RESOURCE_SHORTAGE` (RV3-02).
- `MACH_SEND_NO_BUFFER` reports that the kernel couldn't allocate a message, not a full queue or table (RV3-02).
- `IOSurfaceIsInUse` reads a surface's use count, not `IOSurfaceLock`'s (RV3-04).
- "WPT `css/CSS2` tests are UTF-8, US-ASCII, or UTF-16": a few declare `windows-1252`, `iso-8859-1` and `shift-jis` (RV3-14).

## 2. The claims

### RV3-01. IOSurface lookups in the browser

**Round 3, §1.A:** "`IOSurfaceLookupFromMachPort` creates an active client connection to `IOSurfaceRootUserClient` directly inside the unsandboxed browser process (Z1)"; a compromised GPU process "can synthesize malformed `IOSurface` Mach ports or corrupted IOSurface memory metadata"; "Neither WebKit nor Chromium directly exchanges raw per-tile `IOSurface` Mach ports with the UI process on every frame."

**The baseline:** GR14 sends the browser "each tile's IOSurface as a Mach port"; GR12 has the browser delete each right once it has looked the surface up. S10 sent a right only for each surface the browser hadn't seen ([S10 results](../../research/s10-results.md) §1).

**Tested:**
- **Sourced: WebKit does what Axiom does.** Its UI process receives each backing store's IOSurface as a `MachSendRight` in remote layer tree transactions, turns it into a surface with `WebCore::IOSurface::createFromSendRight`, and sets it as the layer's contents (`[layer setContents:bufferInfo.buffer.get()]`). It caches surfaces by resource identifier, so a right is used only for a buffer it hasn't seen, and it reads the surface's pixel format and its EDR headroom ([K1], `RemoteLayerBackingStore.mm`, `RemoteLayerBackingStoreProperties::layerContentsBufferFromBackendHandle` and `lookupCachedBuffer`).
- **Sourced: Chromium's default doesn't.** With `kRemoteCoreAnimationAPI` declared `FEATURE_ENABLED_BY_DEFAULT`, the browser hosts the GPU process's own CoreAnimation context through a `CALayerHost` and its context ID; it looks surfaces up from ports only because "IOSurfaces can be sent from software compositing, or if remote layers are manually disabled" ([C1], `remote_layer_api.mm`, `display_ca_layer_tree.mm`).
- **The cost of Chromium's way is the GPU process's sandbox.** **Sourced:** Chromium's `gpu.sb` allows `com.apple.CARenderServer` under the comment "Allow communication between the GPU process and the UI server", among 19 global services, LaunchServices' and the preferences daemons' among them ([C1], `sandbox/policy/mac/gpu.sb`). **Observed:** S10's profile allows two Mach lookups, the rendezvous and Metal's compiler service (`spikes/s10/gpu/profiles/gpu.sb`). **Inference:** a context the browser hosts has to be created in the GPU process through CoreAnimation's render server, so that service, at least, joins the profile.
- **Measured: names that aren't surfaces fail safely.** `IOSurfaceLookupFromMachPort` returned no surface for a plain port's right, the null name, `MACH_PORT_DEAD` and a dead name, and consumed none of them (S16 §4). A Mach right names a kernel object, so a sender can't "synthesize" a malformed one; what it controls is the surface's contents and the values attached to it.
- **Measured: the creator's values reach the browser.** The GPU process attached 64 KiB under its own key and 4 KiB of arbitrary bytes as an ICC profile; the browser read both after its lookup, and could remove them (`IOSurfaceRemoveAllValues`), leaving three (S16 §3.3).
- **Inference:** the browser's IOKit connection to IOSurface exists anyway, since AppKit and CoreAnimation use it for the browser's own windows; the lookup adds a kernel call on an untrusted name, which the kernel validates.

**Verdict: partly confirmed.** The precedent claim is wrong for WebKit and right for Chromium's default path, whose alternative widens the GPU process's sandbox by a whole class of services. The concrete exposure is not the port but what rides on the surface: attached values, including colour profiles, that CoreAnimation or the browser may read. **Carried by RV4-07.**

### RV3-02. Port-table exhaustion

**Round 3, §1.B:** "port rights contained in dropped or unreceived messages are **not** automatically reclaimed unless the receiver explicitly parses the `mach_msg_body_t` and calls `mach_port_deallocate`"; "At 60 Hz across multiple tiles, leaking port descriptors will rapidly exhaust the process Mach port limit (default 65,536 entries), resulting in `KERN_RESOURCE_SHORTAGE` or `MACH_SEND_NO_BUFFER`."

**The baseline:** IP5 caps rights per message; IP11 drops a message that is an expected race (`Violation::Ignore`); IP16's browser keeps a backlog when a child's queue is full. No rule says who releases the rights in a dropped, rejected or returned message (observed).

**Tested:**
- **Measured, received messages:** a sandboxed sender's 1,000 messages, each carrying its own right, added exactly 1,000 entries to the receiver's port space when it dropped them without `mach_msg_destroy`, and none when it destroyed them (S16 §4.1).
- **Measured, unreceived messages:** five messages queued in a port when it was destroyed released their rights: the kernel reclaims them. So "dropped or **unreceived**" is half wrong.
- **Measured, failed sends:** a zero-timeout send to a full queue came back with its rights and an extra reference to the destination. **Sourced:** XNU's pseudo-receive returns "the ports in the header … as if they are in the body" ([X1], `ipc_kmsg.c`).
- **Sourced, the ceiling:** "7M == 305834 entries" on the kernel configurations Macs run (131,072 on the small ones) ([X1], `config/MASTER`). At the ceiling a new entry fails with `KERN_NO_SPACE` (`ipc_entry.c`), and XNU then ends the task: "Abort the process if it has hit the system-wide limit for ipc port table size", with a fatal `EXC_RESOURCE` (`RESOURCE_TYPE_PORTS`, `FLAVOR_PORT_SPACE_FULL`) (`osfmk/kern/task.c`). So the limit isn't 65,536, and the outcome is the process's end, not `KERN_RESOURCE_SHORTAGE`. `MACH_SEND_NO_BUFFER` is what a send returns when the kernel can't allocate the message (`mach_msg.c`), not a sign of a full table.
- **Observed, the rate:** tiles are allocated ahead in a bounded pool and reused (GR12, GR8), and a right crosses once per surface (S10), so ordinary frames carry no new rights. The real exposure is a renderer that floods the browser with rights in messages it will drop.

**Verdict: partly confirmed.** The leak is real, a renderer could use it to end the browser, and no rule prevents it. The rate, the limit and the error codes are wrong. **Carried by RV4-05.**

### RV3-03. Ghost memory

**Round 3, §1.C:** "Because the original owning task has ceased to exist, **the kernel's Mach VM subsystem re-attributes the resident memory footprint of those orphaned pages entirely to the surviving mapping task (the GPU process)!**" "this immediate spike will cause the browser's watchdog or macOS memory pressure handler to kill the GPU process every time a tab crashes or navigates."

**The baseline:** PL8 releases resources "when unused"; IR10 has the browser discard what a dead child reported; GR13 covers the GPU process's own death. Nothing says the compositor releases a dead renderer's resources (observed). S10 read footprints "before the helpers exit" (S10 §2), so the claim was untested.

**Tested** (S16 §3, three runs of eight configurations: the compositor in the browser, as at M1, and in a GPU process, as from M2; the renderer killed or exiting; with and without a CPU read):
- **Measured: no re-attribution.** When the renderer died, the compositor's charge didn't rise in any run: 16.3 → 6.0 MiB in the browser, 13.8 → 5.5 MiB in the GPU process, five seconds later.
- **Measured: but nothing was freed.** The 64 MiB stayed resident and dirty in the objects the compositor mapped, their references falling from 3 to 2; within five seconds they were often compressed (14 of 24 runs). They stayed until the compositor unmapped them, **charged to no process.**
- **Measured: a CPU read is what charges.** The compositor's charge rose by 64 MiB when it read the pages from its CPU, before the death (both processes charged at once) or after it.
- **Sourced: why.** XNU charges an internal page to the task whose page tables map it ([X1], `osfmk/arm/pmap/pmap.c`, with the footprint's formula), so a mapping the GPU reads without the CPU carries no charge.

**Verdict: refuted as stated; a different flaw confirmed.** The GPU process isn't killed over B1b. Instead, a dead renderer's memory lingers where no budget sees it, and becomes the compositor's the moment anything reads it. **Carried by RV4-04.**

### RV3-04. `IOSurfaceIsInUse` and the GPU's timeline

**Round 3, §1.D:** "`IOSurfaceIsInUse` is an instantaneous CPU-side read of an internal kernel lock count (`IOSurfaceLock` / `IOSurfaceUnlock`)"; relying on it "without cross-process GPU-timeline synchronization (e.g. `MTLSharedEvent` shared between the GPU process and the browser's CoreAnimation compositor …) introduces race conditions."

**Tested:**
- **Observed:** the SDK's `IOSurfaceRef.h` documents `IOSurfaceIsInUse` with the use-count calls (`IOSurfaceIncrementUseCount`, `IOSurfaceDecrementUseCount`): it "Returns true if an IOSurface is in use by any process in the system", and it is "the only call that should be used for that purpose". `IOSurfaceLock` is a separate call, for the CPU's access.
- **Sourced: both engines decide reuse with it.** WebKit's `IOSurfacePool` skips a surface that `isInUse()`, which calls `IOSurfaceIsInUse` ([K1], `IOSurfacePool.cpp`, `IOSurface.mm`); Chromium's software output device skips buffers that are in use, and its shared-image backing names the call `IsInUseByWindowServer`, noting that "IOSurfaceIsInUse() will always return true if the IOSurface is wrapped in a CVPixelBuffer" ([C1]).
- **Sourced: the writer's side is where the synchronization is.** Chromium: "We have to wait for pending work to be scheduled on the GPU for IOSurface synchronization by the kernel e.g. using waitUntilScheduled on Metal or glFlush on OpenGL", and it waits so before CoreAnimation reads a surface ([C1], `iosurface_image_backing.mm`, `WaitForCommandsToBeScheduled`). Its shared events throttle the GPU process's own frames (`ca_layer_tree_coordinator.mm`, `ApplyBackpressure`); none crosses to the browser.
- **Inference:** Axiom's browser doesn't run Metal; CoreAnimation and the window server read the tiles, so a shared event "with the browser's CoreAnimation compositor" would have no consumer. **Observed:** S10's GPU process waited for each command buffer to complete before sending its commit (S10 §1), a stricter rule than Chromium's, but neither GR12 nor GR14 states any rule.

**Verdict: partly confirmed.** The mechanism is misdescribed, and the reuse check is standard; the writer's rule, and Chromium's caveat for video frames, are missing. **Carried by RV4-10.**

### RV3-05. The 97% target

**Round 3, §2.A:** the target "requires a brand-new engine to outperform Safari (96.4%), Servo (93.7% after 14 years), and Ladybird (91.4% after 6 years)", "an unviable gate that will stall Milestone 1 indefinitely"; recommended: ≥ 88% over core directories at M1, 97% at M3.

**Tested:**
- **Measured: the figures are right.** Over MA4's 5,652 tests, from wpt.fyi's runs at `0f26972422` (and Ladybird's at `a26422aab3`): Chrome 98.2%, Firefox 97.8%, Safari 96.4%, Servo 93.7%, Ladybird 91.4%, median 97.8%, as MA4 already says (`tools/wpt-pass-rates.py --tests`; [W2]).
- **Observed: "beats Safari" follows from the rule,** since a median of three always exceeds the lowest; SB11 sets targets so, for completed areas.
- **Observed: by chapter** (the same runs): the large chapters' medians are 97–100% (normal flow 736 tests, margins and padding 682, positioning 513, borders 504, selectors 468, text 388); the low ones are small (errata 33% of 9, `visudet` 89% of 37, floats 88% of 103).
- **Inference: "unviable" isn't shown.** The project has no deadline (OQ-1). What the target does is put CSS 2.1's long tail into the first milestone, ahead of the work M2 and M3 exist to validate.
- **Observed, and larger:** 5,423 of the 5,652 tests need an XML parser that M1 doesn't build (RV4-01), so as written the gate can't be run at all.

**Verdict: partly confirmed, and superseded.** **Carried by RV4-09** (the target's level) **and RV4-01** (the XML parser).

### RV3-06. References and script

**Round 3, §2.B.1:** the tool "only inspects the *test file* … It **never parses or inspects the `<link rel="match">` reference file**", so tests whose references use script were "falsely classified as 'script-free'".

**Tested:**
- **Observed: true of the tool,** which read only the files the manifest lists as tests.
- **Measured: no effect on the count.** The tool now follows every reference and every chain of references through the manifest; with the 25 files of `css/reference/` added to S9's checkout at the same revision, every reference the tests reach is present, **none contains script and none uses `reftest-wait`,** and 196 tests still contain script. `--tests-only` reproduces Round 1's count: 196, a ceiling of 96.65%.

**Verdict: confirmed for the tool; the count stands.** Fixed in this round: [`tools/wpt-script-deps.py`](../../../tools/wpt-script-deps.py).

### RV3-07. Fonts before the screenshot

**Round 3, §2.B.2:** "the headless browser must implement a native asynchronous font load barrier that delays the screenshot until all web fonts are rasterized."

**Tested:** **observed:** MA1's script-free path "loads one URL, waits natively for the load event and for pending images and fonts, writes a PNG", and TS2 says the same; the headless embedder registers WPT's test fonts so that they resolve by name (TS3). **Observed:** 1,148 of MA4's 5,652 tests link `/fonts/ahem.css` (1,170 mention Ahem), and 9 need another web font (`tools/wpt-script-deps.py --needs`; the counts' method in [§3](#3-sources)).

**Verdict: already covered.** The nine tests with other web fonts wait for M4, which brings `@font-face` (MA2).

### RV3-08. Antialiasing

**Round 3, §2.B.3:** "Even fractional pixel hinting differences or subpixel anti-aliasing variations between rasterizers will cause tests to register false-negative failures."

**Tested:** **background:** a reference test passes when the engine renders the test and its reference the same; another engine's pixels never enter. **Inference:** what matters is consistency within one rasterizer, notably Ahem's square glyphs against boxes, which TX21's path renders in both. TS3 already compares "under the tests' fuzzy tolerances".

**Verdict: refuted.**

### RV3-09. Pseudo-classes

**Round 3, §2.B.4:** "`css/CSS2` contains tests for dynamic pseudo-classes (`:hover`, `:active`, `:focus`). A purely static screenshot runner cannot trigger these states."

**Tested:** **observed:** none of MA4's 5,652 tests uses `:hover`, `:active` or `:focus`; `css/CSS2`'s tests of them need script, or are manual tests, which the selection already leaves out.

**Verdict: refuted.**

### RV3-10. Real processes, concurrently

**Round 3, §3.A:** "**Zero test suites evaluate real-world multi-process concurrency!**", masking races between network chunks and parser yields, frame offers and display-list updates, backpressure and send-possible wake-ups.

**Tested:** **observed:** MA4 runs conformance in real, sandboxed processes on the host clock (RV2-03); MA7 tests the network process killed and stalled and a renderer that stops receiving; MA5 loads the corpus copies and live pages ten times. **Observed:** none of these varies timing on purpose, and MA8's chunking test runs in the parser's harness, not at the socket.

**Verdict: partly confirmed.** **Carried by RV4-11.**

### RV3-11. In-memory queues against Mach

**Round 3, §3.B:** "In-memory queues do not simulate Mach queue limits …, kernel message buffers, or Mach descriptor allocations."

**Tested:** **observed:** IP2 gives in-process channels "the same API (R6)"; neither IP2 nor R6 says they enforce the same queue limits, the zero-timeout send's failure, the returned message or the send-possible notification, which IP16's backlog depends on.

**Verdict: partly confirmed.** **Carried by RV4-11.**

### RV3-12. A hung lock holder

**Round 3, §4.A:** a lock held across `while (true) {}` stays granted, because "the renderer process has not died" and PF19 "pings each renderer's control thread", so "Every other tab … will be permanently blocked." Remedy: "per-request acquisition timeouts, client heartbeats …, and the ability to forcibly break and revoke orphaned locks."

**Tested:**
- **Observed: the detection gap is real.** PF19's renderer deadlines run from input not acknowledged ("5 s without acknowledging input"), and EL §4.7 says the browser "measures how long each renderer takes to acknowledge input". A hung event loop with no input pending is never noticed, and its control thread still answers pings.
- **Sourced: the remedy has no precedent.** Chromium's lock manager releases a lock when "Dropping the handle - either explicitly by script or by process termination", or when script steals it ([C1], `content/browser/locks/lock_manager.h`). Revoking a live agent's lock would break the mutual exclusion the API promises (inference).
- **Sourced: detection by progress has precedent.** Chromium's `HangWatcher` "Monitors registered threads for hangs by inspecting their associated HangWatchStates for deadline overruns" ([C1], `base/threading/hang_watcher.h`). **Background:** Firefox offers to stop a script that has run longer than `dom.max_script_run_time`, whether or not input is waiting (the preference is [F1] in [performance](../../performance.md)).

**Verdict: confirmed for the detection, refuted for the remedy.** Ending the hung agent releases its locks, as SW10 already provides. **Carried by RV4-03.**

### RV3-13. One ancestor bit

**Round 3, §4.B:** frames of A inside B inside A, and of A inside C inside A, share the key (A, A, true), which "allows embedder B and embedder C to establish a cross-embedder communication side-channel through A's shared third-party partition".

**Tested:** **observed:** SW1 adopts Chromium's bit, quoting its reason ([RV2-10](../round-2/03-findings.md#rv2-10-the-storage-key-lacks-the-ancestor-chain-bit)). **Inference:** both frames hold A's code, in a page whose top level is A, which can already talk to B and C directly; B and C can't read A's partition, and could exchange data through it only with A's cooperation.

**Verdict: refuted as a flaw.** A sentence in SW1 could say so; no finding.

### RV3-14. Legacy encodings

**Round 3, §5.A:** writing "WHATWG-compliant decoders for 30+ legacy character sets" before M1 is "severe scope creep"; M1 needs UTF-8 and UTF-16, or `encoding_rs` could be admitted in the sandboxed renderer.

**Tested:**
- **Observed:** TX3 makes "every encoding it defines" Axiom's own, and MA1 builds "the encodings" at M1.
- **Observed, the documents:** of MA4's 5,652 tests, 5,278 are XML documents that are UTF-8 by default or by declaration; 82 start with a UTF-8 byte-order mark; 56 declare UTF-8 in a `<meta>`; 15 are US-ASCII by their `.headers` file or declaration; one declares Shift_JIS, in an ASCII-only file. 220 HTML files declare nothing, and 15 of them hold bytes beyond ASCII, which an HTML parser decodes with its locale's default, windows-1252 in English.
- **Observed, with the files they load:** 38 tests declare, in the test or a stylesheet it loads, an encoding other than UTF-8, UTF-16 and windows-1252, whose labels include US-ASCII and ISO-8859-1. 36 are in `syntax/` (`at-charset-*` and `character-encoding-*`) and 2 in `generated-content/`; the labels are Shift_JIS, ISO-8859-5, -6, -7, -8 and -11, KOI8-R and Big5, and two invalid ones. **So legacy decoders beyond those three can decide at most 38 tests, 0.7%.**
- **Sourced, secondary:** W3Techs puts UTF-8 at 99.1% of websites whose encoding it knows, ISO-8859-1 at 0.8%, windows-1252 at 0.2%, windows-1251 and EUC-JP at 0.1% each, and every other encoding below 0.1% (dated 2026-10-22) [W3]. ISO-8859-1 and windows-1252 are one encoding in the Encoding Standard.
- **Observed:** `encoding_rs` 0.8.42 is `ub-risk-3` by Axiom's audit ([audit](../../research/encoding-rs-audit.md)), with no RustSec advisory in its history ([dated facts](03-reuse-and-scope.md#4-dated-facts-on-the-crates-axiom-would-use)).

**Verdict: confirmed in substance.** **Carried by RV4-08** (staging) **and RV4-02** (the policy that bars `encoding_rs`).

### RV3-15. Staging M1

**Round 3, §5.B:** M1 implements everything "simultaneously before passing a single test"; split it into M1-A ("Single-process, headless embedder, local `file:` only"), M1-B (real processes) and M1-C (network and shell).

**Tested:**
- **Observed:** MA11 already orders M1 in twelve checkpoints, with the harness running end to end at the second (RV1-01).
- **Observed: M1-A as proposed can't run WPT.** Its tests load `/fonts/ahem.css` and `/css/support/` by absolute path from WPT's server (1,180 load Ahem), which MA11's second checkpoint reaches over HTTP/1.1 on the loopback.
- **Observed: M1-A reverses an owner's decision.** RV1-07, in the owner's words: "Run real, sandboxed renderer and network processes from Milestone 1". MA11's first checkpoint launches them, from S4's, S10's and S15's code.
- **Inference:** M1 is still large, and some of its contents serve little at M1 (RV4-08).

**Verdict: partly confirmed.** **Carried by RV4-08.**

### RV3-16. The bootstrap rendezvous

**Round 3, "Additional" §1:** "Any other application running under the user's account can discover and connect to this port"; "Chromium and WebKit do not use global bootstrap names for helper IPC. They use **Mach port inheritance via `posix_spawnattr_setmacpolicyinfo_np` / `mach_ports_register`**".

**Tested:**
- **Sourced: Chromium uses a bootstrap name.** Its server checks in `"%s.MachPortRendezvousServer.%d"`, formatted "with the BaseBundleID and PID of the server", by `bootstrap_check_in`. It receives each request "using the kernel audit token to ascertain the PID of the sender", and gives ports only to PIDs it registered. By default it also validates each client against a code-signing requirement (`kMachPortRendezvousValidatePeerRequirements`, enabled by default), and aborts the rendezvous on a failure only behind a feature that is off by default (`kMachPortRendezvousEnforcePeerRequirements`). Its sandbox profiles allow the name to every helper ([C1], `base/apple/mach_port_rendezvous.cc`, `mach_port_rendezvous_mac.cc`, `sandbox/policy/mac/common.sb`). The repository's [macOS note](../../research/macos-process-confinement.md) already recorded the profile rule.
- **Background:** `posix_spawnattr_setmacpolicyinfo_np` passes data to a MAC policy module, which is how a sandbox can be given to a child, not ports.
- **Observed: the threat is outside the model.** The [threat model](../../security/threat-model.md) §4 lists "local malware running as the user" among those that are "Not adversaries".
- **Observed: a hardening.** IP4 checks that the hello's "PID is the child it spawned". A PID can be reused; the audit token carries the PID's version too, and S15 gives each helper a stable code identity to check.

**Verdict: refuted** for the precedent and the threat. **Carried by RV4-12,** as hardening.

### RV3-17. Synchronous requests and the watchdog

**Round 3, "Additional" §2:** IP13 bounds synchronous requests by PF19, but "the thread making synchronous calls is the event-loop thread, while PF19 pings the control thread."

**Tested:** **observed:** PF21 and IP13 say "their answer is bounded by PF19, since the caller's thread is the one being watched"; PF19 pings control threads and measures input acknowledgements. The network process's storage thread, which answers them, isn't watched either: its control thread answers for it.

**Verdict: confirmed.** **Carried by RV4-03.**

### RV3-18. The verdict and the recommendations

**Round 3:** "Blocked by Findings", then "Conditional go": merge after five amendments (the conformance gate; staging M1; ports and memory; locks and the watchdog; encodings).

**Tested against this round's findings:**
- **Recommendation 1, the gate:** in part, as RV4-09; its ≥ 88% over four directories is less than the data supports (§RV3-05).
- **Recommendation 2, three sub-phases:** M1-A doesn't work as proposed (RV3-15); RV4-08 stages content instead.
- **Recommendation 3:** unmapping a dead renderer's memory, as RV4-04; releasing dropped rights, as RV4-05; a shared event for tiles, replaced by the writer's rule (RV4-10).
- **Recommendation 4:** the watchdog, as RV4-03; revoking locks, rejected.
- **Recommendation 5:** staging encodings, as RV4-08.

**Verdict: agreed that the merge should wait,** because of two findings Round 3 didn't make: [RV4-01](06-findings.md#rv4-01-m1s-css-gate-needs-an-xml-parser-m1-doesnt-build), Critical, and [RV4-02](06-findings.md#rv4-02-the-dependency-policy-ranks-mature-rust-below-c), High.

## 3. Sources

Downloaded with `curl` on 2026-10-22; each repository at the commit named.

- [K1] WebKit at `51e7f3eff5b2` (2026-10-22): `Source/WebKit/Shared/RemoteLayerTree/RemoteLayerBackingStore.mm`, `Source/WebCore/platform/graphics/cocoa/IOSurface.mm`, `Source/WebCore/platform/graphics/cg/IOSurfacePool.cpp` — https://github.com/WebKit/WebKit/tree/51e7f3eff5b249aa01d65cc1eb647aba6b1b9a46
- [C1] Chromium at `d313930d0dbf` (2026-10-22): `ui/base/cocoa/remote_layer_api.mm`, `ui/accelerated_widget_mac/display_ca_layer_tree.mm`, `sandbox/policy/mac/gpu.sb`, `sandbox/policy/mac/common.sb`, `components/viz/service/display_embedder/software_output_device_mac.cc`, `gpu/command_buffer/service/shared_image/iosurface_image_backing.mm`, `content/browser/locks/lock_manager.h`, `base/threading/hang_watcher.h`, `base/apple/mach_port_rendezvous.cc`, `base/apple/mach_port_rendezvous_mac.cc` — https://github.com/chromium/chromium/tree/d313930d0dbf5fd185cddb6132ff82bdbe4a6147
- [X1] XNU `xnu-12377.121.6` (2025-10-16, macOS 26's kernel, the latest published): `config/MASTER`, `osfmk/kern/task.c`, `osfmk/ipc/mach_msg.c`, `osfmk/ipc/ipc_kmsg.c`, `osfmk/arm/pmap/pmap.c` — https://github.com/apple-oss-distributions/xnu/tree/xnu-12377.121.6
- [W2] wpt.fyi's run summaries: Chrome 155, Firefox 158, Safari 27.0 and Servo 0.7.0 at `0f26972422`, and Ladybird 1.0 at `a26422aab3`, as the [standards baseline](../../standards.md) §6 calibrated them — https://wpt.fyi/api/runs?sha=0f26972422
- [W3] **Secondary.** W3Techs, "Usage statistics of character encodings for websites", dated 2026-10-22 — https://w3techs.com/technologies/overview/character_encoding
- This repository: [S16 results](../../research/s16-results.md), [S10 results](../../research/s10-results.md); S9's WPT checkout at `0f26972422`, with `css/reference/` added by `git sparse-checkout add` at the same revision.
