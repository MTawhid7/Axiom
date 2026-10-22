#!/usr/bin/env bash
# Spike S12: routes to a working ThreadSanitizer on macOS 27 (the architecture
# review's RV1-11; docs/research/s12-results.md).
#
# Usage: tools/s12-measure.sh [--nightly TOOLCHAIN] [--old-nightly TOOLCHAIN] ROUTE...
#
# Each route runs S9's s9-race tests under ThreadSanitizer, with the standard
# library rebuilt: the clean test, which must pass, then the planted race,
# which must be reported. TSAN_OPTIONS=abort_on_error=0 makes a reported race
# exit with 66 instead of aborting, so no crash report is written; any report
# a run leaves anyway is moved into the results.
#
# Routes:
#   bundled    S12-Q1: the updated nightly (--nightly, default
#              nightly-2026-10-21) with rustc's own runtime,
#              librustc-nightly_rt.tsan.dylib
#   external   S12-Q2: the older nightly (--old-nightly, default nightly, the
#              2026-04-08 build S9 found broken) with `-Z external-clangrt`,
#              linking the Command Line Tools' libclang_rt.tsan_osx_dynamic.dylib
#   baseline   S9's failure, reproduced: the older nightly with its own runtime
#
# Results go to target/s12-results/<timestamp>/, which git ignores: each run's
# log, the runtime each test binary links (otool -L), and a summary. The
# toolchains must be installed already: rustup toolchain install
# nightly-2026-10-21 --profile minimal --component rust-src.
set -uo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd)
S12="$ROOT/target/s12"
NIGHTLY=nightly-2026-10-21
OLD_NIGHTLY=nightly
CLT_RT=/Library/Developer/CommandLineTools/usr/lib/clang/21/lib/darwin

usage() { sed -n '2,/^set -uo/p' "$0" | sed '$d' | sed 's/^# \{0,1\}//'; }

routes=()
while [ $# -gt 0 ]; do
  case $1 in
    --help | -h) usage; exit 0 ;;
    --nightly) NIGHTLY=$2; shift 2 ;;
    --old-nightly) OLD_NIGHTLY=$2; shift 2 ;;
    bundled | external | baseline) routes+=("$1"); shift ;;
    *) echo "unknown argument: $1 (see --help)" >&2; exit 2 ;;
  esac
done
[ ${#routes[@]} -gt 0 ] || { usage; exit 2; }

OUT="$ROOT/target/s12-results/$(date +%Y%m%d-%H%M%S)"
mkdir -p "$OUT" "$S12"
log() { echo "[$(date +%H:%M:%S)] $*" | tee -a "$OUT/log.txt"; }

{
  echo "macOS $(sw_vers -productVersion) ($(sw_vers -buildVersion))"
  /Library/Developer/CommandLineTools/usr/bin/clang --version | head -1
  for t in "$NIGHTLY" "$OLD_NIGHTLY"; do echo "$t: $(rustc +"$t" -V 2>&1)"; done
} > "$OUT/environment.txt"

outcome() { # a run's log → what happened
  local file=$1
  if grep -q "SIGSEGV" "$file"; then
    echo "crashed (SIGSEGV)"
  else
    echo "$(grep -c 'WARNING: ThreadSanitizer' "$file") reports; $(grep -E '^exit ' "$file")"
  fi
}

collect_crash_reports() {
  local report
  for report in "$HOME/Library/Logs/DiagnosticReports/"s9_race-*.ips; do
    [ -e "$report" ] || continue
    mv "$report" "$OUT/" && log "moved $(basename "$report") into the results"
  done
}

run() { # route toolchain flags test-args...
  local route=$1 toolchain=$2 flags=$3; shift 3
  local name="$route-$([ "$1" = --ignored ] && echo planted || echo clean)"
  (cd "$ROOT" && RUSTFLAGS="$flags" RUSTDOCFLAGS="$flags" TSAN_OPTIONS="abort_on_error=0" \
    cargo +"$toolchain" test --manifest-path "$ROOT/spikes/Cargo.toml" -Zbuild-std --target aarch64-apple-darwin \
    --target-dir "$S12/$route-target" -p s9-race -- "$@") > "$OUT/$name.txt" 2>&1
  echo "exit $?" >> "$OUT/$name.txt"
  local binary
  binary=$(grep -o "(target/s12/$route-target[^)]*)" "$OUT/$name.txt" | head -1 | tr -d '()')
  [ -n "$binary" ] && otool -L "$ROOT/$binary" | grep -i tsan > "$OUT/$name-runtime.txt"
  log "$name: $(outcome "$OUT/$name.txt"); runtime: $(awk '{print $1}' "$OUT/$name-runtime.txt" 2>/dev/null)"
}

for route in "${routes[@]}"; do
  case $route in
    bundled)
      flags="-Zsanitizer=thread"
      run bundled "$NIGHTLY" "$flags" the_atomic_count_is_exact
      run bundled "$NIGHTLY" "$flags" --ignored the_planted_race ;;
    external)
      flags="-Zsanitizer=thread -Zexternal-clangrt -Clink-arg=-L$CLT_RT -Clink-arg=-lclang_rt.tsan_osx_dynamic -Clink-arg=-Wl,-rpath,$CLT_RT"
      run external "$OLD_NIGHTLY" "$flags" the_atomic_count_is_exact
      run external "$OLD_NIGHTLY" "$flags" --ignored the_planted_race ;;
    baseline)
      flags="-Zsanitizer=thread"
      run baseline "$OLD_NIGHTLY" "$flags" the_atomic_count_is_exact ;;
  esac
done
sleep 5 # ReportCrash writes its reports a moment after a crash
collect_crash_reports
log "done: $OUT"
