#!/usr/bin/env bash
# Spike S3 CPU profiles: where each workload spends its time, and what share
# goes to the root stack (S3-Q2).
#
# Usage: tools/s3-profile.sh [--seconds N] [CASE...]
#   --seconds N   how long each case loops under the profiler (default 5)
#   CASE          cases of `s3-probe profile` (default: w1–w4 with every
#                 technique on every stack, then dispatch on every stack)
#
# Builds s3-probe with the `profiling` profile (release plus line tables) and
# a .dSYM bundle, then records each case with samply at 1 kHz. Next to each
# profile it writes a summary from tools/samply-summary.py, narrowed to the
# profiled loop (`--match profiled_iteration`), resolved to source lines, and
# with these shares of self time, by inlined source file:
#   rooting   the root stacks and frames (spikes/s3/gc/src/roots.rs, raw/)
#   builtins  the builtins (spikes/s3/vm/src/builtins.rs)
#   gc        allocation and collection (spikes/s3/gc/src/arena.rs, trace.rs)
# shares.md collects them in one table.
#
# Writes target/s3-results/profiles-<timestamp>/:
#   <case>.json.gz, <case>.json.syms.json, <case>.log, <case>.txt, shares.md
# where <case> has its slashes replaced by dashes. Open a profile in the
# Firefox Profiler UI with: samply load <file>.json.gz
# Written for bash 3.2 (macOS /bin/bash).
set -uo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd)
cd "$ROOT" || exit 2

seconds=5
cases=""
while [ $# -gt 0 ]; do
  case $1 in
    --seconds) seconds=$2; shift 2 ;;
    -h | --help) sed -n '2,25p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
    -*) echo "unknown argument: $1 (see --help)" >&2; exit 2 ;;
    *) cases="$cases $1"; shift ;;
  esac
done
if [ -z "$cases" ]; then
  for w in w1 w2 w3 w4; do
    for t in match table threaded; do
      for s in raw cell refcell; do cases="$cases $w/$t/$s"; done
    done
  done
  cases="$cases dispatch/raw dispatch/cell dispatch/refcell"
fi

command -v samply >/dev/null || { echo "samply is not installed (cargo install --locked samply)" >&2; exit 1; }
cargo build --profile profiling -p s3-bench --bin s3-probe >/dev/null 2>&1 || {
  echo "build failed: cargo build --profile profiling -p s3-bench --bin s3-probe" >&2
  exit 1
}
probe="$ROOT/target/profiling/s3-probe"
rm -rf "$probe.dSYM"
dsymutil "$probe" || exit 1

out="$ROOT/target/s3-results/profiles-$(date +%Y%m%d-%H%M%S)"
mkdir -p "$out"
{
  echo "date:    $(date '+%Y-%m-%d %H:%M:%S %Z')"
  echo "git:     $(git rev-parse --short HEAD)$([ -z "$(git status --porcelain)" ] || echo ' (uncommitted changes present)')"
  echo "samply:  $(samply --version)"
  echo "seconds: $seconds per case"
} >"$out/environment.txt"

{
  echo "| Case | Samples | Rooting | Builtins | GC |"
  echo "|---|---:|---:|---:|---:|"
} >"$out/shares.md"

for case in $cases; do
  base="$out/$(echo "$case" | tr '/' '-')"
  samply record --save-only --unstable-presymbolicate -o "$base.json.gz" -- \
    "$probe" profile "$case" "$seconds" >"$base.log" 2>&1 || {
    echo "samply failed for $case; see $base.log" >&2
    exit 1
  }
  python3 -I "$ROOT/tools/samply-summary.py" "$base.json.gz" --match profiled_iteration \
    --lines "$probe" --top 15 \
    --share 'rooting=^spikes/s3/gc/src/(roots\.rs|raw/)' \
    --share 'builtins=^spikes/s3/vm/src/builtins\.rs' \
    --share 'gc=^spikes/s3/gc/src/(arena|trace)\.rs' >"$base.txt" || exit 1
  share() { grep -E "^share $1:" "$base.txt" | sed -E 's/.*: //'; }
  samples=$(sed -n '1s/ samples.*//p' "$base.txt")
  echo "| $case | $samples | $(share rooting) | $(share builtins) | $(share gc) |" >>"$out/shares.md"
  echo "== $case: $(grep -Eo 'iterations=[0-9]+' "$base.log"), rooting $(share rooting)"
done
echo
cat "$out/shares.md"
echo
echo "results: ${out#"$ROOT"/}"
