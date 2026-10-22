//! S16-Q3's built-in workloads, each a fixed amount of work, so that a
//! counter that measures work should read the same in every run: `cpu`
//! (sorting a fixed pseudo-random array, five times), `alloc` (filling
//! 128 MiB, twice), `disk` (writing 32 MiB and syncing it), and `spin`, the
//! background load, which runs until it is killed.

use std::io::Write as _;
use std::path::Path;

pub(crate) fn run(name: &str, dir: Option<&Path>) -> Result<(), String> {
    match name {
        "cpu" => {
            std::hint::black_box(cpu());
            Ok(())
        }
        "alloc" => {
            alloc();
            Ok(())
        }
        "disk" => disk(dir.ok_or("--dir is required")?),
        "spin" => loop {
            std::hint::spin_loop();
        },
        other => Err(format!("no workload named {other}")),
    }
}

fn cpu() -> u64 {
    let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut total = 0_u64;
    for _ in 0..5 {
        let mut values: Vec<u32> = (0..2_000_000)
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                u32::try_from(state >> 32).unwrap_or(0)
            })
            .collect();
        values.sort_unstable();
        total = values.iter().step_by(1_000).fold(total, |acc, &v| {
            acc.wrapping_mul(31).wrapping_add(u64::from(v))
        });
    }
    total
}

fn alloc() {
    for round in 1..=2_u8 {
        let mut bytes = vec![0_u8; 128 << 20];
        bytes.fill(round);
        std::hint::black_box(bytes[12_345]);
    }
}

fn disk(dir: &Path) -> Result<(), String> {
    let path = dir.join(format!("s16-disk-{}.bin", std::process::id()));
    let block = vec![0xa5_u8; 1 << 20];
    let mut file = std::fs::File::create(&path).map_err(|e| e.to_string())?;
    for _ in 0..32 {
        file.write_all(&block).map_err(|e| e.to_string())?;
    }
    file.sync_all().map_err(|e| e.to_string())?;
    drop(file);
    std::fs::remove_file(&path).map_err(|e| e.to_string())
}
