#!/usr/bin/env bash
# Spike S4 measurements: every S4 scenario against signed release builds.
#
# Usage: tools/s4-measure.sh [--runs N] [--batches N] [--named-profile]
#   --runs N          start-up runs per configuration (default 30)
#   --batches N       timed IPC batches per series (default 5)
#   --named-profile   also check that a named profile kills the helper. Each
#                     such kill leaves a crash report in
#                     ~/Library/Logs/DiagnosticReports, so it is off by default
#
# Signs release copies with tools/s4-sign.sh, then runs `s4-broker` from
# target/s4/bin:
#   probe     S4-Q1, S4-Q2: the 18 probes, control and sandboxed, by each method
#   startup   S4-Q3: spawn-to-ready time, footprint, profile compile and apply times
#   ipc       S4-Q3: echo latency and 1 MiB transfers over socket pairs and Mach
#   jit       S4-Q4: MAP_JIT by signing, entitlement and profile
#   text      S4-Q5: CoreText inside the text profile, then its start-up and footprint
# and saves the kernel's sandbox denial reports logged during the run.
#
# Writes target/s4-results/<timestamp>/: environment.txt, one <command>.txt per
# command, sandbox-denials.txt. Exits non-zero if any command reports a failure.
# Written for bash 3.2 (macOS /bin/bash).
set -uo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd)
cd "$ROOT" || exit 2

runs=30 batches=5 named=""
while [ $# -gt 0 ]; do
  case $1 in
    --runs) runs=$2; shift 2 ;;
    --batches) batches=$2; shift 2 ;;
    --named-profile) named=--named-profile; shift ;;
    -h | --help) sed -n '2,23p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
    *) echo "unknown argument: $1 (see --help)" >&2; exit 2 ;;
  esac
done

tools/s4-sign.sh --release >/dev/null 2>&1 || {
  echo "signing failed: tools/s4-sign.sh --release" >&2
  exit 1
}

out="target/s4-results/$(date +%Y%m%d-%H%M%S)"
mkdir -p "$out"
start=$(date '+%Y-%m-%d %H:%M:%S')
{
  echo "date: $start"
  sw_vers
  echo "kernel: $(uname -v)"
  echo "machine: $(sysctl -n hw.model), $(sysctl -n hw.memsize) bytes"
  echo "power: $(pmset -g batt | head -1)"
  echo "commit: $(git rev-parse HEAD)$(git diff --quiet && git diff --cached --quiet || echo ' (uncommitted changes)')"
  echo "rustc: $(rustc --version)"
  echo "net.local.stream.sendspace: $(sysctl -n net.local.stream.sendspace)"
  for binary in s4-broker s4-helper s4-helper-jit s4-helper-jit1 s4-helper-plain s4-text; do
    flags=$(codesign -dv "target/s4/bin/$binary" 2>&1 | grep -E '^CodeDirectory' | sed 's/.*flags=//; s/ hashes.*//')
    sdk=$(otool -l "target/s4/bin/$binary" | awk '/LC_BUILD_VERSION/{f=1} f&&/sdk/{print $2; exit}')
    echo "$binary: flags=$flags sdk=$sdk"
  done
} >"$out/environment.txt"

failed=0
broker=target/s4/bin/s4-broker
run() { # name args...
  local name=$1
  shift
  echo "== s4-broker $name $*"
  if ! "$broker" "$name" "$@" >"$out/$name.txt" 2>&1; then
    failed=1
    echo "   FAILED (see $out/$name.txt)"
  fi
  tail -1 "$out/$name.txt" | sed 's/^/   /'
}

run probe $named
run startup --runs "$runs"
run ipc --batches "$batches"
run jit
run text --runs "$runs"

# The kernel logs each denial; keep the ones from this run's helpers.
/usr/bin/log show --start "$start" --style compact \
  --predicate 'sender == "Sandbox" AND (eventMessage CONTAINS "s4-helper" OR eventMessage CONTAINS "s4-text")' \
  >"$out/sandbox-denials.txt" 2>&1 || true

echo "results in $out"
exit "$failed"
