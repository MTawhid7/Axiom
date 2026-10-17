//! Builtins that call back into bytecode while holding values: the rooting
//! traffic ADR-0003 expects to dominate (S3-Q2).
//!
//! Each one is ordinary Rust in Pattern B's style. Values it holds across a
//! callback, which may collect, are rooted in a frame and read back after
//! every callback. The scope tokens enforce this: a handle bound to
//! `gc.nogc()` cannot be used after `gc.reborrow()` is passed to a call.
//!
//! Semantics follow JavaScript's `Array.prototype` methods where it matters
//! here: the length is read once, elements that have disappeared by the time
//! they are reached are skipped, and the sort is stable.

use s3_gc::count::{self, Context};
use s3_gc::{GcScope, RootStack, Value};

use crate::heap::Array;
use crate::machine::{Cx, VmError};

/// The builtin functions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Builtin {
    /// `forEach(array, f)`: calls `f(element, index)` for each element.
    ForEach = 0,
    /// `map(array, f)`: a new array of `f(element, index)`.
    Map = 1,
    /// `sortBy(array, compare)`: sorts in place, stably; `compare(x, y) > 0`
    /// puts `y` first. Returns the array.
    SortBy = 2,
}

impl Builtin {
    pub const ALL: [Self; 3] = [Self::ForEach, Self::Map, Self::SortBy];

    #[must_use]
    pub fn from_id(id: u8) -> Option<Self> {
        Self::ALL.get(usize::from(id)).copied()
    }

    /// This builtin as a callable value.
    #[must_use]
    pub const fn value(self) -> Value {
        Value::builtin(self as u8)
    }
}

/// Call builtin `id`. Callbacks' frames start above register `top`.
pub(crate) fn call<S: RootStack>(
    cx: &mut Cx<'_, '_, S>,
    id: u8,
    args: &[Value],
    top: usize,
    gc: GcScope<'_>,
) -> Result<Value, VmError> {
    cx.core.stats.builtin_call();
    let previous = count::set_context(Context::Builtin);
    let result = match Builtin::from_id(id) {
        Some(Builtin::ForEach) => for_each(cx, args, top, gc),
        Some(Builtin::Map) => map(cx, args, top, gc),
        Some(Builtin::SortBy) => sort_by(cx, args, top, gc),
        None => Err(VmError::NotCallable),
    };
    count::set_context(previous);
    result
}

fn arg(args: &[Value], k: usize) -> Value {
    args.get(k).copied().unwrap_or(Value::UNDEFINED)
}

fn index_value(i: usize) -> Value {
    Value::int(i32::try_from(i).expect("array indices fit i32"))
}

fn for_each<S: RootStack>(
    cx: &mut Cx<'_, '_, S>,
    args: &[Value],
    top: usize,
    mut gc: GcScope<'_>,
) -> Result<Value, VmError> {
    let array = arg(args, 0).to_handle::<Array>().ok_or(VmError::NotArray)?;
    let f = arg(args, 1);
    let len = cx.core.heap.arrays[array].items.len();
    let frame = cx.core.heap.core.frame();
    let array = array.scope(&frame);
    let f = f.scope(&frame);
    for i in 0..len {
        let items = &cx.core.heap.arrays[array.get().bind(gc.nogc())].items;
        let Some(&element) = items.get(i) else {
            continue;
        };
        cx.call_value(f.get(), &[element, index_value(i)], top, gc.reborrow())?;
    }
    Ok(Value::UNDEFINED)
}

fn map<S: RootStack>(
    cx: &mut Cx<'_, '_, S>,
    args: &[Value],
    top: usize,
    mut gc: GcScope<'_>,
) -> Result<Value, VmError> {
    let array = arg(args, 0).to_handle::<Array>().ok_or(VmError::NotArray)?;
    let f = arg(args, 1);
    let len = cx.core.heap.arrays[array].items.len();
    let frame = cx.core.heap.core.frame();
    let array = array.scope(&frame);
    let f = f.scope(&frame);
    cx.safepoint(gc.reborrow());
    let result = cx.core.heap.alloc(Array {
        items: vec![Value::UNDEFINED; len],
    });
    let result = result.scope(&frame);
    for i in 0..len {
        let items = &cx.core.heap.arrays[array.get().bind(gc.nogc())].items;
        let Some(&element) = items.get(i) else {
            continue;
        };
        let mapped = cx.call_value(f.get(), &[element, index_value(i)], top, gc.reborrow())?;
        let out = &mut cx.core.heap.arrays[result.get().bind(gc.nogc())].items;
        out[i] = mapped;
    }
    Ok(result.get().to_value())
}

/// A bottom-up merge sort between the array and a scratch array in the heap,
/// as V8 sorts through a temporary heap array. Every comparison re-reads both
/// arrays and the comparator from their roots.
fn sort_by<S: RootStack>(
    cx: &mut Cx<'_, '_, S>,
    args: &[Value],
    top: usize,
    mut gc: GcScope<'_>,
) -> Result<Value, VmError> {
    let array = arg(args, 0).to_handle::<Array>().ok_or(VmError::NotArray)?;
    let compare = arg(args, 1);
    let len = cx.core.heap.arrays[array].items.len();
    let frame = cx.core.heap.core.frame();
    let array = array.scope(&frame);
    if len < 2 {
        return Ok(array.get().to_value());
    }
    let compare = compare.scope(&frame);
    cx.safepoint(gc.reborrow());
    let scratch = cx.core.heap.alloc(Array {
        items: vec![Value::UNDEFINED; len],
    });
    let scratch = scratch.scope(&frame);

    // An element of `from`: the comparator may have shrunk the array.
    let element = |cx: &Cx<'_, '_, S>, from: Value, i: usize| -> Result<Value, VmError> {
        let from = from.to_handle::<Array>().expect("a sort array");
        cx.core.heap.arrays[from]
            .items
            .get(i)
            .copied()
            .ok_or(VmError::BadIndex)
    };

    let mut in_array = true; // which of the two holds the current runs
    let mut width = 1;
    while width < len {
        let mut lo = 0;
        while lo < len {
            let mid = (lo + width).min(len);
            let hi = (lo + 2 * width).min(len);
            let (mut i, mut j) = (lo, mid);
            for k in lo..hi {
                let from = || {
                    if in_array {
                        array.get().to_value()
                    } else {
                        scratch.get().to_value()
                    }
                };
                let take_left = if i < mid && j < hi {
                    let (x, y) = (element(cx, from(), i)?, element(cx, from(), j)?);
                    let order = cx.call_value(compare.get(), &[x, y], top, gc.reborrow())?;
                    let order = cx.coerce_number(order, top + 1, gc.reborrow())?;
                    order <= 0.0 || order.is_nan()
                } else {
                    i < mid
                };
                let v = element(cx, from(), if take_left { i } else { j })?;
                let to = if in_array { scratch.get() } else { array.get() };
                let out = &mut cx.core.heap.arrays[to.bind(gc.nogc())].items;
                *out.get_mut(k).ok_or(VmError::BadIndex)? = v;
                if take_left {
                    i += 1;
                } else {
                    j += 1;
                }
            }
            lo = hi;
        }
        in_array = !in_array;
        width *= 2;
    }
    if !in_array {
        let sorted = cx.core.heap.arrays[scratch.get()].items.clone();
        let items = &mut cx.core.heap.arrays[array.get()].items;
        let n = sorted.len().min(items.len());
        items[..n].copy_from_slice(&sorted[..n]);
    }
    Ok(array.get().to_value())
}
