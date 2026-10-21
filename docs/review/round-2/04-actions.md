# Review Round 2: Actions

| | |
|---|---|
| **Status** | **Round 2, for the owner's disposition** (2026-10-21). If the owner rejects or defers a finding, its rows fall away, and the [disposition log](../README.md#5-the-disposition-log) records why |
| **Question** | Which changes does each finding need, and before which milestone; how are they made; and, with the stopping rule met, what comes next? |
| **See also** | [Risk matrix](01-risk-matrix.md) · [Verification of Round 1](02-verification.md) · [Findings](03-findings.md) · [The review process](../README.md) |
| **Last updated** | 2026-10-21 |

Each row names its finding, the change proposed, the documents and rules it touches, and when the change is done. **No finding needs a spike:** every change is to documents, and the evidence that RV2-05 and RV2-07 call for is gathered at M2 and M4, inside those milestones.

## 1. Before Milestone 1 starts

| Finding | Change | Documents and rules | Done when |
|---|---|---|---|
| [RV2-01](03-findings.md#rv2-01-the-entry-documents-predate-round-1) | The handover brought up to Rounds 1 and 2; the decision map's rows annotated; a step in the review's process for the entry documents | The [handover](../../handover-phase-0.md) §4.6, §4.14, §6, §7, §9–§11; the [decision map](../../decision-map.md) F2, F3, P7, X1–X4 and its Review row; the review's [README](../README.md#4-how-a-round-runs) §4 | A search of the handover for each finding's old wording finds it only beside an "as amended" note |
| [RV2-02](03-findings.md#rv2-02-statements-round-1s-changes-left-stale) | Each stale statement corrected; ADR-0009's decision 7 amended through its history | The documents in the finding's table | The searches of the [verification](02-verification.md#2-method) §2 find no unmarked hit in a living document |
| [RV2-03](03-findings.md#rv2-03-which-process-configuration-runs-the-conformance-suites) | The owner's choice of process configuration for conformance and for time-dependent tests, with a cross-check; EM5's defaults listed | [Testing](../../testing.md) TS2, TS8; [Milestone 1](../../milestone-1.md) MA4, MA8; [EM5](../../architecture/embedder-api.md), EM9; [EL24](../../architecture/event-loop-and-threading.md) | TS2 says which configuration runs each suite, and MA8's criterion matches it |
| [RV2-04](03-findings.md#rv2-04-when-a-shared-helper-dies-or-hangs) | IR10 by process type; Web Locks arbitrated in the browser; PF19's watchdog for every helper; EM4's events for helpers | [IR10](../../security/process-model.md) and the change record; GR13; EM4; [PF19](../../performance.md); [SW3, SW10, SW13](../../architecture/storage-and-workers.md); the histories of ADR-0004 and ADR-0013 | IR10 has a row per process type, and M1's plan can test the network process's death |
| [RV2-13](03-findings.md#rv2-13-never-blocking-beyond-the-browser) | Never blocking for every process that serves several peers; children's sends under EL1; a stalled-renderer test | IR5 and the change record; [IP16](../../architecture/ipc.md); PF21; SG11's note; ADR-0005's history | IP16 names every serving process, and Milestone 1's tests include the stalled renderer |
| [RV2-14](03-findings.md#rv2-14-contradictions-in-ipcs-text) | IP13 takes PF21's rule; IR6 names the network process; the service-worker case; `iosurface` barred from renderers' interfaces; two families; IPC's §8 closed | IP7, IP13, §4.12, §8; IR6 | The IPC design and the process model agree, read side by side |

## 2. Before Milestone 2

| Finding | Change | Documents and rules | Done when |
|---|---|---|---|
| [RV2-05](03-findings.md#rv2-05-two-protocols-the-gpu-process-needs-from-m2) | Accounting by page in the GPU process; a protocol for reusing tile surfaces across processes, tested with a window attached; IPC's open question 4 closed | [PF4](../../performance.md); [GR8, GR12](../../architecture/graphics.md); PL8; IPC §8; the [roadmap](../../roadmap.md)'s M2 | PF4 names the mechanism, and M2's tests include reuse with a window |
| RV2-04, RV2-13 | Their GPU-process parts: GR13 referenced from IR10; the watchdog; never blocking | As above | M2's plan tests the GPU process's death, hang and a stalled renderer |

## 3. Before Milestone 3

| Finding | Change | Documents and rules | Done when |
|---|---|---|---|
| [RV2-10](03-findings.md#rv2-10-the-storage-key-lacks-the-ancestor-chain-bit) | The storage key gains the ancestor chain bit; everything keyed by it uses the whole key; NW8's note; an A→B→A test. NW11's partitioned cookies checked for the same case at M2 | [SW1](../../architecture/storage-and-workers.md), SW9–SW11, SW13; [SI3](../../security/process-model.md) (a); [NW8](../../architecture/networking.md); ADR-0013's history | SW1 states the key, and SW13 lists the test |
| [RV2-11](03-findings.md#rv2-11-the-file-api-has-no-design) | A File API section: blob data, the `blob:` URL store keyed by the whole key, navigation; one rule for the user's files; IP15's exception for the origin private file system; a row in the standards | The storage design; the [standards](../../standards.md#52-script-platform-apis-media-and-network) §5.2; the frame tree §4; SG1, EM5, IP15, SW11; ADR-0013's history | SG1, EM5 and IP15 say the same thing, and the standards list the File API |
| [RV2-12](03-findings.md#rv2-12-stale-statements-in-storage) | Open question 2 closed; SW3 and SG10 corrected; quotas per key within a total per site | The storage design §5, §10; SG10; PF24 | — |
| RV2-04 | Its `sessionStorage` part: restored from the browser's copy after the network process dies | SW3 | SW13's crash tests include it |

## 4. Before Milestone 4

| Finding | Change | Documents and rules | Done when |
|---|---|---|---|
| [RV2-06](03-findings.md#rv2-06-accessibility-geometry-after-the-gpu-process) | Geometry relative to the nearest node the browser holds; advances per grapheme cluster while an assistive technology is active; a hit-test policy | [AX2](../../architecture/accessibility.md), AX4; PL16's note; ADR-0014's history | AX2's format carries what AppKit's geometry queries need from the cache |
| [RV2-07](03-findings.md#rv2-07-accessibility-enabled-by-any-client) | Enablement in tiers, by KVO on VoiceOver and Switch Control; disabling after idle; each tier's memory measured | AX7, AX12; ADR-0014's history | AX12 holds as written |
| [RV2-08](03-findings.md#rv2-08-accessibility-tests-that-cant-run-as-written) | Get Computed Role and Label in TS2's subset; the `aamtests` against the shell's build with a process-ID capability and a trusted runner | AX11; [TS2](../../testing.md); EM9 | AX11 says which suite runs where |
| [RV2-09](03-findings.md#rv2-09-accessibility-notifications-announcements-and-inverted-colours) | The eight notifications; live regions and `ariaNotify` by M7; `inverted-colors` | AX4, AX10; the accessibility design §9 | — |
| [RV2-15](03-findings.md#rv2-15-style-hooks-from-interop-2026) | The four hooks named in style's and layout's §8; `zoom` and `attr()` staged with style's second stage | [Style](../../architecture/style.md#8-staging-and-the-complete-model) §8, ST4, ST12, ST19, ST25; layout §8; ADR-0016's history | — |

## 5. How accepted findings become changes

As in Round 1 ([its actions](../round-1/04-round-2-actions.md#5-how-accepted-findings-become-changes) §5): the owner's words go in the disposition log; the changes are made in checkpoints on `architecture-review`, named `docs(review-r2-cpN): …`, each with the five checks green; an ADR's change is an amendment in its history.

**Proposed checkpoints,** grouped by document:

| Checkpoint | Findings | Documents |
|---|---|---|
| CP1 | RV2-01, RV2-02 | The handover, the decision map, the review's process, and the stale statements |
| CP2 | RV2-03, RV2-04, RV2-13, RV2-14 | The process model, IPC, the event loop, testing, Milestone 1, performance, the embedder API |
| CP3 | RV2-05 | Performance, graphics, the pipeline, IPC's open questions, the roadmap's M2 |
| CP4 | RV2-10, RV2-11, RV2-12 | Storage and workers, the standards, the frame tree, the threat model, networking |
| CP5 | RV2-06–RV2-09 | Accessibility, testing, the pipeline |
| CP6 | RV2-15 | Style, layout |

CP1 and CP2 are what Milestone 1 waits for; CP3–CP6 could follow, but they are cheaper now than at their milestones.

## 6. After the stopping rule

The rule is met for the scope commissioned ([risk matrix](01-risk-matrix.md#4-the-stopping-rule) §4). The options:

1. **End the cycle after a short verification of Round 2's changes.** The changes are made in CP1–CP6, then checked against their findings (a pass like this round's [verification](02-verification.md), not a full round), and the owner decides MA10. **This is the recommendation:** the findings are Medium and Low, local, and checkable by search and reading.
2. **Commission Round 3 on what no round has covered:** the requirements' trace (RQ1–RQ66); the methods of S1–S4; the decision map's couplings; dated facts older than six months, the crates on the critical path first (HarfRust, `read-fonts` and `skrifa`, `vello_cpu`, `quinn-proto`, `redb`, `smallvec`); invalidation (IV); HTML and the DOM; the JavaScript engine's details; networking beyond its keys. Round 1 proposed some of these for Round 2 ([its §6](../round-1/04-round-2-actions.md#6-round-2s-scope)).
3. **Add an independent reviewer:** a person, or a model from another family, given the [handover](../../handover-phase-0.md) once RV2-01 has updated it. That adds the independence no round written by Claude can provide.

The options combine: option 1 for Milestone 1's start, with 2 or 3 on the uncovered areas before the milestones that depend on them (the dated facts before M1; S1–S4's methods before M3, whose collector and engine rest on S1 and S3).
