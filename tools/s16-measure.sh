#!/usr/bin/env bash
# Spike S16, for the architecture review's Round 4 (docs/research/s16-results.md):
# lifetimes of shared memory and port rights, and efficiency counters.
#
# Usage: tools/s16-measure.sh [--runs N] [--cooldown SECONDS] [--counter-runs N]
#                             [--load N] [--allow-battery] [--quick]
#   --runs N            runs of S16-Q1 and S16-Q2 (default 3), each after a cool-down
#                       and once the load average is below 3.0 (PF1; up to 10 minutes)
#   --cooldown SECONDS  idle time before each run (default 10)
#   --counter-runs N    runs of each S16-Q3 workload, per load level (default 10)
#   --load N            spinning processes for S16-Q3's loaded runs (default 4)
#   --allow-battery     run without AC power, or with Low Power Mode on
#   --quick             a short pass for checking the script
#
# Builds the release binary and signs a copy ad hoc with the hardened runtime,
# as Axiom's helpers would be (ADR-0004 D2); the helpers run that copy too,
# under S10's profiles. Each run, after a cool-down:
#   S16-Q1 (a)  memory: a renderer's 16 images of 1,024 x 1,024 shared as memory
#               entries and sampled in place by the compositor, in the browser
#               and in a GPU process; the renderer killed, or told to exit; with
#               and without the compositor reading the pages from the CPU
#   S16-Q1 (b)  surfaces: a GPU process's 32 tiles shown by the browser; the GPU
#               process killed, or told to exit
#   S16-Q2      rights: dropped messages, failed sends, queued messages, lookups
# Then, once, S16-Q3 counters: the built-in workloads (cpu, alloc, disk,
# sandboxed-cpu) and two fixed spike workloads, S10's validation and S3's
# counting interpreter, idle and under --load spinning processes.
# Crash reports the runs leave are moved into the results. Writes
# target/s16-results/<timestamp>/, with environment.txt and loads.txt.
# Written for bash 3.2 (macOS /bin/bash).
set -uo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd)
cd "$ROOT" || exit 2

runs=3 cooldown=10 counter_runs=10 load=4 battery="" quick=""
while [ $# -gt 0 ]; do
  case $1 in
    --runs) runs=$2; shift 2 ;;
    --cooldown) cooldown=$2; shift 2 ;;
    --counter-runs) counter_runs=$2; shift 2 ;;
    --load) load=$2; shift 2 ;;
    --allow-battery) battery=yes; shift ;;
    --quick) quick=yes; shift ;;
    -h | --help) sed -n '2,/^set -uo/p' "$0" | sed '$d' | sed 's/^# \{0,1\}//'; exit 0 ;;
    *) echo "unknown argument: $1 (see --help)" >&2; exit 2 ;;
  esac
done
if [ -n "$quick" ]; then runs=1 cooldown=0 counter_runs=2; fi

power=$(pmset -g batt | head -1)
lpm=$(pmset -g | awk '/lowpowermode/ { print $2 }')
if [ -z "$battery" ] && { ! printf '%s' "$power" | grep -q "AC Power" || [ "${lpm:-0}" != 0 ]; }; then
  echo "not on AC power with Low Power Mode off ($power; lowpowermode ${lpm:-?})." >&2
  echo "Counters would not be comparable; use --allow-battery to run anyway." >&2
  exit 3
fi

cargo build --manifest-path "$ROOT/spikes/Cargo.toml" -q --release -p s16-lifetime || { echo "the release build failed" >&2; exit 1; }
bin=target/s16/bin
mkdir -p "$bin"
cp -f target/release/s16 "$bin/s16"
codesign --sign - --force --identifier org.axiom.s16 --options runtime "$bin/s16" 2>/dev/null || {
  echo "signing s16 failed" >&2
  exit 1
}
app=$bin/s16
s10=target/release/s10
s3count=target/release/s3-count

out="target/s16-results/$(date +%Y%m%d-%H%M%S)"
mkdir -p "$out/crash-reports"
reports="$HOME/Library/Logs/DiagnosticReports"
ls "$reports" > "$out/crash-reports/before.txt" 2>/dev/null
{
  echo "date: $(date '+%Y-%m-%d %H:%M:%S %Z')"
  echo "macOS: $(sw_vers -productVersion) ($(sw_vers -buildVersion)); $(uname -m); $(sysctl -n hw.model)"
  echo "power: $power; lowpowermode ${lpm:-?}"
  echo "rustc: $(rustc --version)"
  echo "binary: $(shasum -a 256 "$app" | cut -c1-16); $(codesign -dv "$app" 2>&1 | grep -E '^(Identifier|CodeDirectory)' | tr '\n' ' ')"
  for b in "$s10" "$s3count"; do
    [ -x "$b" ] && echo "workload: $b $(shasum -a 256 "$b" | cut -c1-16) $(stat -f %Sm "$b")"
  done
  echo "load average: $(sysctl -n vm.loadavg)"
  echo "memory: $(memory_pressure -Q 2>/dev/null | tail -1)"
} > "$out/environment.txt"
cat "$out/environment.txt"

note_load() { echo "$1 $(date '+%H:%M:%S') $(sysctl -n vm.loadavg)" >> "$out/loads.txt"; }
# PF1: wait up to 10 minutes for the one-minute load average to fall below 3.0.
wait_for_load() {
  waited=0
  while [ "$waited" -lt 600 ]; do
    one=$(sysctl -n vm.loadavg | awk '{ print $2 }')
    awk -v l="$one" 'BEGIN { exit !(l < 3.0) }' && return 0
    sleep 15; waited=$((waited + 15))
  done
  echo "   the load average stayed at or above 3.0 for 10 minutes; running anyway" >&2
}

for run in $(seq 1 "$runs"); do
  dir="$out/run$run"
  mkdir -p "$dir"
  echo "== run $run: cool-down ${cooldown}s"
  sleep "$cooldown"
  [ -z "$quick" ] && wait_for_load
  note_load "run$run"
  for placement in browser gpu; do
    for end in kill exit; do
      for touch in "" "--cpu-touch"; do
        name="memory-$placement-$end${touch:+-touch}"
        echo "   $name"
        # shellcheck disable=SC2086 # $touch is empty or one flag
        "$app" memory --compositor "$placement" --end "$end" $touch --out "$dir" \
          > "$dir/$name.out" 2> "$dir/$name.err" || echo "   $name failed (see $dir/$name.err)"
      done
    done
  done
  for end in kill exit; do
    echo "   surfaces-$end"
    "$app" surfaces --end "$end" --out "$dir" > "$dir/surfaces-$end.out" 2> "$dir/surfaces-$end.err" \
      || echo "   surfaces-$end failed (see $dir/surfaces-$end.err)"
  done
  echo "   rights"
  "$app" rights --count 1000 --out "$dir" > "$dir/rights.out" 2> "$dir/rights.err" \
    || echo "   rights failed (see $dir/rights.err)"
done

echo "== counters: ${counter_runs} runs per workload, idle and with $load spinning"
sleep "$cooldown"
for level in 0 "$load"; do
  label=$([ "$level" = 0 ] && echo idle || echo loaded)
  [ -z "$quick" ] && [ "$level" = 0 ] && wait_for_load
  note_load "counters-$label"
  "$app" counters --runs "$counter_runs" --load "$level" --label "$label" --out "$out" \
    > "$out/counters-$label.out" 2>&1 || echo "   counters $label failed"
  if [ -x "$s10" ]; then
    "$app" counters --runs "$counter_runs" --load "$level" --workloads "" --label "$label-s10" --out "$out" \
      -- "$ROOT/$s10" validate --sizes 1000,10000 --iterations 50 > "$out/counters-$label-s10.out" 2>&1 \
      || echo "   counters $label s10 failed"
  fi
  if [ -x "$s3count" ]; then
    "$app" counters --runs "$counter_runs" --load "$level" --workloads "" --label "$label-s3" --out "$out" \
      -- "$ROOT/$s3count" > "$out/counters-$label-s3.out" 2>&1 || echo "   counters $label s3 failed"
  fi
done
note_load "end"

# Crash reports since the start, from this spike's processes, into the results.
ls "$reports" > "$out/crash-reports/after.txt" 2>/dev/null
comm -13 "$out/crash-reports/before.txt" "$out/crash-reports/after.txt" > "$out/crash-reports/new.txt"
moved=0
while IFS= read -r file; do
  case $file in
    s16*) mv "$reports/$file" "$out/crash-reports/" && moved=$((moved + 1)) ;;
  esac
done < "$out/crash-reports/new.txt"
echo "crash reports: $(wc -l < "$out/crash-reports/new.txt" | tr -d ' ') new, $moved from s16 moved into the results"
echo "results: $out"
