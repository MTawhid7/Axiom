//! The root stacks' behaviour, through the public API: pushes, reads,
//! frames, out-of-order frames, marking and relocation, overflow, barriers.
//! Miri runs this file to check `RawStack`'s `unsafe` (R1–R3). `SliceStack`
//! and its field-order control have none.
#![forbid(unsafe_code)]

use s3_gc::{
    Barrier, CardBarrier, CellStack, Forwarding, Handle, Marker, Pacing, RawStack, RefCellStack,
    RootFrame, RootStack, Roots, SliceStack, SliceStackLenLast, Trace, Value, heap, setters,
    slot_buffer,
};

struct Obj {
    next: Option<Handle<'static, Obj>>,
    id: u32,
}

heap! {
    struct Heap {
        objects: Obj,
    }
}

setters! {
    Heap.objects: Obj {
        set_next => next: Option<Handle<'static, Obj>>,
    }
}

impl Trace for Obj {
    fn mark(&self, marker: &mut Marker) {
        marker.mark_opt(self.next);
    }

    fn relocate(&mut self, forwarding: &Forwarding) {
        forwarding.relocate_opt(&mut self.next);
    }
}

struct NoRoots;

impl Roots for NoRoots {
    fn mark(&self, _: &mut Marker) {}
    fn relocate(&mut self, _: &Forwarding) {}
}

fn pushes_and_reads_follow_the_frames<S: RootStack>(stack: &S) {
    let outer = RootFrame::new(stack);
    let values: Vec<Value> = (0..2500).map(Value::int).collect(); // crosses CellStack chunks
    let rooted: Vec<_> = values.iter().map(|v| v.scope(&outer)).collect();
    assert_eq!(stack.height(), values.len());
    {
        let inner = RootFrame::new(stack);
        let extra = Value::TRUE.scope(&inner);
        assert_eq!(extra.get(), Value::TRUE);
        assert_eq!(stack.height(), values.len() + 1);
    }
    assert_eq!(
        stack.height(),
        values.len(),
        "the inner frame popped its root"
    );
    for (scoped, value) in rooted.iter().zip(&values) {
        assert_eq!(scoped.get(), *value);
    }
    let mut seen = Vec::new();
    stack.for_each(|v| seen.push(v));
    assert_eq!(seen, values, "bottom to top");
    drop(rooted);
    drop(outer);
    assert_eq!(stack.height(), 0);
}

fn out_of_order_frames_are_logic_errors_not_memory_errors<S: RootStack>(stack: &S) {
    let outer = RootFrame::new(stack);
    let _a = Value::int(1).scope(&outer);
    let inner = RootFrame::new(stack);
    let b = Value::int(2).scope(&inner);
    drop(outer); // wrong order: pops b's slot too
    assert_eq!(stack.height(), 0);
    let replacement = RootFrame::new(stack);
    let c = Value::int(3).scope(&replacement);
    let _d = Value::int(4).scope(&replacement);
    // b's slot now holds d: a stale read, but a valid value.
    assert_eq!(b.get(), Value::int(4));
    assert_eq!(c.get(), Value::int(3));
    drop(replacement);
    assert_eq!(stack.height(), 0);
    drop(inner); // its height is 1: a truncation never grows the stack
    assert_eq!(stack.height(), 0);
}

fn collection_relocates_every_rooted_value<S: RootStack>(stack: &S) {
    let mut heap: Heap<'_, S> = Heap::new(stack, Pacing::default());
    let frame = heap.core.frame();
    let mut rooted = Vec::new();
    for id in 0..300 {
        heap.alloc(Obj {
            next: None,
            id: 1000,
        }); // garbage in between
        let kept = heap.alloc(Obj { next: None, id });
        rooted.push(kept.scope(&frame));
    }
    heap.collect(&mut NoRoots);
    assert_eq!(heap.objects.len(), 300, "only the rooted objects survive");
    for (id, scoped) in (0..).zip(&rooted) {
        assert_eq!(heap.objects[scoped.get()].id, id);
    }
    drop(rooted);
    drop(frame);
    heap.collect(&mut NoRoots);
    assert!(heap.objects.is_empty());
}

macro_rules! on_every_stack {
    ($($test:ident),+ $(,)?) => {
        mod refcell_stack {
            $(#[test] fn $test() { super::$test(&s3_gc::RefCellStack::default()); })+
        }
        mod cell_stack {
            $(#[test] fn $test() { super::$test(&s3_gc::CellStack::default()); })+
        }
        mod raw_stack {
            $(#[test] fn $test() { super::$test(&s3_gc::RawStack::default()); })+
        }
        mod slice_stack {
            $(#[test] fn $test() {
                let mut buffer = s3_gc::slot_buffer(s3_gc::DEFAULT_CAPACITY);
                super::$test(&s3_gc::SliceStack::new(&mut buffer));
            })+
        }
        mod slice_stack_len_last {
            $(#[test] fn $test() {
                let mut buffer = s3_gc::slot_buffer(s3_gc::DEFAULT_CAPACITY);
                super::$test(&s3_gc::SliceStackLenLast::new(&mut buffer));
            })+
        }
    };
}

on_every_stack!(
    pushes_and_reads_follow_the_frames,
    out_of_order_frames_are_logic_errors_not_memory_errors,
    collection_relocates_every_rooted_value,
);

#[test]
#[should_panic(expected = "root stack overflow")]
fn a_full_raw_stack_panics() {
    let stack = RawStack::with_capacity(4);
    let frame = RootFrame::new(&stack);
    let _rooted: Vec<_> = (0..5).map(|i| Value::int(i).scope(&frame)).collect();
}

#[test]
fn a_raw_stack_starts_zeroed_and_frees_its_slots() {
    let stack = RawStack::with_capacity(3);
    assert_eq!(stack.capacity(), 3);
    assert_eq!(stack.height(), 0);
    let frame = RootFrame::new(&stack);
    let all: Vec<_> = (0..3).map(|i| Value::int(i).scope(&frame)).collect();
    assert_eq!(all[2].get(), Value::int(2));
    drop(all);
    drop(frame);
    drop(stack); // Miri's leak check confirms the allocation is freed
}

#[test]
#[should_panic(expected = "root stack overflow")]
fn a_full_slice_stack_panics() {
    let mut buffer = slot_buffer(4);
    let stack = SliceStack::new(&mut buffer);
    let frame = RootFrame::new(&stack);
    let _rooted: Vec<_> = (0..5).map(|i| Value::int(i).scope(&frame)).collect();
}

#[test]
fn a_slice_stack_borrows_its_buffer_for_its_lifetime() {
    let mut buffer = slot_buffer(3);
    {
        let stack = SliceStack::new(&mut buffer);
        assert_eq!((stack.capacity(), stack.height()), (3, 0));
        let frame = RootFrame::new(&stack);
        let all: Vec<_> = (0..3).map(|i| Value::int(i).scope(&frame)).collect();
        assert_eq!(all[2].get(), Value::int(2));
    }
    // The stack is gone, so the buffer is its owner's again, holding the
    // last values pushed.
    assert_eq!(buffer[2], Value::int(2).to_bits());
}

#[test]
fn the_stacks_store_eight_byte_slots() {
    assert_eq!(size_of::<Value>(), 8);
    // What a Scoped keeps: a slot reference, or S1's (stack, index) pair.
    assert_eq!(size_of::<<RawStack as RootStack>::Slot<'static>>(), 8);
    assert_eq!(size_of::<<CellStack as RootStack>::Slot<'static>>(), 8);
    assert_eq!(
        size_of::<<SliceStack<'static> as RootStack>::Slot<'static>>(),
        8
    );
    assert_eq!(
        size_of::<<SliceStackLenLast<'static> as RootStack>::Slot<'static>>(),
        8
    );
    assert_eq!(size_of::<<RefCellStack as RootStack>::Slot<'static>>(), 16);
}

#[test]
fn setters_write_through_the_card_barrier() {
    let stack = CellStack::default();
    let mut heap: Heap<'_, CellStack, CardBarrier> = Heap::new(&stack, Pacing::default());
    let objects: Vec<_> = (0..2000)
        .map(|id| heap.alloc(Obj { next: None, id }))
        .collect();
    assert_eq!(heap.core.barrier.dirty_cards(), 0);
    heap.set_next(objects[0], Some(objects[1]));
    heap.set_next(objects[1], Some(objects[0])); // same card
    heap.set_next(objects[1999], None); // last card
    assert_eq!(heap.objects[objects[0]].next, Some(objects[1]));
    assert_eq!(heap.core.barrier.dirty_cards(), 2);
    let frame = heap.core.frame();
    let kept = objects[0].scope(&frame);
    heap.collect(&mut NoRoots);
    assert_eq!(
        heap.core.barrier.dirty_cards(),
        0,
        "collection clears the cards"
    );
    assert_eq!(heap.objects.len(), 2, "the rooted object and its cycle");
    let next = heap.objects[kept.get()].next.expect("relocated");
    assert_eq!(heap.objects[next].id, 1);
    let mut cards = CardBarrier::default();
    cards.on_alloc(0, 0);
    cards.post_write(0, 0);
    assert_eq!(cards.dirty_cards(), 1);
}
