#!/usr/bin/env bash
# Spike S9: the test tools on this Mac (docs/testing.md §9).
#
# Usage: tools/s9-measure.sh [--seconds N] [--wpt-dir DIR] STEP...
#
# Steps, in the order they're meant to run:
#   setup      cargo-fuzz 0.13.2 into target/s9/tools; the nightly toolchain's
#              llvm-tools-preview component, for coverage (rustup installs it
#              beside the toolchain; `rustup component remove` takes it away)
#   fuzz       S9-Q1: both fuzz targets built with AddressSanitizer. `planted`
#              runs until it finds the planted bug, at most --seconds; then the
#              crash is reproduced from its artifact. `decode` runs --seconds
#   coverage   S9-Q1: a coverage report of `decode` over the corpus it grew
#   tsan       S9-Q2: s9-race's tests under ThreadSanitizer, with the standard
#              library rebuilt: the clean test, then the planted race
#   wpt-setup  S9-Q3: web-platform-tests at the revision the standards baseline
#              calibrated on, sparsely checked out into target/s9/wpt, and
#              chromedriver for the installed Chrome Canary into
#              target/s9/chromedriver (Chrome for Testing's build)
#   wpt        S9-Q3: wptrunner runs --wpt-dir against Chrome Canary through
#              chromedriver, headless, sampling its processes' memory
#
# Options:
#   --seconds N   length of each fuzzing run (default 600)
#   --wpt-dir D   the WPT directory for `wpt` (default dom/nodes)
#
# Results go to target/s9-results/<timestamp>/, which git ignores. Nothing is
# installed outside target/ except the rustup component. wptrunner's Python
# packages go into a virtual environment inside the WPT checkout, made by the
# interpreter in $WPT_PYTHON (default python3). Fonts are never installed.
set -uo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd)
S9="$ROOT/target/s9"
FUZZ_DIR="$ROOT/spikes/s9/fuzz"
WPT="$S9/wpt"
WPT_REVISION=0f26972422b15f587e2afbd38dbb6ea2397fded0 # wpt.fyi's runs of 2026-10-19 (standards §6)
CANARY="/Applications/Google Chrome Canary.app/Contents/MacOS/Google Chrome Canary"
SECONDS_PER_RUN=600
WPT_DIR=dom/nodes
WPT_PYTHON=${WPT_PYTHON:-python3} # the interpreter that runs ./wpt and makes its virtual environment

usage() { sed -n '2,/^set -uo/p' "$0" | sed '$d' | sed 's/^# \{0,1\}//'; }

steps=()
while [ $# -gt 0 ]; do
  case $1 in
    --help | -h) usage; exit 0 ;;
    --seconds) SECONDS_PER_RUN=$2; shift 2 ;;
    --wpt-dir) WPT_DIR=$2; shift 2 ;;
    setup | fuzz | coverage | tsan | wpt-setup | wpt) steps+=("$1"); shift ;;
    *) echo "unknown argument: $1 (see --help)" >&2; exit 2 ;;
  esac
done
[ ${#steps[@]} -gt 0 ] || { usage; exit 2; }

OUT="$ROOT/target/s9-results/$(date +%Y%m%d-%H%M%S)"
mkdir -p "$OUT" "$S9"
export PATH="$S9/tools/bin:$PATH"
log() { echo "[$(date +%H:%M:%S)] $*" | tee -a "$OUT/log.txt"; }

record_environment() {
  {
    echo "date: $(date '+%Y-%m-%d %H:%M:%S')"
    echo "commit: $(git -C "$ROOT" rev-parse HEAD)$(git -C "$ROOT" diff --quiet HEAD -- spikes/s9 tools/s9-measure.sh || echo ' (uncommitted changes)')"
    echo "macos: $(sw_vers -productVersion)"
    echo "rustc nightly: $(rustc +nightly --version)"
    echo "cargo-fuzz: $(cargo-fuzz --version 2>/dev/null || echo missing)"
    echo "power: $(pmset -g batt | head -1)"
    echo "low power mode: $(pmset -g | awk '/lowpowermode/ {print $2}')"
    echo "load average: $(sysctl -n vm.loadavg)"
    echo "memory free: $(memory_pressure -Q | awk -F': ' '/free percentage/ {print $2}')"
  } > "$OUT/environment.txt"
}

step_setup() {
  log "setup: llvm-tools-preview on nightly"
  rustup component add llvm-tools-preview --toolchain nightly 2>&1 | tee -a "$OUT/setup.txt"
  if [ ! -x "$S9/tools/bin/cargo-fuzz" ]; then
    log "setup: cargo-fuzz 0.13.2 into target/s9/tools"
    (cd "$ROOT" && cargo install cargo-fuzz@0.13.2 --locked --root "$S9/tools") 2>&1 | tail -3 | tee -a "$OUT/setup.txt"
  fi
}

fuzz() { # cargo-fuzz's command, with the spike's directories
  local command=$1
  shift
  (cd "$ROOT" && cargo +nightly fuzz "$command" --fuzz-dir "$FUZZ_DIR" --target-dir "$S9/fuzz-target" "$@")
}

step_fuzz() {
  local corpus="$S9/corpus" artifacts="$S9/artifacts"
  rm -rf "$corpus" "$artifacts"
  mkdir -p "$corpus/decode" "$corpus/planted" "$artifacts/planted" "$artifacts/decode"
  (cd "$ROOT" && cargo run -q -p s9-decoder --bin s9-seeds -- "$corpus/decode") || return 1
  cp "$corpus/decode/"* "$corpus/planted/"
  log "fuzz: building both targets (AddressSanitizer; std rebuilt with it)"
  /usr/bin/time -l bash -c "cd '$ROOT' && PATH='$PATH' cargo +nightly fuzz build --fuzz-dir '$FUZZ_DIR' --target-dir '$S9/fuzz-target'" \
    > "$OUT/fuzz-build.txt" 2>&1 || { log "fuzz: build failed (fuzz-build.txt)"; return 1; }
  local binary
  binary=$(find "$S9/fuzz-target" -path '*/release/planted' -type f -perm -u+x | head -1)
  log "fuzz: binary $binary"
  otool -L "$binary" > "$OUT/fuzz-planted-otool.txt"
  ASAN_OPTIONS=help=1 "$binary" -runs=0 > "$OUT/fuzz-asan-help.txt" 2>&1
  log "fuzz: planted, at most $SECONDS_PER_RUN s"
  local start end
  start=$(date +%s)
  fuzz run planted "$corpus/planted" -- -max_total_time="$SECONDS_PER_RUN" \
    -artifact_prefix="$artifacts/planted/" -print_final_stats=1 > "$OUT/fuzz-planted.txt" 2>&1
  echo "exit $?" >> "$OUT/fuzz-planted.txt"
  end=$(date +%s)
  log "fuzz: planted ended after $((end - start)) s; artifacts: $(ls "$artifacts/planted" | tr '\n' ' ')"
  local crash
  crash=$(ls "$artifacts/planted"/crash-* 2>/dev/null | head -1)
  if [ -n "$crash" ]; then
    xxd "$crash" > "$OUT/fuzz-planted-crash.xxd"
    fuzz run planted "$crash" > "$OUT/fuzz-planted-reproduce.txt" 2>&1
    echo "exit $?" >> "$OUT/fuzz-planted-reproduce.txt"
    log "fuzz: reproduced from the artifact: $(tail -1 "$OUT/fuzz-planted-reproduce.txt")"
  fi
  log "fuzz: decode, $SECONDS_PER_RUN s"
  /usr/bin/time -l bash -c "cd '$ROOT' && PATH='$PATH' cargo +nightly fuzz run --fuzz-dir '$FUZZ_DIR' --target-dir '$S9/fuzz-target' decode '$corpus/decode' -- -max_total_time=$SECONDS_PER_RUN -artifact_prefix='$artifacts/decode/' -print_final_stats=1" \
    > "$OUT/fuzz-decode.txt" 2>&1
  echo "exit $?" >> "$OUT/fuzz-decode.txt"
  log "fuzz: decode ended: $(grep -E '^Done|stat::number_of_executed_units' "$OUT/fuzz-decode.txt" | tr '\n' ' ')"
}

step_coverage() {
  log "coverage: decode over its corpus"
  # Its own target directory: coverage builds would otherwise replace the fuzzing builds.
  (cd "$ROOT" && cargo +nightly fuzz coverage --fuzz-dir "$FUZZ_DIR" --target-dir "$S9/coverage-target" \
    decode "$S9/corpus/decode") > "$OUT/coverage-build.txt" 2>&1 || { log "coverage: failed (coverage-build.txt)"; return 1; }
  local sysroot llvm binary profdata
  sysroot=$(rustc +nightly --print sysroot)
  llvm="$sysroot/lib/rustlib/aarch64-apple-darwin/bin"
  binary=$(find "$S9/coverage-target" -path '*/release/decode' -type f -perm -u+x | head -1)
  profdata="$FUZZ_DIR/coverage/decode/coverage.profdata"
  "$llvm/llvm-cov" report "$binary" -instr-profile="$profdata" \
    "$ROOT/spikes/s9/decoder/src/lib.rs" > "$OUT/coverage-report.txt" 2>&1
  "$llvm/llvm-cov" show "$binary" -instr-profile="$profdata" -show-line-counts-or-regions \
    "$ROOT/spikes/s9/decoder/src/lib.rs" > "$OUT/coverage-show.txt" 2>&1
  log "coverage: $(tail -1 "$OUT/coverage-report.txt")"
}

tsan_outcome() { # a run's log → what happened
  local file=$1
  if grep -q "SIGSEGV" "$file"; then
    echo "crashed (SIGSEGV)"
  else
    echo "$(grep -cE 'WARNING: ThreadSanitizer' "$file") reports; $(tail -1 "$file")"
  fi
}

# Moves the crash reports a sanitizer run left into the results, as tools/s5-measure.sh does.
collect_crash_reports() {
  local report
  for report in "$HOME/Library/Logs/DiagnosticReports/"s9_race-*.ips; do
    [ -e "$report" ] || continue
    mv "$report" "$OUT/" && log "tsan: moved $(basename "$report") into the results"
  done
}

step_tsan() {
  local flags="-Zsanitizer=thread"
  log "tsan: the clean test"
  (cd "$ROOT" && RUSTFLAGS="$flags" RUSTDOCFLAGS="$flags" cargo +nightly test -Zbuild-std \
    --target aarch64-apple-darwin --target-dir "$S9/tsan-target" -p s9-race -- the_atomic_count_is_exact) \
    > "$OUT/tsan-clean.txt" 2>&1
  echo "exit $?" >> "$OUT/tsan-clean.txt"
  log "tsan: clean: $(tsan_outcome "$OUT/tsan-clean.txt")"
  log "tsan: the planted race"
  (cd "$ROOT" && RUSTFLAGS="$flags" RUSTDOCFLAGS="$flags" cargo +nightly test -Zbuild-std \
    --target aarch64-apple-darwin --target-dir "$S9/tsan-target" -p s9-race -- --ignored the_planted_race) \
    > "$OUT/tsan-planted.txt" 2>&1
  echo "exit $?" >> "$OUT/tsan-planted.txt"
  log "tsan: planted: $(tsan_outcome "$OUT/tsan-planted.txt")"
  sleep 5 # ReportCrash writes its reports a moment after the crash
  collect_crash_reports
}

step_wpt_setup() {
  if [ ! -d "$WPT/.git" ]; then
    log "wpt-setup: a sparse checkout of WPT at $WPT_REVISION"
    git clone --filter=blob:none --no-checkout --depth 1 https://github.com/web-platform-tests/wpt.git "$WPT" \
      >> "$OUT/wpt-setup.txt" 2>&1 || { log "wpt-setup: clone failed"; return 1; }
  fi
  # `docs` holds commands.json, which ./wpt reads at start (tools/wpt/paths).
  git -C "$WPT" sparse-checkout set tools docs resources common dom interfaces fonts images media \
    >> "$OUT/wpt-setup.txt" 2>&1
  git -C "$WPT" fetch --depth 1 origin "$WPT_REVISION" >> "$OUT/wpt-setup.txt" 2>&1 \
    && git -C "$WPT" checkout -q "$WPT_REVISION" >> "$OUT/wpt-setup.txt" 2>&1 \
    || { log "wpt-setup: checkout of $WPT_REVISION failed"; return 1; }
  log "wpt-setup: at $(git -C "$WPT" rev-parse HEAD); $(du -sh "$WPT" | cut -f1)"
  local version
  version=$(/usr/libexec/PlistBuddy -c 'Print CFBundleShortVersionString' \
    "/Applications/Google Chrome Canary.app/Contents/Info.plist")
  local dir="$S9/chromedriver/$version"
  if [ ! -x "$dir/chromedriver-mac-arm64/chromedriver" ]; then
    log "wpt-setup: chromedriver $version from Chrome for Testing"
    mkdir -p "$dir"
    curl -fsS -o "$dir/chromedriver.zip" \
      "https://storage.googleapis.com/chrome-for-testing-public/$version/mac-arm64/chromedriver-mac-arm64.zip" \
      || { log "wpt-setup: no chromedriver $version published"; return 1; }
    (cd "$dir" && unzip -q chromedriver.zip)
  fi
  log "wpt-setup: $("$dir/chromedriver-mac-arm64/chromedriver" --version)"
  codesign -dv "$dir/chromedriver-mac-arm64/chromedriver" > "$OUT/chromedriver-codesign.txt" 2>&1
}

# Samples, every 2 s until $1 exits, the resident memory of every process
# whose ancestry reaches it, and the system's free memory.
sample_memory() {
  local root=$1 file=$2
  echo -e "time\tprocesses\trss_mb\tfree_percent" > "$file"
  while kill -0 "$root" 2>/dev/null; do
    ps -axo pid=,ppid=,rss= | awk -v root="$root" -v t="$(date +%s)" \
      -v free="$(memory_pressure -Q | awk -F': ' '/free percentage/ {sub("%", "", $2); print $2}')" '
      { parent[$1] = $2; rss[$1] = $3 }
      END {
        n = 0; total = 0
        for (p in parent) {
          q = p; depth = 0
          while (q != "" && q != root && q != 0 && q != 1 && depth < 64) { q = parent[q]; depth++ }
          if (q == root) { n++; total += rss[p] }
        }
        printf "%s\t%d\t%.1f\t%s\n", t, n, total / 1024, free
      }' >> "$file"
    sleep 2
  done
}

step_wpt() {
  local version dir driver
  version=$(/usr/libexec/PlistBuddy -c 'Print CFBundleShortVersionString' \
    "/Applications/Google Chrome Canary.app/Contents/Info.plist")
  driver="$S9/chromedriver/$version/chromedriver-mac-arm64/chromedriver"
  [ -x "$driver" ] || { log "wpt: run wpt-setup first (no chromedriver for Canary $version)"; return 1; }
  log "wpt: $WPT_DIR against Chrome Canary $version, headless"
  echo "python: $("$WPT_PYTHON" --version 2>&1) ($(command -v "$WPT_PYTHON"))" >> "$OUT/environment.txt"
  (
    cd "$WPT" || exit 1
    exec /usr/bin/time -l "$WPT_PYTHON" ./wpt run --channel canary --binary "$CANARY" --webdriver-binary "$driver" \
      --headless --processes 1 --no-pause-after-test --no-fail-on-unexpected --no-install-fonts \
      --log-wptreport "$OUT/wptreport.json" --log-mach "$OUT/wpt-mach.txt" \
      chrome "$WPT_DIR"
  ) > "$OUT/wpt-run.txt" 2>&1 &
  local pid=$!
  sample_memory "$pid" "$OUT/wpt-memory.tsv" &
  local sampler=$!
  wait "$pid"
  echo "exit $?" >> "$OUT/wpt-run.txt"
  wait "$sampler" 2>/dev/null
  log "wpt: $(tail -1 "$OUT/wpt-run.txt"); peak $(awk -F'\t' 'NR > 1 && $3 > m {m = $3} END {print m}' "$OUT/wpt-memory.tsv") MB resident, lowest free $(awk -F'\t' 'NR > 1 && (min == "" || $4 < min) {min = $4} END {print min}' "$OUT/wpt-memory.tsv")%"
}

record_environment
for step in "${steps[@]}"; do
  case $step in
    setup) step_setup ;;
    fuzz) step_fuzz ;;
    coverage) step_coverage ;;
    tsan) step_tsan ;;
    wpt-setup) step_wpt_setup ;;
    wpt) step_wpt ;;
  esac
done
log "results: $OUT"
