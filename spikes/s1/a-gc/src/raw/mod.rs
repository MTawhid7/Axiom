//! The unsafe core of Pattern A.
//!
//! **All of the crate's `unsafe` lives in this module tree.** The crate root
//! declares `#![deny(unsafe_code)]` and only this module re-allows it, which
//! `tools/check-unsafe-policy.sh` enforces.
//!
//! The `field!` and `unlock!` macros defined here expand to `unsafe` code in the
//! calling crate, so the crate is also registered as `emits-unsafe` (language
//! evaluation §15.3, rule 7).
//!
//! # Invariants
//!
//! Every `SAFETY:` comment in this module refers to these.
//!
//! - **I1: no collection during mutation.** The collector only runs from
//!   `Arena` methods that take `&mut self`. `Arena::mutate` takes `&self`.
//!   Since a `Gc` cannot outlive a mutation scope (I2), nothing is freed or
//!   dropped while a `Gc` exists outside the heap.
//! - **I2: branding.** Every `Gc<'gc, T>`, `GcWeak<'gc, T>` and
//!   `&Mutation<'gc>` is tied to a lifetime that a *higher-ranked* closure
//!   introduces. `'gc` is invariant and generic inside the closure, so pointers
//!   cannot escape the closure or mix between arenas. The only exception is the
//!   arena root, and the root is traced. The arena instantiates `'gc` as
//!   `'static` internally; that is sound because no closure can rely on it.
//! - **I3: complete tracing.** `Trace` implementations report every
//!   `Gc`/`GcWeak` they own (it is an `unsafe` trait; impls come from the derive
//!   or are audited here). So at collection time, every object reachable from the
//!   root gets marked.
//! - **I4: no `Gc` dereference in `Drop`.** The sweep drops values in arbitrary
//!   order, so a traced value's `Drop` must not dereference a `Gc`.
//!   - The derive proves that the deriving type has no `Drop` impl.
//!   - The std impls here cover types whose `Drop` cannot reach a `Gc`
//!     dereference.
//!   - `Cell` and `RefCell` are only `Trace` for `'static` contents, so they
//!     cannot hide a `Gc`.
//! - **I5: layout.** `GcBox<T>` is `#[repr(C)]` with its header first, so a
//!   `NonNull<GcHeader>` and the `NonNull<GcBox<T>>` of the same object convert
//!   into each other. The header's vtable records `T`'s layout and its trace and
//!   drop functions.
//! - **I6: reachable pointers are valid.** Outside a sweep, every `Gc` held by
//!   the root, by a reachable object, or by a live mutation scope points to an
//!   allocated box whose value has not been dropped.
//!   - **Why:** a sweep frees or drops only unmarked boxes, and by I3 nothing
//!     reachable points to them.
//!   - **New pointers** come from `Gc::new` (fresh), from reading reachable
//!     objects, or from `GcWeak::upgrade`, which refuses dead boxes.
//!   - **Weak pointers:** every `GcWeak` held that way points to an *allocated*
//!     box. Marking a `GcWeak` sets `weakly_referenced`, and the sweep then keeps
//!     the box's memory even after dropping its value.
#![allow(unsafe_code)]

mod gc;
mod heap;
mod lock;
mod trace;

pub use gc::{Gc, GcWeak};
pub(crate) use heap::Heap;
pub use heap::Mutation;
pub use lock::{Lock, RefLock, Write};
pub use trace::{Trace, Tracer};
