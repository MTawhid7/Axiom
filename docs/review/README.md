# The Architecture Review

| | |
|---|---|
| **Status** | **Living.** Round 1 delivered on 2026-10-21; **awaiting the owner's dispositions** (§5). No finding changes the baseline until the owner accepts it |
| **Question** | How is the Phase Zero baseline reviewed before Milestone 1 starts: in what format, with which severities, how do findings become changes, and when does the review end? |
| **Builds on** | The owner's decision 8 at Step 6's Gate C (2026-10-20): Milestone 1 waits "until the architecture and roadmap undergo independent, multi-round adversarial review"; the recommendation in [status](../status.md) for the review's brief; the findings format the [handover](../handover-phase-0.md) §3 suggests |
| **Last updated** | 2026-10-21 |

## 1. Purpose and scope

**The owner's words** (Gate C, 2026-10-20): "We are deferring implementation (Milestone 1 kickoff) until the architecture and roadmap undergo independent, multi-round adversarial review." The [handover](../handover-phase-0.md) states the bar: implementation begins only when "no significant flaws or gaps remain, and further review yields no meaningful improvements".

**The scope** is the baseline approved at Gate C (`main` at `4fb9ca6`): every ADR, the threat and process models, the designs under `architecture/`, the binding documents of Step 6, and spikes S1–S9. The handover's §11 is the standing agenda.

**Round 1's commission** (the owner, 2026-10-21) added:
- five focus areas: the staging from L0 to L3; text and fonts; the content tier's 150 MB cap; the JavaScript engine and its bindings; ThreadSanitizer;
- four deliverables: a risk matrix, detailed findings with counter-proposals, challenged assumptions, and actions for the next round.

## 2. Rounds

| Round | Date | Documents | Findings | State |
|---|---|---|---|---|
| 1 | 2026-10-21 | [Risk matrix](round-1/01-risk-matrix.md) · [Findings](round-1/02-findings.md) · [Challenged assumptions](round-1/03-challenged-assumptions.md) · [Actions before Round 2](round-1/04-round-2-actions.md) | 22: 2 Critical, 6 High, 10 Medium, 4 Low | Awaiting the owner's dispositions |

## 3. Findings and severities

**IDs** are `RV<round>-<nn>`, for example RV1-01. They belong to this folder. Spikes the review proposes continue the project's numbering from S9.

**Each finding gives:**
- its severity;
- the milestone it blocks;
- the claim it challenges, quoted with its rule ID;
- the evidence, labelled as the [research conventions](../research/README.md) require and dated;
- the failure mode;
- a counter-proposal;
- the cost of making the change later instead of now;
- the rules it would amend.

**Severities:**

| Severity | Meaning | When it must be settled |
|---|---|---|
| **Critical** | Built as written, a gate can't pass, or a security boundary is weaker than its level claims | Before the milestone it blocks starts |
| **High** | A decision whose later change would rework several crates, or certify a security level again | Before the milestone it blocks starts |
| **Medium** | A gap or an inconsistency whose fix is local, but needed by its milestone | By the milestone it blocks |
| **Low** | A stale fact, a missing decision with an easy default, or a risk to record | By its milestone, or recorded |

**Blocks** names the milestone whose plan shouldn't start until the owner has decided the finding.

## 4. How a round runs

1. **The reviewer reads the baseline** at a fixed commit, and writes findings in §3's format.
2. **The owner decides each finding,** in the disposition log (§5): **accepted**, **rejected** or **deferred**, with a reason; a deferred finding names when it returns.
3. **Accepted changes are made in checkpoints** on `architecture-review`, as Round 1's [actions](round-1/04-round-2-actions.md#5-how-accepted-findings-become-changes) describe:
   - an ADR's change is an amendment, recorded in its decision history with the finding's ID;
   - a spike needs its own authorization.
4. **The next round** verifies the changes, then reviews what the previous round didn't cover in depth.
5. **`architecture-review` merges into `main`** only when the owner says so.

## 5. The disposition log

The owner's decisions, in the owner's words. "Changed" names the commit and documents that carry an accepted change.

| ID | Severity | Blocks | Disposition | The owner's words | Date | Changed |
|---|---|---|---|---|---|---|
| RV1-01 | Critical | M1 | Awaiting | | | |
| RV1-02 | Critical | M1 | Awaiting | | | |
| RV1-03 | High | M1 | Awaiting | | | |
| RV1-04 | High | M3 | Awaiting | | | |
| RV1-05 | High | M4 | Awaiting | | | |
| RV1-06 | High | M3 | Awaiting | | | |
| RV1-07 | High | M1 | Awaiting | | | |
| RV1-08 | High | M1, M3 | Awaiting | | | |
| RV1-09 | Medium | M1 | Awaiting | | | |
| RV1-10 | Medium | M1 | Awaiting | | | |
| RV1-11 | Medium | M1 | Awaiting | | | |
| RV1-12 | Medium | M4 | Awaiting | | | |
| RV1-13 | Medium | M3 | Awaiting | | | |
| RV1-14 | Medium | M4 | Awaiting | | | |
| RV1-15 | Medium | M1 | Awaiting | | | |
| RV1-16 | Medium | M1 | Awaiting | | | |
| RV1-17 | Medium | M1 | Awaiting | | | |
| RV1-18 | Medium | M1, M3 | Awaiting | | | |
| RV1-19 | Low | M5 | Awaiting | | | |
| RV1-20 | Low | M5 | Awaiting | | | |
| RV1-21 | Low | M6 | Awaiting | | | |
| RV1-22 | Low | M6 | Awaiting | | | |

## 6. The stopping rule

**The cycle ends** when a round raises no Critical or High finding, after the previous round's accepted changes have been made and verified. The owner then decides whether to authorize Milestone 1 (MA10). The owner may end or extend the cycle at any time.

## 7. Round 1's coverage and limits

**Read in full:**
- the handover, the charter, the overview and the status log's current state;
- the threat model and the process model;
- ADRs 0001, 0004 and 0008;
- the designs for script and heap, the JavaScript engine, and text and fonts;
- the performance budgets, Milestone 1 and the roadmap's milestones.

**Read by rule, section or targeted search:**
- the other ADRs, through their decisions and their risk and revisit sections;
- the designs for the event loop, the frame tree, IPC, the rendering pipeline, HTML and the DOM, bindings, graphics, images, and networking;
- the standards, testing and dependency documents, and the corpus;
- the results of S5, S8 and S9;
- the open questions and the decision map.

**Not reviewed in depth,** and given to Round 2 ([actions](round-1/04-round-2-actions.md#6-round-2s-scope) §6): accessibility, storage and workers, style's and layout's algorithms, the embedder API, IPC's details, the requirements' trace, and S1–S4's methods.

**Evidence:**
- **No spike was re-run.** One count is new (RV1-01), made with the [tool](../../tools/wpt-script-deps.py) this round adds, over S9's local WPT checkout.
- **S8's raw records** under `target/` were consulted for RV1-10.
- **External sources** were read on 2026-10-21, through an automated fetch; quotations were checked against the fetched text.

**Independence is limited.** Round 1 was written by Claude (Opus 5.5) at the owner's request, and the baseline was also written with Claude's help (see the ADRs' "recorded by Claude"). A reviewer from another model family, or a person, would add independence that this round can't.
