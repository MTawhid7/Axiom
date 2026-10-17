# Project

Axiom is a browser engine designed from first principles for the owner's own Mac: a MacBook Air (M1, 4 performance and 4 efficiency cores, 8 GB) on macOS 27.2, with the Command Line Tools only (no Xcode). The repository is public at github.com/MTawhid7/Axiom (remote `origin`).

- **Phase Zero: research, requirements and architecture.** The only code is throwaway validation spikes.
- **Efficiency is a primary goal:** memory, CPU and power. This machine is the performance floor.
- **The owner is fluent in Rust.** Don't simplify code for learning reasons.

# Starting a session

1. Read `docs/status.md` first: current state, next steps and the session log. Then read `docs/decision-map.md`, and the current step's tracker (`docs/research/step-N-*.md`), which names the next checkpoint.
2. Check the repository (`git status`, `git log`, the current branch) against them before trusting either.
3. The documents are the source of truth. Conversation history and memory are not.

# Authorization and review gates

- **Phase Zero is design-only.** Never write production engine code, create a project skeleton or install dependencies without the owner's explicit authorization for that specific milestone or action. Each spike needs its own authorization, and covers only the questions that authorization names.
- **Stop at review gates** and wait for the owner:
  - approving a new step's plan (commit it as CP1, report it, wait);
  - accepting an ADR (only the owner moves an ADR to Accepted);
  - authorizing code or a spike;
  - merging into `main`;
  - pushing a branch the owner hasn't asked to be pushed.
- **Between gates, keep going.** Run authorized checkpoints without asking for confirmation. Commit each one with every check green, and update the tracker.
- **Conditional approvals count once their condition is met.** For example, "accept the ADR once S4-Q1 passes". Record the evidence and quote the owner's words.
- **No architectural corners are cut** (charter principle 7). There is no deadline: never weaken a security boundary or the architecture for convenience. Stage the implementation, never the architecture.

# Communication & Workflow

- **Decision Reviews:** Whenever presenting review gates, open questions, or next steps to the user ('Decisions for you'), always provide your own explicit recommendation and concise rationale for each item to facilitate decision-making.
- **Milestone reports** end with a numbered list of the decisions only the owner can make, each with a recommendation as above.
- **Report outcomes faithfully.** Show failing output, name skipped steps, and state verified results plainly.
- **Record decisions where they belong.** The owner's words go in the relevant ADR's decision history and in the step tracker. Each substantial session adds an entry to `docs/status.md`.

# Writing the documents

- **Label every claim:** sourced fact (with a link), observed (on this Mac, naming the file or command), inference, hypothesis (with the step that will test it), or background. See `docs/research/README.md`.
- **Sources:** primary first: specifications, engine source and documentation, papers, Apple's documentation and SDK headers. Date each one, both when it was published and when it was checked. Mark secondary sources. Never invent history.
- **Keep distinct:** implemented and planned; standards-compliant and approximate.
- **Dates are absolute** (YYYY-MM-DD).
- **Numbered rules and IDs belong to their document** (for example PM, SI, IR, B and J in Step 3; AC and EL in Step 4). Check existing IDs before adding new ones.
- **Links:** every relative link must resolve, including `#anchors` (`tools/check-doc-links.py`). Name planned documents in code font until they exist.
- **Dependencies** each carry a written justification and a class: required platform facility, replaceable implementation detail, or foundational dependency.
- **ADRs** start from `docs/adr/0000-template.md` and keep a decision-history table.
- **Indexes:** a new document gets a row in `docs/README.md`, and research notes also one in `docs/research/README.md`.

# Repository layout

- `docs/`: status, charter, decision map and open questions, plus:
  - `adr/`;
  - `security/`: the threat model and process model;
  - `research/`: step trackers, research notes, spike results;
  - `technology/`;
  - `architecture/`, from Step 4.
- `spikes/`: throwaway validation code, one directory per spike. Results go in `docs/research/sN-results.md`. See `spikes/README.md`.
- `tools/`: the unsafe-policy checker (`check-unsafe-policy.sh`, `unsafe-policy.txt`), the link checker (`check-doc-links.py`), and measurement and signing scripts. Each prints its usage with `--help`.

# Verification

Five checks must pass before every commit, including commits that change only documents. The first four are the project's "four standard checks"; the fifth checks that every relative Markdown link in the repository resolves, anchors included:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
tools/check-unsafe-policy.sh
tools/check-doc-links.py
```

# Code rules

These apply to spikes now and carry over to engine code.

- **Toolchain:** Rust 1.94.1, pinned in `rust-toolchain.toml`; the 2024 edition; workspace resolver 3. Miri runs on nightly: `cargo +nightly miri test`.
- **Lints:** the workspace lints apply. That means Clippy pedantic, with `undocumented_unsafe_blocks`, `multiple_unsafe_ops_per_block` and `missing_safety_doc` denied.
- **`unsafe` policy** (`docs/technology/language-evaluation.md` §15.3):
  - Crate roots declare `#![forbid(unsafe_code)]`, unless the crate is listed in `tools/unsafe-policy.txt`.
  - Listed crates declare `#![deny(unsafe_code)]`, and allow `unsafe` only in their listed modules, such as `src/platform/`.
  - Every `unsafe` block has a `// SAFETY:` comment naming the numbered invariant it relies on.
  - Parsers, network policy, IPC decoding, and image and font wrappers contain no `unsafe`. FFI lives in `platform` modules.
  - Macro-generated `unsafe` is part of the boundary.
- **Style:** write code like the surrounding code, in naming, comment density and idiom.

# Git workflow

- Each Phase Zero step runs on its own branch, `step-N`, created from `main`.
- Checkpoint commits are named `docs(stepN-cpK): …` or `spikes(stepN-cpK): …`.
- Merge into `main` only when the owner says so, with `git merge --no-ff step-N`. Write a multi-line merge message to a file: `git merge -F -` doesn't read standard input.
- Push a branch only once the owner has asked for it. After that, keep it pushed as checkpoints land. Push `main` only when told to.
- `target/` is ignored and never committed.

# Environment

- **Measurement** uses command-line tools only: `samply`, `/usr/bin/time -l`, `cargo bench` and Miri. Instruments needs Xcode, which isn't installed.
- **zsh shadows `log`.** Use `/usr/bin/log` for the unified log, for example to read sandbox denials.
- **Temporary files** go in the session's scratchpad, never in the repository.
- **Avoid side effects outside the repository.** If a test leaves something behind, such as crash reports in `~/Library/Logs/DiagnosticReports`, clean it up and note it in the status log.
- **Web research:**
  - When an Apple documentation page renders empty, read its JSON form at `https://developer.apple.com/tutorials/data/documentation/<path>.json`.
  - For engine source, read raw GitHub files, and use GitHub API directory listings to find them.
