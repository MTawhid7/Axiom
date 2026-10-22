//! The counting allocator, as this binary's global allocator (ES23).
#![forbid(unsafe_code)]

use std::hint::black_box;

use test_support::{Allocations, CountingAllocator, count_allocations};

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

#[test]
fn counts_one_allocation_with_its_size() {
    let (vector, allocations) = count_allocations(|| black_box(Vec::<u64>::with_capacity(10)));
    assert_eq!(
        allocations,
        Allocations {
            count: 1,
            bytes: 80
        }
    );
    drop(vector);
}

/// The test harness runs the other tests on parallel threads, allocating as
/// they go; none of that counts here, since counts are per thread.
#[test]
fn counts_nothing_when_nothing_allocates() {
    let ((), allocations) = count_allocations(|| {
        black_box(1_u64 + black_box(2));
    });
    assert_eq!(allocations, Allocations::default());
}

#[test]
fn counts_a_reallocation_at_its_new_size() {
    let mut vector = Vec::<u8>::with_capacity(4);
    let ((), allocations) = count_allocations(|| vector.reserve_exact(64));
    assert_eq!(
        allocations,
        Allocations {
            count: 1,
            bytes: 64
        }
    );
}
