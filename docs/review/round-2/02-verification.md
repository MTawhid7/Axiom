# Review Round 2: Verification of Round 1

| | |
|---|---|
| **Status** | **Round 2, disposed** (2026-10-21; [the disposition log](../README.md#55-round-2s-dispositions)). Every one of Round 1's 22 accepted findings is resolved in the binding documents; ten leave stale statements elsewhere, and two expose gaps that Round 2 raises as findings |
| **Question** | Do the CP1–CP6 amendments "completely resolve findings RV1-01 through RV1-22 without introducing regressions or contradictions" (the owner's commission, [the review](../README.md#54-decisions-on-round-1s-results) §5.4)? |
| **Baseline** | `architecture-review` at `ec7c6b3` (2026-10-21): Round 1's six checkpoints, the sweep for stale statements, spikes S10, S12 and S15, and the owner's decisions on their results |
| **See also** | [Risk matrix](01-risk-matrix.md) · [Findings](03-findings.md) · [Actions](04-actions.md) · [Round 1's findings](../round-1/02-findings.md) |
| **Last updated** | 2026-10-21 |

## 1. Summary

**Every accepted finding is resolved** where the [disposition log](../README.md#51-findings) says, in the owner's words: no change is missing, and none departs from what the owner decided.

**Verdicts:**
- **Resolved, 12:** RV1-03, RV1-06, RV1-10, RV1-11, RV1-12, RV1-14, RV1-15, RV1-16, RV1-18, RV1-19, RV1-20, RV1-22.
- **Resolved, with residue, 10:** the binding documents changed, but statements elsewhere still say the old thing. The entry documents, the handover and the decision map, are [RV2-01](03-findings.md#rv2-01-the-entry-documents-predate-round-1); the rest are [RV2-02](03-findings.md#rv2-02-statements-round-1s-changes-left-stale).
- **Of those, two expose gaps** that the changes made real, raised as findings: RV1-02 and RV1-07.

**One regression:** RV1-02's amendment of PL15 gives the browser only the property-tree nodes its layers need, which removes the premise of AX2's geometry ([RV2-06](03-findings.md#rv2-06-accessibility-geometry-after-the-gpu-process)).

**Two contradictions the changes introduced:**
- RV1-07's real processes at M1 against TS8's and MA8's virtual time, which EL24 provides only in one process ([RV2-03](03-findings.md#rv2-03-which-process-configuration-runs-the-conformance-suites));
- PF4, as amended by CP2, charges GPU-side memory "to its renderer" and says it answers IPC's open question 4; S10 then measured tiles charged to the GPU process ([RV2-05](03-findings.md#rv2-05-two-protocols-the-gpu-process-needs-from-m2)).

## 2. Method

1. **Each row of the disposition log** (§5.1 there) was checked against the documents its "Changed" column names: the change is present, says what the owner's words say, and agrees with the rules around it.
2. **Each finding's ID was counted** across `docs/`, outside the review: every one appears in 3 to 27 documents (observed, `grep -rl "RV1-nn" docs`).
3. **The old wording of each finding was searched for** outside the review, in living documents, and each hit read in context: "one process", "unsandboxed process", "until a GPU process", "browser composites", "the browser validates", "5,848", "idle periods", "M8's gate", "ad hoc" and "ad-hoc", "shared memory or IOSurfaces", "as a reference instead", "baseline hypothesis", "USB HID", "move to separate agents", "become their own agents". Hits in research notes and step trackers are records of their time and were left alone; hits marked "as amended" or carrying the finding's ID quote the old text on purpose.
4. **The entry documents** (the [handover](../../handover-phase-0.md), the [decision map](../../decision-map.md) and the [status](../../status.md)) were read for each finding.
5. **The five checks** pass at the baseline, and again with Round 2's documents (the [status](../../status.md)'s session entry).

## 3. Finding by finding

"Where" names the rules that now carry the change; "Left over" names the residue or gap and the Round 2 finding that raises it.

| Finding | The owner's change, in short | Where | Verdict | Left over |
|---|---|---|---|---|
| [RV1-01](../round-1/02-findings.md#rv1-01-milestone-1s-css-target-and-harness-need-script) | An interim CSS target over the 5,652 script-free tests; a script-free harness at M1; the full target and WebDriver at M3; the harness end to end at the second checkpoint | [Milestone 1](../../milestone-1.md) MA4, MA11; [testing](../../testing.md) TS2; [standards](../../standards.md) SB11; ADR-0021 | Resolved, with residue | The handover §4.14, §9 (RV2-01) |
| [RV1-02](../round-1/02-findings.md#rv1-02-compositing-stays-in-the-unsandboxed-browser-through-l3) | The sandboxed GPU/compositor process designed now, built at M2; B1 split; L3 certified on the final topology | The [process model](../../security/process-model.md) §3 note 1, §4.1, §7, §10; ADR-0004 D1; [graphics](../../architecture/graphics.md) GR10–GR14; the [pipeline](../../architecture/rendering-pipeline.md) PL7–PL9, PL15, PL16; [S10](../../research/s10-results.md) | Resolved, with residue and gaps | Residue: graphics §3 and §9; the pipeline's header; layout §7; OQ-24 (RV2-02); the handover (RV2-01). Gaps: the GPU process's death and hangs (RV2-04); accounting and surface reuse (RV2-05); accessibility's geometry, a regression (RV2-06); never blocking (RV2-13) |
| [RV1-03](../round-1/02-findings.md#rv1-03-cross-site-agents-before-l2-the-documents-disagree) | Cross-site frames are separate agents behind remote proxies from M1 | [FT10](../../architecture/frame-tree-and-navigation.md) and §5; SH1; WB20 and the bindings' §8; the overview §6; the roadmap MS5, §4.1 | Resolved | None: the two phrases RV1-03 searched for appear only in the review |
| [RV1-04](../round-1/02-findings.md#rv1-04-the-collectors-end-state-against-the-pause-budgets) | The collector's end-state fixed before M3 in ADR-0008; pauses measured in S11 | [SH10](../../architecture/script-and-heap.md); ADR-0008 decision 8; EL11; JE29; PF15 | Resolved, with residue | OQ-13's resolution (RV2-02); the handover §4.6 (RV2-01) |
| [RV1-05](../round-1/02-findings.md#rv1-05-the-content-tiers-150-mb-cap) | A normalized workload; a gate on slope and plateau; 150 MB a stretch target; zero-copy resources | The [performance](../../performance.md) §4.1, PF11; B3; OQ-25 | Resolved, with residue | Performance §9's first question; the status's ADR-0019 row (RV2-02); the handover §7, §9, §10 (RV2-01) |
| [RV1-06](../round-1/02-findings.md#rv1-06-the-width-of-heap-slots) | The slot width evaluated in S11 before M3 | [JE9](../../architecture/javascript-engine.md), JE10; SH13; the performance §4.1 | Resolved | — |
| [RV1-07](../round-1/02-findings.md#rv1-07-milestone-1-in-one-process) | Real, sandboxed renderer and network processes from M1; single-process mode for tests, fuzzing and Miri | The [roadmap](../../roadmap.md) MS3, MS4, §4.1, §10; the process model §9; IP2; the event loop's L0 row; MA1, MA2, MA6, MA7, MA10–MA12 | Resolved, with residue and gaps | Residue: RQ24, RQ29; the roadmap's summary; script and heap §4; EL24, EM10 (RV2-02); the handover (RV2-01). Gaps: which configuration runs the suites (RV2-03); the network process's death (RV2-04); never blocking (RV2-13) |
| [RV1-08](../round-1/02-findings.md#rv1-08-scope-against-one-developer) | Encodings, Temporal's arithmetic and BigInt as replaceable details; `encoding_rs` graded before M1, `temporal_rs` before M3 | DP1 (g), DP2; the register; [TX3](../../architecture/text-and-fonts.md); the engine's §4, JE13, JE22; the [audit](../../research/encoding-rs-audit.md) | Resolved, with residue | The engine's §3 on `temporal_rs` (RV2-02) |
| [RV1-09](../round-1/02-findings.md#rv1-09-iosurfaces-in-renderers) | Renderers write into memory entries; only the compositor owns IOSurfaces; renderer profiles deny `iokit-open` | The process model §4.1, §10; [IM1](../../architecture/images-and-media.md); GR10; PL7, PL8; V1; MA7 | Resolved, with residue | Images §1 (RV2-02); IP7's `iosurface` type isn't barred from renderers' interfaces (RV2-14) |
| [RV1-10](../round-1/02-findings.md#rv1-10-the-hybrid-text-models-gaps) | Rules for CoreText's emoji advances and `-webkit-font-smoothing`; the hybrid model kept | TX22–TX24, TX13, TX21; SB5; TS16 | Resolved | — |
| [RV1-11](../round-1/02-findings.md#rv1-11-concurrency-checks-without-threadsanitizer) | Concurrency checks in order: Miri, loom, schedule exploration, then TSan | TS11; MA7, MA10, MA12; [S12](../../research/s12-results.md) | Resolved | — |
| [RV1-12](../round-1/02-findings.md#rv1-12-corpus-targets-before-their-capabilities) | Targets re-placed; WebSockets at M4; the census addendum | The roadmap MS5, §4.1, §5; the [corpus](../../corpus.md) §10 | Resolved | — |
| [RV1-13](../round-1/02-findings.md#rv1-13-webassembly-and-the-jit-decided-late) | WebAssembly's architecture before M3, with S14; the JIT at M5's gate | The roadmap MS5, §7; [OQ-3](../../open-questions.md); I6; JE30, JE31 | Resolved, with residue | The overview §5.4; OQ-3's typing error (RV2-02); the handover §7, §9 (RV2-01) |
| [RV1-14](../round-1/02-findings.md#rv1-14-site-compatibility) | A user-agent policy; interventions as data | The standards §5.4, SB13, SB14; MA5, MA12 | Resolved | — |
| [RV1-15](../round-1/02-findings.md#rv1-15-diagnostics-for-one-developer) | Text dumps at M1, crash reports at M2, the console at M3 | [EM9](../../architecture/embedder-api.md); JE27; MA1, MA11 | Resolved | — |
| [RV1-16](../round-1/02-findings.md#rv1-16-failure-semantics-panics-and-overflow) | `panic = "abort"` per executable; checked arithmetic in parsers and validators | [ADR-0002](../../adr/0002-implementation-language.md)'s failure semantics; IR10; IP11; MA7, MA10, MA11; the overview §2 | Resolved | — |
| [RV1-17](../round-1/02-findings.md#rv1-17-the-signing-identity) | A stable self-signed identity, validated by S15 | ADR-0004 D2; the process model §10; MA10, MA12; [S15](../../research/s15-results.md) | Resolved, with residue | EM12; ADR-0009's decision 7 (RV2-02) |
| [RV1-18](../round-1/02-findings.md#rv1-18-build-cost-of-generated-code) | Generated bindings split by interface group; build times measured in S13 | The overview §2; WB22; PF18; MA6, MA11 | Resolved | — |
| [RV1-19](../round-1/02-findings.md#rv1-19-rendering-timing-side-channels) | Pixel stealing in the threat model; a rule and a test | NG4, §9; PL9; V4 | Resolved | — |
| [RV1-20](../round-1/02-findings.md#rv1-20-a-stale-fact-rustls-and-ech) | The stale fact corrected | [Networking](../../architecture/networking.md)'s question 4; the handover | Resolved | — |
| [RV1-21](../round-1/02-findings.md#rv1-21-passkeys-without-the-restricted-entitlement) | The hybrid transport among OQ-16's candidates | OQ-16; the threat model §9; the roadmap's M6, §7 | Resolved, with residue | The threat model §10's summary of OQ-16; OQ-16's typing error (RV2-02); the handover §9 (RV2-01) |
| [RV1-22](../round-1/02-findings.md#rv1-22-daily-use-gaps-and-an-orphaned-decoder) | Daily features scheduled; PDFs in Preview; JPEG's decoder at M1's plan | The roadmap's M5, M6; the standards §5.3; IM2; the register; MA12 | Resolved | — |

## 4. What the verification didn't cover

- **The spikes' results** were taken as the owner approved them (§5.4 there): S10's, S12's and S15's measurements weren't repeated.
- **Choices the checkpoints made where the owner's words left one open** (the log's note under §5.1) were checked for consistency, not re-argued; the owner approved them "as implemented" (decision 4).
- **Research notes and step trackers** keep the old wording as their record, by the project's convention, and weren't searched for staleness.
