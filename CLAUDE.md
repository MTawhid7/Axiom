# Project

Axiom is a browser engine designed from first principles for the owner's own Mac: a MacBook Air (M1, 4 performance and 4 efficiency cores, 8 GB) on macOS 27.2, with the Command Line Tools only (no Xcode). The repository is public at github.com/MTawhid7/Axiom (remote `origin`).

- **Phase Zero (research, requirements and architecture) is complete. Milestone 1, static documents, is under way,** authorized by the owner on 2026-10-23 (`docs/milestone-1-plan.md`). The engine's crates live in `crates/`; Phase Zero's throwaway spikes stay in `spikes/`, a workspace of their own.
- **Efficiency is a primary goal:** memory, CPU and power. This machine is the performance floor.
- **The owner is fluent in Rust.** Don't simplify code for learning reasons.

# Starting a session

1. Read `docs/status.md` first: current state, next steps and the session log. Then read `docs/decision-map.md`, and the current tracker, which names the next checkpoint: Milestone 1's, in `docs/milestone-1-plan.md` §9 (Phase Zero's steps kept theirs in `docs/research/step-N-*.md`).
2. Check the repository (`git status`, `git log`, the current branch) against them before trusting either.
3. The documents are the source of truth. Conversation history and memory are not.

# Authorization and review gates

- **Code needs its milestone's authorization.** Milestone 1 is authorized: its plan and MA10, as the owner approved them on 2026-10-23. Never write engine code beyond the authorized plan, add a dependency the register hasn't approved, or install a tool, without the owner's explicit authorization for that action. Each spike needs its own authorization, and covers only the questions that authorization names.
- **Stop at review gates** and wait for the owner:
  - approving a new milestone's or step's plan (commit it as CP1, report it, wait);
  - approving an architectural interface note before its code is written (ES27; Milestone 1's are at CP3, CP4, CP5 and CP11);
  - accepting an ADR (only the owner moves an ADR to Accepted);
  - approving a dependency, with its audit (DP15);
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
- **Record decisions where they belong.** The owner's words go in the relevant ADR's decision history and in the tracker. Each substantial session adds an entry to `docs/status.md`.

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
- `crates/`: the engine's crates, flat, in the workspace at the root (`docs/architecture/crates.md`). `tools/crate-map.toml` gives each its layer, processes, group, allowed dependencies, `unsafe` status and panic class.
- `spikes/`: throwaway validation code, one directory per spike, in a workspace of its own (`spikes/Cargo.toml`, `spikes/Cargo.lock`). Results go in `docs/research/sN-results.md`. See `spikes/README.md`.
- `supply-chain/` (cargo-vet's audits and policies) and `deny.toml` (cargo-deny's).
- `tools/`, each script printing its usage with `--help`:
  - `check.sh`, the one entry point to the checks (ES25);
  - the checkers it runs: `check-unsafe-policy.sh` with `unsafe-policy.txt`, `check-doc-links.py`, `check-dependencies.py` with `crate-map.toml`, and `check-file-size.py` with `file-size-allowlist.txt`;
  - `install-tools.sh` (cargo-vet and Miri) and `fetch-test-data.sh` (Milestone 1's pinned test data);
  - the audits' census, `unsafe-sites.py` (the compiler's) and `unsafe-census.py` (from sources);
  - measurement, analysis and signing scripts.
- `scratch/`: working files kept between sessions; git ignores it, so nothing there is published.

# Verification

**`tools/check.sh` must pass before every commit,** including commits that change only documents. This is the commit tier (ES25), budgeted at 3 minutes:
- `cargo fmt --check`, Clippy with `-D warnings` and the tests, for the engine's workspace and the spikes';
- the `unsafe` policy, and every relative Markdown link and anchor;
- the crate graph's layering and placement (CR16), and file sizes (ES1);
- `trace`'s recording costs, the TODOs' form (ES17), generated code fresh (ES19), and the tools' self-tests.

**`tools/check.sh --checkpoint` must pass before a checkpoint's last commit.** It adds `cargo doc` with `-D warnings`, Miri on every crate with `unsafe`, `cargo deny check`, `cargo vet` and the TODO list. The conformance suites, PF5's ratchet, loom, fuzzing and T1–T4 join as their checkpoints arrive; until then it lists them as pending.

**Read each check's own exit status,** never through a pipe that hides it.

# Code rules

These are the engineering standards' rules (`docs/technology/engineering-standards.md`) for the engine. The spikes keep the rules they were written under (`spikes/README.md`).

- **Toolchain:** Rust 1.94.1, pinned in `rust-toolchain.toml`; the 2024 edition; workspace resolver 3. Miri runs on `nightly-2026-10-21`, installed by `tools/install-tools.sh`, with its sysroot under `target/miri/`.
- **Lints:** the workspace lints apply:
  - Clippy pedantic, `missing_docs` and the standards' lints, which `-D warnings` makes errors;
  - `undocumented_unsafe_blocks`, `multiple_unsafe_ops_per_block` and `missing_safety_doc`, denied;
  - `clippy.toml`'s disallowed methods and types (ES13). A lint is suppressed only with `#[expect(lint, reason = "…")]`, never a bare `allow` (ES18).
- **`unsafe` policy** (ES11):
  - only in the platform crates' `src/sys/`, later `jit`, and `test-support`'s counting allocator, each listed in `tools/unsafe-policy.txt` with its justification;
  - listed crates declare `#![deny(unsafe_code)]`; every other crate root declares `#![forbid(unsafe_code)]`;
  - one unsafe operation per block, with a `// SAFETY:` comment naming a numbered invariant from its crate's `src/sys/mod.rs`;
  - parsers, network policy, IPC decoding, and image and font wrappers contain no `unsafe`;
  - macro-generated `unsafe` is part of the boundary.
- **Panics and errors:** a panic is a bug (ES9). Errors are typed values, written by hand (ES8). The crate map's `panic-free` and `checked-arithmetic` crates declare their class's lints (ES10).
- **Size:** a file warns at 600 lines and fails at 1,000 (ES1); a function has at most 100 lines (ES2).
- **Dependencies** are declared once, in `[workspace.dependencies]`, with only the features used (ES24, DP18).
- **Style:** write code like the surrounding code, in naming, comment density and idiom.

# Git workflow

- Each milestone runs on its own branch created from `main`: Milestone 1 on `milestone-1`. Phase Zero's steps ran on `step-N`.
- Commits are named `<scope>(m1-cpK): …`, where the scope is a crate, `docs`, `tools` or `workspace` (ES26). Phase Zero's were `docs(stepN-cpK): …` or `spikes(stepN-cpK): …`.
- Merge into `main` only when the owner says so, with `git merge --no-ff <branch>`. Write a multi-line merge message to a file: `git merge -F -` doesn't read standard input.
- Push a branch only once the owner has asked for it. After that, keep it pushed as checkpoints land. Push `main` only when told to.
- `target/` is ignored and never committed.

# Environment

- **Measurement** uses command-line tools only: `samply`, `/usr/bin/time -l`, `cargo bench` and Miri. Instruments needs Xcode, which isn't installed.
- **zsh shadows `log`.** Use `/usr/bin/log` for the unified log, for example to read sandbox denials.
- **Temporary files** go in the session's scratchpad, which can be wiped between sessions. **Files worth keeping go in the project** (the owner, 2026-10-20): reusable scripts in `tools/`, committed and printing their usage with `--help`; other working files in `scratch/`, which git ignores, so they are never published.
- **Avoid side effects outside the repository.** If a test leaves something behind, such as crash reports in `~/Library/Logs/DiagnosticReports`, clean it up and note it in the status log.
- **Web research:**
  - When an Apple documentation page renders empty, read its JSON form at `https://developer.apple.com/tutorials/data/documentation/<path>.json`.
  - For engine source, read raw GitHub files, and use GitHub API directory listings to find them.
