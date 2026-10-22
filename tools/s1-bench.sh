#!/usr/bin/env bash
# Spike S1 runtime benchmarks: run every suite N times, then aggregate.
#
# Usage: tools/s1-bench.sh [--runs N] [--cooldown SECONDS] [--filter TEXT]
#   --runs N            separate process runs (default 3)
#   --cooldown SECONDS  idle time before each run (default 60). The MacBook
#                       Air is fanless, so sustained load throttles it
#   --filter TEXT       only cases whose suite/case contains TEXT
#
# Each run is a fresh `s1-probe bench` process. Inside a run, each case warms
# up for a fixed time and then takes alternating A/B samples (see
# spikes/s1/bench/src/harness.rs). The summary reports, per case, the median of
# the run medians and their range.
#
# Writes target/s1-results/bench-<timestamp>/:
#   environment.txt, run<k>.tsv, run<k>.log, summary.md
# Written for bash 3.2 (macOS /bin/bash).
set -uo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd)
cd "$ROOT" || exit 2

runs=3 cooldown=60 filter=""
while [ $# -gt 0 ]; do
  case $1 in
    --runs) runs=$2; shift 2 ;;
    --cooldown) cooldown=$2; shift 2 ;;
    --filter) filter=$2; shift 2 ;;
    -h | --help) sed -n '2,19p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
    *) echo "unknown argument: $1 (see --help)" >&2; exit 2 ;;
  esac
done

cargo build --manifest-path "$ROOT/spikes/Cargo.toml" --release -p s1-bench --bin s1-probe >/dev/null 2>&1 || {
  echo "build failed: cargo build --release -p s1-bench --bin s1-probe" >&2
  exit 1
}
probe="$ROOT/target/release/s1-probe"
out="$ROOT/target/s1-results/bench-$(date +%Y%m%d-%H%M%S)"
mkdir -p "$out"

thermal() { pmset -g therm 2>/dev/null | grep -Ei 'warning|limit' | tr -s ' \n' ' '; }

{
  echo "date:     $(date '+%Y-%m-%d %H:%M:%S %Z')"
  echo "git:      $(git rev-parse --short HEAD)$([ -z "$(git status --porcelain)" ] || echo ' (uncommitted changes present)')"
  echo "rustc:    $(rustc -V)"
  echo "machine:  $(sysctl -n machdep.cpu.brand_string), $(($(sysctl -n hw.memsize) / 1073741824)) GiB, $(sw_vers -productName) $(sw_vers -productVersion)"
  echo "power:    $(pmset -g batt | sed -n '1p' | sed "s/Now drawing from //; s/'//g")"
  echo "runs:     $runs, cool-down ${cooldown}s${filter:+, filter $filter}"
} | tee "$out/environment.txt"

i=1
while [ "$i" -le "$runs" ]; do
  sleep "$cooldown"
  echo "run $i: load $(sysctl -n vm.loadavg), thermal before: $(thermal)" | tee -a "$out/environment.txt"
  if ! "$probe" bench ${filter:+--filter "$filter"} --out "$out/run$i.tsv" >"$out/run$i.log" 2>&1; then
    echo "run $i failed; see $out/run$i.log" >&2
    tail -20 "$out/run$i.log" >&2
    exit 1
  fi
  echo "run $i: thermal after: $(thermal)" | tee -a "$out/environment.txt"
  i=$((i + 1))
done

files=""
i=1
while [ "$i" -le "$runs" ]; do files="$files $out/run$i.tsv"; i=$((i + 1)); done
# shellcheck disable=SC2086 # word splitting of $files is intended
"$probe" aggregate $files >"$out/summary.md" || exit 1
cat "$out/summary.md"
echo
echo "results: ${out#"$ROOT"/}"
