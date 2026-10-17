# Research Notes: Conventions

| | |
|---|---|
| **Status** | Living document |
| **Last updated** | 2026-10-17 |

This folder holds research notes on design history, modern alternatives, and experimental results. They feed the ADRs and the architecture documents.

## Sourcing rules

1. **Prefer primary sources:**
   - specifications;
   - standards documents;
   - engineering posts from browser and language vendors;
   - source code;
   - research papers;
   - security advisories;
   - reproducible benchmarks.
2. **Label secondary sources** (news coverage, third-party summaries) as *secondary*. Don't let a secondary source carry a decision on its own.
3. **Date everything.** Record when a source was published and when it was checked. Browser and language facts change quickly.
4. **Never invent history.** If the reason behind a past design isn't documented, say so.

## Evidence labels

| Label | Meaning |
|---|---|
| Sourced fact | A claim with a link to its source |
| **Inference** | Reasoning from sourced facts; open to challenge |
| **Hypothesis** | A claim that needs validation, with the validation step named |
| **Background** | Well-established and not re-verified in this document; re-check before relying on it for a decision |

## Note template

```markdown
# <Topic>

| | |
|---|---|
| **Status** | Draft / Reviewed |
| **Question** | What this note answers |
| **Feeds** | ADR-NNNN / architecture section |
| **Last updated** | YYYY-MM-DD |

## Summary
## Background and history
## Current state (with dates)
## Analysis (labelled fact / inference / hypothesis)
## Implications for Axiom
## Open questions
## Sources
```

## Index

| Note | Topic | Status |
|---|---|---|
| [Language evaluation](../technology/language-evaluation.md) | Implementation language. Kept under `technology/` because it is a technology evaluation; §7 doubles as the research note on GC and DOM memory models | Reviewed |
| [Step 2b validation spikes](step-2b-validation-spikes.md) | Plan, design and checkpoint tracker for S1 (GC/DOM heap patterns) and S2 (platform and build) | Complete (2026-10-17) |
| [S1 results](s1-results.md) | GC heap patterns for a shared JS and DOM heap: `unsafe` containment, ceremony, runtime, memory, build cost; recommendation for ADR-0003 | Reviewed (2026-10-17) |
| [S2 results](s2-results.md) | AppKit, Metal and CoreText through objc2 without Xcode: `unsafe` surface, build and run-time cost; proposed build thresholds for ADR-0002 | Reviewed (2026-10-17) |
| [Step 3 plan](step-3-security-and-process-model.md) | Plan and checkpoint tracker for Step 3: threat model, process model, macOS confinement, spike S4 | Complete (2026-10-17) |
| [macOS process confinement](macos-process-confinement.md) | Seatbelt, App Sandbox, hardened runtime, JIT, launch and IPC options on macOS 27; browser precedent; spike S4 design | Reviewed (2026-10-17) |
| [S4 results](s4-results.md) | Custom Seatbelt profiles on macOS 27: probes; start-up, footprint and IPC cost; the JIT matrix; CoreText in a sandbox | Complete (2026-10-17) |
| [macOS runtime facilities](macos-runtime-facilities.md) | Run loops, frame timing, QoS on the M1, surfaces across processes, IME and accessibility on macOS 27; browser precedent | Complete (Step 4, CP2) |
| [Step 4 plan](step-4-architecture-proposal.md) | Plan and checkpoint tracker for Step 4: the architecture proposal v0 (event loop, threads, IPC, frame tree, pipeline, invalidation, embedder API, script and heap) and spike S3 | In progress (plan approved at Gate A) |
