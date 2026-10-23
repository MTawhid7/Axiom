# Audit: `smallvec` 1.16.3

| | |
|---|---|
| **Status** | **Complete** (2026-10-23), for the owner's approval with the dependency (DP15). Performed by Claude (Opus 5.5) at Milestone 1's second checkpoint, as [the plan](../milestone-1-plan.md)'s §2, item 12 asks |
| **Question** | What soundness grade, on Google's scale (DP8), does the `smallvec` that M1's renderer dependencies need earn, and so: may it run in the browser, and if not, on what conditions in the sandboxed helpers? |
| **Answers** | The plan's §2, item 12 ("Graded at the version HarfRust, ICU4X and `vello_cpu` lock"), approved by the owner on 2026-10-23; the [crate architecture](../architecture/crates.md)'s open question 1 (IDNA in the browser) |
| **Feeds** | The [dependency register](../technology/dependency-register.md) §3; [networking](../architecture/networking.md)'s NW2, amended by the owner's conditional approval; `tools/crate-map.toml`; `supply-chain/audits.toml` |
| **Last updated** | 2026-10-23 |

## Summary

- **Grade: `ub-risk-3`,** for 1.16.3 as M1 would build it: aarch64, features `const_generics` and `const_new`, which HarfRust, ICU4X and `kurbo` enable between them.
- **The deciding finding: undefined behaviour reachable from safe code, confirmed by Miri under both of Rust's candidate aliasing models.** `SmallVec::insert_many`, given an iterator that panics while the vector is still inline, drops elements through a pointer that a later `&mut self` call invalidated (`lib.rs` line 1598). Miri reports it under Stacked Borrows and under Tree Borrows (§3). The fix is a line, which passes Miri under both models (Appendix A): build the guard from the pointer the function re-derives. Google's standard tolerates such aliasing violations at `ub-risk-2` only where no stable alternative exists, so this is `ub-risk-3`'s "undefined behavior may be possible under uncommon circumstances". **No crate in M1's tree calls `insert_many`** (observed), so Axiom's build doesn't reach it.
- **Supporting reasons:** safety comments on 11 of 43 blocks; two public `unsafe` functions with imprecise contracts and an `unsafe trait` with none; and Google's own `ub-risk-3` at 1.11.0, consistent with the history: five RustSec advisories through 2021, two in `insert_many`.
- **Everything else holds** under the crate's one documented invariant (§2), and Miri passes eight further probes of its other `unsafe` paths under both models.
- **Consequences** (§5): DP8 admits it only in sandboxed helpers, on conditions this audit meets except the fuzz target, due by Milestone 1's gate. Its dependents inherit the `sandboxed` group: HarfRust, ICU4X's segmenter and normalizer, `kurbo`, `peniko`, `vello_common` and `vello_cpu`. **The owner's conditional approval takes effect:** "If smallvec is graded ub-risk-3 during CP2, approve moving the IDNA domain lookup into the network process as an amendment to NW2." NW2 is amended accordingly.

## 1. What was read

**The crate:** `smallvec` 1.16.3, published 2026-10-10, the newest stable release on 2026-10-23 (crates.io's API [C1]); its source from cargo's registry, which checked it against the lock's checksum. The v1 branch is at 1.16.4, unreleased, with `insert_many` unchanged (observed, GitHub's API, 2026-10-23).

**The configuration** (observed): a scratch manifest outside the repository with M1's renderer dependencies at the versions the spikes locked (`vello_cpu` 0.3.0, HarfRust 0.14.0, `skrifa` 0.48.0, `read-fonts` 0.45.1, ICU4X's segmenter, properties and normalizer 2.3.0), resolved for `aarch64-apple-darwin`. `cargo tree -e features` shows the unified features `const_generics` and `const_new`; `union`, `may_dangle`, `drain_filter`, `specialization`, `write` and the serialization features are off.

**The census, by the compiler** (observed): the crate compiled as a path dependency with `RUSTFLAGS=-Wunsafe_code`, so that rustc reports every `unsafe` site that compiles in this configuration, with `cfg`s evaluated and local macros expanded. **60 sites:** 43 `unsafe` blocks, 9 `unsafe` methods, 1 `unsafe` function, 6 `unsafe impl`s and 1 `unsafe trait`. `tools/unsafe-census.py` agrees on the 43 blocks, 11 of them with a safety comment. **Confirmed with `--force-warn unsafe_code`,** which a crate's own `allow(unsafe_code)` can't lower as it lowers `-W` (the [`vello_cpu` audit](vello-cpu-audit.md)'s finding 8): the same 60 sites, counted by [`tools/unsafe-sites.py`](../../tools/unsafe-sites.py).

**The review:** each site read in `src/lib.rs` (2,852 lines), with the code that establishes its preconditions. A standard review in Google's sense [G1]: each line of `unsafe` code looked at and its patterns reasoned about. For `ub-risk-3`, "the thoroughness of the review is not consequential" [G1].

**Miri** (observed, `nightly-2026-10-21`): nine probes in a scratch crate that depends only on this copy of `smallvec`, each under Stacked Borrows and Tree Borrows (`-Zmiri-tree-borrows`). They cover pushing, popping, growing, shrinking and reserving, inline and spilled; `insert` and `remove` across the spill; `insert_many` with exact, short and long size hints; `insert_many` with a panicking iterator, inline and spilled; `retain` (with a panicking predicate), `dedup` and `drain`; conversions to and from `Vec` and boxed slices; `into_iter` both ways; `from_slice`, `insert_from_slice`, `from_elem`, `from_buf_and_len`, `into_inner` and the `const` constructor; a thousand zero-sized items; `truncate` with a panicking destructor; and `extend` with a panicking iterator.

## 2. Every site, and the invariant it relies on

**The crate's invariants** (named here for the table; S1 is documented in the source, at lines 834–838):
- **S1, storage.** The `capacity` field selects the storage. If `capacity <= inline_capacity()`, the data is the inline variant, and `capacity` is the length. Otherwise the data is the heap variant `{ ptr, len }`: `ptr` came from the global allocator with `layout_array::<Item>(capacity)`, the same layout as `Layout::array`, and `len <= capacity`. Zero-sized items have an inline capacity of `usize::MAX`, so they never spill.
- **S2, initialization.** Items `0..len` are initialized; the rest of the buffer is never read as items.
- **S3, panics and leaks.** Before items move out or a closure runs, the length is lowered so that a panic or a leaked iterator can only leak items, never drop one twice.
- **S4, aliasing.** A pointer from `as_mut_ptr()` or `triple_mut()` is used only until the next `&mut self` call, which re-borrows the inline buffer. The source says so at line 1550: "The set_len above invalidates the previous pointers, so we must re-create them."
- **S5, the array.** `Array` is implemented only for `[T; N]` in this configuration (line 2673), so `A` holds `N` items laid out as an array; `new()` asserts the size and alignment (line 852).

**The standard library's contracts** (`ptr::read`, `write`, `copy`, `copy_nonoverlapping`, `drop_in_place`, `alloc`, `realloc`, `dealloc`, `Vec::from_raw_parts`, `slice::from_raw_parts`) have the usual preconditions, which the invariants supply. "Holds" means the precondition holds by the invariant named, as far as this review followed it.

| Line | Site | What it relies on | Verdict |
|---|---|---|---|
| 345 | `unsafe fn deallocate` | The caller passes a pointer allocated with `layout_array::<T>(capacity)` (S1); undocumented | Holds at its four call sites (1302, 1405, and through `Drop`'s `Vec`) |
| 373, 374 | `unsafe impl Sync`, `Send for Drain` | As `Vec`'s `Drain`: the bound on `A` is the bound on its items (S5) | Holds |
| 383, 397 | `Drain::next`, `next_back`: `ptr::read` | The drained range was cut from the length by `drain` (S3), and the slice iterator yields each item once | Holds |
| 415 | `Drain::drop`: moves the tail back | `tail_start..tail_start + tail_len` is initialized and within the capacity; the pointer is re-derived after `as_mut` (S4) | Holds |
| 731, 738 | `unsafe fn inline`, `inline_mut` | The data is inline (S1); the other variant reaches `debug_unreachable!`, which is `unreachable_unchecked` in release builds | Holds at every call, each behind `spilled()` or after `empty()` |
| 754 | `empty()`: `MaybeUninit::uninit().assume_init()` | The target type is `MaybeUninit<A>`, valid for any bytes; documented | Holds |
| 757, 764, 771 | `unsafe fn into_inline`, `heap`, `heap_mut` | The variant matches (S1), as for `inline` | Holds at every call |
| 783, 784 | `unsafe impl Send`, `Sync for SmallVecData` | As `Vec`: the raw pointer is owned, and the bound on `A` is its items' (S5) | Holds |
| 902 | `from_vec`, inline: copies the `Vec`'s items in | `vec.capacity() <= inline_capacity()`, so its length fits; the `Vec`'s length is zeroed first, so it frees only its buffer (S3) | Holds |
| 964 | `from_buf_and_len` → the unchecked constructor | Asserted `len <= A::size()`; the buffer is fully initialized | Holds |
| 981 | `pub unsafe fn from_buf_and_len_unchecked` | Documented: "len <= A::size()". **Not documented:** that the first `len` items of the `MaybeUninit<A>` be initialized (S2) | **Imprecise contract** (finding 2) |
| 994 | `pub unsafe fn set_len` | Prose only, no `# Safety` section: "it is up to the caller to ensure that the vector is actually the specified size". **Not stated:** `new_len <= capacity()`, and that the items are initialized. On an inline vector, a length past the inline capacity makes `spilled()` true over inline data, which `heap()` then turns into `unreachable_unchecked` | **Imprecise contract** (finding 2) |
| 1051, 1064 | `triple`, `triple_mut` | `spilled()` chooses the variant (S1) | Holds |
| 1119 | `drain`: lowers the length, slices the range, keeps a pointer to the vector | Asserted `start <= end <= len`; the length lowered first (S3); the pointer is non-null, from a reference | Holds |
| 1216 | `push` | Not full, or grown first by `reserve_one_unchecked`, after which the data is on the heap since the new capacity exceeds the inline one (S1); writes at `len < capacity` | Holds |
| 1246 | `pop` | Length checked, then lowered before the read | Holds |
| 1289 | `try_grow`: to inline storage, or a new or reallocated heap buffer | Moving inline copies `len <= new_cap <= inline_capacity` items, then frees the old buffer with its own capacity (S1); `realloc` gets the old layout and a non-zero size checked by `Layout::from_size_align`; on failure nothing has changed | Holds |
| 1401 | `shrink_to_fit` to inline storage | Spilled, and `len <= inline_size()`; frees with the old capacity before replacing it (S1) | Holds |
| 1422 | `truncate`: drops from the end | The length is lowered before each drop (S3) | Holds |
| 1458 | `swap_remove`: `unreachable_unchecked` if `pop` returns `None` | `swap(len - 1, index)` panics first on an empty vector or a bad index, so `pop` has an item | Holds |
| 1472 | `remove` | Asserted `index < len`; length lowered before the read | Holds |
| 1489 | `insert` | Grown to the heap if full (S1); `index > len` panics before the pointer moves, as the comment says | Holds |
| 1532 | `insert_many`: reserves, opens a gap, writes into it | The gap is reserved, the length is zeroed (S3), and a guard drops the items outside the unwritten gap on a panic. **The guard keeps the pointer taken before `set_len(0)`**, whose `&mut self` call invalidates it (S4), and uses it after the iterator panics | **Undefined behaviour, inline storage** (finding 1) |
| 1597 | `DropOnPanic::drop`: drops through its stored pointer | As 1532: the pointer is stale for inline storage | **Undefined behaviour** (finding 1); with heap storage the pointer is the heap's own and stays valid (Miri passes) |
| 1610 | `into_vec`: `Vec::from_raw_parts` | Spilled: pointer, length and capacity are an allocation of `Layout::array`'s layout (S1) | Holds |
| 1640 | `into_inner`: reads the data out | Inline and full (`len == A::size()`), so every item is initialized (S2) | Holds |
| 1686, 1692 | `retain`'s guard: closes holes, sets the length | Documented; the pointer is re-derived inside the guard (S4) | Holds |
| 1701, 1722, 1727, 1733, 1738, 1750 | `retain`'s loop | Documented at each block: `read < original_len`, `write < read`, each item touched once (S3) | Holds |
| 1787 | `dedup_by`: two `&mut` to distinct items, swaps | `w - 1 < r`, so they differ; a panicking predicate leaves only swapped, initialized items, and `truncate` drops the tail (S3) | Holds |
| 1928, 1931 | `pub unsafe fn from_raw_parts` | Documented in a `# Safety` section; `NonNull::new_unchecked` relies on the caller's non-null pointer, checked by a `debug_assert!`; the capacity is asserted past the inline one | Holds for callers that keep the contract |
| 1973 | `from_slice`, inline | `len <= inline_capacity()`; `Copy` items | Holds |
| 2008 | `insert_from_slice` | Reserved first; asserted `index <= len`; the slice can't alias the vector (borrows) | Holds |
| 2059 | `from_elem`, inline | `n <= inline_capacity()`; `SetLenOnDrop` keeps the length right if `clone` panics (S3) | Holds |
| 2078, 2088 | `Deref`, `DerefMut` | Pointer and length from `triple` (S1, S2) | Holds |
| 2323 | `Extend::extend`: writes up to the capacity | Reserved; `SetLenOnDrop` (S3); the buffer doesn't move while the iterator runs | Holds |
| 2376 | `Drop` | Spilled: `Vec::from_raw_parts` frees it (S1); inline: drops `0..len` (S2) | Holds |
| 2453 | `unsafe impl Send for SmallVec` | Its items are `Send` (S5) | Holds |
| 2498, 2519 | `IntoIter::next`, `next_back` | `into_iter` zeroed the inner length; `current..end` are the remaining initialized items (S3) | Holds |
| 2534, 2540 | `IntoIter::as_slice`, `as_mut_slice` | `current..end` | Holds |
| 2548 | `into_iter`: zeroes the length | So that `IntoIter`'s `Drop` alone drops the remaining items (S3) | Holds |
| 2579 | `pub unsafe trait Array` | **No safety documentation** for implementers; mitigated by `new()`'s assertion and by `[T; N]` being the only implementation here (S5) | **Undocumented contract** (finding 2) |
| 2662 | `pub const unsafe fn from_const_with_len_unchecked` | Documented "len <= N"; the array is initialized | Holds |
| 2673 | `unsafe impl Array for [T; N]` | `size()` is `N`, the array's length (S5) | Holds |

## 3. Findings

1. **Undefined behaviour in `insert_many`'s panic path, with inline storage** (observed, Miri). The function takes `start = self.as_mut_ptr()`, then calls `self.set_len(0)`, builds `DropOnPanic { start, … }` from the first pointer, and re-derives `start` for its own writes, as its comment requires (lines 1535–1553). Each `as_mut_ptr()` and `set_len()` goes through `triple_mut()`, which re-borrows the inline buffer as `&mut`, so the guard's pointer is invalid from `set_len(0)` on. If the iterator panics, the guard drops items through it.
   - **Stacked Borrows** (Miri's default): "Undefined Behavior: trying to retag from <142270> for Unique permission at alloc44402[0x8], but that tag does not exist in the borrow stack for this location", at `DropOnPanic::drop` (`lib.rs` 1598); the tag "was later invalidated … by a Unique function-entry retag" inside `insert_many`.
   - **Tree Borrows:** "Undefined Behavior: reborrow through <139165> at alloc44356[0x20] is forbidden", at the same line.
   - **With heap storage,** the pointer is the heap buffer's, which `&mut self` doesn't re-borrow; Miri passes that probe under both models.

   **Reach:** safe code, with an unspilled vector and an iterator that panics after `insert_many` began writing; uncommon. **In M1's tree,** HarfRust, `skrifa`, `read-fonts`, ICU4X's crates, `kurbo`, `peniko`, `vello_common` and `vello_cpu` never call `insert_many` (observed, `grep` over their sources), and Axiom's code never depends on `smallvec` directly (CR11). **Upstream:** present on the v1 branch at 1.16.4; no issue or fix found in `servo/rust-smallvec` (GitHub's search, 2026-10-23), whose 2026 fixes of Stacked Borrows errors (#449, #586) concern 2.0's `splice` and `insert_mut`. Whether to report it is the owner's decision (§5).
2. **Safety documentation** (observed). 11 of the 43 blocks carry a safety comment; the private `unsafe` methods (`inline`, `heap` and the rest) state no preconditions; `set_len` has no `# Safety` section and leaves out `new_len <= capacity()`; `from_buf_and_len_unchecked` leaves out the initialization its `MaybeUninit<A>` argument needs; `unsafe trait Array` has no contract for implementers. Under Google's criteria, `ub-risk-1` needs every unsafe trait and function documented, and `ub-risk-2` documentation "relatively comprehensive" [G1]; here the central invariant is documented, but the rest falls between `ub-risk-2` and `ub-risk-3`.
3. **History** (sourced). Google graded 1.11.0 `ub-risk-3` [G2]; later versions carry only `safe-to-deploy` audits, from Google, Mozilla and the Bytecode Alliance, none a soundness grade [G2, M1, B1]. RustSec records five advisories, all fixed by 1.6.1 [R1]: double free while unwinding in `insert_many` (RUSTSEC-2018-0003), an uninitialized value of any type (RUSTSEC-2018-0018), double free and use-after-free in `grow` (RUSTSEC-2019-0009), memory corruption in `grow` (RUSTSEC-2019-0012), and a buffer overflow in `insert_many` (RUSTSEC-2021-0003).
4. **What is solid** (observed): S1 is documented and every other site holds under it; `retain` was rewritten with a safety comment on each block; capacity arithmetic is checked; allocation failures leave the vector unchanged; zero-sized items never allocate; Miri passes eight probes of the other paths under both models.

## 4. Grade

**`ub-risk-3`.** Finding 1 alone places it there: undefined behaviour reachable from safe code under uncommon circumstances, in an aliasing rule for which a stable alternative is a line away, which Google excludes from `ub-risk-2`'s "extraordinary circumstances" [G1]. Findings 2 and 3 agree. **Not `ub-risk-4`:** the defect needs a panicking iterator on an inline vector, and the rest of the crate holds; typical use doesn't reach undefined behaviour.

## 5. Consequences

- **DP8's conditions for a `ub-risk-3` crate** (the dependency policy, as amended in Round 4):
  - **Sandboxed helpers only,** in the `sandboxed` group, which its dependents inherit: HarfRust 0.14.0, `icu_segmenter` and `icu_normalizer` 2.3.0, `kurbo` 0.13.1, `peniko` 0.6.1, `vello_common` and `vello_cpu` 0.3.0 (observed, `cargo tree -i smallvec`). Each is a renderer crate's dependency, behind `text` or `paint`, which the crate map already places in renderers as `sandboxed`; the normalizer's place follows from NW2's amendment below.
  - **No better-graded crate meets the need:** each of those crates requires `smallvec` 1.x as a non-optional dependency, from `^1.10` (ICU4X) to `^1.15.1` (`kurbo`, `peniko`, `vello_common`) (observed, their manifests), and 2.0 is in beta (2.0.0-beta.3, 2026-10-16 [C1]).
  - **An ADR or named decision accepts it:** the plan's §2, item 12, which the owner approved as written on 2026-10-23: "At `ub-risk-3` it is admitted in renderers under DP8's conditions".
  - **Axiom's audit names each block and its invariant:** §2 above.
  - **A libFuzzer target under AddressSanitizer reaches those blocks,** with no open finding at Milestone 1's gate: **still to do,** in `fuzz/` before M1-CP14, driving sequences of `SmallVec` operations, as upstream's fuzzer does. AddressSanitizer finds memory errors, not aliasing violations, so finding 1 stays a recorded condition rather than a fuzzing result.
  - **One condition of use, added by this audit:** Axiom's code never calls `insert_many`, and a dependency that starts to call it is reviewed again (the register's entry says so).
- **The owner's conditional approval takes effect** (2026-10-23): "If smallvec is graded ub-risk-3 during CP2, approve moving the IDNA domain lookup into the network process as an amendment to NW2." [NW2](../architecture/networking.md) is amended: full UTS #46 processing, which needs ICU4X's normalizer and so `smallvec`, runs only in renderers and the network process, in a sandboxed crate `url-idna` beside `url`; the browser's address bar asks the network process for a typed host's ASCII form, and parses the answer as it parses every URL from another process, in canonical-ASCII mode.
- **Upstream:** a report to `servo/rust-smallvec` with the probe and the one-line fix would help every user of 1.x; it is public, so it waits for the owner.
- **Revisited** when a release fixes finding 1 and documents its `unsafe` APIs, when 2.0 is stable and M1's dependents move to it, or when a dependency starts calling `insert_many`. Each is a new audit (DP15).

## Sources

All read on 2026-10-23.

- [C1] crates.io, `smallvec`'s versions — https://crates.io/api/v1/crates/smallvec
- [G1] Google, *Auditing Standards* (google/rust-crate-audits, main): `ub-risk-0` to `ub-risk-4`, thoroughness — https://github.com/google/rust-crate-audits/blob/main/auditing_standards.md
- [G2] Google, `audits.toml` (google/supply-chain, main): `smallvec` 1.11.0 `ub-risk-3`, "Reviewed in CL 552492992"; `safe-to-deploy` at 1.13.2 and 1.13.2 → 1.14.0 — https://github.com/google/supply-chain/blob/main/audits.toml
- [M1] Mozilla, `audits.toml` (mozilla/supply-chain, main): `safe-to-deploy` deltas through 1.15.2 — https://github.com/mozilla/supply-chain/blob/main/audits.toml
- [B1] The Bytecode Alliance, Wasmtime's `supply-chain/audits.toml`: `safe-to-deploy` deltas through 1.14.0 and to 2.0.0-alpha.10 — https://github.com/bytecodealliance/wasmtime/blob/main/supply-chain/audits.toml
- [R1] The RustSec advisory database, `crates/smallvec/` (cloned by cargo-deny into `target/advisory-dbs/`) — https://github.com/rustsec/advisory-db/tree/main/crates/smallvec
- [U1] `servo/rust-smallvec`: the v1 branch's `Cargo.toml` (1.16.4) and `src/lib.rs`; issues and pull requests #449 and #586 — https://github.com/servo/rust-smallvec
- **Observed:** the scratch manifest and Miri probes (in the session's scratchpad, not committed), and the compiler's census above. The probes are reproduced in [Appendix A](#appendix-a-the-miri-probe-of-finding-1).

## Appendix A: the Miri probe of finding 1

A crate depending only on `smallvec` 1.16.3 with `const_generics` and `const_new`, run with `cargo +nightly-2026-10-21 miri test`, and again with `MIRIFLAGS=-Zmiri-tree-borrows`:

```rust
use smallvec::SmallVec;
use std::panic::{AssertUnwindSafe, catch_unwind};

#[test]
fn insert_many_with_a_panicking_iterator_inline() {
    struct Boom(usize);
    impl Iterator for Boom {
        type Item = String;
        fn next(&mut self) -> Option<String> {
            self.0 += 1;
            if self.0 == 2 { panic!("boom") }
            Some(format!("s{}", self.0))
        }
        fn size_hint(&self) -> (usize, Option<usize>) { (2, None) }
    }
    let mut v: SmallVec<[String; 8]> = (0..3).map(|n| format!("s{n}")).collect();
    assert!(!v.spilled());
    let r = catch_unwind(AssertUnwindSafe(|| v.insert_many(1, Boom(0))));
    assert!(r.is_err());
}
```

**The fix** (observed): in `insert_many`, build `DropOnPanic` after `set_len(0)`, from the pointer re-derived there, so the guard and the writes share one pointer that no later `&mut self` call invalidates. On a copy of 1.16.3 with that change, all nine probes pass under Stacked Borrows and under Tree Borrows:

```rust
            self.set_len(0);

            // The set_len above invalidates the previous pointers, so we must
            // re-create them, and give the guard the re-created one.
            let start = self.as_mut_ptr();
            let mut guard = DropOnPanic {
                start,
                skip: index..(index + lower_size_bound),
                len: old_len + lower_size_bound,
            };
            let ptr = start.add(index);
```
