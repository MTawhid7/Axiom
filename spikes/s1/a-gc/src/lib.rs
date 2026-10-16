//! # Spike S1, Pattern A: a gc-arena-style garbage-collected heap
//!
//! This is a canonical re-implementation of the `gc-arena` design (Ruffle,
//! piccolo):
//! - Pointers are **branded** with a mutation lifetime `'gc`.
//! - **Collection happens only between mutation scopes.** That removes stack
//!   scanning and rooting entirely: the arena root is the only root.
//! - Objects are individually allocated **raw-pointer** boxes. The collector is a
//!   **non-moving, stop-the-world mark–sweep**.
//!
//! ```
//! use s1_a_gc::{unlock, Arena, Gc, Lock, Rootable, Trace};
//!
//! #[derive(Trace)]
//! struct Node<'gc> {
//!     next: Lock<Option<Gc<'gc, Node<'gc>>>>,
//!     value: u32,
//! }
//!
//! struct Root;
//! impl Rootable for Root {
//!     type Root<'gc> = Option<Gc<'gc, Node<'gc>>>;
//! }
//!
//! let mut arena = Arena::<Root>::new(|_| None);
//! arena.mutate_root(|mc, root| {
//!     let a = Gc::new(mc, Node { next: Lock::new(None), value: 1 });
//!     let b = Gc::new(mc, Node { next: Lock::new(Some(a)), value: 2 });
//!     unlock!(Gc::write(mc, a), Node, next).set(Some(b)); // a ⇄ b: a cycle
//!     *root = Some(a);
//! });
//! arena.collect_all();
//! assert_eq!(arena.metrics().objects, 2, "a reachable cycle survives");
//!
//! arena.mutate_root(|_, root| *root = None);
//! arena.collect_all();
//! assert_eq!(arena.metrics().objects, 0, "an unreachable cycle is reclaimed");
//! ```
//!
//! # Where the `unsafe` is
//!
//! All of it is in `src/raw/`, under the invariants I1–I6 documented there. Two
//! macro sources also emit `unsafe` into *other* crates (language evaluation
//! §15.3, rule 7), and both are audited as part of the boundary:
//! - `#[derive(Trace)]` (crate `s1-a-gc-derive`) emits `unsafe impl Trace`;
//! - [`field!`] and [`unlock!`] emit a `Write` projection.
//!
//! # What the compiler enforces
//!
//! Each example below has been checked to fail for its intended reason. Where
//! rustc gives that error a code, the code is pinned in the test.
//!
//! A `Gc` cannot escape its mutation scope ("lifetime may not live long
//! enough"; this borrow-checker error has no code):
//!
//! ```compile_fail
//! use s1_a_gc::{Arena, Gc, Rootable};
//! struct R;
//! impl Rootable for R { type Root<'gc> = (); }
//! let arena = Arena::<R>::new(|_| ());
//! let leaked = arena.mutate(|mc, _| Gc::new(mc, 5_u32));
//! ```
//!
//! Pointers from two arenas cannot be mixed:
//!
//! ```compile_fail,E0521
//! use s1_a_gc::{Arena, Gc, Rootable};
//! struct R;
//! impl Rootable for R { type Root<'gc> = Option<Gc<'gc, u32>>; }
//! let mut a = Arena::<R>::new(|_| None);
//! let b = Arena::<R>::new(|_| None);
//! a.mutate_root(|_, root_a| b.mutate(|mc_b, _| *root_a = Some(Gc::new(mc_b, 1))));
//! ```
//!
//! A collection cannot run while a mutation scope is open (I1):
//!
//! ```compile_fail,E0502
//! use s1_a_gc::{Arena, Gc, Rootable};
//! struct R;
//! impl Rootable for R { type Root<'gc> = (); }
//! let mut arena = Arena::<R>::new(|_| ());
//! arena.mutate(|mc, _| {
//!     let x = Gc::new(mc, 1_u32);
//!     arena.collect_all();
//!     assert_eq!(*x, 1);
//! });
//! ```
//!
//! A derived type cannot implement `Drop` (I4):
//!
//! ```compile_fail,E0119
//! use s1_a_gc::{Gc, Trace};
//! #[derive(Trace)]
//! struct Noisy<'gc> { next: Option<Gc<'gc, u32>> }
//! impl Drop for Noisy<'_> { fn drop(&mut self) {} }
//! ```
//!
//! A `#[trace(require_static)]` field cannot hide a `Gc`:
//!
//! ```compile_fail,E0521
//! use s1_a_gc::{Gc, Trace};
//! #[derive(Trace)]
//! struct Hidden<'gc> { #[trace(require_static)] secret: Option<Gc<'gc, u32>> }
//! ```
//!
//! A plain `Cell` cannot hold a `Gc`, because that would allow writes without a
//! barrier:
//!
//! ```compile_fail,E0521
//! use core::cell::Cell;
//! use s1_a_gc::{Gc, Trace};
//! #[derive(Trace)]
//! struct Sneaky<'gc> { slot: Cell<Option<Gc<'gc, u32>>> }
//! ```
//!
//! # Limitations, which are deliberate for the spike
//!
//! - **The collector is not incremental.** Pauses scale with the live heap. The
//!   write-barrier *API* is kept so that the ceremony being measured is
//!   authentic; the barrier itself only counts.
//! - **`Gc<T>` requires `T: Sized`.** There are no `dyn` objects.
//! - **Allocation is one system-allocator call per object**, as in gc-arena,
//!   with no size-class pages. This is part of the storage confound recorded in
//!   the Step 2b plan (§4.4).
#![deny(unsafe_code)]

mod arena;
mod metrics;
mod raw;

pub use arena::{Arena, Pacing, Rootable};
pub use metrics::Metrics;
pub use raw::{Gc, GcWeak, Lock, Mutation, RefLock, Trace, Tracer, Write};
pub use s1_a_gc_derive::Trace;
