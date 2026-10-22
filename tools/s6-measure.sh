#!/usr/bin/env bash
# Spike S6 measurements: every S6 question on a release build.
#
# Usage: tools/s6-measure.sh [--runs N] [--cooldown SECONDS] [--seconds S]
#                            [--show SECONDS] [--allow-battery] [--quick]
#   --runs N            runs of each measurement (default 3), each after a cool-down
#   --cooldown SECONDS  idle time before each run (default 60): the MacBook Air has
#                       no fan and throttles when hot
#   --seconds S         length of each damage, scrolling and animation run (default 20)
#   --show SECONDS      at the end, show the Display P3 test pattern this long, for
#                       the owner to compare by eye (default 0: not shown)
#   --allow-battery     run without AC power, or with Low Power Mode on
#   --quick             a short pass for checking the script
#
# The method (Step 5 tracker, §10.2) needs the Mac otherwise idle and the window
# visible on the built-in display. The window opens at the top left of the main
# screen and floats above other windows, but it can't appear over another app in
# full screen: leave the desktop showing. Each run records whether the window
# was visible (`unoccluded true`), and the summary flags runs where it wasn't.
#
# Runs, each --runs times after a cool-down:
#   S6-Q1  q1: 120 full redraws, 600 typical updates (typing and a spinner), then
#          10 s idle (wakeups)
#   S6-Q2  q2: a spinner and typing, each with damage-limited and full redraws,
#          --seconds each, with `top` sampling the process's energy impact
#   S6-Q3  q3: scrolling and animation by layers, --seconds each
#   S6-Q4  q4: paths, offscreen: CPU filling and upload against GPU filling
#   S6-Q5  q5: the footprint at each stage, and the colour tags
# Writes target/s6-results/<timestamp>/, with summary.txt.
# Written for bash 3.2 (macOS /bin/bash).
set -uo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd)
cd "$ROOT" || exit 2

runs=3 cooldown=60 seconds=20 show=0 battery="" quick=""
while [ $# -gt 0 ]; do
  case $1 in
    --runs) runs=$2; shift 2 ;;
    --cooldown) cooldown=$2; shift 2 ;;
    --seconds) seconds=$2; shift 2 ;;
    --show) show=$2; shift 2 ;;
    --allow-battery) battery=yes; shift ;;
    --quick) quick=yes; shift ;;
    -h | --help) sed -n '2,31p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
    *) echo "unknown argument: $1 (see --help)" >&2; exit 2 ;;
  esac
done
frames=120 updates=600 idle=10
if [ -n "$quick" ]; then runs=1 cooldown=0 seconds=3 frames=20 updates=60 idle=2; fi

power=$(pmset -g batt | head -1)
lpm=$(pmset -g | awk '/lowpowermode/ { print $2 }')
if [ -z "$battery" ] && { ! printf '%s' "$power" | grep -q "AC Power" || [ "${lpm:-0}" != 0 ]; }; then
  echo "not on AC power with Low Power Mode off ($power; lowpowermode ${lpm:-?})." >&2
  echo "Timings would not be comparable; use --allow-battery to run anyway." >&2
  exit 3
fi

cargo build --manifest-path "$ROOT/spikes/Cargo.toml" -q --release -p s6-compositor || { echo "the release build failed" >&2; exit 1; }
app=target/release/s6-compositor

out="target/s6-results/$(date +%Y%m%d-%H%M%S)"
mkdir -p "$out"
start=$(date '+%Y-%m-%d %H:%M:%S')
{
  echo "date: $start"
  sw_vers
  echo "kernel: $(uname -v)"
  echo "machine: $(sysctl -n hw.model), $(sysctl -n hw.memsize) bytes"
  echo "power: $power; lowpowermode ${lpm:-?}"
  echo "load average at start: $(sysctl -n vm.loadavg)"
  echo "commit: $(git rev-parse HEAD)$(git diff --quiet && git diff --cached --quiet || echo ' (uncommitted changes)')"
  echo "untracked files: $(git status --porcelain | grep -c '^??')"
  echo "rustc: $(rustc --version)"
  echo "options: runs=$runs cooldown=${cooldown}s seconds=$seconds frames=$frames updates=$updates idle=$idle quick=${quick:-no}"
  system_profiler SPDisplaysDataType 2>/dev/null | grep -E "Chipset Model|Resolution|Display Type|Main Display" | sed 's/^ *//'
} >"$out/environment.txt"

failed=0
# run NAME ARGS...: one run of the app; its output to NAME.txt.
run() {
  local name=$1
  shift
  if ! "$app" "$@" >"$out/$name.txt" 2>&1; then
    failed=1
    echo "   FAILED: $app $* (see $out/$name.txt)"
  fi
}
# energy NAME ARGS...: run the app in the background, sampling its energy
# impact with top once a second while it runs.
energy() {
  local name=$1
  shift
  "$app" "$@" >"$out/$name.txt" 2>&1 &
  local pid=$!
  sleep 2
  top -l "$seconds" -s 1 -pid "$pid" -stats pid,cpu,idlew,power 2>/dev/null |
    awk -v pid="$pid" '$1 == pid { print }' >"$out/$name.top.txt"
  wait "$pid" || { failed=1; echo "   FAILED: $app $* (see $out/$name.txt)"; }
}

for r in $(seq 1 "$runs"); do
  echo "== run $r of $runs, after a ${cooldown}s cool-down"
  sleep "$cooldown"
  run "q1-run$r" q1 --frames "$frames" --updates "$updates" --idle "$idle" --trace "$out/q1-run$r.trace.tsv"
  for animation in spinner typing; do
    for damage in tiles full; do
      energy "q2-$animation-$damage-run$r" q2 --animation "$animation" --damage "$damage" \
        --seconds "$seconds" --trace "$out/q2-$animation-$damage-run$r.trace.tsv"
    done
  done
  for motion in scroll animate; do
    run "q3-$motion-run$r" q3 --motion "$motion" --seconds "$seconds" --trace "$out/q3-$motion-run$r.trace.tsv"
  done
  run "q4-run$r" q4 --repeat 5
  run "q5-run$r" q5
done
if [ "$show" != 0 ]; then
  echo "== the Display P3 pattern, ${show}s: compare the three red squares at the window's top left"
  run "q5-show" q5 --show "$show"
fi

{
  echo "S6 results, $start ($(grep '^commit:' "$out/environment.txt"), at the start)"
  echo
  echo "Window visibility per run (each should say unoccluded true):"
  grep -h "^# window" "$out"/q*.txt | sed 's/^# /  /' | sort | uniq -c
  echo
  echo "S6-Q1 (median/p95/max)"
  grep -h "^q1" "$out"/q1-run*.txt | sed 's/^q1/ /'
  echo
  echo "S6-Q2 (median/p95/max), and top's energy impact (mean POWER over the samples after the first, which has no interval)"
  for f in "$out"/q2-*-run*.txt; do
    case $f in *.top.txt) continue ;; esac
    base=${f%.txt}
    printf '  %s\t%s\n' "$(grep -h '^q2' "$f" | cut -f2-6)" \
      "$(awk 'NR > 1 { s += $4; n++ } END { if (n) printf "power %.2f over %d samples", s / n, n }' "$base.top.txt")"
  done
  echo
  echo "S6-Q3 (median/p95/max)"
  grep -h "^q3" "$out"/q3-*-run*.txt | sed 's/^q3/ /'
  echo
  echo "S6-Q4"
  grep -h "^q4" "$out"/q4-run*.txt | sed 's/^q4/ /'
  echo
  echo "S6-Q5"
  grep -h "^q5" "$out"/q5-run*.txt | sed 's/^q5/ /'
} >"$out/summary.txt"

echo "results in $out"
exit "$failed"
