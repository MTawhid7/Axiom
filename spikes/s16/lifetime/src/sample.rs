//! Memory samples for S16-Q1: each process's charge, from its rusage (which
//! the browser reads for any of its children); a process's own breakdown,
//! from `TASK_VM_INFO`, which here only a task reads of itself, so the GPU
//! process reports its own on request; the host's pages; and `footprint`'s
//! report at the stages that matter. Rows are tab-separated, one file per
//! kind.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use crate::platform::memory::{self, HostVm, TaskVm};
use crate::platform::rusage::{self, index};
use crate::platform::time;

/// A compositor's mappings of renderer memory: how many, the bytes resident,
/// dirtied and swapped out in the objects they map, and the fewest and most
/// references any of those objects has.
pub(crate) type Mapped = [u64; 6];

const MAPPED_NAMES: [&str; 6] = [
    "mapped_regions",
    "mapped_resident_mib",
    "mapped_dirtied_mib",
    "mapped_swapped_mib",
    "object_refs_min",
    "object_refs_max",
];

/// Collects one run's samples.
#[derive(Debug)]
pub(crate) struct Recorder {
    name: String,
    start: u64,
    out: Option<PathBuf>,
    processes: Vec<String>,
    host: Vec<String>,
    notes: Vec<String>,
}

fn mib(bytes: u64) -> String {
    format!("{:.2}", to_f64(bytes) / 1_048_576.0)
}

#[allow(clippy::cast_precision_loss)] // byte and page counts far below 2^52
pub(crate) fn to_f64(value: u64) -> f64 {
    value as f64
}

impl Recorder {
    /// A recorder for the run `name`, whose files go in `out`.
    pub(crate) fn new(name: &str, out: Option<PathBuf>) -> Self {
        let mut header = String::from(
            "stage\tms\tprocess\tpid\tphys_footprint_mib\tlifetime_max_mib\tinterval_max_mib",
        );
        for name in TaskVm::NAMES {
            let _ = write!(header, "\t{name}_mib");
        }
        for name in MAPPED_NAMES {
            let _ = write!(header, "\t{name}");
        }
        let mut host = String::from("stage\tms");
        for name in HostVm::NAMES {
            let _ = write!(host, "\t{name}_mib");
        }
        Self {
            name: name.to_owned(),
            start: time::now(),
            out,
            processes: vec![header],
            host: vec![host],
            notes: Vec::new(),
        }
    }

    fn ms(&self) -> u64 {
        time::ns(time::now() - self.start) / 1_000_000
    }

    /// One process's charge, with its own breakdown and its mappings of
    /// renderer memory if it reported them.
    pub(crate) fn process(
        &mut self,
        stage: &str,
        role: &str,
        pid: i32,
        vm: Option<TaskVm>,
        mapped: Option<Mapped>,
    ) {
        let mut row = format!("{stage}\t{}\t{role}\t{pid}", self.ms());
        match rusage::usage(pid) {
            Ok(u) => {
                for i in [
                    index::PHYS_FOOTPRINT,
                    index::LIFETIME_MAX_PHYS_FOOTPRINT,
                    index::INTERVAL_MAX_PHYS_FOOTPRINT,
                ] {
                    let _ = write!(row, "\t{}", mib(u.get(i)));
                }
            }
            Err(e) => {
                let _ = write!(row, "\tunavailable ({e})\t\t");
            }
        }
        match vm {
            Some(vm) => {
                for value in vm.values() {
                    let _ = write!(row, "\t{}", mib(value));
                }
            }
            None => row.push_str(&"\t-".repeat(TaskVm::NAMES.len())),
        }
        if let Some([regions, resident, dirtied, swapped, low, high]) = mapped {
            let _ = write!(
                row,
                "\t{regions}\t{}\t{}\t{}\t{low}\t{high}",
                mib(resident),
                mib(dirtied),
                mib(swapped)
            );
        }
        self.processes.push(row);
    }

    /// This process's charge and breakdown.
    pub(crate) fn this(&mut self, stage: &str, role: &str, mapped: Option<Mapped>) {
        let pid = i32::try_from(std::process::id()).unwrap_or(0);
        let vm = memory::task_vm().ok();
        self.process(stage, role, pid, vm, mapped);
    }

    /// The host's pages.
    pub(crate) fn host(&mut self, stage: &str) {
        let mut row = format!("{stage}\t{}", self.ms());
        let page = memory::page_size() as u64;
        match memory::host_vm() {
            Ok(vm) => {
                for pages in vm.values() {
                    let _ = write!(row, "\t{}", mib(pages * page));
                }
            }
            Err(e) => {
                let _ = write!(row, "\tunavailable ({e})");
            }
        }
        self.host.push(row);
    }

    pub(crate) fn note(&mut self, text: impl Into<String>) {
        self.notes.push(text.into());
    }

    /// `footprint`'s report on `pids` at `stage`, saved beside the tables. (Its
    /// `--unmapped`, which finds memory a process owns but no longer maps,
    /// needs root, so it isn't used.)
    pub(crate) fn footprint(&mut self, stage: &str, pids: &[i32]) {
        let Some(dir) = &self.out else { return };
        let mut command = std::process::Command::new("/usr/bin/footprint");
        for pid in pids {
            command.arg("--pid").arg(pid.to_string());
        }
        let path = dir.join(format!("{}-footprint-{stage}.txt", self.name));
        match command.output() {
            Ok(output) => {
                let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
                text.push_str(&String::from_utf8_lossy(&output.stderr));
                if let Err(e) = write_file(&path, &text) {
                    self.note(format!("footprint at {stage}: {e}"));
                }
            }
            Err(e) => self.note(format!("footprint at {stage}: {e}")),
        }
    }

    /// Write the tables and notes as `NAME-processes.tsv`, `NAME-host.tsv` and
    /// `NAME-notes.txt`, and return all three as text.
    pub(crate) fn finish(&self) -> Result<String, String> {
        let name = &self.name;
        let processes = self.processes.join("\n") + "\n";
        let host = self.host.join("\n") + "\n";
        let notes = self.notes.join("\n") + "\n";
        if let Some(dir) = &self.out {
            write_file(&dir.join(format!("{name}-processes.tsv")), &processes)?;
            write_file(&dir.join(format!("{name}-host.tsv")), &host)?;
            write_file(&dir.join(format!("{name}-notes.txt")), &notes)?;
        }
        Ok(format!("{processes}\n{host}\n{notes}"))
    }
}

pub(crate) fn write_file(path: &Path, text: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(path, text).map_err(|e| format!("{}: {e}", path.display()))
}

/// Sleep `ms` milliseconds.
pub(crate) fn pause(ms: u64) {
    std::thread::sleep(std::time::Duration::from_millis(ms));
}
