//! Writes S9-Q1's seed messages into a directory: `s9-seeds DIR`.
//!
//! The seeds nest lists one deep, never two, so reaching the planted bug is
//! left to the fuzzer.
#![forbid(unsafe_code)]

use std::{env, fs, path::PathBuf, process::ExitCode};

use s9_decoder::{Record, encode};

fn main() -> ExitCode {
    let Some(dir) = env::args_os().nth(1).map(PathBuf::from) else {
        eprintln!("usage: s9-seeds DIR");
        return ExitCode::FAILURE;
    };
    let seeds = [
        vec![],
        vec![Record::Text("hello".to_owned())],
        vec![Record::Numbers(vec![1, 2, 3])],
        vec![Record::List(vec![
            Record::Text("nested".to_owned()),
            Record::Numbers(vec![42]),
        ])],
    ];
    if let Err(error) = fs::create_dir_all(&dir) {
        eprintln!("s9-seeds: {}: {error}", dir.display());
        return ExitCode::FAILURE;
    }
    for (i, records) in seeds.iter().enumerate() {
        let path = dir.join(format!("seed-{i}"));
        if let Err(error) = fs::write(&path, encode(records)) {
            eprintln!("s9-seeds: {}: {error}", path.display());
            return ExitCode::FAILURE;
        }
    }
    ExitCode::SUCCESS
}
