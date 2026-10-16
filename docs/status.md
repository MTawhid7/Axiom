# Status and Handoff Log

> **Read this first when resuming work.** Then check the documents it links to against the actual state of the project directory. Don't assume earlier work is still accurate.

## Current state (2026-10-16)

| Item | State |
|---|---|
| Phase | **Phase Zero: design only.** No implementation is authorized |
| Code | None |
| Dependencies installed | None |
| Version control | Not a git repository ([OQ-8](open-questions.md)) |
| Verified capabilities | None. Nothing is implemented, so nothing has been verified |
| Accepted decisions | [ADR-0001](adr/0001-scope-and-target.md): scope, target, JavaScript strategy, security posture |
| Proposed decisions | [ADR-0002](adr/0002-implementation-language.md): Rust, pending review and spike S1 |

## Next steps

1. **Owner:** review the [language evaluation](technology/language-evaluation.md) and [ADR-0002](adr/0002-implementation-language.md).
2. **Owner:** decide whether to authorize the validation spikes S1 (GC heap), S2 (platform and build), and optionally S3 (dispatch benchmark). They are described in evaluation §16 and would be throwaway code, kept outside `docs/`.
3. **Owner:** answer these open questions where possible:
   - OQ-1: time budget;
   - OQ-2: target sites;
   - OQ-6: language fluency;
   - OQ-8: git;
   - OQ-9: team size.
4. **Next design step:** Step 3, covering the threat model, trust boundaries, and the process and sandbox model on macOS.

## Session log

### 2026-10-16 — Session 1 (Phase Zero, Steps 0–2a)

**What changed**
- Created the `docs/` foundation:
  - README, charter, decision map, open questions, this status log;
  - research conventions;
  - the ADR template, ADR-0001 and ADR-0002;
  - the language evaluation.
- No code or dependencies; nothing outside `docs/`.

**What was verified**
- Target machine facts, read from the system:
  - MacBook Air M1 (`MacBookAir10,1`): 4 performance + 4 efficiency cores, 8 GB RAM;
  - macOS 27.2;
  - Rust 1.94.1, Swift 6.4, Apple clang 21;
  - Command Line Tools only.
- Sources for the evaluation's claims were checked during the session; links are in each document. The open items are listed under "Verification gaps" below.

**Decisions made**
- ADR-0001 **Accepted**. The owner chose:
  - full modern app web as the target;
  - our own JS engine;
  - a sensitive-login security posture;
  - the default scope settings.
- ADR-0002 **Proposed**: Rust, with a policy that confines `unsafe`. Not accepted.

**Verification gaps carried forward**
- Boa's 2026 test262 numbers come from a secondary source.
- Whether objc2 is vendored in Firefox was not confirmed, so the evaluation doesn't claim it.
- The Miri limitations are stated as background; re-check the Miri README before relying on them.
- The Fuzzilli integration path for a Rust engine is a hypothesis.

**Next:** see "Next steps" above.
