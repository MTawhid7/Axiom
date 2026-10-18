# ADR-0014: Accessibility, and the shell's settings and reserved keys

| | |
|---|---|
| **Status** | **Proposed** (2026-10-18), at Step 5's CP9. No spike is needed; the shell milestone validates its objc2 classes first (EM11) |
| **Date** | 2026-10-18 |
| **Deciders** | Project owner (only the owner can move an ADR to Accepted) |
| **Related** | [Accessibility](../architecture/accessibility.md) (AX1–AX12, and §6's questions); [input and accessibility](../architecture/input-and-accessibility.md) IN6–IN9; [embedder API](../architecture/embedder-api.md) EM5, EM6, EM8, EM11; [ADR-0007](0007-frame-tree-pipeline-and-input.md); [ADR-0009](0009-embedder-api-and-shell.md); [ADR-0010](0010-dependency-and-supply-chain-policy.md); decision-map item I5 |

## Decision history

| Date | Event |
|---|---|
| 2026-10-18 | Proposed at Step 5, CP9, with the settings list and the reserved key equivalents for the owner |

## Question

How is web content exposed to assistive technologies through `NSAccessibility`? Two questions were handed to Step 5 for the owner: which settings the shell offers, including how the sensitive-site list and the JIT switches appear; and which key equivalents a page may never override.

## Requirements and constraints

- **IN9:** a cache in the browser, built after the first query, fed by renderer updates, and never asking a renderer synchronously.
- **PL12:** stable node identity, and geometry from fragments.
- **EM11:** objc2-defined classes, validated at the start of the shell milestone.
- **DP9:** no `ub-risk-2` crate on renderer data in the browser.
- **SI5, J1–J7:** sensitive sites and the JIT switches must be presented to the owner.
- **The threat model's browser-interface surface:** pages must not be able to spoof or trap the user.

## Alternatives considered

- **Synchronous queries to renderers.**
- **AccessKit's macOS adapter.**
- **Segmenting text in the browser.**
- **The proposal:** pushed trees, the browser's cache, text markers, and our own bridge.

## Historical context and modern developments

- **Text markers:** Chromium, WebKit and Firefox all implement them on macOS. SDK 27 documents the API publicly for "custom web browser engines".
- **AccessKit** takes its schema and push model from Chromium, but its adapters lack rich text and hypertext.
- **Chromium** reserves the tab, window and quit commands from pages.

Sourced in [accessibility](../architecture/accessibility.md) §3.

## Analysis

| | Proposal | Alternatives |
|---|---|---|
| Responsiveness | Answers from the cache on the main thread | Synchronous queries would block the browser's main thread |
| Web content's text | Text markers, with boundaries from the renderer | AccessKit lacks hypertext; segmenting in the browser violates DP9 |
| Cost when unused | None: built on first query | — |

## Evidence and sources

[Accessibility](../architecture/accessibility.md) §2–§4 and its Sources: Core-AAM 1.2 and HTML-AAM (both 2026-10-12), the engines' bridges, AccessKit's README, SDK 27's HIServices headers.

## Decision

**Recommended: accept AX1–AX12, with the owner's answers to the two questions.** In summary:
1. **Renderers build each document's tree** under HTML-AAM, Core-AAM, the accessible-name computation and WAI-ARIA, once an assistive technology asks. They push the whole tree, then updates (AX1–AX3, AX7).
2. **The browser answers every query from its cache.** Frames are stitched by token against its own records. Actions are routed as input (AX4, AX6).
3. **Text navigation uses macOS's text-marker API,** with word, sentence and line boundaries computed in the renderer (AX5).
4. **The bridge is Axiom's own,** in the shell's `platform` module (AX8).
5. **System accessibility settings become media features** (AX10).
6. **Settings** (§6.1 of the design), recommended:
   - the security group: the sensitive-site list with its reasons, a global and a per-site JIT switch, and the security level shown read-only;
   - privacy, per-site permissions, content, downloads, startup and search, each with its stated default.
7. **Reserved key equivalents** (§6.2), recommended: Chromium's reserved set plus ⌘L and ⌘,, reserved in fullscreen too, with Esc always leaving fullscreen. Every other key equivalent goes to the page first.

## Rationale

- The design follows IN9 and the precedent of three engines, and uses the API macOS now documents for web engines.
- The settings cover every switch the architecture already requires (SI5, J1–J7, NW12, SW6), and nothing speculative.
- Reserving ⌘L keeps the address bar reachable, against spoofing.

## Assumptions

- objc2-defined classes can implement `NSAccessibility` for a large, changing tree (validated first at the shell milestone, EM11).
- VoiceOver uses the documented text-marker attributes in SDK 27 as it does for other engines.

## Risks and unresolved issues

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| objc2-defined classes prove awkward for accessibility | Medium | Medium | EM11 validates first; a Swift shell can host the same bridge behind the message API (ADR-0009) |
| The cache's memory is large on big documents | Medium | Low | Built only on demand; bounded per document (AX12) |
| Reserved keys conflict with a web application the owner uses | Low | Low | The owner adjusts the reserved list |

## Conditions for revisiting

- AccessKit's macOS adapter gains rich text and hypertext. Reconsider it for the bridge.
- Apple changes the text-marker API.

## Consequences

- The shell milestone's first validation includes these accessibility classes (EM11).
- CP13 and CP15 expose ARIA, names, geometry and text runs to the tree builder. CP14 maps AX10's inputs to media features.
