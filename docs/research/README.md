# Research Notes: Conventions

| | |
|---|---|
| **Status** | Living document |
| **Last updated** | 2026-10-19 |

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
| [S3 results](s3-results.md) | Root-stack costs (C1), rooting traffic in an interpreter, dispatch techniques on stable and nightly Rust, node layouts (C2), setters and write barriers (C3) | Complete (2026-10-18), with the CP16 addendum (C1's `SliceStack`) |
| [macOS runtime facilities](macos-runtime-facilities.md) | Run loops, frame timing, QoS on the M1, surfaces across processes, IME and accessibility on macOS 27; browser precedent | Complete (Step 4, CP2) |
| [Step 4 plan](step-4-architecture-proposal.md) | Plan and checkpoint tracker for Step 4: the architecture proposal v0 (event loop, threads, IPC, frame tree, pipeline, invalidation, embedder API, script and heap) and spike S3 | Done (CP1–CP17; merged 2026-10-18) |
| [S5 results](s5-results.md) | The text stack against CoreText on macOS 27 (shaping, rasterization, footprint, speed, size), web fonts through a Rust-only path in the renderer's sandbox, and ImageIO in that sandbox against the safe image decoders | Complete (2026-10-18); the owner's visual checks of S5-Q1 and S5-Q2 done at Gate C (2026-10-20) |
| [S6 results](s6-results.md) | Compositing on Metal on macOS 27: Axiom's own pipelines drawing page-shaped display lists into tiled IOSurface layers of a 2,560 × 1,600 window; damage-limited redraws and their energy; scrolling and animation by layers; CPU against GPU path filling; memory against B1, and Display P3 tags | Complete (2026-10-20), two formal runs: S6-Q1 and S6-Q3 pass at the median; S6-Q5's colour condition met, by the owner's screenshot, and its memory condition not met as measured (B1 revised); ADR-0012 accepted |
| [S7 results](s7-results.md) | The network process on macOS 27: its deny-by-default profile and the rules trust evaluation and DNS-SD need, rustls verified by Security against Network framework's TLS, DNS-SD from kqueue, and one `kevent64` wait driving TLS, QUIC, DNS-SD, Mach and timers | Complete (2026-10-19): S7-Q1, S7-Q3 and S7-Q4 pass; S7-Q2 settles NW6 |
| [Step 5 plan](step-5-subsystems.md) | Plan and checkpoint tracker for Step 5: the subsystems behind Step 4's interfaces (text and fonts, graphics, images and media, networking, storage and workers, accessibility, HTML and the DOM, style, layout and paint, the JavaScript engine, bindings), criteria SC1–SC10, and spikes S5–S7 | Done (CP1–CP19; merged 2026-10-20) |
| [Step 6 plan](step-6-standards-budgets-roadmap.md) | Plan and checkpoint tracker for Step 6: the owner's 13-site corpus and how it is measured, logged out and logged in; the standards baseline (X1), testing (X2) and performance budgets (X3); the roadmap and Milestone 1; criteria RC1–RC10; spike S8, the corpus census | **In progress:** CP1–CP2 ✅; Gate A passed (2026-10-20) |
