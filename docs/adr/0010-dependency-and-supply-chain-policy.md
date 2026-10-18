# ADR-0010: Dependency and supply-chain policy

| | |
|---|---|
| **Status** | **Proposed** (2026-10-18), at Step 5's CP9 |
| **Date** | 2026-10-18 |
| **Deciders** | Project owner (only the owner can move an ADR to Accepted) |
| **Related** | [Dependency policy](../technology/dependency-policy.md) (the full rules, DP1–DP23); [dependency register](../technology/dependency-register.md); [charter](../charter.md) G1 and principle 4; [ADR-0002](0002-implementation-language.md) and the [language evaluation](../technology/language-evaluation.md) §12, §15.3–§15.4; [threat model](../security/threat-model.md) TA4, TA5, §9; [process model](../security/process-model.md) PM2; [Step 5 tracker](../research/step-5-subsystems.md); decision-map item X4 |

## Decision history

| Date | Event |
|---|---|
| 2026-10-18 | X4 added to the decision map by the owner at Step 5's Gate A: "Add X4 (Dependency and Supply-Chain Policy) to docs/decision-map.md, to be decided at CP2 in ADR-0010" |
| 2026-10-18 | The policy written at CP2. DP10 refined at CP3: applying it showed that a tree-wide `ub-risk-1` rule excluded every Rust font stack while admitting C through the exception |
| 2026-10-18 | Proposed at Step 5, CP9 |

## Question

Which code does Axiom write itself, and which may it reuse? How is each dependency justified, classed, audited and placed? How do the `unsafe` and C code inside dependencies count against ADR-0002's rules and the Rule of 2?

## Requirements and constraints

- **Charter principle 4:** implement the core ourselves; reuse supporting infrastructure only with a justification and a class: required platform facility, replaceable implementation detail, or foundational dependency. **G1** lists the core mechanisms.
- **ADR-0002:** no `unsafe` in parsers, network policy, IPC decoding, or image and font wrappers (rule 5); macro-generated `unsafe` counts (rule 7); thresholds T1–T4.
- **PM2, TA5:** the Rule of 2; Apple's complex-format parsers only in sandboxed processes.
- **TA4, §9:** dependencies are trusted at their pinned versions, and fixes arrive only through rebuilds.

## Alternatives considered

- **No reuse,** SerenityOS's former rule.
- **Case-by-case reuse,** with no rules.
- **No `unsafe` in any dependency.**
- **Counting `unsafe`** as the criterion.
- **Grades and process groups,** after Chromium and Google: the proposal.

## Historical context and modern developments

Every engine writes its core and reuses infrastructure. Ladybird relaxed its "no 3rd party code" rule in 2024. Chromium groups its Rust crates by whether they satisfy the Rule of 2: `safe` for any process, `sandbox` for sandboxed processes only, with the least-privileged group inherited transitively. Google's audits grade soundness from `ub-risk-0` to `ub-risk-4`. Firefox audits with `cargo-vet` and imports other organizations' audits, and Servo checks licenses and duplicates with `cargo-deny`. All sourced in the [policy](../technology/dependency-policy.md) §2.

## Analysis

| | No reuse | Case by case | No `unsafe` at all | Grades and groups |
|---|---|---|---|---|
| Schedule | Multiplied: TLS, codecs, shaping, Unicode | Fast | Rules out objc2, which ADR-0009 requires | Reuse where it is safe enough |
| Security | Our own crypto would be a hazard | Rule 5 empty for code we didn't write | Strong but impossible | The Rule of 2 applied to dependencies |
| Consistency | — | None | — | One rule for every checkpoint |

## Evidence and sources

The [policy](../technology/dependency-policy.md) §2 and its Sources: Chromium's `gnrt_config.toml` and audits, Google's auditing standards, Mozilla's `cargo-vet`, Firefox's and Servo's configurations, Ladybird's announcement. Applying the policy at CP3–CP8 tested it against real candidates, as recorded in the [register](../technology/dependency-register.md).

## Decision

**Recommended: accept the [dependency policy](../technology/dependency-policy.md) as binding.** In summary:
1. **The line** (DP1–DP4). Charter G1's mechanisms and the security policies are Axiom's own. The decision map's infrastructure layer, Unicode data, encodings, URL parsing, compression and storage engines may be reused behind interfaces Axiom owns. Axiom never writes its own cryptography. A dependency must save substantial, specialized work.
2. **Classes and records** (DP5–DP7). Every dependency has a class and a register entry covering its whole transitive tree. A foundational dependency needs an ADR.
3. **Grades and groups** (DP8–DP12).
   - Google's `ub-risk` scale, with `ub-risk-3` and `ub-risk-4` not admitted.
   - Chromium-style groups (`trusted`, `sandboxed`, `build`, `test`), inherited transitively.
   - A reused parser of untrusted input must be pure Rust at `ub-risk-2` or better across its tree, and `ub-risk-1` in the browser.
   - C, C++ or assembly parsing untrusted input, Apple's included, is admitted only in a sandboxed helper, with no pure-Rust alternative, and with an ADR.
   - Macro-generated `unsafe` and build-time code are part of the boundary.
4. **The supply chain** (DP13–DP20):
   - crates.io only, with `--locked` builds;
   - `cargo-vet` audits, importing Firefox's organizations;
   - `cargo-deny`'s four checks;
   - a license allow-list that keeps Axiom's own license open;
   - minimal features, and costs measured against T1–T4.
5. **Apple's frameworks** run only in the processes DP21 lists, through `platform`.
6. **Enforcement** (DP22, DP23): `cargo-deny` and `cargo-vet` become checks once the owner authorizes installing them, and `tools/check-dependencies.py` is designed. The spikes are exempt from the register.

## Rationale

- It turns charter principle 4 into rules that each checkpoint can apply, and that each gate can check.
- It closes the gap ADR-0002 left: rule 5 governed only Axiom's own code.
- Its grades and groups come from Chromium's and Google's practice, which applies the Rule of 2 to code a project didn't write.
- Applying it at CP3–CP8 changed real decisions: Axiom writes its own encodings, HTTP/1.1 parser and URL parser, and bars `zune-jpeg`. That shows it has teeth.

## Assumptions

- Published audits from Google, Mozilla, the Bytecode Alliance, ISRG and Embark remain available to import.
- The owner, or Claude preparing work for the owner's approval, can grade crates that have no published grade.

## Risks and unresolved issues

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| A crate everyone uses carries a barred grade, as `smallvec` 1.11.0 does (`ub-risk-3`), and blocks good candidates | Medium | Medium | Grade the adopted version afresh; ask upstream; the register records the risk |
| Auditing costs more time than expected | Medium | Medium | Imported audits; the DP4 bar keeps dependencies few |
| Grades go stale as crates change | High | Low | Relative audits on every update (DP14, DP15) |

## Conditions for revisiting

- The policy blocks a dependency whose safe alternatives are clearly worse, as happened at CP3. Refine the rule and record why.
- Rust gains a mechanism that checks soundness more strongly than audits.

## Consequences

- Every Step 5 subsystem records its dependencies in the [register](../technology/dependency-register.md). CP3–CP8 already have.
- **Open question 21** (new): Axiom's own license, which the repository lacks (observed 2026-10-18), raised for the owner.
- Installing `cargo-deny` and `cargo-vet`, and building `tools/check-dependencies.py`, wait for the owner's authorization with the first engine crate.
