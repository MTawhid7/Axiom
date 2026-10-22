//! Instructions retired, read from `proc_pid_rusage` (ES23). One test in its
//! own binary, since the count covers the whole process.
#![forbid(unsafe_code)]

use std::hint::black_box;

fn work(iterations: u64) -> u64 {
    let mut sum = 0_u64;
    for value in 0..iterations {
        sum = black_box(sum.wrapping_add(black_box(value)));
    }
    sum
}

#[test]
#[cfg_attr(miri, ignore = "Miri can't call proc_pid_rusage")]
fn counts_the_work_steadily() {
    const ITERATIONS: u64 = 1_000_000;
    let (_, first) =
        test_support::count_cpu(|| work(ITERATIONS)).expect("proc_pid_rusage on this process");
    let (_, second) =
        test_support::count_cpu(|| work(ITERATIONS)).expect("proc_pid_rusage on this process");
    let (first, second) = (first.instructions, second.instructions);
    assert!(
        first > ITERATIONS,
        "{first} instructions for {ITERATIONS} iterations"
    );
    // S16 found whole processes' counts steady to 0.15%; a closure's, with the
    // test harness idle around it, within a few percent.
    let (low, high) = (first.min(second), first.max(second));
    assert!(
        high - low < high / 20,
        "counts {first} and {second} differ by more than 5%"
    );
}
