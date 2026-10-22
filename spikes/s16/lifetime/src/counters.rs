//! S16-Q3: are rusage's per-process counters (instructions and cycles,
//! energy, disk writes, peak footprints) filled in for unprivileged and
//! sandboxed children, and readable by their parent once they have exited?
//! And how much do they vary from run to run on fixed work, beside wall
//! time, idle and under a background load? Each run is a fresh child: the
//! parent waits for it to exit without reaping it, reads its usage, then
//! reaps it with `wait4`, whose `struct rusage` gives the CPU times to compare
//! units with.

use std::fmt::Write as _;
use std::fs::File;
use std::os::fd::AsFd as _;
use std::path::{Path, PathBuf};

use s4_sandbox::platform::process::{self, Command};

use crate::launch;
use crate::platform::rusage::{self, Usage, index};
use crate::platform::time;
use crate::sample::{to_f64, write_file};

#[derive(Debug)]
pub(crate) struct Options {
    pub(crate) runs: u32,
    pub(crate) load: u32,
    pub(crate) workloads: Vec<String>,
    pub(crate) command: Vec<String>,
    pub(crate) label: String,
    pub(crate) out: Option<PathBuf>,
}

/// The counters reported, by name, with how each is read from a run.
const METRICS: [(&str, Metric); 25] = [
    ("wall_ns", Metric::Wall),
    ("rusage_user_ns", Metric::RuUser),
    ("rusage_system_ns", Metric::RuSystem),
    ("ri_user_time", Metric::Index(index::USER_TIME)),
    ("ri_system_time", Metric::Index(index::SYSTEM_TIME)),
    ("instructions", Metric::Index(index::INSTRUCTIONS)),
    ("cycles", Metric::Index(index::CYCLES)),
    ("p_instructions", Metric::Index(index::PINSTRUCTIONS)),
    ("p_cycles", Metric::Index(index::PCYCLES)),
    ("user_ptime", Metric::Index(index::USER_PTIME)),
    ("system_ptime", Metric::Index(index::SYSTEM_PTIME)),
    ("energy_nj", Metric::Index(index::ENERGY_NJ)),
    ("p_energy_nj", Metric::Index(index::PENERGY_NJ)),
    ("billed_energy", Metric::Index(index::BILLED_ENERGY)),
    ("serviced_energy", Metric::Index(index::SERVICED_ENERGY)),
    ("diskio_bytesread", Metric::Index(index::DISKIO_BYTESREAD)),
    (
        "diskio_byteswritten",
        Metric::Index(index::DISKIO_BYTESWRITTEN),
    ),
    ("logical_writes", Metric::Index(index::LOGICAL_WRITES)),
    (
        "phys_footprint_at_exit",
        Metric::Index(index::PHYS_FOOTPRINT),
    ),
    (
        "lifetime_max_footprint",
        Metric::Index(index::LIFETIME_MAX_PHYS_FOOTPRINT),
    ),
    (
        "interval_max_footprint",
        Metric::Index(index::INTERVAL_MAX_PHYS_FOOTPRINT),
    ),
    ("pageins", Metric::Index(index::PAGEINS)),
    ("pkg_idle_wkups", Metric::Index(index::PKG_IDLE_WKUPS)),
    ("interrupt_wkups", Metric::Index(index::INTERRUPT_WKUPS)),
    ("runnable_time", Metric::Index(index::RUNNABLE_TIME)),
];

#[derive(Clone, Copy, Debug)]
enum Metric {
    Wall,
    RuUser,
    RuSystem,
    Index(usize),
}

/// One run's measurements.
#[derive(Debug)]
struct Run {
    wall_ns: u64,
    status: i32,
    ru_user_ns: u64,
    ru_system_ns: u64,
    usage: Option<Usage>,
}

impl Run {
    fn get(&self, metric: Metric) -> Option<u64> {
        match metric {
            Metric::Wall => Some(self.wall_ns),
            Metric::RuUser => Some(self.ru_user_ns),
            Metric::RuSystem => Some(self.ru_system_ns),
            Metric::Index(i) => self.usage.map(|u| u.get(i)),
        }
    }
}

pub(crate) fn run(o: &Options) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let dir = o
        .out
        .clone()
        .unwrap_or_else(|| PathBuf::from("target/s16-results/scratch"));
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let mut spinners = Vec::new();
    for _ in 0..o.load {
        let mut command = Command::new(&exe).map_err(|e| e.to_string())?;
        command
            .arg("work")
            .and_then(|c| c.arg("spin"))
            .map_err(|e| e.to_string())?;
        spinners.push(command.spawn().map_err(|e| e.to_string())?);
    }
    let mut table = String::from("workload\tload\trun\tstatus");
    for (name, _) in METRICS {
        let _ = write!(table, "\t{name}");
    }
    table.push('\n');
    let mut summary = format!(
        "{}: {} runs each, {} spinning processes of load\n",
        o.label, o.runs, o.load
    );
    let mut workloads: Vec<String> = o.workloads.clone();
    if !o.command.is_empty() {
        workloads.push(o.command.join(" "));
    }
    let result = (|| -> Result<(), String> {
        for workload in &workloads {
            let mut runs = Vec::new();
            for n in 0..o.runs {
                let run = measure(&exe, workload, o, &dir)?;
                let _ = write!(table, "{workload}\t{}\t{n}\t{}", o.load, run.status);
                for (_, metric) in METRICS {
                    let _ = write!(
                        table,
                        "\t{}",
                        run.get(metric)
                            .map_or_else(|| "-".to_owned(), |v| v.to_string())
                    );
                }
                table.push('\n');
                runs.push(run);
            }
            summarize(&mut summary, workload, &runs);
        }
        Ok(())
    })();
    for spinner in &spinners {
        let _ = spinner.signal(9);
        let _ = spinner.wait();
    }
    result?;
    if let Some(out) = &o.out {
        write_file(&out.join(format!("counters-{}.tsv", o.label)), &table)?;
        write_file(
            &out.join(format!("counters-{}-summary.txt", o.label)),
            &summary,
        )?;
    }
    print!("{summary}");
    Ok(())
}

/// Run `workload` once in a fresh child and read its counters at exit.
fn measure(exe: &Path, workload: &str, o: &Options, dir: &Path) -> Result<Run, String> {
    let null = File::options()
        .write(true)
        .open("/dev/null")
        .map_err(|e| e.to_string())?;
    let started = time::now();
    let child: process::Child = if workload == "sandboxed-cpu" {
        // A renderer under S10's profile; its rendezvous is never looked up.
        launch::spawn(
            "org.axiom.s16.unused",
            "worker",
            "renderer",
            &["--work", "cpu"],
        )?
    } else if o.workloads.iter().any(|w| w == workload) {
        let mut command = Command::new(exe).map_err(|e| e.to_string())?;
        for arg in ["work", workload, "--dir"] {
            command.arg(arg).map_err(|e| e.to_string())?;
        }
        command.arg(dir).map_err(|e| e.to_string())?;
        command.spawn().map_err(|e| e.to_string())?
    } else {
        let (program, args) = o.command.split_first().ok_or("no command")?;
        let mut command = Command::new(Path::new(program)).map_err(|e| e.to_string())?;
        for arg in args {
            command.arg(arg).map_err(|e| e.to_string())?;
        }
        command.stdout(null.as_fd());
        command.spawn().map_err(|e| e.to_string())?
    };
    let pid = child.id();
    rusage::wait_exited(pid).map_err(|e| format!("waitid: {e}"))?;
    let usage = rusage::usage(pid).ok();
    let wall_ns = time::ns(time::now() - started);
    let (status, ru_user_ns, ru_system_ns) =
        rusage::reap(pid).map_err(|e| format!("wait4: {e}"))?;
    Ok(Run {
        wall_ns,
        status,
        ru_user_ns,
        ru_system_ns,
        usage,
    })
}

/// Mean, standard deviation, coefficient of variation, minimum and maximum
/// of each metric; whether it was ever non-zero; and how `ri_user_time`
/// compares with `struct rusage`'s user time, to find its unit.
fn summarize(out: &mut String, workload: &str, runs: &[Run]) {
    let _ = writeln!(
        out,
        "\n{workload}\n  metric\tmean\tsd\tcv%\tmin\tmax\tnon-zero runs"
    );
    for (name, metric) in METRICS {
        let values: Vec<f64> = runs
            .iter()
            .filter_map(|r| r.get(metric))
            .map(to_f64)
            .collect();
        if values.is_empty() {
            let _ = writeln!(out, "  {name}\tunavailable");
            continue;
        }
        let n = to_f64(values.len() as u64);
        let mean = values.iter().sum::<f64>() / n;
        let sd =
            (values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (n - 1.0).max(1.0)).sqrt();
        let cv = if mean > 0.0 { 100.0 * sd / mean } else { 0.0 };
        let min = values.iter().copied().fold(f64::INFINITY, f64::min);
        let max = values.iter().copied().fold(0.0, f64::max);
        let nonzero = values.iter().filter(|v| **v > 0.0).count();
        let _ = writeln!(
            out,
            "  {name}\t{mean:.0}\t{sd:.0}\t{cv:.2}\t{min:.0}\t{max:.0}\t{nonzero}"
        );
    }
    let ratios: Vec<f64> = runs
        .iter()
        .filter_map(|r| {
            let ri = to_f64(r.usage?.get(index::USER_TIME));
            (r.ru_user_ns > 0).then(|| ri / to_f64(r.ru_user_ns))
        })
        .collect();
    if let (Some(first), Some(last)) = (ratios.first(), ratios.last()) {
        let _ = writeln!(
            out,
            "  ri_user_time / rusage user ns: {first:.4} (first run), {last:.4} (last run)"
        );
    }
}
