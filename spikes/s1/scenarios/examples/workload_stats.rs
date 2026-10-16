//! Prints the volume of every workload at every size, plus generation time.
//!
//!     cargo run --release -p s1-scenarios --example workload_stats [seed]
#![forbid(unsafe_code)]

use std::time::Instant;

use s1_scenarios::generate::{Size, Workload};

fn main() {
    let seed = std::env::args()
        .nth(1)
        .map_or(1, |s| s.parse().expect("seed must be a u64"));
    println!(
        "{:<10} {:<6} {:>9} {:>10} {:>10} {:>9} {:>10} {:>10} {:>9} {:>8}",
        "workload",
        "size",
        "ops",
        "nodes",
        "garbage",
        "listeners",
        "invocations",
        "max_live",
        "checkpts",
        "gen_ms"
    );
    for size in Size::ALL {
        for workload in Workload::ALL {
            let start = Instant::now();
            let g = workload.generate(size, seed);
            let elapsed = start.elapsed();
            let s = &g.stats;
            println!(
                "{:<10} {:<6} {:>9} {:>10} {:>10} {:>9} {:>10} {:>10} {:>9} {:>8.1}",
                workload.name(),
                format!("{size:?}"),
                g.script.ops.len(),
                s.nodes_allocated,
                s.garbage_allocated,
                s.listeners_allocated,
                s.invocations,
                s.max_live_nodes,
                g.expected.checkpoints.len(),
                elapsed.as_secs_f64() * 1e3,
            );
        }
    }
}
