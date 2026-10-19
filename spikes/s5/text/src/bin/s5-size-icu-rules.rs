//! # Spike S5, S5-Q4: what ICU4X's rule-based line breaker adds
//!
//! Usage: `s5-size-icu-rules TEXT` prints the number of line-break
//! opportunities, with the segmenter for scripts that need no dictionary or
//! model.
#![forbid(unsafe_code)]

use icu_segmenter::LineSegmenter;
use icu_segmenter::options::LineBreakOptions;

fn main() {
    let text = std::env::args().nth(1).unwrap_or_default();
    let segmenter = LineSegmenter::new_for_non_complex_scripts(LineBreakOptions::default());
    println!("{}", segmenter.segment_str(&text).count());
}
