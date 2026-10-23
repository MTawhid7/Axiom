# Audit: `vello_cpu` 0.3.0 and its tree

| | |
|---|---|
| **Status** | **Complete** (2026-10-23), for the owner's approval with the dependency (DP15). Performed by Claude (Opus 5.5) at Milestone 1's second checkpoint, as [the plan](../milestone-1-plan.md)'s §2, item 13 asks |
| **Question** | What soundness grade, on Google's scale (DP8), does each crate that `vello_cpu` 0.3.0 compiles into M1's renderer earn, and so: on what conditions may the rasterizer the owner adopted (TX12) run there? |
| **Answers** | The plan's §2, item 13 ("`vello_cpu`'s grade … rests on its dependencies, `smallvec` among them"), approved by the owner on 2026-10-23; the owner's adoption of `vello_cpu` "subject to crate dependency grading at the text milestone" ([dependency register](../technology/dependency-register.md) §3) |
| **Feeds** | The dependency register §3; `tools/crate-map.toml` at M1-CP4; `supply-chain/audits.toml` |
| **Last updated** | 2026-10-23 |

## Summary

- **Every crate in the tree is graded, and only `smallvec` falls below `ub-risk-2`.** It was graded `ub-risk-3` in [its own audit](smallvec-audit.md), and the plan admits it in renderers under DP8's conditions. The 18 crates that compile into M1's renderer, as this Mac builds them (aarch64, M1's features):

  | Grade | Crates |
  |---|---|
  | `ub-risk-0`: no `unsafe` | `vello_cpu`, `vello_common`, `kurbo`, `polycool`, `guillotiere`, `linebender_resource_handle`, `thiserror` |
  | `ub-risk-1` | `num-traits` (Google's grade at this version), `euclid` (Google's grade at 0.22.11, carried forward by a delta audit), `peniko` and `color` (Axiom's) |
  | `ub-risk-2` | `bytemuck` and `arrayvec` (Google's grades, carried forward by delta audits); `log`, `foldhash`, `fearless_simd` and `hashbrown` (Axiom's) |
  | `ub-risk-3` | `smallvec` |
- **No unsoundness was found in any of the 17 crates graded here.** The two largest are:
  - **`hashbrown` 0.17.1,** read in full: 224 blocks and 102 other sites. Probes of the paths M1 reaches and of its panic paths pass Miri under both aliasing models, and pass AddressSanitizer on the NEON path that Miri can't reach (§4.6). The standard library ships this exact version on the nightly Axiom uses for Miri, and 0.16.1 on the pinned stable (observed).
  - **`fearless_simd` 0.7.0:** all 27 blocks, its `unsafe trait` and its 42 impls (§4.5).
- **Consequences** (§6):
  - The whole tree runs in the renderer, in DP9's `sandboxed` group, since `smallvec` is in it.
  - The register's row for `vello_cpu`, which read "`trusted` in the renderer … Neither graded", becomes graded and `sandboxed`.
  - Axiom's audits go into `supply-chain/audits.toml` now. `safe-to-deploy` is decided with the crates' adoption at M1-CP4, as for `smallvec` and `encoding_rs`.
  - CP4 also needs DP12's `safe-to-run` audits of the build-time code: seven crates and `thiserror`'s build script, of which imports cover only `quote`. It needs a decision too: `cargo metadata` lists five crates that M1 doesn't compile (finding 7), and `tools/check-dependencies.py` reads `cargo metadata`.
- **The census method was corrected during this audit** (finding 8). `-W unsafe_code` is lowered by a crate's own `allow`, and `peniko` and `color` carry one on their `unsafe` modules. The census now uses `--force-warn` and a committed tool, [`tools/unsafe-sites.py`](../../tools/unsafe-sites.py). The `smallvec` and `encoding_rs` counts are unchanged.

## 1. What was read

**The crates** (observed): the 18 crates `vello_cpu` 0.3.0 compiles, at the versions the spikes locked; publication dates are from crates.io's API [C1]. `vello_cpu` 0.3.0, published 2026-10-02, is the newest release on 2026-10-23. The sources come from cargo's registry, which checked each one against the lock's checksum. Each was unpacked into its own directory in the session's scratchpad.

**The configuration** (observed): a scratch manifest outside the repository with M1's renderer dependencies:
- `vello_cpu` 0.3.0 with `std` and `u8_pipeline`, as S5 and S6 used it;
- HarfRust 0.14.0, `skrifa` 0.48.0 and `read-fonts` 0.45.1;
- ICU4X's segmenter, properties and normalizer 2.3.0;
- `encoding_rs` 0.8.42.

Each audited crate was patched in as a path dependency. `cargo tree -e normal --target aarch64-apple-darwin` shows the features cargo's resolver unifies:
- `bytemuck`: `derive`, `extern_crate_alloc` and `min_const_generics`;
- `hashbrown`: `default-hasher` only;
- `foldhash`: none;
- `fearless_simd`: `std`;
- `peniko` and `color`: `bytemuck` and `std`;
- `euclid`: `std`, without `bytemuck`.

**The census** (observed): `RUSTFLAGS="--force-warn unsafe_code" cargo check --message-format=json --target aarch64-apple-darwin` on that manifest, read by `tools/unsafe-sites.py`. rustc reports every `unsafe` site it compiles in this configuration, with `cfg`s evaluated and macros expanded (§3). `--force-warn` matters (finding 8).

**The reviews.** Each is a standard review in Google's sense [G1]: each line of `unsafe` code looked at and its patterns reasoned about, with similar-looking blocks allowed to be skimmed. They come in three kinds:
- **imports:** where Google grades the exact version (`num-traits`), its audit is imported;
- **delta audits:** where Google grades an earlier version (`euclid`, `arrayvec`, `bytemuck`), the diff to M1's version is read with its context. Google's guideline is that a delta audit "should never result in lowering the `UB-risk-N` value" [G1], so these carry Google's grade forward or fail;
- **full audits:** everywhere else, every compiled site is read with the code that establishes its preconditions.

**Dynamic checks** (observed):
- Miri on `nightly-2026-10-21`, under Stacked Borrows and Tree Borrows (`-Zmiri-tree-borrows`): six probes of `hashbrown`, in a scratch crate that depends only on these copies of `hashbrown` and `foldhash` (Appendix A).
- The same probes natively under AddressSanitizer (`-Zsanitizer=address`).
- One layout probe for `bytemuck`'s nightly-only impl (§4.2).

**The scope is this Mac's build.** Code for other targets, such as `fearless_simd`'s x86 and WebAssembly backends and `hashbrown`'s SSE2 group, wasn't read. Neither was code behind features M1 doesn't enable. Each grade below is for aarch64-apple-darwin with M1's features, and each audit's notes say so, as the `encoding_rs` audit's do.

## 2. The tree on this Mac

| Crate | Version (published) | Reached through | Prior grades [G2] [M1] [B1] [I1] |
|---|---|---|---|
| `vello_cpu` | 0.3.0 (2026-10-02) | Axiom's `text`, and `paint` through it ([crate architecture](../architecture/crates.md)), at M1-CP4 | None |
| `vello_common` | 0.3.0 (2026-10-02) | `vello_cpu` | None |
| `bytemuck` | 1.25.2 (2026-07-19) | `vello_cpu`, `vello_common`, `peniko`, `color` | Google `ub-risk-2` through 1.22.0 |
| `hashbrown` | 0.17.1 (2026-05-09) | `vello_cpu` | No grade. Mozilla `safe-to-deploy` deltas 0.15.2 → 0.17.1 |
| `foldhash` | 0.2.0 (2025-08-23) | `hashbrown` | Google `ub-risk-2` through 0.1.5. Mozilla `safe-to-deploy` 0.1.5 → 0.2.0 |
| `fearless_simd` | 0.7.0 (2026-08-11) | `vello_common` | None |
| `peniko` | 0.6.1 (2026-05-15) | `vello_common` | None |
| `color` | 0.3.3 (2026-05-05) | `peniko` | None |
| `kurbo` | 0.13.1 (2026-05-13) | `peniko` | None |
| `arrayvec` | 0.7.8 (2026-07-02) | `kurbo`, `polycool` | Google `ub-risk-2` at 0.7.6 |
| `polycool` | 0.4.0 (2025-10-23) | `kurbo` | None |
| `smallvec` | 1.16.3 (2026-10-10) | `vello_common`, `peniko`, `kurbo` | Axiom `ub-risk-3` ([audit](smallvec-audit.md)) |
| `linebender_resource_handle` | 0.1.1 (2025-09-16) | `peniko` | None |
| `guillotiere` | 0.7.0 (2026-03-18) | `vello_common` | None |
| `euclid` | 0.22.14 (2026-03-18) | `guillotiere` | Google `ub-risk-1` at 0.22.11 |
| `num-traits` | 0.2.19 (2024-05-03) | `euclid` | **Google `ub-risk-1` at 0.2.19** |
| `log` | 0.4.34 (2026-08-22) | `vello_common` | Google `ub-risk-1` at 0.4.20, `ub-risk-2` from 0.4.22 through 0.4.26 |
| `thiserror` | 2.0.21 (2026-09-23) | `vello_common` | None |

**Build-time code, under DP12** (observed, `cargo tree`):
- the procedural macros `bytemuck_derive` 1.12.1 and `thiserror-impl` 2.0.21, with `syn` 3.0.9, `quote` 1.0.47, `proc-macro2` 1.0.107 and `unicode-ident` 1.0.27;
- `autocfg` 1.5.1, for `num-traits`'s build script;
- `thiserror`'s own build script, which compiles a probe with the current `rustc` and writes a module and `cfg`s to `OUT_DIR`.

DP12 asks `safe-to-run` of each when it enters the lock, at M1-CP4. Imported audits cover `quote` 1.0.47, through Google to 1.0.40 and Mozilla's deltas to 1.0.47. For the rest the imports either stop short or don't exist:
- `proc-macro2`: Mozilla to 1.0.106;
- `unicode-ident`: Google to 1.0.18;
- `thiserror` and `thiserror-impl`: ISRG to 2.0.18;
- `syn` 3, `bytemuck_derive` and `autocfg` 1.5: none.

So Axiom's own `safe-to-run` audits are due for those at CP4.

**What `cargo metadata` adds** (observed; finding 7): `glifo` 0.4.0 as a dependency of `vello_cpu`, with `skrifa` 0.44.0, `read-fonts` 0.41.0, `font-types` 0.12.7 and `once_cell` 1.21.4 under it; `raw-entry` on `hashbrown`; and `std` on `foldhash`. `cargo tree`, which runs cargo's feature resolver, shows none of these compiled.

## 3. The census

`python3 -I tools/unsafe-sites.py MESSAGES.json --crate …`, with the messages of the build in §1 (observed, 2026-10-23). The safety-comment column is the tool's heuristic: "safety" within six lines before a block or in a comment inside it.

| Crate | Blocks | With a safety comment | `unsafe fn`s | `unsafe` methods (declared / implemented) | `unsafe impl`s | `unsafe trait`s |
|---|---|---|---|---|---|---|
| `hashbrown` 0.17.1 | 224 | 70 | 1 | 2 / 83 | 15 | 1 |
| `bytemuck` 1.25.2 | 92 | 28 | 18 | 0 / 1 | 133 | 9 |
| `smallvec` 1.16.3 | 43 | 11 | 1 | 0 / 9 | 6 | 1 |
| `arrayvec` 0.7.8 | 37 | 3 | 2 | 1 / 8 | 2 | 0 |
| `fearless_simd` 0.7.0 | 27 | 5 | 0 | 0 / 0 | 42 | 1 |
| `foldhash` 0.2.0 | 7 | 6 | 2 | 0 / 0 | 1 | 0 |
| `log` 0.4.34 | 3 | 0 | 2 | 0 / 0 | 0 | 0 |
| `num-traits` 0.2.19 | 1 | 1 | 0 | 0 / 0 | 0 | 0 |
| `peniko` 0.6.1 | 0 | 0 | 0 | 0 / 0 | 28 | 0 |
| `color` 0.3.3 | 0 | 0 | 0 | 0 / 0 | 22 | 0 |
| The other eight | 0 | — | 0 | 0 / 0 | 0 | 0 |

The heuristic undercounts:
- `log`'s comments explain two of its three blocks without using the word.
- Many of `hashbrown`'s `raw.rs` blocks follow a long `// SAFETY:` comment more than six lines up, or rest on a `# Safety` section on their function. A looser one-off count, a safety comment anywhere above the block in its function or a `# Safety` contract on it, finds 119 of 224: 100 of 146 in `raw.rs`, 7 of 27 in `map.rs`, 4 of 32 in `table.rs` and none of the NEON group's 7 (observed, a script in the session's scratchpad).

## 4. Crate by crate

### 4.1 Seven crates without `unsafe`

`vello_cpu` and `vello_common` declare `#![forbid(unsafe_code)]`. `kurbo` 0.13.1, `polycool` 0.4.0, `guillotiere` 0.7.0, `linebender_resource_handle` 0.1.1 and `thiserror` 2.0.21 declare nothing. None has the word `unsafe` anywhere in its Rust sources, tests, examples, benches and `cfg`-disabled code included, except a doc comment in `linebender_resource_handle`'s `blob.rs` ("this function is not unsafe") (observed, `grep -rnw unsafe`). Google's `ub-risk-0` lets automation reject any `unsafe` text, with exceptions for comments [G1], so these are `ub-risk-0`. `thiserror`'s build script is DP12's concern, not a grade (§2).

### 4.2 Google's grades, carried to M1's versions

- **`num-traits` 0.2.19:** Google's `ub-risk-1` at this exact version, "a single line of float-to-int unsafe with decent safety comments" [G2]. The census finds that one block, at `cast.rs` line 284, expanded by a macro. Read here too, it holds: each of its four callers checks the range first, and NaN and the infinities fail those comparisons. Google's audit is imported.
- **`euclid` 0.22.11 → 0.22.14:** Google's `ub-risk-1` at 0.22.11 [G2].
  - **The diff's only new `unsafe`** (observed, `diff -ru`) is a pair of impls for the new `ScaleOffset2D`, both under `cfg(feature = "bytemuck")`: `Zeroable` and `Pod`.
  - **They hold.** The struct is `#[repr(C)]` with four fields of type `T` and a `PhantomData<(Src, Dst)>`, so it has no padding: same-sized fields, and a zero-sized one of alignment 1. `Copy` is implemented for `T: Copy`, and the `Pod` impl requires `Src` and `Dst` to be `'static`. That is the pattern of the 40 other impls Google reviewed at 0.22.11.
  - **Not compiled here,** since M1 leaves `euclid`'s `bytemuck` feature off.

  Carried forward: `ub-risk-1`.
- **`arrayvec` 0.7.6 → 0.7.8:** Google's `ub-risk-2` at 0.7.6 [G2]. The 125-line diff (observed) adds:
  - a safe `spare_capacity_mut` that returns `&mut self.xs[len..]`;
  - lifetime elision on `Drain`;
  - `LenUint = u16` on 16-bit targets;
  - documentation;
  - one fix: in `extend`'s zero-sized branch, `mem::forget(elt)` now stops a value from being dropped twice. That is a soundness improvement.

  Carried forward: `ub-risk-2`.
- **`bytemuck` 1.22.0 → 1.25.2:** Google's `ub-risk-2` chain through 1.22.0 [G2]. The raw diff of `src/` is 8,216 lines because 1.22.0's files have CRLF line endings and 1.25.2's don't. Ignoring whitespace it is 344 lines (observed, `diff -ruwB`). The new `unsafe`:
  - **`NoUninit` for arrays of `char`, `bool` and each `NonZero` integer** (`min_const_generics`, **compiled here**). Each element type is already `NoUninit`, and arrays have no padding between elements. Holds.
  - **`AnyBitPattern` for `[MaybeUninit<T>; N]` where `T: AnyBitPattern`** (`zeroable_maybe_uninit`, not compiled). As the existing `MaybeUninit<T>` impl, per element. Holds.
  - **`ZeroableInOption` for function pointers** of arity 0 to 13 and the Rust, C, system and unwinding ABIs (compiled). The all-zero `Option<fn…>` is `None` for any argument types and any ABI, which the standard library's `Option` documentation guarantees [S1]. Holds.
  - **`Pod` and `Zeroable` for `core::simd::Simd<T, N>`** (nightly only, `nightly_portable_simd`, not compiled), rewritten without the `LaneCount<N>: SupportedLaneCount` bound that portable SIMD removed in favour of a post-monomorphization lane limit [U1] [U2]. The impls are sound only if `Simd<T, N>` has no padding for every lane count the limit admits. On `nightly-2026-10-21` here, `Simd<f32, 3>` is 12 bytes with alignment 4, `Simd<u8, 3>` 3 bytes and `Simd<u8, 5>` 5 bytes (observed, a probe printing `size_of` and `align_of`), so the lanes are packed. Holds on this toolchain.
  - **The rest:** `core::error::Error` impls, a `#[doc(hidden)]` module of safe helpers for the derive macros, AVX-512 types moved from a nightly feature to `avx512_simd` (x86 only), and documentation.

  Carried forward: `ub-risk-2`.

### 4.3 `log` 0.4.34 and `foldhash` 0.2.0, read in full

**`log` 0.4.34** (observed, `lib.rs`; the 2021 edition; no features enabled), five sites:
- **`set_max_level_racy` and `set_logger_racy`:** `unsafe fn`s, each with a `# Safety` section ("only safe to call when it cannot race with any other calls").
- **`max_level` (line 1438):** transmutes the stored `usize` to `LevelFilter`, which is `#[repr(usize)]` with discriminants 0 to 5. Its comment explains why that is sound: the private atomic starts at 0 (`Off`), and its only writers store `level as usize`. Holds.
- **`set_logger_inner` (line 1529):** writes the `static mut LOGGER` only after its compare-and-exchange has moved the state from `UNINITIALIZED` to `INITIALIZING`, so exactly one thread writes. It publishes the write with a `Release` store of `INITIALIZED`. Holds, but without a safety comment.
- **`logger` (line 1629):** reads `LOGGER` only after an `Acquire` load sees `INITIALIZED`. Its comment explains the ordering. Holds.

Google graded 0.4.20 `ub-risk-1` and 0.4.22 `ub-risk-2`, noting "one exception" to its documentation [G2]. **Axiom's grade: `ub-risk-2`,** for the sound but uncommented write.

**`foldhash` 0.2.0** (observed; no features, so no `std`), ten sites:
- **The global seed storage is unchanged from 0.1.x** but for its length: an `UnsafeCell` behind a three-state `AtomicU8` used as a spin lock. It has three sites:
  - **`unsafe impl Sync`:** the cell is written only in the exclusive `LOCKED` state and read only in the permanent `INIT` state;
  - **the write:** after a successful `compare_exchange_weak` (`Acquire`), followed by a `Release` store of `INIT`;
  - **the read:** in `GlobalSeed::get`, through a zero-sized token whose only constructor ensures `INIT`.

  Google's reviewers of 0.1.3 found nothing wrong. One called it "an interesting approach" and another would have used `LazyLock` where `std` is available [W1]. They graded the crate `ub-risk-2`.
- **New since 0.1.5:** `hash_bytes_long`, an `unsafe fn` whose `# Safety` contract is a length over 16, and `load`, an `unsafe fn` whose contract is `offset + 8 <= len`. Its body reads an unaligned `u64`.
  - **The 256-byte loop** loads up to offset 96 while the length exceeds 256.
  - **The 128-byte loop** loads up to offset 64 while it exceeds 128.
  - **The tail** loads at `len - 16` through `len - 64` only behind `len >= 32`, `64` and `96` checks, with `len` at least 17 by the contract.
  - **The one caller** checks `len > 16`.

  Every load is in bounds. Holds.

**Axiom's grade: `ub-risk-2`.** The code is documented, and the reading found no defect. It stays at Google's grade because the spin lock's subtlety is what Google's grade weighed.

### 4.4 `peniko` 0.6.1 and `color` 0.3.3: trait impls only

Both crates' `unsafe` is `bytemuck` impls, in an `impl_bytemuck.rs` that permits `unsafe_code` for itself: with `allow` in `peniko` and `expect` in `color`. Each impl has a `// Safety:` comment, checked against its type (observed):
- **`peniko`, 28 impls:** `NoUninit`, `Zeroable`, `CheckedBitPattern` and `Contiguous` on each of seven enums: `Compose`, `Extend`, `Fill`, `ImageAlphaType`, `ImageFormat`, `ImageQuality` and `Mix`. Each enum is `#[repr(u8)]`, fieldless, with explicit discriminants from 0 to its last variant, without a gap: 0–13, 0–2, 0–1, 0–1, 0–1, 0–2 and 0–15. So zero is a valid value (`Zeroable`). `MIN_VALUE` is the variant at 0 and `MAX_VALUE` the last (`Contiguous`). `*bits <= MAX_VALUE` accepts exactly the valid values (`CheckedBitPattern`).
- **`color`, 22 impls:**
  - `Pod`, `Zeroable` and `TransparentWrapper` for `AlphaColor`, `OpaqueColor` and `PremulColor`. Each is `#[repr(transparent)]` over `[f32; N]` with a `PhantomData<CS>`, where `ColorSpace: Clone + Copy + 'static`.
  - `Pod` and `Zeroable` for `Rgba8` and `PremulRgba8`, each `#[repr(C)]` with four `u8` fields.
  - `TransparentWrapper<T>` for `CacheKey<T>`, which is `#[repr(transparent)]`.
  - The four enum traits for `ColorSpaceTag` and `HueDirection`. `ColorSpaceTag` declares `Aces2065_1 = 15` out of order among 0 to 14, which still leaves 0–15 without a gap; `HueDirection` is 0–3.

**Axiom's grade: `ub-risk-1` for each.** The contiguity that `Contiguous` and `CheckedBitPattern` rely on is checked by `const` blocks in each crate's tests, compiled only under `cfg(test)`. So a later version that adds a variant with a gap would need a fresh review, which DP14's delta audit of each update provides.

### 4.5 `fearless_simd` 0.7.0

On aarch64 the crate compiles its NEON backend. That backend is generated, by the crate's own `fearless_simd_gen` according to its header (observed). The census finds 27 blocks, one `unsafe trait` and 42 impls.

- **`SimdPod` (`transmute.rs`):** an `unsafe trait` with a `# Safety` section: implementors are `Copy` and have no pointers or padding, and every bit pattern is valid. It is `pub`, but its module is private, so no other crate can name it.
- **Its impls:**
  - the ten integer and float scalars;
  - `[T; N]`;
  - the 30 NEON vector types and their `x2` and `x4` tuples, which are `#[repr(C)]` structs of vectors;
  - 30 `Aligned128`, `Aligned256` and `Aligned512` wrappers, from one macro. Each wrapper is `#[repr(C, align(16 | 32 | 64))]` around a single field, and the macro asserts at compile time that the wrapper is no larger than what it wraps, so no trailing padding.

  Holds.
- **Four checked conversions** (`checked_transmute_copy`, `checked_transmute_store`, `checked_cast_ref` and `checked_cast_mut`), each the crate's one route to its operation. They `const`-assert equal sizes, and the casts also assert source alignment at least the destination's. Over `SimdPod` types, every resulting value is valid, and every write keeps the source valid. Holds; each has a safety comment.
- **The `kernel!` macro** calls an inner `#[target_feature]` function. Its safety comment says the SIMD token proves the features. Tokens come from `Neon::assume_supported`, a `#[target_feature(enable = "neon")]` function whose `# Safety` section requires NEON, or from detection. Holds.
- **`Level::baseline`** (`lib.rs` line 673) makes the NEON token under `cfg(target_feature = "neon")`, so NEON is statically on. Holds, without a comment.
- **The generated backend's 21 blocks:**
  - `vectorize` calls its NEON-enabled inner function with the token in hand;
  - ten pairs of `vld4q_*` and `vst4q_*` read and write through `&[T; 4·L]` and `&mut [T; 4·L]`, where `L` is the lane count: exactly the 64 bytes the instruction touches, at the element alignment the reference guarantees.

  Holds. They have no comments, but they are structurally identical.

**Axiom's grade: `ub-risk-2`.** The reading found no defect, and the hand-written machinery is well documented. It isn't `ub-risk-1` because the generated blocks are uncommented and the token discipline is documented only in places.

### 4.6 `hashbrown` 0.17.1

**What compiles** (observed): `alloc.rs`'s no-defaults allocator, since neither `nightly` nor `allocator-api2` is on; `control/` with the NEON group, chosen by `cfg_if!` for aarch64 with NEON and not Miri; `raw.rs`; and the `map`, `set` and `table` APIs. `raw_entry`, `rustc_entry`, `rayon` and `serde` are gated out.

The crate is in the 2024 edition, with `unsafe_op_in_unsafe_fn` set to warn in its manifest, so every unsafe operation is inside an explicit block, even in `unsafe fn` bodies. The 326 sites are:
- 224 blocks;
- 83 `unsafe` methods implemented, plus 2 declared and 1 `unsafe fn`, whose contracts are the preconditions;
- 15 `unsafe impl`s;
- the crate-private `unsafe trait Allocator`.

**`vello_cpu`'s use** (observed, `render.rs`): one `HashMap<u32, Arc<Pixmap>>`, an image registry, with `insert`, `get`, `remove` and `clear`. The grade covers the whole crate as compiled.

**The invariants** (named here; `raw.rs`'s comments and `# Safety` sections state most of them):
- **H1, layout.** One allocation: `[padding][T_{n−1} … T_0][C_0 … C_{n−1}][C_0 … C_{W−1}]`, where:
  - `n = bucket_mask + 1` is a power of two;
  - `W` is the group width, 8 with NEON;
  - `ctrl` points at `C_0`;
  - bucket `i`'s element ends `i` elements before `ctrl`.

  `calculate_layout_for` computes it with checked arithmetic and an `isize::MAX` bound, and it always has `n + W` control bytes.
- **H2, control bytes.** Each `C_i` is `EMPTY`, `DELETED` or full (top bit clear, seven bits of hash). Bucket `i` holds an initialized `T` exactly when `C_i` is full.
  - The trailing `W` bytes mirror the first `W`, since `set_ctrl` writes both.
  - When `n < W`, the mirror sits at `W + i`, and `C_n … C_{W−1}` stay `EMPTY`.
- **H3, counts.** `items` counts the full buckets, and `growth_left` the insertions left within the load factor. Every allocated table keeps at least one `EMPTY` bucket, so every probe ends. Insertion reserves first.
- **H4, the empty singleton.** A table with `bucket_mask == 0` points at a static group of `W` `EMPTY` tags, which is never written: `growth_left == 0` forces an allocation before any insert, and `clear_no_drop` skips it. Lookups read it safely.
- **H5, buckets.** A `Bucket<T>` points one past its element in a live table; for zero-sized types it holds the index plus one, as a bare address. It is valid until the table is resized, rehashed or freed. The public types tie it to a borrow of the table: `PhantomData` lifetimes on iterators, and `&mut` in entries.
- **H6, panics.** An element moves or drops only after its control byte says so:
  - `erase` clears the byte before dropping, and `replace_bucket_with` removes before calling the closure;
  - guards cover `clear`, `clone`, `clone_from`, resizing (the new table is freed, the old one intact), in-place rehashing (elements not yet rehashed are dropped) and draining (the table is left empty).

  A panic can leak, never drop twice.
- **H7, NEON.** A group loads `W` bytes at any position up to `bucket_mask`, and stores aligned at multiples of `W` below `n`. H1's `n + W` bytes keep every such access in bounds. NEON is in aarch64-apple-darwin's baseline.
- **H8, the allocator.** `alloc` and `dealloc` get the layout `calculate_layout_for` computes. Its size is never zero (`n + W ≥ 9`), and deallocation recomputes the same layout from `bucket_mask`.

**Every site, by area** (observed; each read with the code that establishes its precondition):

| Area | Sites | What it relies on | Verdict |
|---|---|---|---|
| `alloc.rs`: the private `Allocator` trait and `Global` | 7 | H8. Only `Global` implements the trait, since no other crate can name it. `alloc`'s non-zero size isn't commented | Holds |
| `util.rs`: `invalid_mut` | 1 | Transmutes an integer to a pointer, which the standard library calls "a largely unspecified operation" whose pointer may be used for zero-sized accesses only [S2]. `hashbrown` uses it only for zero-sized buckets and their dangling pointers. `ptr::without_provenance_mut`, stable since Rust 1.84, is the documented route (finding 1) | Holds |
| `scopeguard.rs`: `into_inner` | 1 | Reads the value and the closure out of a `ManuallyDrop` once each | Holds |
| `control/`: `bitmask.rs`, `tag.rs`, `group/neon.rs` | 12 | H7 for the NEON group's `load`, `load_aligned` and `store_aligned`. The other intrinsics touch only registers. A byte-swapped non-zero word stays non-zero (32-bit Arm only, though compiled). `fill_tag` writes within its slice | Holds |
| `raw.rs`: `Bucket` | 22 | H1 and H5, with `# Safety` sections on each `unsafe` method. `as_ptr` is a safe method whose pointer arithmetic needs a live bucket (finding 1) | Holds |
| `raw.rs`: layout and allocation | 21 | H1, H8. `new_uninitialized`'s path for an oversized allocation isn't taken with `Global`, which returns the size requested | Holds |
| `raw.rs`: probing and control bytes | 45 | H1–H4 and H7. `fix_insert_index` handles the false match small tables get from their `EMPTY` padding, given H3's free bucket. `ctrl_slice` is safe but needs a table that isn't the singleton (finding 1) | Holds |
| `raw.rs`: insert, find, remove, `replace_bucket_with`, `get_disjoint_*` | 24 | H2, H3, H5, H6. `insert_at_index` takes only an index `find_or_find_insert_index` returned, with no change in between. `get_disjoint_mut` refuses duplicates; the `unchecked` form documents that it doesn't check | Holds |
| `raw.rs`: growth | 32 | H1–H3, H6. `resize_inner` copies bitwise into a guarded new table and frees the old allocation only after the swap. `rehash_in_place`'s guard drops the elements still marked `DELETED` | Holds (probes 2 and 3) |
| `raw.rs`: `clear`, `clone`, `clone_from`, drop | 23 | H2, H6. A freshly allocated table has `items == 0` until its control bytes are copied, so a panic in between never reads uninitialized control bytes | Holds (probes 4 and 5) |
| `raw.rs`: iterators (`RawIterRange`, `RawIter`, `FullBucketsIndices`, `RawIntoIter`, `RawDrain`, `RawIterHash`, `RawExtractIf`) | 45 | H1, H2, H5, H6. The item count stops reading before the end. Small tables read only `EMPTY` padding past it. `RawDrain` swaps an empty table into the map and writes the drained one back on drop | Holds |
| `raw.rs`: `Send` and `Sync` for `RawTable`, the iterators and `Bucket` | 9 | `RawTable` and its owning iterators require `T: Send` or `T: Sync`, as `Vec` does. `RawIterRange` and `Bucket` are unconditional, and the public iterators' `PhantomData` sets the real bounds | Holds |
| `map.rs` | 39 | H5: iterators borrow the map, and entries hold `&mut`. `insert` uses the bucket or index `find_or_find_insert_index` returned. The `unchecked` methods are `unsafe` with documented contracts. `IterMut` is `Send` for `K: Send`, as `slice::IterMut` is | Holds |
| `set.rs` | 6 | `replace` borrows the key from an `UnsafeCell` so it can move the key while the entry still holds the borrow. The entry functions it calls don't read the key again | Holds (probe 6) |
| `table.rs` | 39 | `HashTable`'s API, built from the same patterns as `map.rs`. `UnsafeIter` yields raw pointers, whose dereference is the caller's `unsafe` | Holds |

**The dynamic checks** (observed; the probes are in Appendix A):
- **Six probes,** each passing natively, under Miri with Stacked Borrows and with Tree Borrows, and under AddressSanitizer, with no report:
  1. `vello_cpu`'s use, through several resizes, tombstones and refills;
  2. a hasher that panics in the middle of a resize: every element survives;
  3. a hasher that panics in the middle of an in-place rehash, asserted to be in place: the table stays consistent;
  4. `clone` and `clone_from` with a panicking `Clone`;
  5. `clear` with a panicking `Drop`;
  6. `retain`, `extract_if`, a partial `drain` and a partial `into_iter`, `replace_entry_with` with a panicking closure, `HashSet::replace` and `get_disjoint_mut`.
- **A live-object counter** checks that no element is dropped twice or leaked where the code promises not to leak.
- **Miri can't reach the NEON group:** `hashbrown` selects its portable group under `cfg(miri)` (finding 2). The native AddressSanitizer run covers the NEON loads and stores, for memory errors but not aliasing.

**The standard library's copy** (observed): `library/Cargo.lock` in the rust-src of `nightly-2026-10-21` pins `hashbrown` 0.17.1, and Rust 1.94.1's pins 0.16.1. So this code, with the standard library's features, is the `HashMap` every Rust program built with that nightly uses. Mozilla's `safe-to-deploy` deltas cover 0.15.2 through 0.17.1 [M1]. Neither is a soundness grade, but both are evidence of scrutiny that a reading alone doesn't give.

**Axiom's grade: `ub-risk-2`.** No defect was found. `raw.rs`'s documentation is thorough: most blocks are explained by a safety comment or a `# Safety` contract, and its layout diagrams match the code. Below `ub-risk-1` because of:
- informal internal invariants, such as safe internal functions with unstated preconditions (finding 1);
- wrappers with few comments;
- a review as large as this one ("it may not be good enough for a crate to have sound unsafe code if that unsafe code is too difficult to confidently review" [G1]).

## 5. Findings

1. **`hashbrown`: internal safe functions with unstated preconditions** (observed). The crate is sound in use; none of these is reachable wrongly from its public API.
   - **`RawTableInner::ctrl_slice`** builds a `&mut [MaybeUninit<Tag>]` of `bucket_mask + 1 + W` bytes. On the empty singleton, that is one byte more than the static it points to, and the static is read-only. All three callers (one a test) exclude the singleton.
   - **`Bucket::as_ptr`** offsets a pointer that must be live.
   - **`offset_from`** has no `# Safety` section.
   - **`util::invalid_mut`** makes pointers by transmuting integers, which the standard library leaves largely unspecified and rustc 1.94.1 warns about (`integer_to_ptr_transmutes`). It is sound for the zero-sized accesses `hashbrown` makes, and its own `FIXME` anticipates `ptr::without_provenance_mut`.
   - **`prev_pow2`'s documentation** calls passing zero "undefined behavior". In fact that is a shift overflow, a panic in debug builds and a wrong value in release ones, and no caller can pass zero.
2. **Miri covers `hashbrown`'s portable group, not the NEON one** (observed, `control/group/mod.rs`). The NEON path's evidence is the reading and AddressSanitizer, which finds out-of-bounds accesses but not aliasing violations.
3. **`log`: one uncommented block** (§4.3), sound by the state machine around it.
4. **`foldhash` 0.2.0's new unaligned reads**, which Google's grade predates, are each bounded by a length check (§4.3).
5. **`bytemuck`'s nightly `Simd` impls** depend on portable SIMD's layout, which packs odd lane counts on this nightly (§4.2). If a later nightly pads them, those impls become unsound, but only for crates that enable `nightly_portable_simd`.
6. **`peniko`'s and `color`'s enum impls** depend on discriminants without gaps, which only their tests check (§4.4); verified for these versions.
7. **`cargo metadata` lists crates that don't compile** (observed): `glifo` 0.4.0, with `skrifa` 0.44.0, `read-fonts` 0.41.0, `font-types` 0.12.7 and `once_cell` 1.21.4 under it, and features (`hashbrown`'s `raw-entry`, `foldhash`'s `std`) that `cargo tree` shows are off. **The cause is a weak dependency feature** (observed): `vello_cpu`'s `std` feature names `glifo?/std`. With that one entry removed from a copy of the manifest, `cargo metadata` drops `glifo`. So it resolves a weak feature as though the dependency were enabled, along with the features `glifo` asks of `hashbrown` (`raw-entry`) and `foldhash`, and `cargo tree` doesn't. Consequences at M1-CP4, when `vello_cpu` enters the lock:
   - `tools/check-dependencies.py`, which reads `cargo metadata`, would ask DP22 for register entries for five crates that are never built, and would count `skrifa` and `read-fonts` twice. Its placement checks already over-approximate on purpose (CR4).
   - Whether `cargo-vet` and `cargo-deny` see the same graph is to be checked then.
8. **The census method had a gap, now closed** (observed). rustc gives a crate's own lint attributes precedence over `-W unsafe_code` on the command line, and `peniko`'s and `color`'s `unsafe` modules carry one (`allow` and `expect`). So the first census, with `-W`, missed their 50 impls. `--force-warn unsafe_code`, which no attribute can lower, finds them.
   - **Rerun with `--force-warn`,** the counts for `smallvec`, `encoding_rs` and `simdutf8` are unchanged, and those audits now say so.
   - **The method is a committed tool,** `tools/unsafe-sites.py`, with a self-test.
   - **`tools/unsafe-census.py`,** which reads sources without building, now reports a `cfg` it can't evaluate and counts it as compiled, instead of crashing on `hashbrown`'s macro definitions.
9. **`vello_common` logs through `log`, and Axiom installs no `log` logger.** So its records stop at one relaxed atomic load. Whether to bridge them into `trace`'s log (OB) is a choice for M1-CP4's wiring (inference: they are warnings of misuse, useful in development).

## 6. Grades and consequences

| Crate | Grade | Basis | Recorded in `supply-chain/audits.toml` |
|---|---|---|---|
| `vello_cpu` 0.3.0, `vello_common` 0.3.0, `kurbo` 0.13.1, `polycool` 0.4.0, `guillotiere` 0.7.0, `linebender_resource_handle` 0.1.1, `thiserror` 2.0.21 | `ub-risk-0` | No `unsafe` (§4.1) | Axiom's, each with `does-not-implement-crypto` |
| `num-traits` 0.2.19 | `ub-risk-1` | Google, this version | Imported when it enters the lock |
| `euclid` 0.22.14 | `ub-risk-1` | Google 0.22.11 and Axiom's delta | Axiom's delta 0.22.11 → 0.22.14, and `does-not-implement-crypto` at 0.22.14 |
| `peniko` 0.6.1, `color` 0.3.3 | `ub-risk-1` | Axiom (§4.4) | Axiom's |
| `bytemuck` 1.25.2 | `ub-risk-2` | Google through 1.22.0 and Axiom's delta | Axiom's delta 1.22.0 → 1.25.2 |
| `arrayvec` 0.7.8 | `ub-risk-2` | Google 0.7.6 and Axiom's delta | Axiom's delta 0.7.6 → 0.7.8 |
| `log` 0.4.34, `foldhash` 0.2.0, `fearless_simd` 0.7.0, `hashbrown` 0.17.1 | `ub-risk-2` | Axiom (§4.3, §4.5, §4.6) | Axiom's |
| `smallvec` 1.16.3 | `ub-risk-3` | Axiom ([audit](smallvec-audit.md)) | Already recorded |

- **Placement:** the whole tree runs in the renderer, in DP9's `sandboxed` group. `vello_cpu`, `vello_common`, `peniko` and `kurbo` inherit it from `smallvec` (DP8), and their other dependencies inherit it from them (DP9) unless separately approved as `trusted`. Every crate meets the `sandboxed` policy, `ub-risk-3` or better, and every crate but `smallvec` meets `ub-risk-2`, which would also admit it as `trusted` if a browser-side crate ever needed it (DP9).
- **DP8's conditions** apply only to `smallvec`, and its audit tracks them, the fuzz target included.
- **DP10 doesn't apply:** `vello_cpu` isn't a parser in its categories. It does rasterize geometry that web content and web fonts control, which makes the renderer's fuzzing at M1-CP14 worth pointing at it (inference).
- **`safe-to-deploy`** for each crate, and the crypto criterion in its final form, are decided with the adoption at M1-CP4, against the real lock, as for `smallvec` and `encoding_rs`. That is also when DP12's build-time audits are due (§2), and when finding 7 is settled.
- **The register:** `vello_cpu`'s row is graded and `sandboxed`; the `vello_cpu` tree in §5's list carries these grades.
- **Revisited** with each update (DP14's delta audits), or if M1's features change what compiles: for example, if `vello_cpu`'s `text` feature brings `glifo` in, or `euclid`'s `bytemuck` feature is enabled. Each is a new audit (DP15).

## Sources

All read on 2026-10-23.

- [C1] crates.io's API: each crate's version record (publication dates) and newest stable version — https://crates.io/api/v1/crates/vello_cpu (and likewise for each crate in §2)
- [G1] Google, *Auditing Standards* (google/rust-crate-audits, main): the soundness criteria, thoroughness, and "Delta audits should describe the final version" — https://github.com/google/rust-crate-audits/blob/main/auditing_standards.md
- [G2] Google, `audits.toml` (google/supply-chain, main): `num-traits` 0.2.19 `ub-risk-1` ("Contains a single line of float-to-int unsafe with decent safety comments"); `euclid` 0.22.11 `ub-risk-1`; `arrayvec` 0.7.6 `ub-risk-2`; `bytemuck` through 1.22.0 `ub-risk-2`; `log` 0.4.20 `ub-risk-1` and 0.4.22 to 0.4.26 `ub-risk-2` ("Unsafety is generally very well-documented, with one exception"); `foldhash` 0.1.3 to 0.1.5 `ub-risk-2` — https://github.com/google/supply-chain/blob/main/audits.toml
- [M1] Mozilla, `audits.toml` (mozilla/supply-chain, main): `hashbrown` `safe-to-deploy` 0.12.3 and deltas 0.15.2 → 0.17.1; `foldhash` 0.1.5 → 0.2.0; `log` 0.4.26 → 0.4.29 — https://github.com/mozilla/supply-chain/blob/main/audits.toml
- [B1] The Bytecode Alliance, Wasmtime's `supply-chain/audits.toml`: `hashbrown`, `log` and `foldhash` `safe-to-deploy` entries — https://github.com/bytecodealliance/wasmtime/blob/main/supply-chain/audits.toml
- [I1] ISRG, `supply-chain/audits.toml` (divviup/libprio-rs, as Firefox imports it): `num-traits` and `thiserror` deltas — https://raw.githubusercontent.com/divviup/libprio-rs/main/supply-chain/audits.toml
- [W1] Chromium code review 6071306, comments on `foldhash` 0.1.3's `seed.rs` (Łukasz Anforowicz and Adrian Taylor, 2024-12-04 and 2024-12-05; read through Gerrit's REST API) — https://chromium-review.googlesource.com/c/chromium/src/+/6071306
- [U1] `bytemuck` issue #343, "`zeroable.rs` uses `LaneCount` which has been removed" (closed 2026-01-30) — https://github.com/Lokathor/bytemuck/issues/343
- [U2] rust-lang/portable-simd pull request #485, "Remove LaneCount in favor of #[rustc_simd_monomorphize_lane_limit] attribute" (merged 2025-10-07) — https://github.com/rust-lang/portable-simd/pull/485
- [S1] The standard library, `Option`'s representation: transmuting zero bytes to `None` is sound "always" for "`fn`, `extern "C" fn`", which "remains true for `unsafe` variants, any argument/return types, and any other ABI" (observed in Rust 1.94.1's `core/src/option.rs`) — https://doc.rust-lang.org/core/option/index.html#representation
- [S2] The standard library, `transmute`'s "Transmutation between pointers and integers": "Transmuting *integers to pointers* is a largely unspecified operation … Doing non-zero-sized memory accesses with a pointer constructed this way is currently considered undefined behavior"; and `ptr::without_provenance_mut` (observed in Rust 1.94.1's `core/src/intrinsics/mod.rs` and `core/src/ptr/mod.rs`) — https://doc.rust-lang.org/core/mem/fn.transmute.html
- **Observed:** the scratch manifest, the census messages, the Miri and AddressSanitizer runs and the `Simd` layout probe (in the session's scratchpad, not committed); the probes are reproduced in Appendix A; `tools/unsafe-sites.py` over the census.

## Appendix A: the `hashbrown` probes

A scratch crate whose `Cargo.toml` depends on the unpacked `hashbrown` 0.17.1 (`default-features = false, features = ["default-hasher"]`) and patches `foldhash` to its unpacked 0.2.0. Run with `cargo +1.94.1 test`, then `MIRI_SYSROOT=<repo>/target/miri/sysroot cargo +nightly-2026-10-21 miri test`, with and without `MIRIFLAGS=-Zmiri-tree-borrows`, then `RUSTFLAGS=-Zsanitizer=address cargo +nightly-2026-10-21 test --target aarch64-apple-darwin`. All six pass in each (observed, 2026-10-23).

```rust
//! Miri probes of hashbrown 0.17.1: the paths vello_cpu 0.3.0 reaches
//! (`HashMap<u32, Arc<_>>`: insert, get, remove, clear, drop, with growth and
//! in-place rehashing), and the panic paths the audit reasons about.

#[cfg(test)]
mod tests {
    use hashbrown::{HashMap, HashSet};
    use std::cell::Cell;
    use std::hash::{Hash, Hasher};
    use std::panic::{AssertUnwindSafe, catch_unwind};
    use std::sync::Arc;

    // vello_cpu's image registry: HashMap<u32, Arc<Pixmap>>.
    #[test]
    fn registry_like_use() {
        let mut map: HashMap<u32, Arc<Vec<u8>>> = HashMap::new();
        assert!(map.get(&7).is_none()); // lookup in the empty singleton
        assert!(map.remove(&7).is_none());
        map.clear(); // clear on the empty singleton
        for i in 0..300u32 {
            assert!(map.insert(i, Arc::new(vec![i as u8; 3])).is_none()); // several resizes
        }
        for i in (0..300u32).step_by(2) {
            assert_eq!(map.remove(&i).unwrap()[0], i as u8); // tombstones
        }
        for i in 300..600u32 {
            map.insert(i, Arc::new(vec![1])); // in-place rehash or resize as tombstones fill
        }
        for i in 0..600u32 {
            assert_eq!(map.get(&i).is_some(), i >= 300 || i % 2 == 1);
        }
        assert_eq!(map.insert(301, Arc::new(vec![9])).unwrap()[0], 1); // replace
        map.clear();
        assert!(map.is_empty());
        for i in 0..20u32 {
            map.insert(i, Arc::new(vec![]));
        }
        drop(map);
    }

    // A key whose Hash and Clone panic when PANIC_AT counts down to one, and
    // whose live instances are counted, to check for double drops and leaks.
    thread_local! {
        static PANIC_AT: Cell<usize> = const { Cell::new(usize::MAX) };
        static LIVE: Cell<isize> = const { Cell::new(0) };
    }
    #[derive(PartialEq, Eq)]
    struct Key(u32);
    impl Key {
        fn new(v: u32) -> Self {
            LIVE.with(|l| l.set(l.get() + 1));
            Key(v)
        }
    }
    impl Clone for Key {
        fn clone(&self) -> Self {
            PANIC_AT.with(|p| {
                let left = p.get();
                p.set(left.saturating_sub(1));
                assert!(left != 1, "clone panics");
            });
            Key::new(self.0)
        }
    }
    impl Drop for Key {
        fn drop(&mut self) {
            LIVE.with(|l| l.set(l.get() - 1));
        }
    }
    impl Hash for Key {
        fn hash<H: Hasher>(&self, h: &mut H) {
            PANIC_AT.with(|p| {
                let left = p.get();
                p.set(left.saturating_sub(1));
                assert!(left != 1, "hash panics");
            });
            self.0.hash(h);
        }
    }
    fn arm(n: usize) {
        PANIC_AT.with(|p| p.set(n));
    }
    fn disarm() {
        PANIC_AT.with(|p| p.set(usize::MAX));
    }
    fn live() -> isize {
        LIVE.with(Cell::get)
    }

    #[test]
    fn resize_with_panicking_hasher() {
        let mut map = HashMap::new();
        for i in 0..28 {
            map.insert(Key::new(i), i);
        }
        // The next insert grows the table; panic in the middle of rehashing.
        arm(10);
        let r = catch_unwind(AssertUnwindSafe(|| map.insert(Key::new(100), 100)));
        assert!(r.is_err());
        disarm();
        // resize_inner leaves the old table intact: every element is still there.
        for i in 0..28 {
            assert_eq!(map.get(&Key::new(i)), Some(&i));
        }
        map.insert(Key::new(200), 200);
        drop(map);
        assert_eq!(live(), 0, "no leak and no double drop");
    }

    #[test]
    fn rehash_in_place_with_panicking_hasher() {
        let mut map = HashMap::with_capacity(28);
        for i in 0..28 {
            map.insert(Key::new(i), i);
        }
        assert_eq!(map.capacity(), 28);
        for i in 0..20 {
            map.remove(&Key::new(i)); // leaves tombstones; few items remain
        }
        // Fill the freed slots until an insert finds growth_left == 0 with few
        // items, which rehashes in place; panic during it.
        let mut panicked = false;
        for i in 1000..1100 {
            arm(3);
            let r = catch_unwind(AssertUnwindSafe(|| map.insert(Key::new(i), i)));
            disarm();
            if r.is_err() {
                panicked = true;
                break;
            }
        }
        assert!(panicked);
        assert_eq!(map.capacity(), 28, "rehashed in place, not resized");
        // The guard dropped the elements it couldn't rehash; the rest is consistent.
        let n = map.len();
        let mut seen = 0;
        for (k, v) in &map {
            assert_eq!(k.0, *v);
            seen += 1;
        }
        assert_eq!(seen, n);
        map.insert(Key::new(5000), 5000);
        drop(map);
        assert_eq!(live(), 0, "no leak and no double drop");
    }

    #[test]
    fn clone_and_clone_from_with_panicking_clone() {
        let mut a = HashMap::new();
        for i in 0..40 {
            a.insert(Key::new(i), i);
        }
        arm(17);
        let r = catch_unwind(AssertUnwindSafe(|| a.clone()));
        assert!(r.is_err());
        disarm();
        let mut b = HashMap::new();
        for i in 0..5 {
            b.insert(Key::new(i + 500), i);
        }
        arm(23);
        let r = catch_unwind(AssertUnwindSafe(|| b.clone_from(&a)));
        assert!(r.is_err());
        disarm();
        assert!(b.is_empty()); // the guard cleared it
        b.clone_from(&a);
        assert_eq!(b.len(), 40);
        drop(a);
        drop(b);
        assert_eq!(live(), 0);
    }

    struct Bomb(bool);
    impl Drop for Bomb {
        fn drop(&mut self) {
            assert!(!self.0, "drop panics");
        }
    }

    #[test]
    fn clear_with_panicking_drop() {
        let mut map = HashMap::new();
        for i in 0..30u32 {
            map.insert(i, Bomb(i == 11));
        }
        let r = catch_unwind(AssertUnwindSafe(|| map.clear()));
        assert!(r.is_err());
        assert!(map.is_empty()); // leaked the rest, didn't double-drop
        map.insert(1, Bomb(false));
        assert!(map.contains_key(&1));
    }

    #[test]
    fn retain_extract_drain_into_iter_and_entries() {
        let mut map: HashMap<u32, String> = (0..100).map(|i| (i, i.to_string())).collect();
        map.retain(|k, _| k % 3 != 0);
        let odd: Vec<_> = map.extract_if(|k, _| k % 2 == 1).collect();
        assert!(!odd.is_empty());
        let r = catch_unwind(AssertUnwindSafe(|| {
            map.retain(|k, _| {
                assert!(*k != 50, "predicate panics");
                true
            })
        }));
        assert!(r.is_err());
        let mut d = map.drain();
        let _ = d.next();
        drop(d); // a partial drain
        assert!(map.is_empty());
        let map2: HashMap<u32, String> = (0..50).map(|i| (i, i.to_string())).collect();
        let mut it = map2.into_iter();
        let _ = it.next();
        drop(it); // a partial into_iter
        let mut map3: HashMap<u32, String> = (0..10).map(|i| (i, i.to_string())).collect();
        if let hashbrown::hash_map::Entry::Occupied(e) = map3.entry(3) {
            let r = catch_unwind(AssertUnwindSafe(|| {
                e.replace_entry_with(|_, _| panic!("closure panics"));
            }));
            assert!(r.is_err());
        }
        assert!(!map3.contains_key(&3));
        assert_eq!(map3.len(), 9);
        let mut set: HashSet<String> = HashSet::new();
        set.insert("a".to_string());
        assert_eq!(set.replace("a".to_string()).as_deref(), Some("a"));
        assert_eq!(set.replace("b".to_string()), None);
        let [x, y] = map3.get_disjoint_mut([&1, &2]);
        *x.unwrap() += "!";
        *y.unwrap() += "?";
    }
}
```
