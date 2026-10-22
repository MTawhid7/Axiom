//! Recording allocates nothing (OB5, OB27): counted with the counting
//! allocator as this binary's global allocator.
#![forbid(unsafe_code)]

use test_support::{Allocations, CountingAllocator, count_allocations};
use trace::{Category, Level, Name, OwnedRing};

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

#[test]
fn recording_every_kind_allocates_nothing() {
    let ring: &'static OwnedRing =
        Box::leak(Box::new(OwnedRing::new(1 << 16).expect("a power of two")));
    trace::install(ring.storage().expect("a valid ring"));
    let name = Name::new(1);
    let ((), allocations) = count_allocations(|| {
        for value in 0..100 {
            trace::span_begin(Category::Style, name, &[value, 2]);
            trace::event(Category::Style, name, &[value]);
            trace::counter(Category::Style, name, value);
            trace::gauge(Category::Style, name, value);
            trace::histogram(Category::Style, name, value);
            trace::flow_begin(Category::Ipc, name, value);
            trace::flow_end(Category::Ipc, name, value);
            trace::log(Level::Info, Category::Style, name, "a log of a few words");
            trace::span_end(Category::Style, name);
        }
    });
    assert_eq!(allocations, Allocations::default());

    // A full ring drops records, still without allocating.
    let small: &'static OwnedRing =
        Box::leak(Box::new(OwnedRing::new(128).expect("a power of two")));
    trace::install(small.storage().expect("a valid ring"));
    let ((), allocations) = count_allocations(|| {
        for value in 0..1000 {
            trace::counter(Category::Style, name, value);
        }
    });
    assert_eq!(allocations, Allocations::default());
    assert!(trace::uninstall().is_some_and(|dropped| dropped > 0));
}
