//! S10-Q3: what PL9's validation costs on page-sized lists, in one process:
//! the decoder copies and checks lists of 1,000 items, of 10,000, and of
//! S6's page, 44,372; then changed bytes are fed to it, to show it refuses
//! them without panicking.

use std::fmt::Write as _;
use std::hint::black_box;

use crate::display_list::{self, Context};
use crate::platform::time;
use crate::stats::summarize;

#[allow(clippy::cast_precision_loss)] // sizes far below 2^52
fn to_f64(n: usize) -> f64 {
    n as f64
}

pub(crate) fn run(
    sizes: &[usize],
    iterations: usize,
    out: Option<&std::path::Path>,
) -> Result<(), String> {
    let registered = [1_u32];
    let tiles = 70;
    let context = Context {
        registered: &registered,
        tiles,
    };
    let mut text = String::from(
        "S10-Q3: decoding and validating a display list in one process (copy, then check: IR7, PL9)\n\n\
         | Items | Bytes | Median µs | p95 µs | ns per item | MB/s |\n|---|---|---|---|---|---|\n",
    );
    for &items in sizes {
        let mut list = display_list::page(items, 2560.0, Some(1));
        list.damage = (0..tiles).collect();
        let mut bytes = Vec::new();
        list.encode(&mut bytes);
        // Check the list round-trips; then decode it for 100 ms, so the core
        // reaches its working frequency before the samples.
        let decoded = display_list::decode(&bytes, &context).map_err(|v| v.to_string())?;
        if decoded != list {
            return Err("the list didn't round-trip".to_owned());
        }
        let warm_until = time::now() + time::ticks(100_000_000);
        while time::now() < warm_until {
            black_box(display_list::decode(black_box(&bytes), &context).is_ok());
        }
        let mut samples = Vec::with_capacity(iterations);
        for _ in 0..iterations {
            let started = time::now();
            let decoded = display_list::decode(black_box(&bytes), &context);
            black_box(&decoded);
            samples.push(
                to_f64(usize::try_from(time::ns(time::now() - started)).unwrap_or(0)) / 1_000.0,
            );
        }
        let s = summarize(&samples);
        let _ = writeln!(
            text,
            "| {items} | {} | {:.1} | {:.1} | {:.1} | {:.0} |",
            bytes.len(),
            s.median,
            s.p95,
            s.median * 1_000.0 / to_f64(items),
            to_f64(bytes.len()) / s.median
        );
    }
    // Every single-byte change of a 1,000-item list either decodes or is
    // refused; a panic would end the run.
    let list = display_list::page(1_000, 2560.0, Some(1));
    let mut bytes = Vec::new();
    list.encode(&mut bytes);
    let (mut refused, mut accepted) = (0_usize, 0_usize);
    for at in 0..bytes.len() {
        for flip in [0x01_u8, 0x80, 0xff] {
            let mut changed = bytes.clone();
            changed[at] ^= flip;
            match display_list::decode(&changed, &context) {
                Ok(_) => accepted += 1,
                Err(_) => refused += 1,
            }
        }
    }
    let _ = writeln!(
        text,
        "\nChanged bytes: {} single-byte changes to a 1,000-item list; {refused} refused, {accepted} still valid (a changed colour or position is a valid list); no panic",
        refused + accepted
    );
    print!("{text}");
    if let Some(dir) = out {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        std::fs::write(dir.join("validate.md"), &text).map_err(|e| e.to_string())?;
    }
    Ok(())
}
