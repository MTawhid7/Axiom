# ADR-0001: Scope, long-term target, JavaScript strategy, and security posture

| | |
|---|---|
| **Status** | **Accepted** (2026-10-16) |
| **Date** | 2026-10-16 |
| **Deciders** | Project owner (Phase Zero Step 1 questionnaire); recorded by Claude |
| **Related** | [Charter](../charter.md), [Decision map](../decision-map.md), [ADR-0002](0002-implementation-language.md) |

## Question

Three questions were decided together:
- What long-term target must the architecture be able to reach?
- Where does the JavaScript engine come from?
- What security posture must the browser eventually support?

They come first because they shape every expensive-to-reverse decision after them: the language, the DOM memory model, the process model, and the event loop.

## Requirements and constraints

- **Vision (charter):** a modern, high-performance, secure, minimal browser engine built from first principles. It is a personal browser for the owner's Mac, and the owner wants it genuinely useful, with a credible path toward contemporary web standards.
- **Core we implement ourselves (charter):**
  - parsing and DOM;
  - CSS and style;
  - layout;
  - painting and rendering orchestration;
  - incremental updates;
  - navigation and loading coordination;
  - security policies.
- **Environment:** an 8 GB, fanless M1 MacBook Air running macOS 27.2.

## Alternatives considered

Each question was presented with its alternatives and trade-offs. Claude's recommendation at the time is noted for the record.

| Question | Alternatives | Claude's recommendation | **Decision** |
|---|---|---|---|
| R1. Long-term target | Document web only · Scripted web, selectively · Full modern app web | Scripted web, selectively | **Full modern app web** |
| R2. JavaScript engine | Embed a mature engine · Write our own · Decide after research · No JavaScript | Embed a mature engine | **Write our own** |
| R3. Security posture | Sensitive logins too · General browsing without sensitive accounts · Curated sites only | Sensitive logins too | **Sensitive logins too** |
| Defaults | Presented as "recorded unless you object" | — | **Accepted (no objection)** |

## Decision

1. **R1. Target.** The architecture must be able to reach the **full modern app web**: Docs, Figma and YouTube-class applications.
   - This is a *reachability* requirement, not a milestone order.
   - Milestones still begin with static documents.
2. **R2. JavaScript.** Axiom will have **its own JavaScript engine**. The JS engine joins the core we implement ourselves.
3. **R3. Security.** The architecture must be able to reach a posture suitable for **sensitive logins**: email, banking, and accounts with write access.
   - That means process isolation, sandboxing, and eventually site isolation.
   - Sensitive use is **gated** until the isolation milestone is verified.
4. **Defaults.**
   - Apple Silicon and the current macOS only.
   - Single user, single profile.
   - No sync, accounts, cloud services, extensions or enterprise administration.
   - The browser's own interface stays in one language; web content must render correctly in all languages, added in stages.
   - Accessibility is designed in; the milestone that implements it is to be decided.
   - Performance budgets are measured on the target M1 with 8 GB.
   - DRM-protected media is a non-goal, because access depends on licensing rather than engineering.

## Rationale

The owner's goals combine first-principles understanding with genuine usefulness and a credible path to the modern web. These choices maximize long-term capability and learning, and accept a much longer schedule in return.

R3 follows the principle that downgrading a security posture later is easy, while upgrading one is expensive. Firefox's multi-process retrofit (Electrolysis) and its site-isolation retrofit (Fission) each took years; see the [decision map](../decision-map.md).

## Consequences

1. **One unified GC heap for JS and the DOM**, as in Ladybird.
   - The DOM memory model is designed together with our engine.
   - The triangle of language, JS engine and DOM memory reduces to "language × our GC design".
2. **The language choice must weigh suitability for writing a GC, a VM and a JIT**, and how safely GC references can be held throughout the codebase. See [ADR-0002](0002-implementation-language.md).
3. **Multiple agents.** Workers and `SharedArrayBuffer` imply one heap and one event loop per agent. This affects both the GC design and the process model.
4. **JIT policy.**
   - The engine starts with an interpreter.
   - Any JIT comes only after process isolation and sandboxing exist.
   - A no-JIT mode stays available, for example for sensitive sites. Apple's Lockdown Mode and Edge's enhanced security mode both disable JIT compilation.
   - macOS constraints, recorded for later: `MAP_JIT`, per-thread W^X switching, and the `allow-jit` entitlement.
5. **App-web capabilities must not be ruled out by the design:**
   - a GPU process for WebGL and WebGPU;
   - media through system codecs;
   - storage APIs;
   - service-worker interception inside Fetch;
   - **WebAssembly, where writing our own versus embedding one is open** ([OQ-3](../open-questions.md)).
6. **Schedule is the dominant project risk**, driven by the JS engine and the app-web target. Mitigations:
   - strict milestone staging;
   - strong test oracles (test262, WPT, html5lib-tests);
   - measurable acceptance criteria for each milestone.
7. **Per-process memory budgets are first-class.** 8 GB of RAM, combined with site isolation and one JS heap per process, makes memory a primary constraint.

## Assumptions

- One human developer, with AI assistance ([OQ-9](../open-questions.md)).
- A multi-year horizon; the time budget is still unstated ([OQ-1](../open-questions.md)).

## Risks

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| JS engine and app-web work dominate the schedule | High | High | Milestones deliver a useful static-document engine first; acceptance criteria for each milestone |
| JIT becomes an exploitable attack surface | High, if a JIT is built | High | Interpreter first; JIT only after sandboxing and isolation; no-JIT mode kept |
| Memory pressure on 8 GB with several processes | Medium | Medium | Per-process budgets; measure early |
| Scope creep toward app-web features before the core is solid | Medium | High | Phase gates; ADRs required for new capability areas |

## Conditions for revisiting

- **R2.** Revisit if, after the JS-engine milestones, conformance or performance plateaus and target-corpus sites stay unusable for a long time. Embedding an engine is the fallback, but it would be costly: by then the DOM memory model is built around our own GC.
- **R1, R2.** Revisit if the time budget ([OQ-1](../open-questions.md)) proves incompatible with the scope.
- **Licensing.** Revisit the DRM non-goal only if licensing conditions change.
