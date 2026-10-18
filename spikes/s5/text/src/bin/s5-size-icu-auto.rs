//! # Spike S5, S5-Q4: what ICU4X's line breaker adds with its "auto" data
//!
//! Usage: `s5-size-icu-auto TEXT` prints the number of line-break
//! opportunities, with the LSTM models for South-East Asian scripts and the
//! dictionary for Chinese and Japanese, as TX1 specifies.
#![forbid(unsafe_code)]

use icu_segmenter::LineSegmenter;
use icu_segmenter::options::LineBreakOptions;

fn main() {
    let text = std::env::args().nth(1).unwrap_or_default();
    let segmenter = LineSegmenter::new_auto(LineBreakOptions::default());
    println!("{}", segmenter.segment_str(&text).count());
}
