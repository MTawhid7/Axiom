#!/usr/bin/env bash
# Spike S1 CPU profiles: where each pattern spends its time, per case.
#
# Usage: tools/s1-profile.sh [--seconds N] [CASE...]
#   --seconds N   how long each case loops under the profiler (default 5)
#   CASE          cases of `s1-probe profile` (default: build traverse mutate
#                 dispatch collect-live scenario-pressure scenario-churn)
#
# Builds s1-probe with the `profiling` profile (release plus line tables) and
# a .dSYM bundle, then records each (pattern, case) with samply at 1 kHz.
# Next to each profile it writes a summary from tools/samply-summary.py,
# narrowed to the profiled loop (`--match profiled_iteration`) and resolved to
# source lines.
#
# Writes target/s1-results/profiles-<timestamp>/:
#   <case>-<a|b>.json.gz, <case>-<a|b>.json.syms.json, <case>-<a|b>.txt
# Open one in the Firefox Profiler UI with: samply load <file>.json.gz
# Written for bash 3.2 (macOS /bin/bash).
set -uo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd)
cd "$ROOT" || exit 2

seconds=5
cases=""
while [ $# -gt 0 ]; do
  case $1 in
    --seconds) seconds=$2; shift 2 ;;
    -h | --help) sed -n '2,19p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
    -*) echo "unknown argument: $1 (see --help)" >&2; exit 2 ;;
    *) cases="$cases $1"; shift ;;
  esac
done
[ -n "$cases" ] || cases="build traverse mutate dispatch collect-live scenario-pressure scenario-churn"

command -v samply >/dev/null || { echo "samply is not installed (cargo install --locked samply)" >&2; exit 1; }
cargo build --manifest-path "$ROOT/spikes/Cargo.toml" --profile profiling -p s1-bench --bin s1-probe >/dev/null 2>&1 || {
  echo "build failed: cargo build --profile profiling -p s1-bench --bin s1-probe" >&2
  exit 1
}
probe="$ROOT/target/profiling/s1-probe"
rm -rf "$probe.dSYM"
dsymutil "$probe" || exit 1

out="$ROOT/target/s1-results/profiles-$(date +%Y%m%d-%H%M%S)"
mkdir -p "$out"
{
  echo "date:    $(date '+%Y-%m-%d %H:%M:%S %Z')"
  echo "git:     $(git rev-parse --short HEAD)$([ -z "$(git status --porcelain)" ] || echo ' (uncommitted changes present)')"
  echo "samply:  $(samply --version)"
  echo "seconds: $seconds per case"
} >"$out/environment.txt"

for case in $cases; do
  for pattern in a b; do
    base="$out/$case-$pattern"
    samply record --save-only --unstable-presymbolicate -o "$base.json.gz" -- \
      "$probe" profile "$pattern" "$case" "$seconds" >"$base.log" 2>&1 || {
      echo "samply failed for $case/$pattern; see $base.log" >&2
      exit 1
    }
    python3 -I "$ROOT/tools/samply-summary.py" "$base.json.gz" --match profiled_iteration \
      --lines "$probe" --top 15 >"$base.txt" || exit 1
    echo "== $case / $pattern: $(grep -Eo 'iterations=[0-9]+' "$base.log")"
    sed -n '/^Self time, by compiled function/,/^$/p' "$base.txt" | sed -n '2,6p'
  done
done
echo
echo "results: ${out#"$ROOT"/}"
