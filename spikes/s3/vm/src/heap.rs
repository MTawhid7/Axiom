//! The interpreter's heap: objects, arrays and closures in typed arenas, and
//! the register file as one traced region (ADR-0008 SH6, part 1).

use core::cell::Cell;
use std::collections::HashMap;

use s3_gc::{Barrier, Forwarding, Marker, RootStack, Roots, Trace, Value, heap};

use crate::digest::Digest;

/// An object with four slots. 32 bytes.
#[derive(Debug)]
pub struct Object {
    pub slots: [Value; 4],
}

/// A dense array.
#[derive(Debug)]
pub struct Array {
    pub items: Vec<Value>,
}

/// A function and the environment it closes over.
#[derive(Debug)]
pub struct Closure {
    pub function: u32,
    /// An object whose slots are the captured variables, or `undefined`.
    pub env: Value,
}

heap! {
    /// One agent's heap.
    #[derive(Debug)]
    pub struct VmHeap {
        objects: Object,
        arrays: Array,
        closures: Closure,
    }
}

impl Trace for Object {
    fn mark(&self, marker: &mut Marker) {
        for &value in &self.slots {
            marker.mark_value(value);
        }
    }

    fn relocate(&mut self, forwarding: &Forwarding) {
        for value in &mut self.slots {
            forwarding.relocate_value(value);
        }
    }
}

impl Trace for Array {
    fn mark(&self, marker: &mut Marker) {
        for &value in &self.items {
            marker.mark_value(value);
        }
    }

    fn relocate(&mut self, forwarding: &Forwarding) {
        for value in &mut self.items {
            forwarding.relocate_value(value);
        }
    }
}

impl Trace for Closure {
    fn mark(&self, marker: &mut Marker) {
        marker.mark_value(self.env);
    }

    fn relocate(&mut self, forwarding: &Forwarding) {
        forwarding.relocate_value(&mut self.env);
    }
}

/// The register file's used prefix, traced as one region: executing bytecode
/// roots nothing value by value. Registers are `Cell`s, so the collector
/// relocates them through a shared reference while the interpreter holds
/// windows into them.
pub(crate) struct Registers<'a>(pub(crate) &'a [Cell<Value>]);

impl Roots for Registers<'_> {
    fn mark(&self, marker: &mut Marker) {
        for register in self.0 {
            marker.mark_value(register.get());
        }
    }

    fn relocate(&mut self, forwarding: &Forwarding) {
        for register in self.0 {
            register.set(forwarding.relocated(register.get()));
        }
    }
}

/// The canonical digest (see [`crate::digest`]) of `value` and everything it
/// reaches.
pub fn digest<S: RootStack, B: Barrier>(heap: &VmHeap<'_, S, B>, value: Value) -> u64 {
    let mut d = Digest::default();
    let mut seen = HashMap::new();
    let mut stack = vec![value];
    while let Some(v) = stack.pop() {
        if let Some(x) = v.as_f64() {
            d.number(x);
        } else if let Some(b) = v.as_bool() {
            d.boolean(b);
        } else if let Some(id) = v.as_builtin() {
            d.builtin(id);
        } else if v == Value::UNDEFINED {
            d.undefined();
        } else if v == Value::NULL {
            d.null();
        } else {
            let n = seen.len();
            if let Some(&first) = seen.get(&v.identity()) {
                d.backref(first);
                continue;
            }
            seen.insert(v.identity(), n);
            if let Some(o) = v.to_handle::<Object>() {
                d.object();
                stack.extend(heap.objects[o].slots.iter().rev());
            } else if let Some(a) = v.to_handle::<Array>() {
                let items = &heap.arrays[a].items;
                d.array(items.len());
                stack.extend(items.iter().rev());
            } else if let Some(c) = v.to_handle::<Closure>() {
                d.closure(heap.closures[c].function);
                stack.push(heap.closures[c].env);
            } else {
                unreachable!("every value kind is covered: {v:?}");
            }
        }
    }
    d.finish()
}
