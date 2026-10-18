//! What the machine did, for S3-Q2 (feature `count`). Without the feature the
//! counters stay zero and every update compiles to nothing.
#![expect(
    clippy::inline_always,
    reason = "counting must vanish from the dispatch loops when the feature is off"
)]
#![cfg_attr(
    not(feature = "count"),
    expect(
        clippy::unused_self,
        reason = "the counters are only updated with the feature"
    )
)]

use crate::bytecode::op;

/// Counts since the last [`Vm::take_stats`](crate::Vm::take_stats).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stats {
    /// Instructions executed, by opcode.
    pub by_op: [u64; op::COUNT],
    /// Bytecode calling a closure.
    pub bytecode_calls: u64,
    /// Native code calling a closure: builtins' callbacks, hooks, the entry.
    pub native_calls: u64,
    /// Builtins called, from bytecode or native code.
    pub builtin_calls: u64,
    /// Objects converted to numbers through their hook.
    pub coercions: u64,
}

impl Default for Stats {
    fn default() -> Self {
        Self {
            by_op: [0; op::COUNT],
            bytecode_calls: 0,
            native_calls: 0,
            builtin_calls: 0,
            coercions: 0,
        }
    }
}

impl Stats {
    /// Instructions executed.
    #[must_use]
    pub fn instructions(&self) -> u64 {
        self.by_op.iter().sum()
    }

    #[inline(always)]
    pub(crate) fn instruction(&mut self, opcode: u8) {
        #[cfg(feature = "count")]
        {
            self.by_op[usize::from(opcode)] += 1;
        }
        #[cfg(not(feature = "count"))]
        let _ = opcode;
    }

    #[inline(always)]
    pub(crate) fn bytecode_call(&mut self) {
        #[cfg(feature = "count")]
        {
            self.bytecode_calls += 1;
        }
    }

    #[inline(always)]
    pub(crate) fn native_call(&mut self) {
        #[cfg(feature = "count")]
        {
            self.native_calls += 1;
        }
    }

    #[inline(always)]
    pub(crate) fn builtin_call(&mut self) {
        #[cfg(feature = "count")]
        {
            self.builtin_calls += 1;
        }
    }

    #[inline(always)]
    pub(crate) fn coercion(&mut self) {
        #[cfg(feature = "count")]
        {
            self.coercions += 1;
        }
    }
}
