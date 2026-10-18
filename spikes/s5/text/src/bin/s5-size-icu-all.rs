//! # Spike S5, S5-Q4: what ICU4X adds for everything text needs
//!
//! Usage: `s5-size-icu-all TEXT`: line, word, grapheme and sentence
//! segmentation with the "auto" data, and each character's script from
//! `icu_properties`.
#![forbid(unsafe_code)]

use icu_properties::CodePointMapData;
use icu_properties::props::Script;
use icu_segmenter::options::{
    LineBreakOptions, SentenceBreakInvariantOptions, WordBreakInvariantOptions,
};
use icu_segmenter::{GraphemeClusterSegmenter, LineSegmenter, SentenceSegmenter, WordSegmenter};

fn main() {
    let text = std::env::args().nth(1).unwrap_or_default();
    let lines = LineSegmenter::new_auto(LineBreakOptions::default())
        .segment_str(&text)
        .count();
    let words = WordSegmenter::new_auto(WordBreakInvariantOptions::default())
        .segment_str(&text)
        .count();
    let graphemes = GraphemeClusterSegmenter::new().segment_str(&text).count();
    let sentences = SentenceSegmenter::new(SentenceBreakInvariantOptions::default())
        .segment_str(&text)
        .count();
    let scripts = CodePointMapData::<Script>::new();
    let latin = text
        .chars()
        .filter(|&c| scripts.get(c) == Script::Latin)
        .count();
    println!("{lines} {words} {graphemes} {sentences} {latin}");
}
