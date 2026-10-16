//! # Spike S1, Pattern B: a Nova-style garbage-collected heap
//!
//! This is a canonical re-implementation of the Nova engine's heap design:
//! - **Typed arenas.** One dense `Vec` per object type (the [`heap!`] macro).
//! - **`u32` handles.** A [`Handle<'a, T>`] is an index bound to a GC-scope
//!   lifetime: 4 bytes, and `Option<Handle>` is 4 bytes too.
//! - **Scopes.** [`GcScope`] / [`NoGcScope`] tokens make "a handle used after a
//!   possible collection" a borrow error (see [`scope`](GcScope)). The escape
//!   hatches are `bind` and `unbind`.
//! - **An explicit root context.** A [`RootFrame`] plus [`Scoped`] handles
//!   keep values alive across safepoints.
//! - **Stop-the-world mark–compact.** Survivors are ranked, using popcount
//!   over the mark bitsets, so no forwarding table is allocated per object.
//!   Every handle in heap data and roots is then rewritten.
//!
//! ```
//! use s1_b_gc::{heap, Forwarding, Handle, Marker, Pacing, Roots, Trace};
//!
//! struct Node { next: Option<Handle<'static, Node>>, value: u32 }
//! heap! { struct Heap { nodes: Node } }
//! impl Trace for Node {
//!     fn mark(&self, m: &mut Marker) { m.mark_opt(self.next) }
//!     fn relocate(&mut self, f: &Forwarding) { f.relocate_opt(&mut self.next) }
//! }
//! struct Globals { root: Option<Handle<'static, Node>> }
//! impl Roots for Globals {
//!     fn mark(&self, m: &mut Marker) { m.mark_opt(self.root) }
//!     fn relocate(&mut self, f: &Forwarding) { f.relocate_opt(&mut self.root) }
//! }
//!
//! let mut heap = Heap::new(Pacing::default());
//! let mut globals = Globals { root: None };
//! let a = heap.alloc(Node { next: None, value: 1 });
//! let b = heap.alloc(Node { next: Some(a), value: 2 });
//! heap.nodes[a].next = Some(b); // a ⇄ b: a cycle
//! heap.alloc(Node { next: None, value: 3 }); // garbage
//! globals.root = Some(a);
//!
//! heap.collect(&mut globals);
//! assert_eq!(heap.nodes.len(), 2, "a reachable cycle survives; garbage is compacted away");
//! let a = globals.root.unwrap(); // the collector relocated the root
//! assert_eq!(heap.nodes[heap.nodes[a].next.unwrap()].value, 2);
//!
//! globals.root = None;
//! heap.collect(&mut globals);
//! assert!(heap.nodes.is_empty(), "an unreachable cycle is reclaimed");
//! ```
//!
//! # Safety model
//!
//! **This crate contains no `unsafe` at all** (`#![forbid(unsafe_code)]`), and
//! neither does anything the [`heap!`] macro expands to.
//! - Handles are bounds-checked indices, so the worst a stale handle can do is
//!   name the wrong object or panic. It can never cause undefined behaviour.
//! - The scope discipline enforces *correctness* at compile time, and debug
//!   builds back it up at run time: every handle carries the collection epoch
//!   it was issued in, and every access checks it.
//! - This is the essential difference from Pattern A, where memory safety
//!   itself rests on the branding and on complete tracing.
//!
//! A rooted handle cannot outlive its frame:
//!
//! ```compile_fail,E0597
//! # use s1_b_gc::{heap, Forwarding, Marker, Pacing, Trace};
//! # struct Node;
//! # heap! { struct Heap { nodes: Node } }
//! # impl Trace for Node { fn mark(&self, _: &mut Marker) {} fn relocate(&mut self, _: &Forwarding) {} }
//! let mut heap = Heap::new(Pacing::default());
//! let node = heap.alloc(Node);
//! let rooted = {
//!     let frame = heap.core.frame();
//!     node.scope(&frame)
//! };
//! let _ = rooted.get();
//! ```
//!
//! # Limitations, which are deliberate for the spike
//!
//! - **The collector is not incremental.** Pauses scale with the live heap,
//!   as in Pattern A.
//! - **Accounting counts `size_of` the stored objects**, not `Vec` growth
//!   slack. Peak resident memory is measured externally.
#![forbid(unsafe_code)]

mod arena;
mod handle;
mod roots;
mod scope;
mod trace;

pub use arena::{
    __compact, __mark_item, Arena, HeapCore, HeapLayout, Metrics, Pacing, Store, alloc,
    collect_heap,
};
pub use handle::{Handle, WeakHandle};
pub use roots::{RootFrame, RootStack, Scoped};
pub use scope::{GcScope, GcToken, NoGcScope};
pub use trace::{Allocated, Forwarding, Marker, Roots, Trace};

/// Declare a heap: one [`Arena`] per object type, plus a [`HeapCore`].
///
/// ```ignore
/// heap! {
///     pub struct DomHeap {
///         nodes: NodeData,
///         listeners: ListenerData,
///     }
/// }
/// ```
///
/// The expansion contains:
/// - the struct, with one public `Arena` field per type and a `core: HeapCore`;
/// - `Allocated` impls, which number the arenas in field order;
/// - `Store` and `HeapLayout` impls;
/// - `new(pacing)`, `alloc(value)` (never collects), and `collect(&mut roots)`.
///
/// It expands to safe code only. Declare one heap per module.
#[macro_export]
macro_rules! heap {
    (
        $(#[$meta:meta])*
        $vis:vis struct $name:ident {
            $($field:ident : $ty:ty),+ $(,)?
        }
    ) => {
        $(#[$meta])*
        $vis struct $name {
            $(pub $field: $crate::Arena<$ty>,)+
            pub core: $crate::HeapCore,
        }

        #[allow(non_camel_case_types, dead_code)]
        #[derive(Clone, Copy)]
        enum __S1BArenaIds {
            $($field,)+
        }

        $(
            impl $crate::Allocated for $ty {
                const ARENA: usize = __S1BArenaIds::$field as usize;
            }

            impl $crate::Store<$ty> for $name {
                #[inline]
                fn arena(&self) -> &$crate::Arena<$ty> {
                    &self.$field
                }
                #[inline]
                fn arena_mut(&mut self) -> &mut $crate::Arena<$ty> {
                    &mut self.$field
                }
            }
        )+

        impl $crate::HeapLayout for $name {
            fn arena_lens(&self) -> ::std::vec::Vec<usize> {
                ::std::vec![$(self.$field.len()),+]
            }

            fn mark_item(&self, arena: usize, index: usize, marker: &mut $crate::Marker) {
                $(
                    if arena == __S1BArenaIds::$field as usize {
                        return $crate::__mark_item(&self.$field, index, marker);
                    }
                )+
                ::core::unreachable!("unknown arena {arena}")
            }

            fn compact_all(&mut self, forwarding: &$crate::Forwarding) {
                $($crate::__compact(&mut self.$field, forwarding);)+
            }

            fn size(&self) -> (usize, usize) {
                (
                    0 $(+ self.$field.len())+,
                    0 $(+ self.$field.len() * ::core::mem::size_of::<$ty>())+,
                )
            }

            fn core(&mut self) -> &mut $crate::HeapCore {
                &mut self.core
            }
        }

        impl $name {
            /// An empty heap.
            $vis fn new(pacing: $crate::Pacing) -> Self {
                Self {
                    $($field: $crate::Arena::default(),)+
                    core: $crate::HeapCore::new(pacing),
                }
            }

            /// Allocate without a safepoint: this never collects.
            #[inline]
            $vis fn alloc<T: $crate::Allocated>(&mut self, value: T) -> $crate::Handle<'static, T>
            where
                Self: $crate::Store<T>,
            {
                $crate::alloc(self, value)
            }

            /// A full collection, rooted at the root stack and at `roots`.
            $vis fn collect<R: $crate::Roots + ?Sized>(&mut self, roots: &mut R) {
                $crate::collect_heap(self, roots);
            }
        }
    };
}
