//! The text corpus (`spikes/s5/corpus/corpus.txt`), compiled in so that a
//! sandboxed helper never reads it from disk.

const CORPUS: &str = include_str!("../../corpus/corpus.txt");

/// One line of the corpus.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Sample {
    pub(crate) id: &'static str,
    pub(crate) language: &'static str,
    /// A font name for CoreText, or `system` for its UI font.
    pub(crate) base: &'static str,
    pub(crate) text: &'static str,
}

pub(crate) fn samples() -> Vec<Sample> {
    CORPUS
        .lines()
        .filter(|line| !line.starts_with('#') && !line.trim().is_empty())
        .filter_map(|line| {
            let mut fields = line.splitn(4, '\t');
            Some(Sample {
                id: fields.next()?,
                language: fields.next()?,
                base: fields.next()?,
                text: fields.next()?,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_corpus_covers_every_script_s5_q1_names() {
        let ids: Vec<&str> = samples().iter().map(|s| s.id).collect();
        for required in [
            "latin",
            "greek",
            "cyrillic",
            "arabic",
            "hebrew",
            "devanagari",
            "bengali",
            "tamil",
            "thai",
            "khmer",
            "myanmar",
            "han-simplified",
            "han-traditional",
            "japanese",
            "hangul",
            "emoji",
        ] {
            assert!(ids.contains(&required), "{required}");
        }
        assert!(samples().iter().all(|s| !s.text.is_empty()));
    }
}
