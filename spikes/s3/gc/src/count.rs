//! Root-stack operation counters, for S3-Q2 (feature `count`).
//!
//! Each frame opened, value rooted and rooted value read is counted against
//! the current [`Context`], which the interpreter sets: bytecode, a builtin,
//! or a coercion hook. Without the feature every function here is empty, so
//! callers need no `cfg` of their own and timing builds pay nothing.

/// Who is running native code that touches the root stack.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Context {
    /// The interpreter executing bytecode.
    Interpreter,
    /// A builtin (`forEach`, `map`, `sortBy`, …) or other native code.
    Builtin,
    /// The interpreter converting an object to a number through its hook.
    Coercion,
}

/// Root operations, indexed by [`Context`] (`Context::X as usize`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RootCounts {
    pub frames: [u64; 3],
    pub pushes: [u64; 3],
    pub reads: [u64; 3],
}

impl RootCounts {
    /// Every operation, in every context.
    #[must_use]
    pub fn total(&self) -> u64 {
        (0..3)
            .map(|c| self.frames[c] + self.pushes[c] + self.reads[c])
            .sum()
    }

    /// Every operation performed in `context`.
    #[must_use]
    pub fn in_context(&self, context: Context) -> u64 {
        let c = context as usize;
        self.frames[c] + self.pushes[c] + self.reads[c]
    }
}

#[cfg(feature = "count")]
mod imp {
    use core::cell::Cell;

    use super::{Context, RootCounts};

    thread_local! {
        static CONTEXT: Cell<Context> = const { Cell::new(Context::Interpreter) };
        static COUNTS: Cell<RootCounts> = const {
            Cell::new(RootCounts { frames: [0; 3], pushes: [0; 3], reads: [0; 3] })
        };
    }

    pub(super) fn set_context(context: Context) -> Context {
        CONTEXT.with(|c| c.replace(context))
    }

    pub(super) fn take() -> RootCounts {
        COUNTS.with(Cell::take)
    }

    pub(super) fn bump(field: fn(&mut RootCounts) -> &mut [u64; 3]) {
        let context = CONTEXT.with(Cell::get) as usize;
        COUNTS.with(|c| {
            let mut counts = c.get();
            field(&mut counts)[context] += 1;
            c.set(counts);
        });
    }
}

/// Set the context for the operations that follow; returns the previous one.
#[inline]
pub fn set_context(context: Context) -> Context {
    #[cfg(feature = "count")]
    return imp::set_context(context);
    #[cfg(not(feature = "count"))]
    {
        let _ = context;
        Context::Interpreter
    }
}

/// The counts since the last call, which resets them. All zero without the
/// feature.
#[must_use]
pub fn take() -> RootCounts {
    #[cfg(feature = "count")]
    return imp::take();
    #[cfg(not(feature = "count"))]
    RootCounts::default()
}

/// Whether this build counts.
#[must_use]
pub const fn enabled() -> bool {
    cfg!(feature = "count")
}

#[inline]
pub(crate) fn frame() {
    #[cfg(feature = "count")]
    imp::bump(|c| &mut c.frames);
}

#[inline]
pub(crate) fn push() {
    #[cfg(feature = "count")]
    imp::bump(|c| &mut c.pushes);
}

#[inline]
pub(crate) fn read() {
    #[cfg(feature = "count")]
    imp::bump(|c| &mut c.reads);
}
