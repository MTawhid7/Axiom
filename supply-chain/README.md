# Supply chain

The engine's audits, recorded with [cargo-vet](https://mozilla.github.io/cargo-vet/) 0.10.2 under the [dependency policy](../docs/technology/dependency-policy.md)'s DP15, and checked by `tools/check.sh --checkpoint`. cargo-vet rewrites the files here and drops comments from them, so their reasons are written down here instead. The spikes' workspace is exempt (DP23).

| File | What it holds |
|---|---|
| `config.toml` | The imported audit sets, the mapping of Google's criteria, and each engine crate's policy |
| `audits.toml` | Axiom's criteria, and Axiom's own audits as they are made (DP8's grades among them) |
| `imports.lock` | The imported audits that apply to the engine's dependency tree, as fetched; `cargo vet --locked` uses this copy |

## Imports

DP15 imports the audits of the organizations Firefox imports, as Firefox's `supply-chain/config.toml` names them (last changed 2026-10-21, read 2026-10-23): the Bytecode Alliance (Wasmtime's audits), Embark Studios, Google (its aggregated set), ISRG (libprio-rs's audits) and Mozilla.

Google's set carries the soundness grades DP8 uses, from Google's [auditing standards](https://github.com/google/rust-crate-audits/blob/main/auditing_standards.md). cargo-vet keeps an imported set's own criteria in a private namespace, so `config.toml` maps Google's onto Axiom's: `ub-risk-0` to `ub-risk-4` onto the same names, Google's `-thorough` variants onto the plain grades, and `crypto-safe` and `does-not-implement-crypto` onto the same names. Other imported criteria are discarded.

## Criteria

`audits.toml` defines Google's criteria locally, as Google defines them: each grade implies the weaker ones (`ub-risk-0` implies `ub-risk-1`, and so on to `ub-risk-4`), and `does-not-implement-crypto` implies `crypto-safe` (DP3). cargo-vet's built-in `safe-to-run` and `safe-to-deploy` complete them.

## Policies

Each engine crate has a policy, which `tools/check-dependencies.py` keeps in step with its group and panic class in `tools/crate-map.toml` (rule DP15 of that tool). A policy applies to the crate and its whole dependency tree:

| The crate | Its policy | Why |
|---|---|---|
| A build or test crate | `safe-to-run` | DP15: code that runs here but never ships |
| A `panic-free` crate | `safe-to-deploy`, `ub-risk-1`, `crypto-safe` | It handles data from less trusted processes inside the browser, where a reused crate needs `ub-risk-1` (DP9, DP10) |
| Another `trusted` crate | `safe-to-deploy`, `ub-risk-2`, `crypto-safe` | DP9: `trusted` code may run in any process |
| A `sandboxed` crate | `safe-to-deploy`, `ub-risk-3`, `crypto-safe` | DP8: `ub-risk-3` only in sandboxed helpers, under DP8's conditions; `ub-risk-4` is never admitted |

## Audits

An audit names who made it and what was read (DP15), and the owner approves it with the dependency it admits. Axiom's own audits of `smallvec`, `vello_cpu`'s tree and `encoding_rs` are recorded here at M1-CP2, with their reports in `docs/research/`.
