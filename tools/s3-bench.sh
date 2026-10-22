#!/usr/bin/env bash
# Spike S3 measurements: every suite N times, aggregated; counts; and the
# nightly tail-call technique.
#
# Usage: tools/s3-bench.sh [--runs N] [--cooldown SECONDS] [--filter TEXT] [--no-nightly]
#   --runs N            separate process runs (default 3)
#   --cooldown SECONDS  idle time before each run (default 60). The MacBook
#                       Air is fanless, so sustained load throttles it
#   --filter TEXT       only cases whose suite/case contains TEXT
#   --no-nightly        skip the nightly runs that add the `become` technique
#
# 1. Each run is a fresh `s3-probe bench` process (Rust 1.94.1, release).
#    Inside a run, each case warms up for a fixed time, then rotates its
#    samples across its variants (spikes/s3/bench/src/harness.rs). Processes
#    run at the default QoS of a command-line process, as S1's did:
#    `taskpolicy` can only lower QoS.
# 2. `s3-count`, built with the `count` feature, counts instructions, calls
#    and root operations for every workload and technique (S3-Q2).
# 3. Unless --no-nightly: the `vm` suite N more times, built with the pinned
#    nightly and the `nightly-tail` feature, which adds the tail-call
#    technique (S3-Q3). Its other rows show what the newer compiler changes
#    by itself.
#
# Writes target/s3-results/bench-<timestamp>/:
#   environment.txt, run<k>.tsv, run<k>.log, summary.md, counts.md,
#   nightly-run<k>.tsv, nightly-run<k>.log, nightly-summary.md
# Written for bash 3.2 (macOS /bin/bash).
set -uo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd)
cd "$ROOT" || exit 2

runs=3 cooldown=60 filter="" nightly=1
while [ $# -gt 0 ]; do
  case $1 in
    --runs) runs=$2; shift 2 ;;
    --cooldown) cooldown=$2; shift 2 ;;
    --filter) filter=$2; shift 2 ;;
    --no-nightly) nightly=0; shift ;;
    -h | --help) sed -n '2,29p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
    *) echo "unknown argument: $1 (see --help)" >&2; exit 2 ;;
  esac
done

build() { # description, cargo arguments...
  local what=$1; shift
  # The spikes are their own workspace since M1-CP2 (spikes/Cargo.toml).
  cargo "$@" --manifest-path "$ROOT/spikes/Cargo.toml" >/dev/null 2>&1 || { echo "build failed ($what): cargo $*" >&2; exit 1; }
}
build stable build --release -p s3-bench --bin s3-probe
build count build --release -p s3-bench --features count --bin s3-count
probe="$ROOT/target/release/s3-probe"
counter="$ROOT/target/release/s3-count"
if [ "$nightly" = 1 ]; then
  build nightly +nightly build --release -p s3-bench --features nightly-tail --bin s3-probe \
    --target-dir "$ROOT/target/s3-nightly"
  nightly_probe="$ROOT/target/s3-nightly/release/s3-probe"
fi

out="$ROOT/target/s3-results/bench-$(date +%Y%m%d-%H%M%S)"
mkdir -p "$out"

thermal() { pmset -g therm 2>/dev/null | grep -Ei 'warning|limit' | tr -s ' \n' ' '; }

{
  echo "date:     $(date '+%Y-%m-%d %H:%M:%S %Z')"
  echo "git:      $(git rev-parse --short HEAD)$([ -z "$(git status --porcelain)" ] || echo ' (uncommitted changes present)')"
  echo "rustc:    $(rustc -V)"
  [ "$nightly" = 1 ] && echo "nightly:  $(rustc +nightly -V)"
  echo "machine:  $(sysctl -n machdep.cpu.brand_string), $(($(sysctl -n hw.memsize) / 1073741824)) GiB, $(sw_vers -productName) $(sw_vers -productVersion)"
  echo "power:    $(pmset -g batt | sed -n '1p' | sed "s/Now drawing from //; s/'//g")"
  echo "runs:     $runs, cool-down ${cooldown}s${filter:+, filter $filter}"
} | tee "$out/environment.txt"

# run_probe <binary> <file prefix> <k> [filter]
run_probe() {
  local binary=$1 prefix=$2 k=$3 only=${4:-}
  sleep "$cooldown"
  echo "$prefix$k: load $(sysctl -n vm.loadavg), thermal before: $(thermal)" | tee -a "$out/environment.txt"
  if ! "$binary" bench ${only:+--filter "$only"} --out "$out/$prefix$k.tsv" >"$out/$prefix$k.log" 2>&1; then
    echo "$prefix$k failed; see $out/$prefix$k.log" >&2
    tail -20 "$out/$prefix$k.log" >&2
    exit 1
  fi
  echo "$prefix$k: thermal after: $(thermal)" | tee -a "$out/environment.txt"
}

summarize() { # binary, prefix, output file
  local binary=$1 prefix=$2 file=$3 files="" k=1
  while [ "$k" -le "$runs" ]; do files="$files $out/$prefix$k.tsv"; k=$((k + 1)); done
  # shellcheck disable=SC2086 # word splitting of $files is intended
  "$binary" aggregate $files >"$out/$file" || exit 1
}

k=1
while [ "$k" -le "$runs" ]; do run_probe "$probe" run "$k" "$filter"; k=$((k + 1)); done
summarize "$probe" run summary.md

"$counter" >"$out/counts.md" || { echo "s3-count failed" >&2; exit 1; }

if [ "$nightly" = 1 ]; then
  k=1
  while [ "$k" -le "$runs" ]; do run_probe "$nightly_probe" nightly-run "$k" "vm/"; k=$((k + 1)); done
  summarize "$nightly_probe" nightly-run nightly-summary.md
fi

cat "$out/summary.md"
echo
echo "results: ${out#"$ROOT"/}"
