# Review Round 2: Risk Matrix

| | |
|---|---|
| **Status** | **Round 2, disposed** (2026-10-21): the owner accepted every finding as proposed ([the disposition log](../README.md#55-round-2s-dispositions)). This page summarizes; the [findings](03-findings.md) carry the evidence, and the [verification](02-verification.md) Round 1's changes |
| **Question** | Do Round 1's changes hold; what do the subsystems Round 1 deferred get wrong; and is the stopping rule met? |
| **See also** | [Verification of Round 1](02-verification.md) · [Findings](03-findings.md) · [Actions](04-actions.md) · [The review process](../README.md) |
| **Last updated** | 2026-10-21 |

## 1. Summary

**15 findings: no Critical, no High, 10 Medium, 5 Low.** Severities are defined in the [review process](../README.md#3-findings-and-severities).

**Round 1's changes hold.** All 22 accepted findings are resolved as the owner worded them ([verification](02-verification.md)). What they left behind:
- **stale statements:** in the entry documents, the handover and the decision map ([RV2-01](03-findings.md#rv2-01-the-entry-documents-predate-round-1)), and across designs, requirements, open questions and the status ([RV2-02](03-findings.md#rv2-02-statements-round-1s-changes-left-stale));
- **gaps the new topology made real:** which process configuration runs the suites ([RV2-03](03-findings.md#rv2-03-which-process-configuration-runs-the-conformance-suites)); what happens when a shared helper dies or hangs ([RV2-04](03-findings.md#rv2-04-when-a-shared-helper-dies-or-hangs)); two protocols the GPU process needs ([RV2-05](03-findings.md#rv2-05-two-protocols-the-gpu-process-needs-from-m2)); never blocking, beyond the browser ([RV2-13](03-findings.md#rv2-13-never-blocking-beyond-the-browser));
- **one regression:** accessibility's geometry ([RV2-06](03-findings.md#rv2-06-accessibility-geometry-after-the-gpu-process)).

**The deferred subsystems are sound in their architecture.** Their findings are gaps and local errors:
- **accessibility:** geometry, enablement, tests, notifications (RV2-06 to RV2-09);
- **storage and workers:** the storage key's ancestor bit, the File API, stale statements (RV2-10 to RV2-12);
- **IPC:** never blocking beyond the browser, and contradictions in the text (RV2-13, RV2-14);
- **style and layout:** four hooks the "complete model" should name (RV2-15);
- **the embedder API:** its issues belong to RV2-02, RV2-03 and RV2-04.

**Six findings block Milestone 1's plan,** four Medium and two Low. All are document changes; none needs a spike.

**The stopping rule is met** for the scope the owner commissioned (§4).

## 2. The matrix

Ranked by severity, then by the milestone each blocks.

| ID | Severity | Area | Finding | Blocks |
|---|---|---|---|---|
| [RV2-01](03-findings.md#rv2-01-the-entry-documents-predate-round-1) | Medium | Program | The handover and the decision map still describe the baseline before Round 1: one process at M1, compositing in the browser to M9, the old collector, cap and targets | M1 |
| [RV2-03](03-findings.md#rv2-03-which-process-configuration-runs-the-conformance-suites) | Medium | Testing | Virtual time exists only in single-process mode, and M1 runs real processes; no document says which runs WPT and the gate. EM5's defaults are unstated | M1 |
| [RV2-04](03-findings.md#rv2-04-when-a-shared-helper-dies-or-hangs) | Medium | Robustness | IR10 recovers renderers only. The network process's death is undesigned; no helper has a watchdog; Web Locks in the network process would break mutual exclusion across a restart | M1 (M2, M3, M8) |
| [RV2-13](03-findings.md#rv2-13-never-blocking-beyond-the-browser) | Medium | Security, IPC | IR5 and IP16 keep only the browser from blocking. The network and GPU processes serve every renderer, so one renderer could stall them, against SG11. IP16's "sends wait" contradicts EL1 | M1 (M2) |
| [RV2-05](03-findings.md#rv2-05-two-protocols-the-gpu-process-needs-from-m2) | Medium | Performance, graphics | PF4 charges GPU memory "to its renderer", with no mechanism, and S10 found tiles charged to the GPU process. Reusing tiles across processes is undesigned | M2 |
| [RV2-10](03-findings.md#rv2-10-the-storage-key-lacks-the-ancestor-chain-bit) | Medium | Security, storage | The storage key has no ancestor chain bit, so an A→B→A frame gets A's first-party storage; Chromium treats it as third-party. NW8's network key is at parity | M3 |
| [RV2-11](03-findings.md#rv2-11-the-file-api-has-no-design) | Medium | Standards, security | No design for `Blob`, `File`, `FileReader` or the `blob:` URL store the frame tree relies on. SG1, EM5 and IP15 disagree on the user's files | M3 |
| [RV2-06](03-findings.md#rv2-06-accessibility-geometry-after-the-gpu-process) | Medium | Accessibility | A regression: after RV1-02 the browser can't place every node "without asking". No hit-test policy; no positions within text runs | M4 |
| [RV2-07](03-findings.md#rv2-07-accessibility-enabled-by-any-client) | Medium | Accessibility, performance | Any client's first query enables full trees; Chrome sees 5–10% of users with accessibility on through password managers and antivirus tools | M4 |
| [RV2-08](03-findings.md#rv2-08-accessibility-tests-that-cant-run-as-written) | Medium | Accessibility, testing | WPT's role and name tests need WebDriver commands TS2 lacks; the platform tests need AppKit, a process-ID capability and a trusted runner | M4 |
| [RV2-02](03-findings.md#rv2-02-statements-round-1s-changes-left-stale) | Low | Documents | Statements Round 1's changes left stale in designs, requirements, open questions and the status | M1 |
| [RV2-14](03-findings.md#rv2-14-contradictions-in-ipcs-text) | Low | IPC | IP13 against PF21 on timeouts, IR6 against IP13 on the target; `iosurface` not barred from renderers; two interface families missing | M1 |
| [RV2-12](03-findings.md#rv2-12-stale-statements-in-storage) | Low | Storage | Quotas still "Step 6"; SG10 understates what the network process holds; quotas per site against keys | M3 |
| [RV2-09](03-findings.md#rv2-09-accessibility-notifications-announcements-and-inverted-colours) | Low | Accessibility | macOS's eight web notifications; live regions both in and "later" while Twitch's chat needs them; `ariaNotify`; inverted colours | M4 |
| [RV2-15](03-findings.md#rv2-15-style-hooks-from-interop-2026) | Low | Style, layout | Interop 2026's anchor positioning (a new cascade origin, applied during layout), `zoom` and `attr()` fit the architecture but touch rules the complete model says won't change | M4 |

**By area and severity:**

| Area | Medium | Low |
|---|---|---|
| Round 1's changes: documents | 1 (RV2-01) | 1 (RV2-02) |
| Processes, robustness and testing | 2 (RV2-03, RV2-04) | — |
| IPC | 1 (RV2-13) | 1 (RV2-14) |
| The GPU process | 1 (RV2-05) | — |
| Storage and workers | 2 (RV2-10, RV2-11) | 1 (RV2-12) |
| Accessibility | 3 (RV2-06–RV2-08) | 1 (RV2-09) |
| Style and layout | — | 1 (RV2-15) |

## 3. The commission, answered

1. **"Verify that the CP1–CP6 amendments completely resolve findings RV1-01 through RV1-22 without introducing regressions or contradictions."**
   - **Resolved, all 22,** in the binding documents ([verification](02-verification.md)).
   - **Not without residue:** ten leave stale statements elsewhere (RV2-01, RV2-02).
   - **One regression** (RV2-06) and **two contradictions** introduced (RV2-03, RV2-05). The new topology also makes real two gaps that predate Round 1 (RV2-04, RV2-13).
2. **The in-depth audit:**
   - **Accessibility (ADR-0014): sound,** with four gaps. The push model with a browser-side cache is the industry's design, and the bridge is rightly Axiom's own. But geometry broke with RV1-02, enablement is too eager, the tests can't run as written, and the notifications are incomplete (RV2-06 to RV2-09).
   - **Storage and workers (ADR-0013): sound,** with two gaps and stale text. Storage in the network process, with keys derived from endpoints, holds. The storage key misses the ancestor bit; the File API has no design; Web Locks sit where a restart breaks them (RV2-10, RV2-11, RV2-12, RV2-04).
   - **Style and layout (ST1–ST36, LY1–LY32): sound.** The cascade, invalidation, the nested lifecycle and LayoutNG's contract hold against Blink, Stylo, WebKit and Ladybird. One Low: four hooks the complete model should name (RV2-15), and a stale line in layout's §7 (RV2-02).
   - **The embedder API (EM1–EM12): sound.** Its defaults (EM5), test time (EM9), helper events (EM4) and signing (EM12) belong to RV2-03, RV2-04 and RV2-02.
   - **IPC (IP13–IP22): sound in transport and encoding.** Its never-block rules cover only the browser (RV2-13), and its text contradicts itself and PF21 in places (RV2-14).
3. **"Evaluate whether the stopping rule (zero Critical or High findings) is met":** yes, for the scope commissioned (§4).

## 4. The stopping rule

**The rule** ([the review process](../README.md#6-the-stopping-rule) §6): "The cycle ends when a round raises no Critical or High finding, after the previous round's accepted changes have been made and verified."

**Evaluation:**
- **Round 1's accepted changes are made:** CP1–CP6, the sweep, and spikes S10, S12 and S15, approved by the owner on 2026-10-21.
- **They are verified:** by this round ([verification](02-verification.md)).
- **Round 2 raises no Critical or High finding.**

**So the rule is met, and the cycle may end.** The owner then decides whether to authorize Milestone 1 (MA10).

**Why no finding is High.** Each Medium was tested against the High definition, "a decision whose later change would rework several crates, or certify a security level again":
- each fix is a rule or a protocol in one or two documents;
- built, each touches one crate or one interface;
- nothing is built yet, and L1 is first certified at M2, so none re-certifies a level.

The closest is RV2-13: as written, SG11's containment at L1 fails for the shared helpers. But the fix is a send policy in the IPC crate before that crate exists.

**Three qualifications, for the owner to weigh:**
1. **Scope.** The rule is met for the five areas the owner commissioned and for the verification. Round 1 also proposed the requirements' trace, the methods of S1–S4, the decision map's couplings and a re-check of dated facts ([Round 1's actions](../round-1/04-round-2-actions.md#6-round-2s-scope) §6). The owner's commission didn't include them, so this round didn't review them, and a finding there could still be High ([the review](../README.md#8-round-2s-coverage-and-limits) §8).
2. **Independence.** The same model family wrote the baseline and both rounds.
3. **The Medium findings that block M1** are settled "by the milestone it blocks", by the severity table: disposed, and the accepted changes made, before M1's plan starts.

## 5. What should be settled before Milestone 1 starts

Each needs the owner's disposition, then a change in documents. The detail is in the [actions](04-actions.md#1-before-milestone-1-starts).

| Finding | The decision |
|---|---|
| RV2-01 | Update the handover and the decision map; add the step to the review's process |
| RV2-02 | Correct the stale statements |
| RV2-03 | Which process configuration runs conformance and which runs time-dependent tests; EM5's defaults |
| RV2-04 | IR10 by process type; Web Locks in the browser; a watchdog for every helper |
| RV2-13 | The never-block rules for every helper that serves several peers; children's sends under EL1 |
| RV2-14 | IPC's text made consistent |
