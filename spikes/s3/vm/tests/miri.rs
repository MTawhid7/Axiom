//! Every workload, small, on C1's root stack, with a collection at every
//! allocation: sized so that Miri can run it,
//! `cargo +nightly miri test -p s3-vm --test miri`. It checks `RawStack`'s
//! `unsafe` (`s3-gc`'s `src/raw/`) under the builtins' real rooting traffic,
//! including collections in the middle of `map` and `sortBy`.
#![forbid(unsafe_code)]

use s3_gc::{Pacing, RawStack, RootStack};
use s3_vm::workloads::{w1_arith, w2_churn, w3_callbacks, w4_tree};
use s3_vm::{Loaded, Technique, Vm};

#[test]
fn every_workload_runs_on_the_raw_stack_under_stress() {
    for w in [
        w1_arith(30),
        w2_churn(2, 20),
        w3_callbacks(12),
        w4_tree(12, 12),
    ] {
        for technique in [Technique::Match, Technique::Threaded] {
            let stack = RawStack::default();
            let loaded = Loaded::<RawStack>::new(w.program.clone()).expect("verified");
            let mut vm = Vm::new(&stack, Pacing::STRESS);
            let value = vm.run(&loaded, technique, &w.args).expect("runs");
            assert_eq!(vm.digest(value), w.expected, "{} on {technique}", w.name);
            assert!(
                vm.metrics().collections > 0,
                "{}: stress pacing collects",
                w.name
            );
            assert_eq!(stack.height(), 0, "every frame was popped");
        }
    }
}
