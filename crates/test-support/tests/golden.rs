//! Golden files (ES21), in a directory under target/.
#![forbid(unsafe_code)]

use std::path::PathBuf;

use test_support::{Mismatch, Mode, check_golden};

#[expect(
    clippy::disallowed_methods,
    reason = "ES13: test crates may use files; a fresh directory under target/"
)]
fn scratch(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("test-support-golden")
        .join(name);
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

#[test]
#[cfg_attr(
    miri,
    ignore = "Miri isolates the file system, and golden files hold no unsafe"
)]
fn a_missing_file_is_a_mismatch_until_blessed() {
    let path = scratch("missing").join("dump.txt");
    assert!(matches!(
        check_golden(Mode::Compare, &path, "a\n"),
        Err(Mismatch::Missing(_))
    ));
    check_golden(Mode::Bless, &path, "a\n").expect("bless writes the file");
    check_golden(Mode::Compare, &path, "a\n").expect("the blessed file matches");
}

#[test]
#[cfg_attr(
    miri,
    ignore = "Miri isolates the file system, and golden files hold no unsafe"
)]
fn a_changed_text_is_reported_with_its_lines() {
    let path = scratch("changed").join("dump.txt");
    check_golden(Mode::Bless, &path, "one\ntwo\n").expect("bless writes the file");
    match check_golden(Mode::Compare, &path, "one\nthree\n") {
        Err(Mismatch::Differs(_, diff)) => assert_eq!(diff, "    2 - two\n    2 + three\n"),
        other => panic!("expected a difference, got {other:?}"),
    }
}

#[test]
#[cfg_attr(
    miri,
    ignore = "Miri isolates the file system, and golden files hold no unsafe"
)]
#[expect(
    clippy::disallowed_methods,
    reason = "ES13: test crates may use files; reads the rewritten golden file"
)]
fn blessing_rewrites_only_a_changed_file() {
    let path = scratch("rewrite").join("dump.txt");
    check_golden(Mode::Bless, &path, "old\n").expect("bless writes the file");
    check_golden(Mode::Bless, &path, "new\n").expect("bless rewrites the file");
    assert_eq!(
        std::fs::read_to_string(&path).expect("the file exists"),
        "new\n"
    );
}
