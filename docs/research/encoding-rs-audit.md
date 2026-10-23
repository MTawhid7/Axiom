# Audit: `encoding_rs` 0.8.42

| | |
|---|---|
| **Status** | **Complete** (2026-10-21), for the owner's approval with the dependency (DP15). Performed by Claude (Opus 5.5) at the owner's request, for the architecture review's RV1-08. **Round 4** (the owner's dispositions, 2026-10-22; [the review](../review/README.md) §5.9): DP8 now admits `ub-risk-3` crates in sandboxed helpers on DP10's conditions, and `encoding_rs` is adopted at M1 behind TX3's interface (RV4-02, RV4-08). This grade stands; M1's plan adds the per-block audit of invariants and the fuzz target the amended DP8 asks for. **The per-block audit is §6** (2026-10-23, Milestone 1's second checkpoint, the plan's §2, item 14): every compiled `unsafe` block across the tree, `simdutf8` included, named with the invariant it relies on, and a differential test of both UTF-8 validators against the standard library. The fuzz target remains, at M1-CP5 |
| **Question** | Does the current release of `encoding_rs`, with its SIMD feature off, earn a soundness grade of `ub-risk-2` or better on Google's scale, so that DP8 admits it in place of Axiom's own encodings (TX3)? |
| **Answers** | The owner's disposition of RV1-08 (2026-10-21): "Audit and grade modern encoding_rs (SIMD off) before Milestone 1, and temporal_rs before Milestone 3" ([the review](../review/README.md) §5.1) |
| **Feeds** | [Text and fonts](../architecture/text-and-fonts.md) TX3 and §4; the [dependency register](../technology/dependency-register.md) §4; [ADR-0011](../adr/0011-text-fonts-and-unicode.md) |
| **Last updated** | 2026-10-23: §6, the per-block audit (M1-CP2). Earlier: 2026-10-21 |

## Summary

- **Grade: `ub-risk-3`** for `encoding_rs` 0.8.42 as it builds on this Mac (aarch64 with NEON, default feature `alloc`, `simd-accel` off). **DP8 doesn't admit it** (until Round 4 amended DP8, RV4-02, after which `encoding_rs` is adopted at M1), so TX3 stands: the encodings are Axiom's own, in safe Rust, behind Axiom's interface, with `encoding_rs` as the differential reference and fuzzing oracle.
- **Three reasons,** each enough on its own under Google's criteria [G2]:
  1. **Safety documentation covers 22 of the 143 `unsafe` blocks that compile** (15%). Google graded 0.8.33 `ub-risk-3` because it "Needs extensive safety comments" [G1]; that reason still holds.
  2. **A new pattern rests on an under-specified question of Rust's semantics:** an empty `asm!` block is used so that freshly written pages can be treated as an initialized `&mut [u8]` without zeroing them (`lib.rs`, around line 5180), where zeroing is a stable alternative.
  3. **SIMD can't be turned off on this Mac.** On aarch64 with NEON, 0.8.42 validates every input of 64 bytes or more with `simdutf8`'s NEON code, an unconditional dependency for that target. `simdutf8` 0.1.5 has no published soundness grade, and Mozilla's audit of its aarch64 path compared it with the SSE 4.2 path rather than reviewing it [M1].
- **What improved since 0.8.33** (observed): unsafe functions fell from 26 to 2, `ascii.rs` has no `unsafe` left, compiled `unsafe` blocks fell from 173 to 143, and `single_byte.rs` documents every block.
- **No unsound code was demonstrated.** The grade reflects documentation and reliance on unsettled semantics, as Google's criteria place them; it is not a found bug.

- **At M1-CP2** (§6, 2026-10-23): every compiled `unsafe` block across the tree is named with the invariant it relies on, which DP8 as amended requires. **Every block holds,** and four million random buffers find `encoding_rs`'s and `simdutf8`'s validators agreeing with the standard library on every one. The grade stays `ub-risk-3` (findings 1 and 3), and `simdutf8` 0.1.5, read in full, earns `ub-risk-2`, correcting finding 5's inference. Two further findings: one `&mut str` write lacks the panic guard its siblings use, and the `asm!` barrier is reached only from six `String` and `Vec` conveniences, which TX3's interface can avoid.

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

## 6. M1-CP2: every block and its invariant

**Why:** DP8, as Round 4 amended it, admits a `ub-risk-3` crate in sandboxed helpers only if "Axiom's audit names each `unsafe` block across its tree and the invariant the block relies on" (the [dependency policy](../technology/dependency-policy.md)). The plan's §2, item 14 schedules it for M1-CP2, before `encoding` is shaped.

**What was read** (observed, 2026-10-23): `encoding_rs` 0.8.42 and `simdutf8` 0.1.5 at the versions M1 resolves, with the features M1 enables: `encoding_rs` with `alloc` only, `simdutf8` with `aarch64_neon` and `public_imp` as `encoding_rs` asks, on `aarch64-apple-darwin`. **The census is the compiler's:** both crates built as path dependencies with `RUSTFLAGS=-Wunsafe_code`, so rustc reports every compiled site. **Confirmed with `--force-warn unsafe_code`,** which a crate's own `allow(unsafe_code)` can't lower as it lowers `-W` (the [`vello_cpu` audit](vello-cpu-audit.md)'s finding 8): the same counts, by [`tools/unsafe-sites.py`](../../tools/unsafe-sites.py). **`encoding_rs`: 145 sites,** 143 blocks, one `unsafe impl` and one `unsafe fn`, as Round 1's tool counted. **`simdutf8`: 56,** of which only 4 are blocks: the crate is in the 2018 edition, so each `unsafe fn` body is unsafe throughout, and the review covers those bodies too. Each site was read with the code that establishes its precondition.

### 6.1 The invariants

**`encoding_rs`'s** (named here; E7, E8 and E9 are stated in its source, the rest only inferable from it):
- **E1, lookahead in UTF-8.** The UTF-8 loops read `read + 1` to `read + 3` only while `read + 4 <= src.len()`, which every entry to an inner loop checks; their tails check `new_read > src.len()` before reading.
- **E2, the lookup table.** `UTF8_DATA.table` has 384 entries, indexed by a byte plus 0x80, at most 383.
- **E3, UTF-16 output.** `convert_utf8_to_utf16_up_to_invalid` writes at `written < dst.len()`: `ascii_to_basic_latin` returns a non-ASCII byte only with room left for one unit, each write is followed by a check of `written == dst.len()`, and a surrogate pair first checks `written + 1 == dst.len()`.
- **E4, UTF-8 output.** `convert_utf16_to_utf8_partial_inner` checks `written + 4 <= dst.len()` before each code point, which writes at most four bytes; the comment says the bounds checks are elided on purpose.
- **E5, a `&str` source.** `Utf8Source` and `convert_str_to_utf16` read a `&str`, so a lead byte's continuation bytes exist; `convert_str_to_utf16` asserts `dst.len() >= src.len()`, and UTF-16 never needs more units than UTF-8 has bytes.
- **E6, scalar values.** `char::from_u32_unchecked` gets a BMP unit outside 0xD800–0xDFFF, a high and a low surrogate each checked in range and combined into 0x10000–0x10FFFF, or a sequence decoded from a `&str` (E5).
- **E7, handles.** A destination's `write_code_unit` writes at `pos` because `check_space_*` handed out the handle only with room: `pos < len` for a BMP unit, `pos + 1 < len` for UTF-16's surrogate pair, `pos + 3 < len` for UTF-8's four bytes (the source: "OK, because we checked before handing out a handle").
- **E8, unaligned UTF-16.** `UnalignedU16Slice` holds a pointer valid for `2 * len` bytes (documented); `at` asserts `i < len` and `tail` asserts `from <= len`; it is built only over `min(src.len() / 2, …)` units. Its raw pointer carries no lifetime ("XXX the return value should be restricted not to outlive self"), so each use stays inside its source slice's borrow, which every caller does.
- **E9, single-byte tables.** The 128-entry tables are indexed by a non-ASCII byte minus 0x80, and the unchecked accesses run under `converted < length`, the shorter buffer's length; `single_byte.rs` documents each block.
- **E10, UTF-8 validity.** `from_utf8_unchecked`, `as_mut_vec`, `as_bytes_mut` and `String::from_utf8_unchecked` rely on two things: that `utf8_valid_up_to`, `ascii_valid_up_to` and `iso_2022_jp_ascii_valid_up_to` are right, which the differential test below checks for the first; and that the decoders write valid UTF-8 up to `written`, with at most `MAX_STRIDE_SIZE` bytes of garbage after it for ASCII-compatible decoders, which the `&mut str` writers zero, along with orphaned continuation bytes. A `scopeguard` zeroes the whole slice if a decoder panics.
- **E11, spare capacity.** `minimally_init` writes one byte per page and passes the pointer to an empty `asm!`, so the compiler must treat the spare capacity as initialized; the caller asserts `old_len + written <= capacity()` before `set_len`, trusting the decoder to have overwritten every byte up to `written` (Round 1's finding 3).

**`simdutf8`'s** (inferred; its public `unsafe` functions document only U1):
- **U1, NEON.** Every intrinsic needs NEON, which is part of `aarch64-apple-darwin`'s baseline; `encoding_rs` calls the NEON validator under `cfg(all(target_arch = "aarch64", target_feature = "neon"))` and documents it.
- **U2, whole chunks.** Full 64-byte chunks are read only below `len - len % 64`; `SimdInput::new` reads 64 bytes from a slice its callers guarantee holds them, a precondition stated nowhere; a tail is copied, fewer than 64 bytes, into a zeroed, aligned 64-byte buffer before it is read; `check_block` indexes a four-element array by literal indices.
- **U3, a correct validator.** `from_utf8` and `from_utf8_mut` build a `&str` after validation, so `simdutf8`'s algorithm must accept exactly valid UTF-8, as must the NEON validator `encoding_rs` relies on in E10.

### 6.2 Every block

Structurally identical blocks are listed together, as Google's standard allows ("ditto") [G2].

| File | Lines | What the blocks do | Invariant | Verdict |
|---|---|---|---|---|
| `utf_8.rs` | 81 | `simdutf8`'s NEON validator, for inputs of 64 bytes or more | U1 (documented), U2, U3 | Holds |
| `utf_8.rs` | 201, 209, 221, 222, 234, 248, 249, 250, 264 | `utf8_valid_up_to`'s inner loop: unchecked reads | E1 | Holds |
| `utf_8.rs` | 224, 253, 315, 406, 448, 529 | Lookups in `UTF8_DATA.table` | E2 | Holds |
| `utf_8.rs` | 373, 389, 403, 404, 424, 443, 444, 445, 471 | `convert_utf8_to_utf16_up_to_invalid`: unchecked reads | E1 | Holds |
| `utf_8.rs` | 377, 391, 415, 429, 459, 460, 473 | The same function's unchecked UTF-16 writes | E3 | Holds |
| `utf_8.rs` | 748, 758, 775, 792, 811 | `convert_utf16_to_utf8_partial_inner`: each code point's unchecked writes | E4 | Holds |
| `handles.rs` | 90, 107, 140, 753, 1113 | `UnalignedU16Slice`: its constructor, `at`, `tail`, and two constructions | E8 (documented) | Holds |
| `handles.rs` | 611, 952 | `write_code_unit` for UTF-16 and UTF-8 destinations | E7 (documented) | Holds |
| `handles.rs` | 1166, 1176, 1212, 1268, 1338 | `char::from_u32_unchecked` on UTF-16 units and pairs | E6 | Holds |
| `handles.rs` | 1471, 1478, 1485, 1512, 1561, 1608, 1662 | `char::from_u32_unchecked` in `Utf8Source`, over a `&str` | E5, E6 | Holds |
| `lib.rs` | 3144, 3238, 3250 | `from_utf8_unchecked` on a fully validated input | E10 | Holds |
| `lib.rs` | 3163, 3263 | `as_mut_vec`, extended by the validated prefix (documented) | E10 | Holds |
| `lib.rs` | 4064, 4186 | `decode_to_str` and its variant: `as_bytes_mut` under a zeroing guard (documented) | E10 | Holds |
| `lib.rs` | 4121, 4136, 4242, 4258 | `decode_to_string` and its variant: spare capacity, then `set_len` (documented) | E10, E11 | Holds, on Round 1's finding 3 |
| `lib.rs` | 4772, 4822 | Encoding into a `Vec<u8>`'s spare capacity, then `set_len` (documented) | E11 | Holds, on Round 1's finding 3 |
| `lib.rs` | 5186, 5192, 5207, 5215 | `minimally_init` and `pointer_escapes`: the empty `asm!` barrier, then `&mut [u8]` over spare capacity (documented) | E11 | Round 1's finding 3 |
| `single_byte.rs` | 60, 187, 259 | Table lookups by a non-ASCII byte (documented) | E9 | Holds |
| `single_byte.rs` | 196, 215, 225, 238 | The single-byte decoder's unchecked reads and writes (documented) | E9 | Holds |
| `single_byte.rs` | 430, 454, 516, 527, 539 | The single-byte encoder's unchecked reads and writes (documented) | E9 | Holds |
| `single_byte.rs` | 463 | A surrogate pair into a `char`, its range worked out in the comment | E6 | Holds |
| `mem.rs` | 446–599 (25 blocks) | `is_utf8_bidi`'s inner loop: unchecked reads and table lookups | E1, E2 | Holds |
| `mem.rs` | 649–787 (17 blocks) | The same function's tail, after `new_read > src.len()` checks | E1, E2 | Holds |
| `mem.rs` | 1240, 1259, 1260, 1269, 1270, 1271 | `convert_str_to_utf16`: unchecked reads of a `&str` | E5 | Holds |
| `mem.rs` | 1242, 1247, 1264, 1276, 1277 | The same function's unchecked writes | E5 | Holds |
| `mem.rs` | 1416 | `convert_utf16_to_str_partial`: `as_bytes_mut` under a zeroing guard (documented) | E10 | Holds |
| `mem.rs` | 1554 | `convert_latin1_to_str_partial`: `as_bytes_mut` **without** the zeroing guard | E10 | **Holds only while the conversion can't panic** (finding 6) |
| `mem.rs` | 1696, 1718, 1724, 1769 | `decode_latin1` and `encode_latin1_lossy`: validated or converted output, spare capacity (documented) | E10, E11 | Holds, on Round 1's finding 3 |
| `simdutf8` `compat.rs`, `basic.rs` | 80, 96; 37, 52 | `from_utf8` and `from_utf8_mut` after validation | U3 | Holds |
| `simdutf8` `aarch64/mod.rs` | 7, 17, 26, 36 | Dispatch to the NEON validators, with a scalar fallback under 64 bytes | U1 | Holds |
| `simdutf8` `aarch64/neon.rs` | 18–239 (19 methods) | Thin wrappers over NEON intrinsics on registers; `load_from` copies 16 bytes from its caller's pointer; `vld1q_u8` loads from a local array | U1, U2 | Holds |
| `simdutf8` `algorithm.rs` | 13–184 | The check's arithmetic on vector values, and `check_block`'s literal indices | U1 | Holds |
| `simdutf8` `algorithm.rs` | 214, 270, 279 | The basic and compat validators: whole chunks, then a copied tail | U2 | Holds |
| `simdutf8` `algorithm.rs` | 365–541 | The streaming validators of `public_imp`, which `encoding_rs` doesn't call: copies bounded by 64-byte buffers | U2 | Holds |
| `simdutf8` `helpers.rs` | 41, 48 | The tail's copy, fewer than 64 bytes, unaligned 8 bytes at a time | U2 | Holds |
| `simdutf8` `basic.rs` | 118–191 | `unsafe fn`s declared by the `public_imp` traits, documented as needing the CPU features | U1 | Holds |

### 6.3 The differential test

Both validators against `std::str::from_utf8` (observed, 2026-10-23, a scratch crate built in release with debug assertions and overflow checks): **4,000,000 random buffers** of 1 to 400 bytes, built from valid scalar values of every length, a third of them mostly ASCII so that the SIMD paths fall out of and back into ASCII. A quarter had one fault spliced in at a random position, from lone continuation bytes, overlongs, surrogates, values above U+10FFFF, impossible leads and truncated sequences; a quarter were cut at a random byte. **1,137,634 were invalid,** 619,379 of them failing at byte 64 or later, inside the SIMD chunks or across their boundaries. `encoding_rs`'s `utf8_valid_up_to` and `simdutf8`'s NEON validator returned the standard library's `valid_up_to` on every buffer. A test is evidence, not proof; the fuzz target at M1-CP5 continues it.

### 6.4 Findings at M1-CP2

6. **A `&mut str` written without the zeroing guard** (observed, `mem.rs` 1554). `convert_latin1_to_str_partial` writes into `dst.as_bytes_mut()` directly, where its siblings put the slice under a `scopeguard` that zeroes it if a conversion panics. It is sound only while `convert_latin1_to_utf8_partial` can't panic halfway through, which this review found no way for it to do, but nothing enforces. It matters only where panics unwind: Axiom's release builds abort (RV1-16), so a panic there ends the process before the `str` is used.
7. **The `asm!` barrier is avoidable** (observed). `minimally_init` (Round 1's finding 3) is reached only from `decode_to_string`, `decode_to_string_without_replacement`, encoding into a `Vec<u8>` with or without replacement, `decode_latin1` and `encode_latin1_lossy`. The slice-based functions (`decode_to_utf8`, `decode_to_utf16`, `encode_from_utf8` and their kin) never reach it. **A condition of use:** TX3's interface, in the `encoding` crate, calls the slice-based functions into buffers Axiom owns and has initialized, and never the six conveniences.
8. **`simdutf8` 0.1.5 graded `ub-risk-2`,** correcting finding 5's inference. Read in full on this target: its public `unsafe` functions document their one precondition, the CPU feature; the internal preconditions are informal (`SimdInput::new`'s 64 bytes), which `ub-risk-2` allows ("Unsafe blocks may rely on informal invariants") [G2]; every caller upholds them; and the differential test agrees with the standard library. Mozilla's `safe-to-deploy` covers 0.1.5 [M1].
9. **The rest of the tree** (observed): `cfg-if` 1.0.5 has no `unsafe` (`ub-risk-0`); `scopeguard` 1.2.0 carries Google's `ub-risk-2` at this version [G1]; `multiversion_no_op` 1.0.0 is a 43-line attribute that returns the function it annotates unchanged, with no `unsafe` in it or in what it emits; `rustversion` 1.0.23 runs only in `encoding_rs`'s build script, so DP12 asks of it `safe-to-run`, not a grade (Google's audits reach 1.0.20), at the adoption.

### 6.5 The grade, and DP8's conditions

**`encoding_rs` 0.8.42 stays `ub-risk-3`:** findings 1 and 3 stand, since 22 of the 143 blocks carry a safety comment and the invariants above are mostly inferred, and the `asm!` barrier rests on an argument about inline assembly's semantics where zeroing would be a stable alternative. No block was found unsound. **DP8's conditions:**
- **sandboxed helpers only,** in the `sandboxed` group, which `encoding` and its dependents carry in `tools/crate-map.toml`;
- **no better-graded crate meets the need:** Round 4's finding RV4-02;
- **accepted by the owner:** "Adopt encoding_rs at Milestone 1 under the amended DP8." (RV4-08, 2026-10-22);
- **every block and its invariant named:** this section;
- **a libFuzzer target under AddressSanitizer through TX3's interface:** at M1-CP5, before any legacy encoding is decoded (the plan's §2, item 14);
- **conditions of use** from this section: finding 7's slice-based functions only.

## Sources

All read on 2026-10-21.

- [C1] crates.io, the versions of `encoding_rs` and `simdutf8`, with their publication dates and checksums — https://crates.io/api/v1/crates/encoding_rs/versions ; https://crates.io/api/v1/crates/simdutf8/versions
- [G1] Google, `audits.toml` (google/supply-chain, main): `encoding_rs` 0.8.33 `ub-risk-3`, `scopeguard` 1.2.0 `ub-risk-2`, `cfg-if` 1.0.0 `ub-risk-0`, `rustversion` to 1.0.20 `ub-risk-0` — https://github.com/google/supply-chain/blob/main/audits.toml
- [G2] Google, *Auditing Standards* (google/rust-crate-audits, main), the definitions of `ub-risk-0` to `ub-risk-4` and of thorough reviews — https://github.com/google/rust-crate-audits/blob/main/auditing_standards.md
- [M1] Mozilla, `audits.toml` (mozilla/supply-chain, main): `encoding_rs` 0.8.31 and 0.8.32, `simdutf8` 0.1.5 — https://github.com/mozilla/supply-chain/blob/main/audits.toml
- **Observed:** the three crates under `target/review-r1/` (not committed), and `tools/unsafe-census.py`'s output above.
