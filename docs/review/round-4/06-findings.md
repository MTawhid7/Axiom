# Review Round 4: Findings

| | |
|---|---|
| **Status** | **Round 4, delivered** (2026-10-22), for the owner's dispositions. Nothing here changes the baseline until the owner decides each finding (the review's [§4](../README.md#4-how-a-round-runs)) |
| **Question** | Which of Round 3's claims hold when tested; where does the baseline build what serves few uses, or write what mature software already does; is efficiency first-class in its validation as well as its goals; and what is wrong in the areas no round has reviewed? |
| **Covers** | The baseline on `architecture-review` at `e71803b` (2026-10-21), Round 3 ([verification](02-verification.md)), the owner's three themes ([reuse and scope](03-reuse-and-scope.md), [efficiency](04-efficiency.md)) and the areas no round reviewed ([unreviewed areas](05-unreviewed-areas.md)), with spike S16 ([results](../../research/s16-results.md)) |
| **Summary** | [Risk matrix](01-risk-matrix.md) · [Verification of Round 3](02-verification.md) · [Actions](07-actions.md) |
| **Last updated** | 2026-10-22 |

**How to read it.** As in Rounds [1](../round-1/02-findings.md) and [2](../round-2/03-findings.md), each finding gives:
- its severity, its area, and the milestone it blocks (whose plan shouldn't start until the owner has decided it);
- the claim it challenges, quoted from the baseline with its rule ID;
- the evidence, labelled as the [research conventions](../../research/README.md) require: **sourced**, with a numbered source below; **observed**, on this Mac or in the repository, naming the file or command; **measured**, by a spike, named; **inference**, **hypothesis** or **background**;
- the failure mode, a counter-proposal, the cost of making the change later instead of now, and the rules it would amend.

Severities are defined in the [review process](../README.md#3-findings-and-severities). Every external source was read on 2026-10-22, from copies downloaded with `curl` at the revisions the sources name; the one secondary source is marked. SDK headers are those of the Command Line Tools' `MacOSX27.0.sdk`.

**The WPT counts** come from S9's checkout at `0f26972422`, MA4's revision, with the 25 files of `css/reference/` added at the same revision by `git sparse-checkout add`. The commands:
- `python3 -I tools/wpt-script-deps.py target/s9/wpt target/s9/MANIFEST-sparse-0f26972422.json --dir css/CSS2 --exclude css/CSS2/tables/ css/CSS2/inline-svg- --needs`, with `--list-free` and `--list-without script,xml` for the lists;
- `python3 -I tools/wpt-file-facts.py target/s9/wpt LIST`, a tool this round adds, for encodings and fonts;
- `python3 -I tools/wpt-pass-rates.py` over wpt.fyi's run summaries [W1], with `--tests LIST`.

---

## RV4-01. M1's CSS gate needs an XML parser M1 doesn't build

**Critical** · Conformance and HTML · Blocks M1 (built as written, its gate can't pass) · Challenges [MA4](../../milestone-1.md) and MA1; [HD9](../../architecture/html-and-dom.md) and the design's §8; the [roadmap](../../roadmap.md)'s M4

**The claim.**
- MA4: M1 "targets **the 5,652 that need none**" of script, at "**≥ 97% of the 5,652** (at least 5,483 tests)".
- MA1 builds, in `html`, "the parser with encoding sniffing and fragments": HTML's.
- HD's staging puts "the XML parser" in its "Next" row, and the roadmap's M4 lists "the preload scanner, the sanitizer and the XML parser".

**Evidence.**
- **Observed: 5,375 of the 5,652 tests are XHTML files** (`.xht`), 1 is an `.xml` file, and 276 are HTML.
- **Observed: WPT's server sends them as XML.** `wptserve`'s `constants.py` maps `"application/xhtml+xml": ["xht", "xhtm", "xhtml"]` [T1], so a browser parses them with its XML parser; HTML's parser never sees them.
- **Observed, with their references:** **5,423 of the 5,652 need XML,** because the test or a reference it reaches is an XML document. Only **229** need neither script nor XML.
- **Observed: so without XML, M1 could pass at most 229 tests, 4.1%,** where the gate asks for 5,483. On those 229, Chrome passes 91.3%, Firefox 95.6% and Safari 88.2% [W1].
- **Observed: no round caught it.** Round 1's RV1-01 counted script, with a tool that read only the test files; Round 3 asked about references and fonts, not media types ([verification](02-verification.md#rv3-06-references-and-script)).
- **Observed: the parser is small and already designed.** HD §3 chose "a non-validating parser for XML 1.0 and its namespaces, which is far smaller than HTML's (HD9)", with external entities never fetched and internal expansion bounded.

**Failure mode.** M1 builds everything MA1 lists, then can't run its own conformance target: about 96% of the selection fails at the parse, whatever the quality of style and layout, and the gate fails. The likeliest outcome is worse: the target is quietly changed at the tenth checkpoint (MA11), after the harness, style, layout and paint were built to it.

**Counter-proposal.**
1. **Bring HD9's XML parser forward to M1,** scoped to what M1 uses: XML 1.0 and its namespaces, non-validating, for documents served with an XML media type over HTTP and for `file:` URLs by extension. Well-formedness errors are fatal, as XML requires, and shown as an error document. XHTML's public identifiers resolve to HTML's named character references, compiled in, as HD9 already says. `DOMParser` and `XMLHttpRequest`'s XML wait for script (M3), and SVG documents for SVG (M4).
2. **Its oracle at M1:** the 5,423 tests themselves, a fuzz target added to MA7's list, and xml5ever as a differential reference (TS7), as HD9 plans.
3. **The alternatives,** for the owner:
   - **reuse an XML tokenizer** behind HD's interface, with Axiom's own tree builder. Parsing is G1's core and DP1 (a)'s, so this amends both. The candidates' dated facts are in [reuse and scope](03-reuse-and-scope.md#4-dated-facts-on-the-crates-axiom-would-use) §4: `quick-xml` had two denial-of-service advisories in 2026, both fixed; `xmlparser` hasn't been released since 2023-09-30. **Not recommended:** the parser is small, and a tokenizer is most of it;
   - **serve the `.xht` tests as HTML.** Rejected: that changes what they test, and the rates would no longer compare with wpt.fyi's (SB10);
   - **re-base M1's target on HTML tests only.** Rejected: 229 tests don't measure CSS 2.1.
4. **Recompute MA4's figures** at the pinned revision (MA3) with `--needs`, which now reports XML, web fonts, SVG, form controls and print, so that a capability missing from a milestone shows in its target.

**Cost now and later.** Now: an XML tokenizer and tree builder at M1, with a fuzz target. Later: discovering, ten checkpoints into M1, that the selection can't run.

**Amends.** MA1, MA2, MA4, MA7's fuzz targets and MA11's second checkpoint; HD §8 (the XML parser, for documents, in the first document milestone's row); the roadmap's M1 and M4; RQ40; ADR-0015 and ADR-0021, in their histories.

---

## RV4-02. The dependency policy ranks mature Rust below C

**High** · Dependencies · Blocks M1 (its text stack, and the decisions MA12 leaves to its plan: `smallvec`'s grade and JPEG's decoder) · Challenges [DP8](../../technology/dependency-policy.md) and DP10; charter principle 4 and DP4; [TX3](../../architecture/text-and-fonts.md) and text §4's `smallvec` risk

**The claim.**
- DP8: "**`ub-risk-3` and `ub-risk-4` are not admitted.** Axiom has no deadline that would justify a 'necessary evil' (charter principle 7)."
- DP10: C, C++ or assembly that parses untrusted input is "admitted only if all three hold: it runs in a sandboxed helper; no pure-Rust alternative meets the requirement, as a source or a measurement shows; an ADR accepts it."
- Text §4, on `smallvec`, which HarfRust and ICU4X's segmenter require: "If that version is still `ub-risk-3`, DP8 bars it, and the fix is upstream: a better-graded `smallvec`, or none."

**Evidence.**
- **Sourced: DP8 applies one grade's description to two.** Google's standards, as the policy quotes them (§2, [G1]): `ub-risk-3` is "a nontrivial, but not necessarily unacceptable risk", where projects "need to make judgement calls about where, when, and by whom these crates may be used"; `ub-risk-4` is "a risk of causing undefined behavior with typical use", a "necessary evil". DP8's reason quotes the second for both.
- **Observed: the ordering is inverted.** A C or C++ parser has a path in (DP10's three conditions); a pure-Rust crate graded `ub-risk-3` has none, in any process. Yet a C library is unsafe code throughout, and a `ub-risk-3` crate is safe Rust except for blocks whose risk has been judged nontrivial (inference). The policy has made this mistake once already: DP10's CP3 refinement says the stricter CP2 rule "was the reverse of the risk ordering it meant to express", since it excluded every Rust font parser while CoreText "remained admissible through the exception". The same reversal remains one grade higher.
- **Observed: where it lands** (the dated facts are in [reuse and scope](03-reuse-and-scope.md#4-dated-facts-on-the-crates-axiom-would-use) §4; crates.io [I1], RustSec [R1], the audits [V1]):

  | Crate | What needs it | Its grade | Its record |
  |---|---|---|---|
  | `smallvec` 1.16.3 | HarfRust and ICU4X's segmenter: M1's shaping and line breaking (TX1, TX2) | `ub-risk-3`, Google's only grade, at 1.11.0; later versions have `safe-to-deploy` audits without a grade | Five advisories, the last in 2021; 332 million recent downloads |
  | `encoding_rs` 0.8.42 | The Encoding Standard's codecs (TX3), Firefox's | `ub-risk-3` by Axiom's own audit (2026-10-21) and by Google's at 0.8.33 | No advisory in its history |
  | `zune-jpeg` 0.5.15 | JPEG (IM2) | `ub-risk-3` at 0.4.19 (Google) | No advisory. `jpeg-decoder`, the other candidate, hasn't been released since 2025-06-21 and has no grade |

- **Observed: the consequence at M1.** MA12 leaves `smallvec`'s grade to M1's plan. If Axiom's audit agrees with Google's, DP8 bars HarfRust and ICU4X's segmenter, and with them the text design ADR-0011 accepted; the fix the design names is upstream, outside the project's control. Meanwhile TX3 writes every encoding the Encoding Standard defines, and IM2 may write a JPEG decoder, because their mature Rust implementations are graded `ub-risk-3`.
- **Quoted: the owner's direction** (2026-10-22): "take full advantage of mature, battle-tested software wherever appropriate, provided it meets our security, compatibility, and architectural requirements", but "evaluate each dependency carefully for known vulnerabilities, legacy constraints, maintenance risks, and long-term suitability rather than assuming that maturity alone guarantees quality."
- **Observed: principle 4 asks reuse to prove itself, and DP4 narrows it:** "Reuse supporting infrastructure only with an explicit justification"; "A dependency must save substantial, specialized work". DP6's register entry already records most of what the owner listed: maintenance, advisory history, precedent, alternatives.

**Failure mode.** M1's plan grades `smallvec` at `ub-risk-3`; the text stack can't be built as designed, and the choices are a fork, Axiom's own shaper and segmenter, or a policy exception made under schedule pressure. Elsewhere Axiom writes and fuzzes codecs that mature, maintained crates already provide, while a C library would have been admissible.

**Counter-proposal.**
1. **Rank by risk.** Amend DP8 so that a `ub-risk-3` crate is admitted under the conditions DP10 sets for C:
   - it runs only in sandboxed helpers (DP9's `sandboxed` group, inherited);
   - no better-graded alternative meets the requirement, as a source or a measurement shows;
   - an ADR, or a named decision in its subsystem's ADR, accepts it, with a revisit condition;
   - and two conditions of Axiom's own: **its audit names each `unsafe` block and the invariant it relies on,** and **a fuzz target reaches those blocks** under AddressSanitizer (TS9).
   `ub-risk-4` stays barred, and the browser's `trusted` group keeps its bar (DP9).
2. **Apply it at M1's plan** (MA12): grade `smallvec` at the version adopted; admit it, if it is `ub-risk-3`, with HarfRust and ICU4X in the renderer. Decide TX3 and IM2 again under the amended rule: `encoding_rs` behind TX3's interface (RV4-08 weighs what M1 needs), and `zune-jpeg` against `jpeg-decoder` and Axiom's own.
3. **Move the burden of proof for supporting infrastructure** (DP2's list), which the owner's direction implies: a mature implementation that meets DP8–DP10 is preferred, unless a recorded reason (security, fit with Axiom's data model, efficiency, maintenance) favours writing our own. Principle 4's first sentence, "Implement the core ourselves", and G1's list stay as they are (presumed: the owner's default for this round).

**Cost now and later.** Now: amendments to DP8, DP4 and ADR-0010, and a decision at M1's plan that would have been made anyway. Later: the text stack redesigned or forked during M1, and codecs written for encodings that 1% of websites use (W3Techs, secondary [W2]) and that decide 38 of M1's 5,652 tests ([verification](02-verification.md#rv3-14-legacy-encodings)).

**Amends.** DP8, DP10's note, DP4; charter principle 4's wording, if the owner amends it; TX3; IM2; text §4's risk; MA12; ADR-0010, and ADR-0011 and ADR-0012 where their decisions change.

---

## RV4-03. A hung event loop goes unnoticed

**Medium** · Event loop, IPC and storage · Blocks M1 (PF19's watchdog and synchronous requests exist from M1; Web Locks from M3) · Challenges [PF19](../../performance.md), PF21; [EL §4.7](../../architecture/event-loop-and-threading.md); [IP13](../../architecture/ipc.md); [MA5](../../milestone-1.md)

**The claim.**
- PF19: "The browser pings each renderer's control thread once a second. **First deadline, 5 s** without acknowledging input: a non-modal offer to stop the page. **Second deadline, 30 s** ... the renderer is ended". Helpers: "a helper that doesn't answer for **5 s** is ended".
- EL §4.7: "The browser measures how long each renderer takes to acknowledge input."
- PF21: synchronous requests "have no timeout of their own: their answer is bounded by PF19, since the caller's thread is the one being watched"; IP13: "bounded by PF19's watchdog over the caller's thread".
- MA5: "no hang: no task past PF19's 5 s deadline".

**Evidence.**
- **Observed: every deadline starts from input.** A renderer whose event loop runs one task forever, with no input pending, is never noticed: its control thread, a different thread, answers every ping. A hidden tab never has input pending.
- **Observed: the watched thread isn't the waiting one.** IP13 sends synchronous requests to "the network process's storage or I/O thread (EL8)"; PF19 pings the network process's control thread. If the storage thread hangs, the control thread answers, every renderer waiting on `document.cookie` or `localStorage` waits for ever, and the network process is never recovered.
- **Observed: Web Locks.** SW10 releases a renderer's locks when it dies. A renderer whose script loops while holding a lock doesn't die, so every other agent of that storage key waits for ever on it (Round 3's §4.A, [RV3-12](02-verification.md#rv3-12-a-hung-lock-holder)).
- **Observed: MA5 already asks for what PF19 doesn't measure,** a task's duration.
- **Sourced: precedent for watching progress.** Chromium's `HangWatcher` "Monitors registered threads for hangs by inspecting their associated HangWatchStates for deadline overruns" [C1]. **Background:** Firefox offers to stop a script that has run past `dom.max_script_run_time`, whether or not input is waiting ([performance](../../performance.md) [F1]).
- **Sourced: precedent against revoking locks.** Chromium's lock manager releases a lock when its handle is dropped, "either explicitly by script or by process termination", or when it is stolen by request [C1].

**Failure mode.** A background tab's infinite loop holds a Web Lock, and the owner's other tabs of that site stop working with nothing on screen to say why. Or a network-process thread stalls, and every page's storage calls hang without the watchdog firing.

**Counter-proposal.**
1. **Watch progress, not just input** (EL §4.7, PF19). Each watched thread records when its current task began, one atomic store per task: every event loop; the network process's I/O and storage threads; from M2, the GPU process's threads. The control thread's reply to each ping carries the age of the oldest running task.
2. **The deadlines apply to that age as well as to unacknowledged input:**
   - **a renderer:** at 5 s the offer to stop the page, in its tab, shown when the tab is visible, and at once if another agent waits on a lock it holds; at 30 s, as now;
   - **a helper:** a thread past 5 s ends and recovers the helper (IR10), as an unanswered ping does now.
3. **Correct PF21 and IP13:** a synchronous request is bounded by the deadline of the thread that answers it, and by its caller's own task deadline.
4. **No lock revocation:** a hung holder is ended under item 2, which releases its locks (SW10), as Chromium's do.
5. **Tests:** MA5's "no task past 5 s" uses the same measure; MA7 adds a stalled storage thread, which must be detected and the network process recovered.

**Cost now and later.** Now: a timestamp per task, a field in the ping's reply, and two tests. Later: progress reporting retrofitted into every event loop and helper, and hangs found only when the owner clicks.

**Amends.** PF19, PF21; EL §4.7 and EL17's interrupt; IP13; SW10's note; MA5, MA7; ADR-0006 and ADR-0019, in their histories.

---

## RV4-04. A dead renderer's memory, charged to no one

**Medium** · Graphics and performance · Blocks M1 (the browser's compositor maps renderer memory from M1; the GPU process from M2) · Challenges [PL8](../../architecture/rendering-pipeline.md); [IR10](../../security/process-model.md); [PF4](../../performance.md)

**The claim.**
- PL8: renderer resources are "released when unused"; a renderer's pages stay "charged to the renderer that allocated them" (S10).
- IR10: "When a child dies, the browser discards everything that child reported".
- PF4: "a renderer's memory entries stay on its ledger".

**Evidence** (S16-Q1; [results](../../research/s16-results.md#3-s16-q1-memory-after-the-process-that-shared-it-dies) §3).
- **Measured: Round 3's prediction doesn't happen.** When a renderer that shared 64 MiB of images died, the compositor's charge didn't rise, in either placement or either way of dying: the browser's fell from 16.3 to 6.0 MiB, the GPU process's from 13.8 to 5.5 MiB.
- **Measured: but nothing was freed.** The 64 MiB stayed resident and dirty in the objects the compositor maps, their references falling from 3 to 2, and within 5 s they had often moved to the compressor (14 of 24 runs). **They were charged to no process** until the compositor unmapped them.
- **Measured: a CPU read charges.** Reading the pages from the compositor's CPU charged them to it: to both processes at once while the renderer lived (the compositor at 80.3 MiB, the renderer at 65.7), and to the compositor alone after the death (69.5 MiB).
- **Sourced: why.** XNU charges an internal page to each task whose page tables map it ([X1], `osfmk/arm/pmap/pmap.c`), so memory the GPU reads through a mapping, without the CPU, carries no charge.
- **Observed: no rule makes the compositor release a dead renderer's resources.** IR10 speaks of what the child "reported", PL8 of resources "unused", and GR13 of the GPU process's own death.

**Failure mode.** Each renderer crash, or each renderer ended under memory pressure, leaves its images resident and invisible to every budget (B1, B3, B6, PF4) until something else unmaps them: the memory pressure that ended it is relieved by nothing. A CPU fallback, such as a software path or a copy, double-counts every image while its renderer lives.

**Counter-proposal.**
1. **PL8, IR10: release at death.** On a renderer's death (IP6's notification, `EVFILT_PROC`), the compositor drops at once every resource that renderer registered: its Metal objects and its mappings. At M1 the browser's compositor thread does it; from M2 the GPU process, which the browser tells.
2. **PL8: no CPU reads of renderer memory.** Any CPU path over renderer pixels is a copy into the compositor's own memory, charged to its ledger and counted by PF4's accounting by page.
3. **PF4: count what is mapped.** The compositor's accounting by page includes the resident pages of the renderer memory it maps, read with `mach_vm_region` as S16 did, so a leak shows in the totals.
4. **A test at M1** (MA7): a renderer with registered images is killed; 1 s later, the compositor maps nothing of it.

**Cost now and later.** Now: a release path keyed by renderer, one rule and a test. Later: memory leaks that no budget sees, found by the owner as the Mac slows.

**Amends.** PL8; IR10; PF4; GR13's note; MA7; ADR-0007 and ADR-0012, in their histories.

---

## RV4-05. Rights in messages that are dropped, rejected or returned

**Medium** · IPC · Blocks M1 · Challenges [IP9](../../architecture/ipc.md), IP11 and IP16; [IP5](../../architecture/ipc.md)

**The claim.**
- IP9: "The `platform` layer receives the Mach message and hands the decoder plain bytes plus a typed list of handles."
- IP11: "`Violation::Ignore` means an expected race ...: the message is dropped." Undecodable messages and wrong handles are `Terminate`.
- IP16: "**The browser sends to children without blocking** (a zero send timeout ...). If a child's queue is full, the browser keeps a bounded backlog for that channel."
- None says what happens to the port rights in a message that is dropped, rejected or handed back.

**Evidence** (S16-Q2; [results](../../research/s16-results.md#4-s16-q2-port-rights) §4, identical in three runs).
- **Measured:** 1,000 messages from a sandboxed sender, each carrying its own right, dropped without `mach_msg_destroy`, added **1,000 entries** to the receiver's port space; destroyed, they added none, and the sender got 1,000 no-senders notifications.
- **Measured:** a zero-timeout send to a full queue failed with `MACH_SEND_TIMED_OUT` and handed the message back, **with its rights and one more reference to the destination,** both leaked unless destroyed. **Sourced:** XNU's pseudo-receive handles "the ports in the header ... as if they are in the body" ([X1], `ipc_kmsg.c`).
- **Measured:** messages still queued when their port is destroyed release their rights: the kernel cleans those.
- **Sourced: the cost of a leak.** A task's port table holds at most 305,834 entries on Macs' kernels, and a task that reaches it is ended by a fatal `EXC_RESOURCE` ([X1], `config/MASTER`, `osfmk/kern/task.c`; [RV3-02](02-verification.md#rv3-02-port-table-exhaustion)).

**Failure mode.** A compromised renderer sends the browser messages with rights that the browser will drop as races (stale frame IDs, IP11's `Ignore`) or reject; each leaves an entry, and after about 305,000 the kernel ends the browser, and every tab with it. A full queue under load leaks the browser's own rights, a few at a time.

**Counter-proposal.**
1. **IP9, IP11: a received message has an owner until every handle has one.** The `platform` layer moves each decoded handle into a typed owner whose drop deallocates it; a message that is dropped, rejected or can't be parsed is destroyed with `mach_msg_destroy` before anything else happens. The generated decoders take the handle list by value, so no path can forget it.
2. **IP16: a returned message is kept whole or destroyed.** The backlog keeps a failed send's returned message, with its names, to retry it; otherwise it is destroyed, releasing the destination's extra reference.
3. **A test against a lying peer** (V2, M1's IPC fuzz targets): a peer sends 100,000 messages carrying rights that the receiver must drop or reject; the receiver's port space ends where it began (`mach_port_space_basic_info`, as S16 read it).

**Cost now and later.** Now: an ownership rule in the `platform` layer and the generated decoders, and one test. Later: a remotely triggerable way to end the browser, found by fuzzing at M2 or not at all.

**Amends.** IP9, IP11, IP16; V2's cases in [testing](../../testing.md) TS14; ADR-0005, in its history.

---

## RV4-06. Efficiency is checked only at gates

**Medium** · Performance and testing · Blocks M1 (its plan sets the baselines) · Challenges [PF5](../../performance.md); [TS1](../../testing.md)'s sixth layer and TS13; [RQ5](../../requirements.md); [MA6](../../milestone-1.md)

**The claim.**
- PF5: every budget is "a benchmark or test with that threshold (TS13), run under PF1 at each milestone gate that touches it."
- TS1: performance benchmarks run "at each gate, and before merging a change that touches a hot path."
- RQ5: "Efficiency with measurable budgets on the target machine", traced to "Every gate".

**Evidence** (the full audit is in [efficiency](04-efficiency.md)).
- **Observed: efficiency has been a goal since 2026-10-16** (charter G4), with budgets for memory (B1–B7), frames, long tasks, pauses, idle wakeups, hidden pages and energy, the build (T1–T4) and the engine's own limits (PF13–PF24).
- **Observed: between gates, nothing measures.** A milestone's checkpoints can each add a little memory or time, within the budget, and the gate sees only the total.
- **Observed: budgets the owner named are missing:** start-up time; CPU time per page load; the bundle's size, and the profile's and the cache's on disk (NW10's cache is "bounded in size", with no size); bytes written to disk; background activity beyond idle wakeups; behaviour under memory pressure.
- **Measured: cheap, steady counters exist** (S16-Q3; [results](../../research/s16-results.md#5-s16-q3-counters) §5). Instructions retired varied 0.03–0.15% from run to run on fixed work, idle or with four spinning processes, while wall time varied 1.1–37%; disk bytes written were exact; peak footprints were exact idle. All are filled in for sandboxed children and read by the parent at exit, without privileges.
- **Measured: energy is a gate measurement:** it moved 8.5% under load on the same work, so it needs PF1's conditions.

**Failure mode.** Efficiency erodes checkpoint by checkpoint and is discovered at the gate, when the change that cost it is buried under a milestone's work; or a budget that doesn't exist (start-up, disk, writes) is never defended at all.

**Counter-proposal.**
1. **Validate in three tiers** ([efficiency](04-efficiency.md) §4):
   - **every commit:** deterministic counts as unit tests (allocations, sizes, item, message and wakeup counts, on EL24's virtual clock);
   - **every checkpoint:** instructions retired, peak footprint and bytes written on fixed workloads, against a baseline committed in the repository that **only ratchets down:** a regression past a stated tolerance fails the checkpoint unless its commit records the reason;
   - **every gate:** PF1's formal measurements, as now, with wall time and energy.
2. **New budgets, PF25–PF30,** each set at M1's plan as a hypothesis and measured at M1: start-up; CPU per corpus load; the bundle, profile and cache on disk; bytes written per hour, idle and browsing; background activity; memory pressure.
3. **An efficiency section in each milestone plan and design change:** what the change costs in memory, CPU, energy and disk, and how that is measured.
4. **RQ5** traced to all three tiers.

**Cost now and later.** Now: a measurement harness of rusage reads, a baseline file and a ratchet, built in M1's first checkpoints. Later: regressions whose cause is unknown, and budgets added after the code that misses them.

**Amends.** PF5, new PF25–PF30, PF2's tools; TS1, TS13; RQ5; MA6, MA11; the milestone plan's template (MS7, MS10); charter G4's note; ADR-0019, in its history.

---

## RV4-07. What the browser takes from the GPU process's surfaces

**Medium** · Graphics and security · Blocks M2 (the GPU process sends surfaces from M2) · Challenges [GR14](../../architecture/graphics.md) and GR12

**The claim.**
- GR14: the GPU process sends the browser "each tile's IOSurface as a Mach port (IP1)"; the browser keeps the CoreAnimation layer tree, "setting those IOSurfaces as layer contents".
- GR12: "The browser deletes each surface's port right as soon as it has looked the surface up".

**Evidence.**
- **Sourced: the model has precedent.** WebKit's UI process receives each backing store as a `MachSendRight`, turns it into a surface with `IOSurface::createFromSendRight`, caches it by identifier, and sets it as a layer's contents; it reads the surface's pixel format and its EDR headroom [K1]. Chromium's default instead hosts the GPU process's own CoreAnimation context, whose cost is a GPU sandbox that reaches the window server's render service among 19 global services [C1] ([RV3-01](02-verification.md#rv3-01-iosurface-lookups-in-the-browser)).
- **Measured: names that aren't surfaces fail safely.** `IOSurfaceLookupFromMachPort` returned nothing for a plain port, the null name, `MACH_PORT_DEAD` and a dead name, consuming none (S16 §4).
- **Measured: a surface carries its creator's values.** The GPU process attached 64 KiB under a key of its own and 4 KiB of arbitrary bytes as an ICC profile; the browser read both, and could remove them (S16 §3.3).
- **Observed: GR14 doesn't say** whether a right crosses once per surface or with every commit, or what the browser checks before using a surface. S10 sent a right only for each new surface.
- **Inference:** CoreAnimation colour-manages a surface by its colour tags, which S6 found it honours, so whatever the GPU process attaches as colour data would be parsed outside the GPU process's sandbox.

**Failure mode.** A compromised GPU process attaches malformed colour data that the browser's frameworks or the window server parse, or sends a surface whose size or format disagrees with the commit, and the browser uses it.

**Counter-proposal.**
1. **One right per surface lifetime:** a commit names a surface the browser has seen by ID, as S10 and WebKit do.
2. **Validate each new surface before use:** width, height, pixel format, bytes per row and allocation size, against the commit's declared values and GR13's limits; a mismatch is a violation (IP11).
3. **Strip, then tag:** the browser removes every attached value after lookup (`IOSurfaceRemoveAllValues`), and sets the colour space from the commit's validated field, from a fixed list (hypothesis: that setting works on a looked-up surface, tested at M2).
4. **Record the alternative:** CoreAnimation contexts hosted by ID, Chromium's default, as considered and rejected for the GPU sandbox's cost, in graphics §9.

**Cost now and later.** Now: four sentences in GR14 and a test at M2. Later: a parsing surface in the browser that L3's certification (M6) would have to find.

**Amends.** GR14, GR12; graphics §9; IP11's list; ADR-0012, in its history.

---

## RV4-08. M1's scope against value

**Medium** · Program · Blocks M1 · Challenges [MA1](../../milestone-1.md); [NW §8](../../architecture/networking.md); [TX3](../../architecture/text-and-fonts.md); [HD §8](../../architecture/html-and-dom.md); [input §5](../../architecture/input-and-accessibility.md)

**The claim.** MA1 builds, among the rest:
- `net`: "URLs, `file:`, HTTP/1.1 and HTTP/2 over rustls";
- `text`: "the encodings", which TX3 defines as "every encoding" the Encoding Standard defines;
- input: "the mirrors and caches exist" at L0 (input §5), so IN7's IME mirror for web content;
- and HD stages the preload scanner at M4, a milestone after script arrives.

**Evidence** ([reuse and scope](03-reuse-and-scope.md) §5).
- **HTTP/2:** no criterion of M1 needs it. The harness speaks "plain HTTP/1.1 to wptrunner's server on the loopback" (MA11); the corpus copies are local; and S7's HTTP/1.1 client fetched `www.wikipedia.org` (observed, S7 §3.2). Building it means framing, HPACK, flow control, priorities and coalescing (NW4), and two fuzz targets (MA7). Its value is fewer connections on live pages, which M2's disk cache and cookie jar would measure with it (inference).
- **Encodings:** **38 of the 5,652 tests, 0.7%,** declare an encoding other than UTF-8, UTF-16 and windows-1252, in the test or a stylesheet it loads; W3Techs puts UTF-8 at 99.1% of websites and windows-1252's labels at 1.0% (secondary, [W2]) ([verification](02-verification.md#rv3-14-legacy-encodings)). Every one of TX3's decoders also needs its index tables, tests and a fuzz target.
- **Text entry into pages:** no criterion of M1 types into a page: MA5's checks load, read and follow links, and only 3 of the 5,652 tests use form controls. The rendering of form controls is M4's (the roadmap). The shell's own text input is EM11's validation, which stays.
- **The preload scanner** looks ahead while the parser waits for a script; before script there is nothing to wait for, so it first pays at M3 (inference).
- **Rejected: Round 3's M1-A** ("Single-process, headless embedder, local `file:` only") can't run WPT, whose tests load `/fonts/ahem.css` and `/css/support/` from WPT's server, and reverses RV1-07 ([RV3-15](02-verification.md#rv3-15-staging-m1)).

**Failure mode.** M1 spends checkpoints and fuzzing time on protocols, codecs and editing that none of its criteria exercise, and their defects surface only when a later milestone first uses them.

**Counter-proposal.** Stage by value, keeping every interface (MS4) and the complete model (principle 7):
1. **HTTP/2 and HPACK to M2,** with the disk cache and cookie jar; M1's `net` keeps HTTP/1.1 over rustls.
2. **Encodings at M1:** UTF-8, UTF-16 (both byte orders) and windows-1252 with all its labels, behind TX3's interface; **the rest at M4,** with web fonts and the next document stage, or **all of them at M1 through `encoding_rs`,** if the owner admits it under RV4-02's amended DP8 (recommended, if RV4-02 is accepted).
3. **Text entry into web content to M4,** with form controls: IN7's mirror for pages and editing in text controls (HD26, HD28). M1 keeps forms' model and submission (HD26; MA7's password rule).
4. **The preload scanner to M3,** with script.
5. **Kept, with their reasons:** tables (the corpus's infoboxes, LY10), DNS-SD and one wait (S7), HSTS and mixed content (G1's security policies), the objc2 validation (EM11, which retires OQ-11's risk early), the Rust path for the test fonts (TX21).

**Cost now and later.** Now: rows moved between milestones, with no interface changed. Later: features carried untested for a milestone or two, which is what M1 would carry anyway.

**Amends.** MA1, MA2, MA7's fuzz list, MA11's eighth checkpoint; NW §8; TX3's staging; HD §8; input §5; the roadmap's M1–M4; ADRs 0011, 0013, 0015 and 0021, in their histories.

---

## RV4-09. M1's CSS target asks for the long tail first

**Medium** · Conformance · Blocks M1 · Challenges [MA4](../../milestone-1.md)'s target; SB11's use for M1

**The claim.** MA4: "**≥ 97% of the 5,652**": "the median of Chrome (98.2%), Firefox (97.8%) and Safari (96.4%) on those same tests", treating CSS 2.1 as an area M1 completes (SB11).

**Evidence** (wpt.fyi's runs at `0f26972422` [W1], over the 5,652).
- **Observed: the large chapters are near 100% everywhere,** and the hard ones are small:

  | Directory | Tests | Chrome | Firefox | Safari | Servo | Ladybird | Median |
  |---|---|---|---|---|---|---|---|
  | `normal-flow` | 736 | 99.3 | 99.3 | 97.1 | 99.3 | 89.0 | 99.3 |
  | `margin-padding-clear` | 682 | 99.3 | 99.1 | 99.1 | 98.8 | 96.6 | 99.1 |
  | `positioning` | 513 | 99.8 | 99.8 | 97.1 | 99.8 | 96.7 | 99.8 |
  | `borders` | 504 | 100 | 100 | 97.4 | 100 | 96.2 | 100 |
  | `selectors` | 468 | 97.9 | 90.6 | 97.4 | 95.3 | 95.5 | 97.4 |
  | `text` | 388 | 98.2 | 97.9 | 97.4 | 82.7 | 93.6 | 97.9 |
  | `backgrounds` | 333 | 99.7 | 99.7 | 90.1 | 96.7 | 97.0 | 99.7 |
  | `floats` | 103 | 88.3 | 90.3 | 81.6 | 96.1 | 76.7 | 88.3 |
  | `visudet` | 37 | 64.9 | 89.2 | 89.2 | 64.9 | 54.1 | 89.2 |
  | `css21-errata` | 9 | 33.3 | 66.7 | 33.3 | 66.7 | 66.7 | 33.3 |

  The full table, every directory, is in [reuse and scope](03-reuse-and-scope.md#53-m1s-css-target) §5.3. The median is of Chrome, Firefox and Safari.
- **Observed: Servo passes 93.7% and Ladybird 91.4%** of the 5,652, after years of work; 97% asks M1 for more than either has.
- **Observed: SB11 already allows the alternative:** "An area **started** at a milestone may have a lower, interim target, stated with its reason in the milestone's plan." CSS 2.1 is completed at M3 in any case, when the 196 tests with script join and "the full target, over all 5,848, applies" (MA4).
- **Inference:** the last few percent are edge cases, such as errata and `visudet`'s, that Wikipedia and MDN don't depend on, and that M2 and M3, which exist to validate the sandbox and script, don't need either. Round 3 called the target "unviable"; with no deadline (OQ-1) it is reachable, but it puts CSS 2.1's long tail ahead of everything after M1.

**Failure mode.** M1 stays open for the last 7% of CSS 2.1's edge cases while the sandbox, script and the rest of the roadmap wait; or the target is lowered at the gate, with less evidence than now.

**Counter-proposal,** three options for the owner:
1. **Recommended: CSS 2.1 as an area M1 starts** (SB11). **≥ 90% of the 5,652** (5,087 tests), and **in each directory of 100 tests or more, no more than 10 points below its median,** so no chapter is skipped; **the median target at M3,** over all 5,848, as MA4 already plans.
2. **Keep ≥ 97%** at M1, as now.
3. **Round 3's ≥ 88% over four directories:** not recommended, because it leaves chapters the corpus uses, such as `text` and `lists`, untargeted.

**Cost now and later.** Now: MA4's row. Later: the same decision made at M1's gate under pressure.

**Amends.** MA4 (and its RV1-01 note); the roadmap's M1 and M3; ADR-0021, in its history.

---

## RV4-10. The tile protocol's writer side

**Low** · Graphics · Blocks M2 · Challenges [GR12](../../architecture/graphics.md)

**The claim.** GR12 (Round 2): "a spare is drawn into only when no process reads it", checked with `IOSurfaceIsInUse`. It says nothing of when a tile's drawing must be complete before the browser shows it.

**Evidence.**
- **Sourced: the synchronization is the writer's.** Chromium: "We have to wait for pending work to be scheduled on the GPU for IOSurface synchronization by the kernel e.g. using waitUntilScheduled on Metal or glFlush on OpenGL", before CoreAnimation reads a surface; its shared events throttle its own frames and don't cross processes [C1] ([RV3-04](02-verification.md#rv3-04-iosurfaceisinuse-and-the-gpus-timeline)).
- **Observed:** S10's GPU process waited for each command buffer to complete before committing, a stricter rule that costs latency.
- **Sourced:** Chromium notes that "IOSurfaceIsInUse() will always return true if the IOSurface is wrapped in a CVPixelBuffer", so it ignores the check for video frames [C1].

**Failure mode.** A GPU process that commits before its commands are scheduled shows half-drawn tiles; one that waits for completion adds a frame of latency; video frames at M7 never leave the pool.

**Counter-proposal.** GR12 states the writer's rule: **a tile is committed once the command buffer that drew it is scheduled** (`waitUntilScheduled`), and frames in flight are limited by completion; M2 measures it against S10's wait for completion. Video frames wrapped in `CVPixelBuffer`s are pooled by their own release, not by `IOSurfaceIsInUse` (M7).

**Cost now and later.** Two sentences now; at M2, a flicker or a frame of latency to diagnose.

**Amends.** GR12; the media design's video frames; ADR-0012, in its history.

---

## RV4-11. Channels in one process, and stress in many

**Low** · IPC and testing · Blocks M1 · Challenges [IP2](../../architecture/ipc.md); [TS8](../../testing.md); [MA7](../../milestone-1.md), MA8

**The claim.**
- IP2: "a channel is an in-memory queue with the same API (R6)", used "with the deterministic scheduler (EL24)".
- MA7 and MA8 test helpers killed and stalled, a renderer that stops receiving, and parsing in random chunks.

**Evidence** ([RV3-10](02-verification.md#rv3-10-real-processes-concurrently), [RV3-11](02-verification.md#rv3-11-in-memory-queues-against-mach)).
- **Observed:** "the same API" doesn't promise the same limits or failures. IP16's backlog depends on a zero-timeout send that fails at a full queue, returns its message and later reports that sending is possible again; an unbounded in-memory queue never exercises any of that.
- **Observed:** no test varies timing between real processes on purpose; MA8's chunking runs in the parser's harness, not at the network process's socket.

**Failure mode.** Single-process tests pass while the backlog and its returned messages (RV4-05) are untested until real queues fill under load.

**Counter-proposal.**
1. **IP2: in-memory channels model Mach's behaviour:** each endpoint's queue limit (PF21), a failing zero-timeout send that returns the message, send-possible notifications, and handles' lifetimes.
2. **A stress configuration** in real processes for M1's gate: the corpus loads and the `css/CSS2` selection run once more with random delays injected at the IPC layer of a test build, and with network data delivered in random chunks at the socket; results must equal the unstressed run's.

**Cost now and later.** Now: a model of Mach's queue in the test transport, and one configuration. Later: races found in daily use.

**Amends.** IP2; TS8; MA7, MA8; ADR-0005, in its history.

---

## RV4-12. The rendezvous checks a PID, not a process

**Low** · IPC and security · Blocks M2 (audit tokens arrive at M2, MA2) · Challenges [IP4](../../architecture/ipc.md); ADR-0004 D2

**The claim.** IP4: "The browser checks that the audit token's PID is the child it spawned (R3), and pins that token to the channel."

**Evidence** ([RV3-16](02-verification.md#rv3-16-the-bootstrap-rendezvous)).
- **Sourced: Chromium uses a rendezvous name too,** `"%s.MachPortRendezvousServer.%d"`, checks each sender's PID from the kernel's audit token, gives ports only to PIDs it registered, and by default validates each client's code-signing requirement [C1]. Round 3's claim that the engines inherit ports instead is wrong.
- **Observed:** local malware is outside the [threat model](../../security/threat-model.md) (§4, "Not adversaries"), so the name's global visibility isn't an exposure in itself.
- **Inference:** a PID can be reused. The audit token also carries the PID's version, which tells a new process from an old one with the same PID. Nothing in IP4 refuses a second hello from a child that already has its channel.

**Failure mode.** Narrow: a process that inherits a dead helper's PID, or a compromised helper saying hello again, is matched to a channel it shouldn't have.

**Counter-proposal.** IP4: **pin the PID and its version,** and **accept one hello per child spawned;** a second is a violation. **Consider the code requirement** S15's stable identity makes possible, as Chromium validates by default, at M2 with audit tokens.

**Cost now and later.** A few lines now; later, a hardening pass at L1.

**Amends.** IP4; ADR-0004 D2's note; ADR-0005, in its history.

---

## RV4-13. Spikes measured on an earlier macOS build

**Low** · Method · Blocks M1 (its plan relies on S5–S7) · Challenges [TS16](../../testing.md)

**The claim.** TS16: "When it changes, the checks that depend on the system run again", S4's, S5's, S6's and S7's included.

**Evidence** ([unreviewed areas](05-unreviewed-areas.md#2-the-methods-of-s1s4-and-the-spikes-builds) §2).
- **Observed:** S4–S7's results record macOS 26B5112a; S10, S12 and S16 record 26B118. S15 re-ran S4's probes on 2026-10-21, without recording the build. S5's text comparison, S6's compositing and S7's network checks haven't run on 26B118.
- **Observed:** S1–S3, S8, S9 and S15 record "macOS 27.2" without the build.

**Failure mode.** M1's plan builds on S5's CoreText calibration (TX12, TX13), S6's frame figures and S7's sandbox rules (NW17) as measured on an older system.

**Counter-proposal.** Re-run S5's, S6's and S7's existing scripts once on the current build before M1's plan adopts their figures (the owner's authorization, since they are spikes); every results document records the build from now on, as PF1 already asks of formal runs.

**Cost now and later.** About an hour of runs now; later, a calibration that disagrees with the system the browser runs on.

**Amends.** TS16's record; the spikes' README; the status.

---

## RV4-14. Stale statements, and couplings the decision map lacks

**Low** · Program · Blocks M1 (the entry documents, RV2-01's rule) · Challenges [requirements](../../requirements.md), [invalidation](../../architecture/invalidation.md) §8, [networking](../../architecture/networking.md) §8, [HTML and the DOM](../../architecture/html-and-dom.md) §10, the [decision map](../../decision-map.md)

**Evidence** (observed; the details are in [unreviewed areas](05-unreviewed-areas.md)).
- **Requirements:** RQ54 and RQ56 place B2 and B4 at M2, which MA6 measures at M1 (RV1-07); RQ61 places hangs and IPC limits at M2, which MA7 tests at M1 (RV2-04, RV2-13); RQ30 waits for "ThreadSanitizer when its runtime works", which S12 found (RV1-11); RQ40 changes with RV4-01.
- **Invalidation §8:** its first and third questions were settled at Step 6 (PF19, PF20) and Step 5 (I2's damage-limited tiles); its second ("Step 5") has passed its step unsettled.
- **Networking §8:** "L1: The network process sandboxed", sandboxed from M1 since RV1-07.
- **HTML §10, question 3:** the parser's time budget and read-ahead, "Step 6, on the corpus", answered by PF23.
- **The decision map:** §2 item 3 still plans to "split processes later"; §4 item 1 keeps "a single-process debugging mode", which RV1-07 narrowed to tests, fuzzing and Miri.
- **Couplings this round found and the map doesn't show:** P5 with X1 and X2 (the XML parser and M1's gate, RV4-01); X4 with I1 and I4 (the dependency policy and the text and image stacks, RV4-02); P1 with F3 (hang detection and the helpers' watchdogs, RV4-03); I2 with X3 (shared memory and the budgets' accounting, RV4-04).

**Counter-proposal.** Correct each, and add the four couplings to the decision map's §2, in the checkpoint that makes the related change (RV2-01's rule).

**Cost now and later.** Text now; later, a reader misled at M1's plan.

**Amends.** RQ30, RQ40, RQ54, RQ56, RQ61; invalidation §8; networking §8; HTML §10; the decision map's §2 and §4.

---

## RV4-15. An advisory in the spikes' lock

**Low** · Dependencies · Blocks nothing (the spikes are exempt, DP23) · Challenges [TS15](../../testing.md)'s routine, which starts with the tools (DP22)

**Evidence.**
- **Sourced:** `rustls-webpki` 0.103.15, in `Cargo.lock` through S7's `rustls`, is affected by RUSTSEC-2026-0382 (2026-10-20): "Certificate verification may accept distrusted intermediates"; 0.103.16 fixes it [R1].
- **Observed:** Axiom's design verifies certificates with the system (NW, I3), so the engine's exposure would be smaller than a webpki verifier's; the spikes are throwaway (DP23).
- **Observed:** of the other 2026 advisories against crates on Axiom's path (`rustls`, `aws-lc-sys`, `quinn-proto`, `ruzstd`, `h2`, `quick-xml`), the lock holds fixed versions or none of the crate.

**Counter-proposal.** Update `rustls-webpki` in the lock with the next spike commit, and run TS15's weekly check by hand at each review gate until DP22's tools are authorized.

**Amends.** The lock; TS15's interim practice.

---

## RV4-16. Child frames' geometry after the GPU process

**Low** · Invalidation and frame tree · Blocks M2 · Challenges [IV13](../../architecture/invalidation.md)

**The claim.** IV13: "The new rectangle goes to the browser in the parent's display-list update (PL7). The browser sends the child its new viewport".

**Evidence.**
- **Observed:** from M2 display lists go to the GPU process, not the browser (RV1-02; PL7–PL9). PL16 was amended so that the GPU process forwards hit-test regions with the layer commit, and RV2-06 did the same for accessibility's bounds; IV13 wasn't.

**Counter-proposal.** IV13: from M2 the GPU process forwards each child frame's token and rectangle with the layer commit, as PL16's regions, and the browser checks it against the parent frame's bounds before sending the child its viewport; at M1 the compositor thread does the same.

**Amends.** IV13; FT12's note; ADR-0007, in its history.

---

## RV4-17. M2's cache and cookie jar wait on a storage engine chosen at M3

**Low** · Networking and storage · Blocks M2 · Challenges the [roadmap](../../roadmap.md)'s M2 and its decision table; [storage](../../architecture/storage-and-workers.md) §8 and SW13

**The claim.**
- The roadmap's M2: "the network's first stage completes: the partitioned disk cache and cookie jar, CORS"; MA2 leaves "the cookie jar and the disk cache (M2)".
- The roadmap's decisions: "**The storage engine** (SW2): `redb` or SQLite | Open | M3 | The crash tests of the first storage milestone".
- Storage §8: "First storage milestone | `localStorage`, `sessionStorage`, the cookie jar and the cache on the engine; the crash tests that decide it; with script at M3".

**Evidence.** **Observed:** M2 builds two stores whose engine, and the crash tests that choose it, the documents place at M3. NW10 stores the cache "in a format CP7 chooses together with storage".

**Failure mode.** M2 either builds its cache and cookie jar on a provisional store, migrated at M3, or makes SW2's decision without SW13's crash tests.

**Counter-proposal.** Move SW2's decision and SW13's crash tests for the cache and the cookie jar to M2's plan, where those stores first exist; M3 adds Web Storage on the same engine.

**Amends.** The roadmap's M2, M3 and its decision table; storage §8 and SW13; NW10's note.

---

## What holds, by area

Credited so that later rounds don't reopen them, each checked in this round:

| Area | What holds |
|---|---|
| **Shared memory** (S16-Q1) | Renderer images sampled in place stay charged to the renderer while it lives, and the compositor isn't charged for them unless its CPU reads them: PL8's no-copy path is sound, and Round 3's "ghost memory" doesn't happen |
| **Surfaces to the browser** | GR14's handoff of IOSurfaces by Mach right is WebKit's model; the browser deleting each right after lookup (GR12) leaves no port entry behind (S16 §3.3) |
| **IPC** | Mach as transport, typed handles out of the bytes (IP9), zero-timeout sends (IP16) and bounded queues all hold; the kernel releases rights in messages queued at a port's destruction, so only received and returned messages need Axiom's care |
| **Rendezvous** | A bootstrap name checked by the audit token's PID is Chromium's design too (IP4) |
| **Web Locks** | Arbitration in the browser (SW10), released on death, matches Chromium's lock manager; revocation isn't needed once hangs are detected (RV4-03) |
| **The harness** | References carry no script; fonts are awaited; Ahem renders through the same path in test and reference; no test needs dynamic pseudo-classes: Round 3's §2.B concerns don't hold beyond the tool |
| **The storage key** | One ancestor bit, at parity with Chromium, opens no channel between embedders |
| **Efficiency** | A goal since the charter (G4), with budgets on this Mac for memory, frames, pauses, idle and energy, and spikes that measured before deciding |

---

## Sources

All read on 2026-10-22. Repository documents are linked where they are cited, and observations name their files or commands.

- [C1] Chromium at `d313930d0dbf` (2026-10-22): `ui/base/cocoa/remote_layer_api.mm`, `ui/accelerated_widget_mac/display_ca_layer_tree.mm` and `ca_layer_tree_coordinator.mm`, `sandbox/policy/mac/gpu.sb` and `common.sb`, `components/viz/service/display_embedder/software_output_device_mac.cc`, `gpu/command_buffer/service/shared_image/iosurface_image_backing.mm`, `content/browser/locks/lock_manager.h`, `base/threading/hang_watcher.h`, `base/apple/mach_port_rendezvous.cc` and `mach_port_rendezvous_mac.cc` — https://github.com/chromium/chromium/tree/d313930d0dbf5fd185cddb6132ff82bdbe4a6147
- [K1] WebKit at `51e7f3eff5b2` (2026-10-22): `Source/WebKit/Shared/RemoteLayerTree/RemoteLayerBackingStore.mm` — https://github.com/WebKit/WebKit/tree/51e7f3eff5b249aa01d65cc1eb647aba6b1b9a46
- [X1] XNU `xnu-12377.121.6` (2025-10-16, macOS 26's kernel, the latest published): `config/MASTER`, `osfmk/kern/task.c`, `osfmk/ipc/ipc_entry.c`, `ipc_kmsg.c` and `mach_msg.c`, `osfmk/arm/pmap/pmap.c` — https://github.com/apple-oss-distributions/xnu/tree/xnu-12377.121.6
- [T1] web-platform-tests at `0f26972422` (2026-10-18): `css/CSS2`, `css/reference`, `tools/wptserve/wptserve/constants.py` — https://github.com/web-platform-tests/wpt/tree/0f26972422b15f587e2afbd38dbb6ea2397fded0
- [W1] wpt.fyi's run summaries at `0f26972422`: Chrome, Firefox, Safari and Servo, and Ladybird's at `a26422aab3`, the runs the [standards baseline](../../standards.md) §6 calibrated — https://wpt.fyi/api/runs?sha=0f26972422
- [W2] **Secondary.** W3Techs, "Usage statistics of character encodings for websites", dated 2026-10-22 — https://w3techs.com/technologies/overview/character_encoding
- [R1] RustSec's advisory database at `4b587f0099` (2026-10-21) — https://github.com/rustsec/advisory-db/tree/4b587f00996ebe5c05a73eaef105ef241e876414
- [V1] `cargo-vet` audits, at `main` on 2026-10-22, from cargo-vet's registry: Google's — https://raw.githubusercontent.com/google/supply-chain/main/audits.toml ; Mozilla's — https://raw.githubusercontent.com/mozilla/supply-chain/main/audits.toml ; Chromium's — https://raw.githubusercontent.com/chromium/chromium/main/third_party/rust/chromium_crates_io/supply-chain/audits.toml ; the Bytecode Alliance's — https://raw.githubusercontent.com/bytecodealliance/wasmtime/main/supply-chain/audits.toml ; Embark's — https://raw.githubusercontent.com/EmbarkStudios/rust-ecosystem/main/audits.toml ; ISRG's — https://raw.githubusercontent.com/divviup/libprio-rs/main/supply-chain/audits.toml
- [I1] crates.io's API, read by `tools/crate-facts.py` — https://crates.io/api/v1/crates/
- **Local:** the Command Line Tools' `MacOSX27.0.sdk` (`IOSurface/IOSurfaceRef.h`, `mach/mach.h`, `sys/resource.h`); spike S16's records under `target/s16-results/20261022-114108/`. The downloaded copies of the sources above are kept under `scratch/review-r4/` and `target/review-r4/` (not committed).
