#!/usr/bin/env bash
# Spike S10: the GPU/compositor process that the architecture review adds
# (RV1-02, RV1-09): its hop, zero-copy resources and display-list validation
# (docs/research/s10-results.md).
#
# Usage: tools/s10-measure.sh [--runs N] [--cooldown SECONDS] [--frames N]
#                             [--allow-battery] [--quick]
#   --runs N            runs of each measurement (default 3), each after a cool-down
#   --cooldown SECONDS  idle time before each run (default 30)
#   --frames N          frames in each hop run (default 600: 10 s at 60 Hz)
#   --allow-battery     run without AC power, or with Low Power Mode on
#   --quick             a short pass for checking the script
#
# Builds the release binary and signs a copy ad hoc with the hardened runtime,
# as Axiom's helpers would be (ADR-0004 D2); the helpers run that copy too.
# Each run, after a cool-down:
#   S10-Q3  validate: lists of 1,000, 10,000 and 44,372 items decoded and
#           validated in one process, and every single-byte change of a
#           1,000-item list
#   S10-Q1  hop: --frames frames at 60 Hz under the real profiles (gpu.sb and
#           renderer.sb), then unsandboxed, for comparison
#   S10-Q2  resources: 16 images of 1,024 x 1,024 by each path: no-copy, blit,
#           and iosurface (the old design's renderer, renderer-iosurface.sb)
#   start-up: a cold shader compile (a new nonce), then a warm start
# Once, after the runs:
#   removal tests: each rule of profiles/gpu.sb taken out in turn, and the
#   failure it causes checked. Rules whose loss only disables Metal's caches
#   are checked on the second start under the variant, after a warm-up under
#   the real profile, since every start rewrites the shared cache
#   reporting runs: everything allowed and logged, warm and cold, listing
#   what the GPU process attempts, so every denial under the real profile can
#   be explained (the kernel doesn't log some denials)
# Crash reports the runs leave (two removals cause crashes) are moved into the
# results. Each cold start adds an entry to Metal's per-user shader cache,
# which Metal bounds and manages; its size is recorded before and after.
# Writes target/s10-results/<timestamp>/, with summary.txt.
# Written for bash 3.2 (macOS /bin/bash).
set -uo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd)
cd "$ROOT" || exit 2

runs=3 cooldown=30 frames=600 battery="" quick=""
while [ $# -gt 0 ]; do
  case $1 in
    --runs) runs=$2; shift 2 ;;
    --cooldown) cooldown=$2; shift 2 ;;
    --frames) frames=$2; shift 2 ;;
    --allow-battery) battery=yes; shift ;;
    --quick) quick=yes; shift ;;
    -h | --help) sed -n '2,/^set -uo/p' "$0" | sed '$d' | sed 's/^# \{0,1\}//'; exit 0 ;;
    *) echo "unknown argument: $1 (see --help)" >&2; exit 2 ;;
  esac
done
if [ -n "$quick" ]; then runs=1 cooldown=0 frames=120; fi

power=$(pmset -g batt | head -1)
lpm=$(pmset -g | awk '/lowpowermode/ { print $2 }')
if [ -z "$battery" ] && { ! printf '%s' "$power" | grep -q "AC Power" || [ "${lpm:-0}" != 0 ]; }; then
  echo "not on AC power with Low Power Mode off ($power; lowpowermode ${lpm:-?})." >&2
  echo "Timings would not be comparable; use --allow-battery to run anyway." >&2
  exit 3
fi

cargo build -q --release -p s10-gpu || { echo "the release build failed" >&2; exit 1; }
bin=target/s10/bin
mkdir -p "$bin"
cp -f target/release/s10 "$bin/s10"
codesign --sign - --force --identifier org.axiom.s10 --options runtime "$bin/s10" 2>/dev/null || {
  echo "signing s10 failed" >&2
  exit 1
}
app=$bin/s10
profile=spikes/s10/gpu/profiles/gpu.sb
metal_cache="$(getconf DARWIN_USER_CACHE_DIR)com.apple.metal"

out="target/s10-results/$(date +%Y%m%d-%H%M%S)"
mkdir -p "$out/crash-reports" "$out/profiles"
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
  echo "options: runs=$runs cooldown=${cooldown}s frames=$frames quick=${quick:-no}"
  flags=$(codesign -dv "$app" 2>&1 | grep -E '^CodeDirectory' | sed 's/.*flags=//; s/ hashes.*//')
  echo "s10: flags=$flags"
  echo "Metal's shader cache at the start: $(du -sk "$metal_cache" 2>/dev/null | cut -f1) KiB"
} >"$out/environment.txt"

failed=0
# run NAME ARGS...: one browser run; its output to NAME.txt.
run() {
  local name=$1
  shift
  if ! "$app" "$@" >"$out/$name.txt" 2>&1; then
    failed=1
    echo "   FAILED: $app $* (see $out/$name.txt)"
  fi
}
# Crash reports newer than MARKER, moved into the results with PREFIX.
collect_crashes() {
  local marker=$1 prefix=$2 report
  sleep 5 # the system writes them asynchronously
  for report in $(find "$HOME/Library/Logs/DiagnosticReports" -maxdepth 1 -name 's10-*.ips' -newer "$marker"); do
    mv "$report" "$out/crash-reports/$prefix-$(basename "$report")"
  done
}

nonce_base=$(date +%s)
for r in $(seq 1 "$runs"); do
  echo "== run $r of $runs, after a ${cooldown}s cool-down"
  sleep "$cooldown"
  echo "run $r: load average $(sysctl -n vm.loadavg)" >>"$out/loads.txt"
  run "q3-validate-run$r" validate --out "$out/q3-run$r"
  run "q1-hop-run$r" hop --frames "$frames" --out "$out/q1-hop-run$r"
  run "q1-hop-none-run$r" hop --frames "$frames" --gpu-profile none --renderer-profile none \
    --out "$out/q1-hop-none-run$r"
  for path in no-copy blit iosurface; do
    run "q2-$path-run$r" resources --path "$path" --out "$out/q2-run$r"
  done
  S10_SHADER_NONCE="$nonce_base-$r" run "startup-cold-run$r" hop --frames 30
  S10_SHADER_NONCE="$nonce_base-$r" run "startup-warm-run$r" hop --frames 30
done
end_main=$(date '+%Y-%m-%d %H:%M:%S')
collect_crashes "$out/environment.txt" runs

# --- Removal tests: each rule out in turn; the failure it should cause --------
echo "== removal tests"
# variant NAME SED-SCRIPT: profiles/rm-NAME.sb, gpu.sb edited by SED-SCRIPT.
variant() { sed "$2" "$profile" >"$out/profiles/rm-$1.sb"; }
variant rendezvous '/^(allow mach-lookup (global-name (param "RENDEZVOUS")))/d'
variant executable-dir '/^(allow file-read-metadata (literal (param "EXECUTABLE_DIR")))/d'
variant agx-client '/^  (iokit-user-client-class "AGXDeviceUserClient")/d'
variant iosurface-client 's/^  (iokit-user-client-class "IOSurfaceRootUserClient"))/  )/'
variant agx-bundle '/^(allow file-read\* (regex #"^\/System\/Library\/Extensions\/AGXMetal/d'
variant compiler-hash '/^(allow file-read-data$/,/agxa_compiler_hash/d'
variant compiler-service '/^(allow mach-lookup (xpc-service-name "com.apple.MTLCompilerService"))/d'
variant metal-cache '/^(allow file-read\* file-write\* (subpath (param "METAL_CACHE")))/d'
variant var 's/^  (literal "\/var")$//'
variant user-cache-dir 's/^  (literal (param "USER_CACHE_DIR")))/  )/'
variant memsize '/^(allow sysctl-read (sysctl-name "hw.memsize"))/d'
: >"$out/q1-removal.txt"
# result NAME PASS|FAIL DETAIL
result() { printf 'rm-%s\t%s\t%s\n' "$1" "$2" "$3" >>"$out/q1-removal.txt"; [ "$2" = PASS ] || failed=1; }
# removed NAME: whether the variant differs from gpu.sb (else it is a FAIL).
removed() {
  cmp -s "$out/profiles/rm-$1.sb" "$profile" || return 0
  result "$1" FAIL "the rule was not found in gpu.sb"
  return 1
}
# removal NAME EXPECTED [NONCE]: one start under the variant; PASS if its
# output contains EXPECTED.
removal() {
  local name=$1 expected=$2 nonce=${3:-}
  removed "$name" || return
  env ${nonce:+S10_SHADER_NONCE=$nonce} "$app" hop --frames 30 --gpu-profile "file:$out/profiles/rm-$name.sb" \
    >"$out/q1-removal-$name.txt" 2>&1
  if grep -q -- "$expected" "$out/q1-removal-$name.txt"; then
    result "$name" PASS "$(grep -m1 -- "$expected" "$out/q1-removal-$name.txt" | cut -c1-200)"
  else
    result "$name" FAIL "expected $expected"
  fi
}
# crash NAME SYMBOL: one start under the variant; PASS if the GPU process
# left a crash report whose crashing thread names SYMBOL.
crash() {
  local name=$1 symbol=$2 marker="$out/profiles/rm-$1.sb" report
  removed "$name" || return
  touch "$marker"
  "$app" hop --frames 30 --gpu-profile "file:$out/profiles/rm-$name.sb" >"$out/q1-removal-$name.txt" 2>&1
  collect_crashes "$marker" "rm-$name"
  report=$(grep -l -- "$symbol" "$out/crash-reports/rm-$name-"*.ips 2>/dev/null | head -1)
  if [ -n "$report" ]; then
    result "$name" PASS "crashed in $symbol ($(basename "$report"))"
  else
    result "$name" FAIL "expected a crash in $symbol"
  fi
}
# ready FILE: "LIBRARY PIPELINES", the start-up stages in ms.
ready() { sed -n 's/.*shader library \([0-9.]*\), pipelines \([0-9.]*\)).*/\1 \2/p' "$1"; }
# uncached NAME: a warm-up start under gpu.sb, then two under the variant; PASS
# if the second still compiles (shader library >= 50 ms or pipelines >= 10 ms)
# where gpu.sb's own warm start doesn't.
uncached() {
  local name=$1 base second
  removed "$name" || return
  "$app" hop --frames 30 >"$out/q1-removal-$name-warmup.txt" 2>&1
  "$app" hop --frames 30 >"$out/q1-removal-$name-base.txt" 2>&1
  "$app" hop --frames 30 --gpu-profile "file:$out/profiles/rm-$name.sb" >"$out/q1-removal-$name-1.txt" 2>&1
  "$app" hop --frames 30 --gpu-profile "file:$out/profiles/rm-$name.sb" >"$out/q1-removal-$name.txt" 2>&1
  base=$(ready "$out/q1-removal-$name-base.txt")
  second=$(ready "$out/q1-removal-$name.txt")
  if [ -n "$base" ] && [ -n "$second" ] && echo "$base $second" | awk '{ exit !($1 < 50 && $2 < 10 && ($3 >= 50 || $4 >= 10)) }'; then
    result "$name" PASS "second start compiles: library ${second% *} ms, pipelines ${second#* } ms (gpu.sb: ${base% *}, ${base#* })"
  else
    result "$name" FAIL "second start: '${second:-no start}' (gpu.sb: '${base:-no start}')"
  fi
}
removal rendezvous "look_up: Mach error"
crash executable-dir __CFCheckCFInfoPACSignature
removal agx-client "no Metal device"
removal iosurface-client "IOSurfaceCreate failed"
removal agx-bundle "no Metal device"
removal compiler-service "Unable to reach MTLCompilerService" "$nonce_base-removal"
crash memsize AGXG13GFamilyDevice
uncached compiler-hash
uncached metal-cache
uncached var
uncached user-cache-dir
end_removal=$(date '+%Y-%m-%d %H:%M:%S')

# --- Reporting runs: everything allowed and logged ---------------------------
echo "== reporting runs"
report_start=$(date '+%Y-%m-%d %H:%M:%S')
"$app" hop --frames 60 --gpu-profile report >"$out/q1-report-warm.txt" 2>&1
report_mid=$(date '+%Y-%m-%d %H:%M:%S')
S10_SHADER_NONCE="$nonce_base-report" "$app" hop --frames 60 --gpu-profile report \
  >"$out/q1-report-cold.txt" 2>&1
sleep 3 # the log is written asynchronously
# pids ROLE FILES...: the PIDs a run's "processes:" line gives ROLE.
pids() {
  local role=$1
  shift
  sed -n "s/^processes: browser \([0-9]*\), GPU process \([0-9]*\), renderer \([0-9]*\)/\1 \2 \3/p" "$@" |
    awk -v role="$role" '{ print (role == "gpu" ? $2 : $3) }'
}
# attempted PID START END: what that process did, by kind.
attempted() {
  /usr/bin/log show --start "$2" --end "$3" --style compact \
    --predicate "sender == \"Sandbox\" AND eventMessage CONTAINS \"s10($1)\"" 2>&1 |
    grep ' allow ' | sed 's/.*Sandbox: s10([0-9]*) //' | sed "s|$HOME|~|g" | sort | uniq -c | sort -rn
}
attempted "$(pids gpu "$out/q1-report-warm.txt")" "$report_start" "$report_mid" >"$out/q1-attempted-warm.txt"
attempted "$(pids gpu "$out/q1-report-cold.txt")" "$report_mid" "$(date '+%Y-%m-%d %H:%M:%S')" \
  >"$out/q1-attempted-cold.txt"

# Denials logged under the real profiles during the runs, by process.
/usr/bin/log show --start "$start" --end "$end_main" --style compact \
  --predicate 'sender == "Sandbox" AND eventMessage CONTAINS "s10("' >"$out/sandbox-denials.txt" 2>&1 || true
# denials ROLE: the denials of that role's processes, by kind.
denials() {
  local list
  list=$(pids "$1" "$out"/q1-hop-run*.txt "$out"/q2-*-run*.txt | tr '\n' ' ')
  grep ' deny' "$out/sandbox-denials.txt" | awk -v pids=" $list" '{
    if (match($0, /s10\([0-9]+\)/)) { pid = substr($0, RSTART + 4, RLENGTH - 5); if (index(pids, " " pid " ")) print }
  }' | sed 's/.*Sandbox: s10([0-9]*) //' | sed "s|$HOME|~|g" | sort | uniq -c | sort -rn
}
collect_crashes "$out/environment.txt" late
echo "Metal's shader cache at the end: $(du -sk "$metal_cache" 2>/dev/null | cut -f1) KiB" >>"$out/environment.txt"

# --- Summary -----------------------------------------------------------------
# pct P: the P-th percentile (nearest rank) of the numbers on standard input.
pct() {
  sort -g | awk -v p="$1" '{ v[NR] = $1 } END {
    if (NR == 0) { print "-"; exit }
    i = int((NR * p + 99) / 100); if (i < 1) i = 1; if (i > NR) i = NR
    printf "%.1f\n", v[i] }'
}
# stage A B FILES...: µs from column A to column B of the frames' CSVs, for
# the steady frames (20 onwards).
stage() {
  local a=$1 b=$2
  shift 2
  awk -F, -v a="$a" -v b="$b" 'FNR > 1 && $1 >= 20 { printf "%.3f\n", ($b - $a) / 1000 }' "$@"
}
# table FILES...: each stage's median, p95, p99 and maximum, over every run.
table() {
  local line
  echo "  | Stage | Median µs | p95 µs | p99 µs | Max µs |"
  echo "  |---|---|---|---|---|"
  for line in "offer → renderer|2|3" "renderer: build and encode|3|4" "renderer → GPU process|4|5" \
    "GPU process: decode and validate|5|6" "GPU process: render, wait for the GPU|6|7" \
    "GPU process: build the commit|7|8" "GPU process → browser (the hop)|8|9" \
    "browser: surfaces, contents, commit|9|10" "end to end|2|10"; do
    local name=${line%%|*} rest=${line#*|}
    local a=${rest%%|*} b=${rest#*|} values
    values=$(stage "$a" "$b" "$@")
    printf '  | %s | %s | %s | %s | %s |\n' "$name" "$(echo "$values" | pct 50)" \
      "$(echo "$values" | pct 95)" "$(echo "$values" | pct 99)" "$(echo "$values" | pct 100)"
  done
  local gpu
  gpu=$(awk -F, 'FNR > 1 && $1 >= 20 { printf "%.3f\n", $11 * 1000000 }' "$@")
  printf '  | GPU time (command buffer) | %s | %s | %s | %s |\n' "$(echo "$gpu" | pct 50)" \
    "$(echo "$gpu" | pct 95)" "$(echo "$gpu" | pct 99)" "$(echo "$gpu" | pct 100)"
}
{
  echo "S10 results, $start ($(grep '^commit:' "$out/environment.txt"), at the start)"
  echo "$(grep '^load average' "$out/environment.txt"); per run: $(tr '\n' ';' <"$out/loads.txt")"
  echo
  echo "S10-Q3: validation, per run"
  for f in "$out"/q3-run*/validate.md; do
    echo "  $(basename "$(dirname "$f")"):"
    sed -n '3,$p' "$f" | sed 's/^/    /'
  done
  echo
  for mode in "" "-none"; do
    echo "S10-Q1: the hop, $([ -z "$mode" ] && echo "under gpu.sb and renderer.sb" || echo "unsandboxed"), every run's steady frames"
    table "$out"/q1-hop$mode-run*/hop-frames.csv
    for f in "$out"/q1-hop$mode-run*/hop-summary.md; do
      echo "  $(basename "$(dirname "$f")"):"
      grep -E '^(GPU process ready|per update|CPU per frame|the page|footprints|tile surfaces)' "$f" | sed 's/^/    /'
    done
    echo
  done
  echo "S10-Q2: resources, per path and run"
  for path in no-copy blit iosurface; do
    for f in "$out"/q2-run*/resources-$path.md; do
      echo "  $(basename "$(dirname "$f")") $path:"
      sed -n '2,4p' "$f" | sed 's/^/    /'
    done
  done
  echo
  echo "Start-up (GPU process ready: device, shader library, pipelines)"
  for f in "$out"/startup-*-run*.txt; do
    printf '  %s: %s\n' "$(basename "$f" .txt)" \
      "$(sed -n 's/^GPU process ready (device and pipelines) in \(.*\); linear.*/\1/p' "$f")"
  done
  echo
  echo "Removal tests (each rule of gpu.sb out in turn)"
  sed 's/^/  /' "$out/q1-removal.txt"
  echo
  echo "Denials logged under the real profiles, during the runs"
  echo "  GPU process:"
  denials gpu | sed 's/^/    /'
  echo "  renderer:"
  denials renderer | sed 's/^/    /'
  echo
  echo "Attempted by the GPU process with everything allowed: q1-attempted-warm.txt" \
    "($(wc -l <"$out/q1-attempted-warm.txt" | tr -d ' ') kinds), q1-attempted-cold.txt" \
    "($(wc -l <"$out/q1-attempted-cold.txt" | tr -d ' ') kinds)"
  echo "Crash reports: $(find "$out/crash-reports" -name '*.ips' | wc -l | tr -d ' ')" \
    "($(cd "$out/crash-reports" && ls | tr '\n' ' '))"
  grep "^Metal's shader cache" "$out/environment.txt"
} >"$out/summary.txt"

echo "results in $out"
exit "$failed"
