# Audit: `encoding_rs` 0.8.42

| | |
|---|---|
| **Status** | **Complete** (2026-10-21), for the owner's approval with the dependency (DP15). Performed by Claude (Opus 5.5) at the owner's request, for the architecture review's RV1-08. **Round 4** (the owner's dispositions, 2026-10-22; [the review](../review/README.md) §5.9): DP8 now admits `ub-risk-3` crates in sandboxed helpers on DP10's conditions, and `encoding_rs` is adopted at M1 behind TX3's interface (RV4-02, RV4-08). This grade stands; M1's plan adds the per-block audit of invariants and the fuzz target the amended DP8 asks for |
| **Question** | Does the current release of `encoding_rs`, with its SIMD feature off, earn a soundness grade of `ub-risk-2` or better on Google's scale, so that DP8 admits it in place of Axiom's own encodings (TX3)? |
| **Answers** | The owner's disposition of RV1-08 (2026-10-21): "Audit and grade modern encoding_rs (SIMD off) before Milestone 1, and temporal_rs before Milestone 3" ([the review](../review/README.md) §5.1) |
| **Feeds** | [Text and fonts](../architecture/text-and-fonts.md) TX3 and §4; the [dependency register](../technology/dependency-register.md) §4; [ADR-0011](../adr/0011-text-fonts-and-unicode.md) |
| **Last updated** | 2026-10-21 |

## Summary

- **Grade: `ub-risk-3`** for `encoding_rs` 0.8.42 as it builds on this Mac (aarch64 with NEON, default feature `alloc`, `simd-accel` off). **DP8 doesn't admit it** (until Round 4 amended DP8, RV4-02, after which `encoding_rs` is adopted at M1), so TX3 stands: the encodings are Axiom's own, in safe Rust, behind Axiom's interface, with `encoding_rs` as the differential reference and fuzzing oracle.
- **Three reasons,** each enough on its own under Google's criteria [G2]:
  1. **Safety documentation covers 22 of the 143 `unsafe` blocks that compile** (15%). Google graded 0.8.33 `ub-risk-3` because it "Needs extensive safety comments" [G1]; that reason still holds.
  2. **A new pattern rests on an under-specified question of Rust's semantics:** an empty `asm!` block is used so that freshly written pages can be treated as an initialized `&mut [u8]` without zeroing them (`lib.rs`, around line 5180), where zeroing is a stable alternative.
  3. **SIMD can't be turned off on this Mac.** On aarch64 with NEON, 0.8.42 validates every input of 64 bytes or more with `simdutf8`'s NEON code, an unconditional dependency for that target. `simdutf8` 0.1.5 has no published soundness grade, and Mozilla's audit of its aarch64 path compared it with the SSE 4.2 path rather than reviewing it [M1].
- **What improved since 0.8.33** (observed): unsafe functions fell from 26 to 2, `ascii.rs` has no `unsafe` left, compiled `unsafe` blocks fell from 173 to 143, and `single_byte.rs` documents every block.
- **No unsound code was demonstrated.** The grade reflects documentation and reliance on unsettled semantics, as Google's criteria place them; it is not a found bug.

## 1. What was read

**The crates** (observed, 2026-10-21): `encoding_rs` 0.8.42 (published 2026-09-24) and 0.8.33 (2023-08-23), and `simdutf8` 0.1.5 (2024-09-22), each downloaded from crates.io into its own directory under `target/review-r1/`, its SHA-256 checked against crates.io's index [C1], and unpacked. Nothing was built or run: neither crate's build script, procedural macros nor tests.

**The method:**
- **A census** of the `unsafe` each configuration compiles, with [`tools/unsafe-census.py`](../../tools/unsafe-census.py), added for this audit. It evaluates `#[cfg]` attributes and `cfg_if!` branches for aarch64 with NEON and the features given, leaves out tests and modules the root gates out, and counts safety comments before or inside each block.
- **A review by pattern,** as Google's standard allows for "similar-looking unsafe blocks" [G2]: each category of block, with representative sites read in full: `utf_8.rs`'s validator (lines 74–160 and 195–270), `handles.rs`'s unaligned reads and writes (lines 95–125, 750–760, 950–957), and `lib.rs`'s string writes and initialization (lines 4060–4090, 5150–5235). **It is a standard review, not a thorough one.** Google: "If the code is known to be riskier than `ub-risk-2`, then the thoroughness of the review is not consequential" [G2].
- **The published audits** of `encoding_rs` and its dependencies, from Google, Mozilla and Chromium [G1, M1].

## 2. The tree on this Mac

From `Cargo.toml` of 0.8.42 (observed): edition 2024, `rust-version` 1.88.

| Crate | Why it is in the tree | Published grade |
|---|---|---|
| `cfg-if` 1.0 | Always | `ub-risk-0` (Google) [G1] |
| `scopeguard` 1 | Always, no default features: it restores a `String`'s bytes if decoding panics | `ub-risk-2` at 1.2.0 (Google) [G1] |
| `multiversion_no_op` 1.0.0 | Always: `lib.rs` imports its `multiversion` attribute off x86 (line 772), where it presumably does nothing (inference from its name; not read) | None found |
| `rustversion` 1.0.19 | Build dependency of the build script, which sets the cfg `slow_mm_packus_epi16` for some compiler versions | `ub-risk-0` through 1.0.20 (Google) [G1] |
| **`simdutf8` 0.1.5** | **Always on aarch64, x86 and wasm32 with SIMD,** with the features `aarch64_neon` and `public_imp` | **None.** Mozilla: `safe-to-deploy`, with "Audit of aarch64 and Wasm is by comparing the code with the SSE 4.2 case" [M1] |
| `core_detect`, `multiversion` | x86 only | Not on this target |
| `any_all_workaround`, `rustversion` | With `simd-accel` only | Not in this configuration |
| `serde` | With `serde` only | Not in this configuration |

**`simdutf8`'s call site** (observed, `utf_8.rs` lines 74–86): under `cfg(all(target_arch = "aarch64", target_feature = "neon"))`, inputs of 64 bytes or more go to `simdutf8::compat::imp::aarch64::neon::validate_utf8`, with the comment "The cfg check above ensures that the precondition, NEON availability on aarch64, is satisfied." NEON is part of the aarch64 macOS baseline (background), so this path is always taken here.

**`simdutf8` 0.1.5's aarch64 path** (observed, counted by `grep -c unsafe`): `implementation/aarch64/neon.rs` (20 sites, 244 lines), the shared `implementation/algorithm.rs` (25 sites, 578 lines), and a few in `implementation/mod.rs`, `helpers.rs` and `compat.rs`: about 50 `unsafe` sites, intrinsics and chunked pointer loads, with two safety comments, both in `algorithm.rs`.

## 3. The census

`python3 -I tools/unsafe-census.py <crate> --features alloc` (aarch64 and NEON by default), observed 2026-10-21:

| | 0.8.33 | 0.8.42 |
|---|---|---|
| `unsafe` blocks that compile | 173 | **143** |
| … with a safety comment before or inside | 0 | **22** |
| `unsafe` functions | 26 | **2** |
| Files with `unsafe` | `ascii.rs`, `handles.rs`, `lib.rs`, `mem.rs`, `single_byte.rs`, `utf_8.rs` | `handles.rs`, `lib.rs`, `mem.rs`, `single_byte.rs`, `utf_8.rs` |
| Blocks by first operation | `get_unchecked` 103, other calls 43, raw pointers 22, `from_utf8_unchecked` 5 | `get_unchecked` 103, other calls 31, `from_utf8_unchecked` 5, raw pointers 2, `from_raw_parts` 1, `asm!` 1 |

With `simd-accel` on, 0.8.42 compiles 149 blocks, 28 of them documented, and 5 unsafe functions.

## 4. Findings

1. **Bounds checks elided on undocumented invariants** (103 blocks; observed). The validator in `utf_8.rs` reads `src.get_unchecked(read + 1)` to `read + 3`, relying on `read + 4 <= src.len()`, which each "Next lead" branch checks before continuing the inner loop (lines 195–270). The invariant holds as far as this review followed it, but nothing states it, and the same shape repeats across `mem.rs`'s conversion loops and the decoders. Under Google's criteria, `ub-risk-2` needs documentation that is "relatively comprehensive"; `ub-risk-3` is where it "may not be adequately comprehensive or precise" and blocks "rely on under-documented or inferred invariants" [G2].
2. **Writes into `String`s** (4 blocks; observed, `lib.rs` around 4064 and 4186). `as_bytes_mut` hands the decoder a `String`'s bytes, and a `scopeguard` zeroes them if a crate-internal bug panics mid-write, so the `String` stays UTF-8. Documented, and sound as far as read.
3. **Uninitialized pages as `&mut [u8]`** (2 blocks and an `unsafe fn`; observed, `lib.rs` lines 5150–5235). To avoid zeroing a buffer, the code writes one byte per page so the pages are mapped, then passes the pointer to an empty `asm!("/* {0} */", in(reg) ptr)`, which the comment describes as "the safe Rust story for the `asm!` block", citing a 2026-03-13 post on inline assembly. It then makes a `&mut [u8]` over memory Rust never saw initialized. The comment adds that on targets without `asm!`, and under Miri, the code instead relies on "materializing `&mut [u8]` to unitialized memory isn't UB after all", citing a forum thread. That reliance on unsettled semantics is Google's `ub-risk-3` description, "may rely on specific interpretations of undefined behavior that are under-specified" [G2], and a stable alternative exists: zeroing the buffer. The `asm!` template is empty, so no machine code runs; it is an optimization barrier, not the assembly DP8 means.
4. **Unaligned 16-bit reads** (2 blocks; observed, `handles.rs` lines 95–125). `UnalignedU16Slice::at` asserts the index, then copies two bytes into a `MaybeUninit<u16>`. Documented and sound as far as read.
5. **SIMD that the feature doesn't control** (observed, §2). Disabling `simd-accel` leaves `simdutf8`'s NEON validator in place on aarch64. For DP10, a reused parser needs `ub-risk-2` or better "across its whole tree", and `simdutf8` 0.1.5 has no grade; its aarch64 code has two safety comments across about 50 sites, so on the evidence of documentation alone it couldn't be graded `ub-risk-2` either (inference).

**History of the grade** (sourced): Google graded 0.8.33 `ub-risk-3`: "Needs extensive safety comments", pointing to encoding_rs pull request 101 [G1]. Mozilla's 0.8.31 audit, by the crate's author, records two caveats: "The crate does things that are documented to be UB but that do not appear to actually be UB due to integer types differing from the general rule" (encoding_rs issue 79), and re-reviewing "the code that reinterprets buffers of integers as SIMD vectors" (issue 87) [M1].

## 5. Grade and consequences

**`ub-risk-3`**, by findings 1, 3 and 5; finding 1 alone repeats Google's reason for 0.8.33.

**Consequences:**
- **DP8 bars 0.8.42,** so [TX3](../architecture/text-and-fonts.md) keeps the encodings Axiom's own, in safe Rust. As the owner's disposition reclassified them (DP2), they sit behind an interface Axiom owns, so a later release could replace them inside one crate.
- **`encoding_rs` stays in the `test` group:** WPT's `encoding` tests and differential fuzzing against it (TS7) are what pay down the logic risk RV1-08 raised, that new decoders add state-machine errors a fuzzed production decoder has already fixed.
- **Reconsidered** when a release documents its `unsafe` comprehensively, drops the uninitialized-memory reliance or makes it optional, and lets aarch64 builds leave out `simdutf8`, or when `simdutf8` earns `ub-risk-2`. Each would be a new audit (DP15).
- **The owner approves this audit with the dependency** (DP15). It is recorded in the [register](../technology/dependency-register.md) §4.

## Sources

All read on 2026-10-21.

- [C1] crates.io, the versions of `encoding_rs` and `simdutf8`, with their publication dates and checksums — https://crates.io/api/v1/crates/encoding_rs/versions ; https://crates.io/api/v1/crates/simdutf8/versions
- [G1] Google, `audits.toml` (google/supply-chain, main): `encoding_rs` 0.8.33 `ub-risk-3`, `scopeguard` 1.2.0 `ub-risk-2`, `cfg-if` 1.0.0 `ub-risk-0`, `rustversion` to 1.0.20 `ub-risk-0` — https://github.com/google/supply-chain/blob/main/audits.toml
- [G2] Google, *Auditing Standards* (google/rust-crate-audits, main), the definitions of `ub-risk-0` to `ub-risk-4` and of thorough reviews — https://github.com/google/rust-crate-audits/blob/main/auditing_standards.md
- [M1] Mozilla, `audits.toml` (mozilla/supply-chain, main): `encoding_rs` 0.8.31 and 0.8.32, `simdutf8` 0.1.5 — https://github.com/mozilla/supply-chain/blob/main/audits.toml
- **Observed:** the three crates under `target/review-r1/` (not committed), and `tools/unsafe-census.py`'s output above.
