//! Without the counting allocator, a count refuses to read zero (ES23).
#![forbid(unsafe_code)]

#[test]
#[should_panic(expected = "isn't this test binary's global allocator")]
fn refuses_to_count_without_the_counting_allocator() {
    let _ = test_support::count_allocations(|| ());
}
