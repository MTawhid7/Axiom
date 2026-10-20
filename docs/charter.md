# Axiom — Project Charter

| | |
|---|---|
| **Status** | Draft. Sections marked *Accepted* are confirmed in [ADR-0001](adr/0001-scope-and-target.md) |
| **Last updated** | 2026-10-20: the target-site corpus defined (§8) |

## 1. Vision

Design and build a modern, high-performance, secure, minimal browser engine from first principles, for personal use on the owner's Mac.

> Minimal does not mean a toy. Modern does not mean unnecessarily complicated.

The aim is the smallest well-engineered system that delivers meaningful browser functionality, strong security properties, efficient resource use, and excellent performance within its chosen scope. Getting there requires understanding *why* existing engines are designed as they are, and keeping only the constraints that still apply.

## 2. Goals

| ID | Goal |
|---|---|
| G1 | Understand and implement the core browser-engine mechanisms ourselves (see the list below) |
| G2 | Be genuinely useful on a defined and growing set of real websites, with an architecture that can reach the full modern app web (*Accepted*, ADR-0001 R1) |
| G3 | Reach a security posture adequate for sensitive logins, in verified stages (*Accepted*, ADR-0001 R3) |
| G4 | **Efficiency is a primary architectural goal, not an afterthought:** low memory use, CPU efficiency and low power consumption, with measurable budgets on the target machine (owner direction, 2026-10-16) |
| G5 | Make decisions from evidence, record them in ADRs, and keep these docs as the source of truth |

The core mechanisms in G1 are:
- document parsing and DOM;
- CSS parsing and style;
- layout;
- painting and rendering orchestration;
- incremental updates;
- navigation and resource-loading coordination;
- browser security policies;
- the JavaScript engine (*Accepted*, ADR-0001 R2).

## 3. Users

One user, the project owner, on their own Mac. There are no other users, accounts or profiles.

## 4. Target environment

These facts were verified on 2026-10-16.

| Item | Value |
|---|---|
| Machine | MacBook Air (M1, `MacBookAir10,1`), fanless |
| CPU | Apple M1: 4 performance and 4 efficiency cores |
| Memory | **8 GB** |
| OS | macOS 27.2 |
| Toolchains present | Rust 1.94.1; Swift 6.4; Apple clang 21 — **Command Line Tools only** (no Xcode, so no Instruments) |

This machine is also the **performance floor**: budgets are set and measured here.

## 5. Scope decisions (*Accepted*, ADR-0001)

- **R1.** The architecture must be able to reach the full modern app web. Milestones still start with static documents.
- **R2.** Our own JavaScript engine. JS objects and the DOM share one GC heap.
- **R3.** A security posture suitable for sensitive logins. Sensitive use is gated until isolation is verified.
- **Platform.** Apple Silicon and the current macOS only.

## 6. Non-goals (deliberate)

| Non-goal | Rationale | Revisit trigger |
|---|---|---|
| Cross-platform support | Personal macOS browser; lets us use Apple APIs without abstraction layers | Not planned |
| Intel Macs and older macOS versions | Newest APIs without fallbacks | Not planned |
| Sync, accounts, cloud services | Single local user | Not planned |
| Extension ecosystem | Large attack surface and API burden; not needed by a single user | Owner request plus an ADR |
| Enterprise administration | Not applicable | Not planned |
| Multi-profile management | Single user | Owner request |
| Translating the browser's own interface | Single user. **Web content in all languages is in scope**, in stages | Owner request |
| DRM-protected media | Access depends on licensing, not engineering | Licensing changes |
| Bug-for-bug compatibility with legacy browsers beyond the specs | The specs' own quirks-mode behaviour **is** in scope; behaviour outside the specs is added only when the target corpus needs it | Corpus evidence |

## 7. Constraints

- **Memory.** 8 GB of RAM makes per-process memory budgets and the number of processes first-class design inputs.
- **Thermals.** A fanless machine throttles under sustained load, so benchmarks must control for it.
- **Toolchain.** Command Line Tools only. Instruments requires Xcode ([OQ-7](open-questions.md)). Measurement uses CLI tools only: `samply`, `/usr/bin/time -l`, `cargo bench`, Miri.
- **Team.** One human developer, fluent in Rust, with AI assistance. Being the only human developer is an *assumption* ([OQ-9](open-questions.md)).
- **Time.** No calendar deadline and no fixed hourly budget. Work proceeds in bite-sized checkpoints, each leaving the workspace clean and working ([OQ-1](open-questions.md), resolved).
- **Process.** Phase Zero is design-only. No implementation code, project skeleton or dependency installation happens without explicit authorization from the owner for a specific milestone or preparatory action.

## 8. Success criteria (Draft)

**Phase Zero** is complete when:
- the owner has approved an architecture proposal and a roadmap;
- every foundational decision has an Accepted ADR;
- the target-site corpus is defined;
- the measurable acceptance criteria for Milestone 1 are written.

**Every milestone** will have acceptance criteria that can be measured. They will draw on:
- conformance: pass rates on selected WPT directories, html5lib-tests and test262;
- target-corpus checks: content renders and is readable, links navigate, and there are no crashes or hangs;
- performance budgets measured on the M1;
- security gates, for example no sensitive logins until isolation is verified.

**Target-site corpus:** defined by the owner on 2026-10-20: 13 sites, from documents (Wikipedia, MDN) to applications (Figma, VS Code for the Web, Discord, Prism), several of which need a login ([OQ-2](open-questions.md); the [Step 6 tracker](research/step-6-standards-budgets-roadmap.md) §2.4).

## 9. Principles

1. Research before committing to an architecture. Separate fundamental constraints from historical ones.
2. Build the smallest well-engineered system. No technology chasing.
3. Security and performance are architectural concerns, not later phases.
4. Implement the core ourselves. Reuse supporting infrastructure only with an explicit justification, stating whether it is:
   - a required platform facility;
   - a replaceable implementation detail; or
   - a foundational dependency.
5. Keep these pairs distinct in every document:
   - fact, inference and hypothesis;
   - implemented and planned;
   - standards-compliant and approximate.
6. Identify technical debt openly. Avoid both premature generalization and shortcuts that make later security work disproportionately hard.
7. **No architectural corners cut for convenience** (owner direction, 2026-10-17). The project has no calendar deadline, so weaker security boundaries are never accepted to save time. Where a feature is staged, the architecture beneath it is designed for the complete model from day one.

## 10. Governance

- Decisions are recorded as ADRs ([template](adr/0000-template.md)). An ADR moves from Proposed to Accepted **only** with the owner's explicit confirmation.
- [status.md](status.md) is updated at the end of every substantial session, as the handoff record.
- When resuming work, inspect the current project state and these documents first. Don't rely on conversational memory.
