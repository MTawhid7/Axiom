//! Golden files (ES21): stable outputs compared with files under the calling
//! crate's `tests/golden/`, rewritten only when `AXIOM_BLESS=1` is set, so
//! that every change to one is a reviewed change in the commit's diff.

use std::fmt::{self, Write as _};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Whether a golden file is compared or rewritten.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Compare the text with the file; a missing file is a mismatch.
    Compare,
    /// Write the text to the file when it differs (`AXIOM_BLESS=1`).
    Bless,
}

impl Mode {
    /// `Bless` when the environment sets `AXIOM_BLESS=1`, else `Compare`.
    #[must_use]
    pub fn from_env() -> Self {
        #[expect(
            clippy::disallowed_methods,
            reason = "ES13: test crates may read the environment; AXIOM_BLESS is ES21's switch"
        )]
        let bless = std::env::var_os("AXIOM_BLESS");
        if bless.is_some_and(|value| value == "1") {
            Self::Bless
        } else {
            Self::Compare
        }
    }
}

/// Why a golden file didn't match.
#[derive(Debug)]
pub enum Mismatch {
    /// The file doesn't exist.
    Missing(PathBuf),
    /// The file exists and differs; the first differing lines, numbered.
    Differs(PathBuf, String),
    /// The file couldn't be read or written.
    Io(PathBuf, io::Error),
}

impl fmt::Display for Mismatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing(path) => {
                write!(
                    f,
                    "golden file {} is missing; run the test with AXIOM_BLESS=1 to create it",
                    path.display()
                )
            }
            Self::Differs(path, diff) => write!(
                f,
                "golden file {} differs (expected, then actual); if the change is intended, run with AXIOM_BLESS=1 \
                 and review the diff:\n{diff}",
                path.display()
            ),
            Self::Io(path, error) => write!(f, "golden file {}: {error}", path.display()),
        }
    }
}

impl std::error::Error for Mismatch {}

/// Compares `actual` with the golden file at `path`, or rewrites it in
/// [`Mode::Bless`].
#[expect(
    clippy::disallowed_methods,
    reason = "ES13: test crates may read and write files; golden files are ES21's"
)]
pub fn check_golden(mode: Mode, path: &Path, actual: &str) -> Result<(), Mismatch> {
    let expected = match fs::read_to_string(path) {
        Ok(text) => Some(text),
        Err(error) if error.kind() == io::ErrorKind::NotFound => None,
        Err(error) => return Err(Mismatch::Io(path.to_path_buf(), error)),
    };
    if expected.as_deref() == Some(actual) {
        return Ok(());
    }
    match (mode, expected) {
        (Mode::Bless, _) => {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|error| Mismatch::Io(path.to_path_buf(), error))?;
            }
            fs::write(path, actual).map_err(|error| Mismatch::Io(path.to_path_buf(), error))
        }
        (Mode::Compare, None) => Err(Mismatch::Missing(path.to_path_buf())),
        (Mode::Compare, Some(expected)) => Err(Mismatch::Differs(
            path.to_path_buf(),
            diff(&expected, actual),
        )),
    }
}

/// The first few lines that differ, each as its expected and actual form.
fn diff(expected: &str, actual: &str) -> String {
    const SHOWN: usize = 12;
    let (mut expected_lines, mut actual_lines) = (expected.lines(), actual.lines());
    let mut out = String::new();
    let mut shown = 0;
    for number in 1.. {
        match (expected_lines.next(), actual_lines.next()) {
            (None, None) => break,
            (left, right) if left == right => {}
            (left, right) => {
                let (left, right) = (left.unwrap_or("<end>"), right.unwrap_or("<end>"));
                // Writing to a String can't fail.
                _ = writeln!(out, "{number:>5} - {left}\n{number:>5} + {right}");
                shown += 1;
                if shown == SHOWN {
                    out.push_str("      …\n");
                    break;
                }
            }
        }
    }
    if out.is_empty() {
        // Only the final newline differs.
        out.push_str("      the texts differ only in their trailing newline\n");
    }
    out
}

/// Asserts that a text matches the calling crate's golden file
/// `tests/golden/<name>`, or rewrites the file when `AXIOM_BLESS=1` is set
/// (ES21).
///
/// ```ignore
/// test_support::assert_golden!("dom/simple.txt", &dump);
/// ```
#[macro_export]
macro_rules! assert_golden {
    ($name:expr, $actual:expr) => {{
        let path = ::std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/golden")
            .join($name);
        if let Err(mismatch) = $crate::check_golden($crate::Mode::from_env(), &path, $actual) {
            panic!("{mismatch}");
        }
    }};
}

#[cfg(test)]
mod tests {
    use super::diff;

    #[test]
    fn diff_numbers_each_differing_line() {
        assert_eq!(diff("a\nb\nc\n", "a\nx\nc\n"), "    2 - b\n    2 + x\n");
    }

    #[test]
    fn diff_marks_a_missing_line_as_the_end() {
        assert_eq!(diff("a\n", "a\nb\n"), "    2 - <end>\n    2 + b\n");
    }

    #[test]
    fn diff_explains_a_trailing_newline() {
        assert!(diff("a\n", "a").contains("trailing newline"));
    }
}
