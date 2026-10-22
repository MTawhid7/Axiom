//! The framework on a fixture generator, in a directory under target/.
#![forbid(unsafe_code)]
#![expect(
    clippy::disallowed_methods,
    reason = "ES13: a build tool's tests read and write files"
)]

use std::fs;
use std::path::{Path, PathBuf};

use codegen::{Error, Generator, Output, Stale, check, header, write};

/// Reads `input.txt`'s numbers into a constant array, written unformatted.
struct Numbers {
    output: &'static str,
}

impl Generator for Numbers {
    fn name(&self) -> &'static str {
        "numbers"
    }

    fn inputs(&self) -> &[&'static str] {
        &["input.txt"]
    }

    fn generate(&self, root: &Path) -> Result<Vec<Output>, Error> {
        let path = root.join("input.txt");
        let text = fs::read_to_string(&path).map_err(|error| Error::Io(path.clone(), error))?;
        let numbers: Vec<&str> = text.split_whitespace().collect();
        Ok(vec![Output {
            path: PathBuf::from(self.output),
            source: format!(
                "/// Numbers.\npub const NUMBERS: [u32; {}] = [{}];",
                numbers.len(),
                numbers.join(",")
            ),
        }])
    }
}

fn workspace(name: &str, input: &str) -> PathBuf {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("codegen-framework")
        .join(name);
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("a directory under target/");
    fs::write(root.join("rustfmt.toml"), "style_edition = \"2024\"\n").expect("rustfmt.toml");
    fs::write(root.join("input.txt"), input).expect("the input");
    root
}

fn generators(output: &'static str) -> Vec<Box<dyn Generator>> {
    vec![Box::new(Numbers { output })]
}

const OUTPUT: &str = "crates/demo/src/generated/numbers.rs";

#[test]
fn writes_formatted_output_under_its_header_then_finds_it_fresh() {
    let root = workspace("fresh", "1 2 3");
    assert_eq!(
        write(&root, &generators(OUTPUT)).unwrap(),
        [PathBuf::from(OUTPUT)]
    );
    let text = fs::read_to_string(root.join(OUTPUT)).unwrap();
    assert!(
        text.starts_with(&header("numbers", &["input.txt"])),
        "{text}"
    );
    assert!(
        text.contains("pub const NUMBERS: [u32; 3] = [1, 2, 3];\n"),
        "rustfmt spaced it: {text}"
    );
    assert_eq!(check(&root, &generators(OUTPUT)).unwrap(), []);
    // Deterministic: a second run writes nothing.
    assert_eq!(
        write(&root, &generators(OUTPUT)).unwrap(),
        Vec::<PathBuf>::new()
    );
}

#[test]
fn finds_an_output_stale_when_its_input_changes_or_it_is_missing() {
    let root = workspace("stale", "1 2 3");
    write(&root, &generators(OUTPUT)).unwrap();
    fs::write(root.join("input.txt"), "4 5").unwrap();
    let stale = Stale {
        path: PathBuf::from(OUTPUT),
        missing: false,
    };
    assert_eq!(check(&root, &generators(OUTPUT)).unwrap(), [stale]);
    fs::remove_file(root.join(OUTPUT)).unwrap();
    let missing = Stale {
        path: PathBuf::from(OUTPUT),
        missing: true,
    };
    assert_eq!(check(&root, &generators(OUTPUT)).unwrap(), [missing]);
}

#[test]
fn refuses_an_output_outside_a_crates_generated_directory() {
    let root = workspace("placement", "1");
    for path in [
        "crates/demo/src/numbers.rs",
        "generated/numbers.rs",
        "crates/demo/generated/numbers.rs",
    ] {
        let leaked: &'static str = Box::leak(path.to_owned().into_boxed_str());
        assert!(
            matches!(write(&root, &generators(leaked)), Err(Error::Placement(_))),
            "{path}"
        );
    }
}

#[test]
fn reports_invalid_rust_as_a_formatting_failure() {
    let root = workspace("invalid", "1 2 ]");
    assert!(matches!(
        write(&root, &generators(OUTPUT)),
        Err(Error::Format(..))
    ));
}
