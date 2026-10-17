#!/usr/bin/env bash
# Spike S2 runtime measurements: binary size, time to first frame, idle cost.
#
# Usage: tools/s2-measure.sh [--runs N] [--idle SECONDS]
#   --runs N          smoke runs and idle runs to make (default 5 and 3)
#   --idle SECONDS    how long each idle run keeps the window open (default 12)
#
# Builds s2-window (release), then records:
#   - binary size, as built and stripped;
#   - N smoke runs (`s2-window --smoke`) under /usr/bin/time -l. Each gives
#     the app's own timings (shaders ready, window shown, first frame
#     presented, all from main), wall time, peak memory footprint, max RSS,
#     and instructions retired. Each smoke run also checks the frame's pixels;
#     a failure stops the script;
#   - idle runs (`s2-window --seconds SECONDS`): the window is open and
#     receives no input. Three seconds in, `top` samples %CPU, idle wakeups,
#     its energy-impact score (POWER) and memory footprint over two seconds;
#     `ps` reads accumulated CPU time at the start and end of a later window.
#
# Smoke and idle runs open a window on screen for their duration.
# Writes target/s2-results/<timestamp>/: environment.txt, smoke.tsv, idle.tsv, summary.md
# Written for bash 3.2 (macOS /bin/bash); the summary uses python3 -I.
set -uo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd)
cd "$ROOT" || exit 2

runs=5 idle_runs=3 idle=12
while [ $# -gt 0 ]; do
  case $1 in
    --runs) runs=$2; idle_runs=$2; shift 2 ;;
    --idle) idle=$2; shift 2 ;;
    -h | --help) sed -n '2,24p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
    *) echo "unknown argument: $1 (see --help)" >&2; exit 2 ;;
  esac
done

cargo build --release -p s2-window >/dev/null 2>&1 || {
  echo "build failed: cargo build --release -p s2-window" >&2
  exit 1
}
app="$ROOT/target/release/s2-window"
out="$ROOT/target/s2-results/$(date +%Y%m%d-%H%M%S)"
mkdir -p "$out"

stripped="$out/s2-window.stripped"
cp "$app" "$stripped" && strip "$stripped"
{
  echo "date:     $(date '+%Y-%m-%d %H:%M:%S %Z')"
  echo "git:      $(git rev-parse --short HEAD)$([ -z "$(git status --porcelain)" ] || echo ' (uncommitted changes present)')"
  echo "rustc:    $(rustc -V)"
  echo "machine:  $(sysctl -n machdep.cpu.brand_string), $(($(sysctl -n hw.memsize) / 1073741824)) GiB, $(sw_vers -productName) $(sw_vers -productVersion)"
  echo "load avg: $(sysctl -n vm.loadavg)"
  echo "binary:   $(stat -f %z "$app") bytes as built, $(stat -f %z "$stripped") bytes stripped"
} | tee "$out/environment.txt"

printf 'run\tshaders_ms\tshown_ms\tfirst_frame_ms\twall_s\tpeak_footprint\tmax_rss\tinstructions\n' >"$out/smoke.tsv"
i=1
while [ "$i" -le "$runs" ]; do
  log=$(/usr/bin/time -l -o "$out/time.tmp" "$app" --smoke 2>&1) || {
    echo "smoke run $i failed:" >&2
    echo "$log" >&2
    exit 1
  }
  echo "$log" >"$out/smoke-$i.log"
  awk -v run="$i" -v text="$(echo "$log" | tr '\n' ' ')" '
    / real /                      { wall = $1 }
    /maximum resident set size/   { rss = $1 }
    /peak memory footprint/       { peak = $1 }
    /instructions retired/        { ins = $1 }
    END {
      match(text, /MSL compiled at [0-9.]+/); shaders = substr(text, RSTART + 16, RLENGTH - 16)
      match(text, /window shown at [0-9.]+/); shown = substr(text, RSTART + 16, RLENGTH - 16)
      match(text, /first frame presented at [0-9.]+/); first = substr(text, RSTART + 25, RLENGTH - 25)
      printf "%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n", run, shaders, shown, first, wall, peak, rss, ins
    }' "$out/time.tmp" >>"$out/smoke.tsv"
  sleep 1
  i=$((i + 1))
done

printf 'run\ttop_cpu_pct\ttop_idle_wakeups\ttop_power\ttop_mem\tps_rss_kib\tps_cpu_delta_s\tps_window_s\n' >"$out/idle.tsv"
i=1
while [ "$i" -le "$idle_runs" ]; do
  "$app" --seconds "$idle" >"$out/idle-$i.log" 2>&1 &
  pid=$!
  sleep 3
  sample=$(top -l 2 -s 2 -pid "$pid" -stats cpu,idlew,power,mem 2>/dev/null | tail -1)
  t0=$(ps -o time= -p "$pid")
  window=$((idle - 7))
  sleep "$window"
  t1=$(ps -o time= -p "$pid")
  rss=$(ps -o rss= -p "$pid" | tr -d ' ')
  wait "$pid" || { echo "idle run $i failed; see $out/idle-$i.log" >&2; exit 1; }
  echo "$i $sample $rss $t0 $t1 $window" | awk '
    function secs(t,   parts, n) { n = split(t, parts, ":"); return n == 2 ? parts[1] * 60 + parts[2] : parts[1] }
    { printf "%s\t%s\t%s\t%s\t%s\t%s\t%.2f\t%s\n", $1, $2, $3, $4, $5, $6, secs($8) - secs($7), $9 }' >>"$out/idle.tsv"
  sleep 1
  i=$((i + 1))
done
rm -f "$out/time.tmp"

python3 -I - "$out" >"$out/summary.md" <<'PY'
import os, statistics, sys
out = sys.argv[1]
def rows(name):
    with open(os.path.join(out, name), encoding="utf-8") as f:
        header = f.readline().rstrip("\n").split("\t")
        return [dict(zip(header, line.rstrip("\n").split("\t"))) for line in f if line.strip()]
smoke, idle = rows("smoke.tsv"), rows("idle.tsv")
def spread(values):
    values = sorted(values)
    return f"{statistics.median(values):.1f} (range {values[0]:.1f}–{values[-1]:.1f})"
print(f"Smoke runs: {len(smoke)}, each checked against the coverage mask. Times are from main().\n")
print("| Measure | Median (range) |")
print("|---|---|")
for key, label, scale in (
    ("shaders_ms", "Metal device and shaders ready (ms)", 1),
    ("shown_ms", "Window shown (ms)", 1),
    ("first_frame_ms", "First frame presented (ms)", 1),
    ("wall_s", "Whole smoke run, wall (s)", 1),
    ("peak_footprint", "Peak memory footprint (MiB)", 1 / 1048576),
    ("max_rss", "Max RSS (MiB)", 1 / 1048576),
    ("instructions", "Instructions retired (millions)", 1e-6),
):
    print(f"| {label} | {spread([float(r[key]) * scale for r in smoke])} |")
print(f"\nIdle runs: {len(idle)}; the window is open with no input.\n")
print("| Measure | Per run |")
print("|---|---|")
print("| top: %CPU | " + ", ".join(r["top_cpu_pct"] for r in idle) + " |")
print("| top: idle wakeups (per sample) | " + ", ".join(r["top_idle_wakeups"] for r in idle) + " |")
print("| top: energy impact (POWER) | " + ", ".join(r["top_power"] for r in idle) + " |")
print("| top: memory footprint | " + ", ".join(r["top_mem"] for r in idle) + " |")
print("| ps: RSS (MiB, includes shared framework pages) | " + ", ".join(f"{int(r['ps_rss_kib']) / 1024:.1f}" for r in idle) + " |")
print("| ps: CPU time accumulated while idle | " + ", ".join(f"{r['ps_cpu_delta_s']} s over {r['ps_window_s']} s" for r in idle) + " |")
PY
cat "$out/summary.md"
echo
echo "results: ${out#"$ROOT"/}"
