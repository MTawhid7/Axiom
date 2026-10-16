//! The arena: the root object, mutation scopes, and collection. This module is
//! entirely safe code. The branding argument (I2) needs no lifetime transmutes,
//! because the mutation closures are higher-ranked over `'gc`.

use core::fmt;

use crate::metrics::Metrics;
use crate::raw::{Heap, Mutation, Trace};

/// Names an arena's root type, which is generic over the mutation lifetime.
///
/// ```ignore
/// struct DomRoot;
/// impl Rootable for DomRoot {
///     type Root<'gc> = Document<'gc>;
/// }
/// ```
pub trait Rootable: 'static {
    type Root<'gc>: Trace + 'gc;
}

/// When [`Arena::collect_debt`] actually collects.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pacing {
    /// The minimum allocation budget between collections, in bytes.
    pub min_bytes: usize,
    /// Collect once the bytes allocated since the last collection reach this
    /// percentage of the bytes that survived it (or `min_bytes`, if that is
    /// larger).
    pub growth_percent: usize,
}

impl Pacing {
    /// The allocation budget, in bytes, after a collection that left
    /// `survived_bytes` live.
    #[must_use]
    pub const fn budget(&self, survived_bytes: usize) -> usize {
        let proportional = survived_bytes.saturating_mul(self.growth_percent) / 100;
        if proportional > self.min_bytes {
            proportional
        } else {
            self.min_bytes
        }
    }
}

impl Default for Pacing {
    /// A 256 KiB minimum budget, and 100% growth (the heap may double between
    /// collections).
    fn default() -> Self {
        Self {
            min_bytes: 256 * 1024,
            growth_percent: 100,
        }
    }
}

/// A garbage-collected arena whose root is `R::Root<'gc>`.
///
/// - Allocation and mutation happen inside [`mutate`](Self::mutate) and
///   [`mutate_root`](Self::mutate_root).
/// - Collection happens only *between* them: [`collect_all`](Self::collect_all)
///   and [`collect_debt`](Self::collect_debt) need `&mut self` (invariant I1).
pub struct Arena<R: Rootable> {
    // `root` is declared first so that it is dropped first, while every box is
    // still allocated (I4).
    root: R::Root<'static>,
    mutation: Mutation<'static>,
}

impl<R: Rootable> Arena<R> {
    /// Create an arena with default [`Pacing`]. `f` builds the root.
    pub fn new(f: impl for<'gc> FnOnce(&Mutation<'gc>) -> R::Root<'gc>) -> Self {
        Self::with_pacing(Pacing::default(), f)
    }

    /// Create an arena with explicit pacing.
    pub fn with_pacing(
        pacing: Pacing,
        f: impl for<'gc> FnOnce(&Mutation<'gc>) -> R::Root<'gc>,
    ) -> Self {
        let mutation = Mutation::new(Heap::new(pacing));
        // `f` is generic over `'gc`, so instantiating it at `'static` can't leak
        // pointers anywhere except into the root, which is traced (I2).
        let root = f(&mutation);
        Self { root, mutation }
    }

    /// Run a mutation scope with shared access to the root.
    pub fn mutate<T>(&self, f: impl for<'gc> FnOnce(&Mutation<'gc>, &R::Root<'gc>) -> T) -> T {
        f(&self.mutation, &self.root)
    }

    /// Run a mutation scope with exclusive access to the root.
    pub fn mutate_root<T>(
        &mut self,
        f: impl for<'gc> FnOnce(&Mutation<'gc>, &mut R::Root<'gc>) -> T,
    ) -> T {
        f(&self.mutation, &mut self.root)
    }

    /// A full stop-the-world collection.
    pub fn collect_all(&mut self) {
        self.mutation.heap_mut().collect(&self.root);
    }

    /// Collect if the allocation budget is spent ([`Pacing`]). Returns whether
    /// it collected.
    pub fn collect_debt(&mut self) -> bool {
        let due = self.mutation.wants_collection();
        if due {
            self.collect_all();
        }
        due
    }

    /// Current heap metrics.
    pub fn metrics(&self) -> Metrics {
        self.mutation.metrics()
    }
}

impl<R: Rootable> fmt::Debug for Arena<R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Arena")
            .field("metrics", &self.metrics())
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::Pacing;

    #[test]
    fn budget_is_the_larger_of_the_minimum_and_the_growth_share() {
        let pacing = Pacing {
            min_bytes: 1000,
            growth_percent: 50,
        };
        assert_eq!(pacing.budget(0), 1000);
        assert_eq!(pacing.budget(1000), 1000);
        assert_eq!(pacing.budget(10_000), 5_000);
        assert_eq!(pacing.budget(usize::MAX), usize::MAX / 100);
    }
}
