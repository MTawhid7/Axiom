#!/usr/bin/env bash
# Spike S5 measurements: every S5 question, against signed release builds.
#
# Usage: tools/s5-measure.sh [--runs N] [--cooldown SECONDS] [--mutants N]
#                            [--crash-checks] [--allow-battery] [--quick]
#   --runs N            timed runs (default 3), each after a cool-down
#   --cooldown SECONDS  idle time before each timed run (default 60). The
#                       MacBook Air has no fan, and throttles when hot
#   --mutants N         malformed variants of each seed font for S5-Q3
#                       (default 500)
#   --crash-checks      also run what is expected to crash: S4's unchanged
#                       profiles with ImageIO and with CoreText drawing colour
#                       emoji, and the removal of each of S5's ImageIO rules.
#                       Each crash leaves a report in
#                       ~/Library/Logs/DiagnosticReports; the script moves this
#                       run's reports into the results
#   --allow-battery     run without AC power, or with Low Power Mode on
#                       (timings are then not comparable)
#   --quick             a short pass for checking the script: one run, no
#                       cool-down, 20 mutants, fewer repetitions
#
# Needs target/s5/webfonts (tools/s5-fetch.sh) and target/s5/images
# (tools/s5-images.sh). Builds release binaries and signs copies ad hoc with
# the hardened runtime in target/s5/bin. Then, untimed:
#   S5-Q1  s5-text shape: unsandboxed (sheets, glyph detail), under S4's text
#          profile and S5's, and with HarfRust applying tracking; CoreText's
#          glyphs under each profile compared with unsandboxed, run by run
#   S5-Q2  s5-text raster: unsandboxed with sheets, and under S5's text profile,
#          compared with unsandboxed
#   S5-Q3  s5-webfont on every WPT file, under S4's renderer profile and
#          unsandboxed, against the manifest's expectations; then --mutants
#          variants of each of eight seed fonts, each in its own sandboxed
#          process with a 10 s watchdog. Mutants that crash or hang are kept
#   S5-Q4  s5-text footprint (five helpers per configuration, on the whole
#          corpus and on its Latin line); executable sizes, release and stripped
#   S5-Q5  s5-image: Rust's pixels against ImageIO's; every image's pixels
#          under the renderer profile with S5's rules; ten decodes of each
#          large JPEG per decoder and profile, for determinism; with
#          --crash-checks, each rule removed in turn
# and then, --runs times: s5-text raster-timing (S5-Q2) and speed (S5-Q4), and
# every test image through each decoder in a fresh process: cold, after a
# warm-up decode, and after a warm-up decode inside the profile a renderer would
# use for that decoder (S5-Q5). Last, the kernel's sandbox denial reports.
#
# Writes target/s5-results/<timestamp>/, with summary.txt first. Exits non-zero
# if a command fails unexpectedly.
# Written for bash 3.2 (macOS /bin/bash).
set -uo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd)
cd "$ROOT" || exit 2

runs=3 cooldown=60 mutants=500 crash="" battery="" quick="" helpers=5 repeat=5
while [ $# -gt 0 ]; do
  case $1 in
    --runs) runs=$2; shift 2 ;;
    --cooldown) cooldown=$2; shift 2 ;;
    --mutants) mutants=$2; shift 2 ;;
    --crash-checks) crash=yes; shift ;;
    --allow-battery) battery=yes; shift ;;
    --quick) quick=yes; shift ;;
    -h | --help) sed -n '2,47p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
    *) echo "unknown argument: $1 (see --help)" >&2; exit 2 ;;
  esac
done
if [ -n "$quick" ]; then runs=1 cooldown=0 mutants=20 helpers=1 repeat=1; fi

W=target/s5/webfonts
I=target/s5/images
[ -d "$W/css" ] || { echo "no web fonts: run tools/s5-fetch.sh" >&2; exit 2; }
[ -f "$I/tiny.png" ] || { echo "no test images: run tools/s5-images.sh" >&2; exit 2; }

power=$(pmset -g batt | head -1)
lpm=$(pmset -g | awk '/lowpowermode/ { print $2 }')
if [ -z "$battery" ] && { ! printf '%s' "$power" | grep -q "AC Power" || [ "${lpm:-0}" != 0 ]; }; then
  echo "not on AC power with Low Power Mode off ($power; lowpowermode ${lpm:-?})." >&2
  echo "Timings would not be comparable; use --allow-battery to run anyway." >&2
  exit 3
fi

cargo build --manifest-path "$ROOT/spikes/Cargo.toml" -q --release -p s5-text -p s5-rust-text -p s5-image || {
  echo "the release build failed" >&2
  exit 1
}
bin=target/s5/bin
mkdir -p "$bin"
for binary in s5-text s5-webfont s5-image; do
  cp -f "target/release/$binary" "$bin/$binary"
  codesign --sign - --force --identifier "org.axiom.s5.$binary" --options runtime "$bin/$binary" 2>/dev/null || {
    echo "signing $binary failed" >&2
    exit 1
  }
done
text=$bin/s5-text webfont=$bin/s5-webfont image=$bin/s5-image

out="target/s5-results/$(date +%Y%m%d-%H%M%S)"
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
  echo "options: runs=$runs cooldown=${cooldown}s mutants=$mutants helpers=$helpers repeat=$repeat crash-checks=${crash:-no} quick=${quick:-no}"
  for binary in s5-text s5-webfont s5-image; do
    flags=$(codesign -dv "$bin/$binary" 2>&1 | grep -E '^CodeDirectory' | sed 's/.*flags=//; s/ hashes.*//')
    sdk=$(otool -l "$bin/$binary" | awk '/LC_BUILD_VERSION/ { f = 1 } f && /sdk/ { print $2; exit }')
    echo "$binary: flags=$flags sdk=$sdk"
  done
  echo "s5-webfont links: $(otool -L "$bin/s5-webfont" | tail -n +2 | awk '{ print $1 }' | tr '\n' ' ')"
  echo "web fonts: $(awk '$1 == "commit" { print "WPT " $2; exit }' spikes/s5/corpus/webfonts.txt), $(find "$W" -type f | wc -l | tr -d ' ') files"
  echo "test images (SHA-256):"
  (cd "$I" && shasum -a 256 *.*)
} >"$out/environment.txt"

failed=0
step() { echo "== $*"; }
# check FILE CMD...: run CMD into FILE; a non-zero status is a failure.
check() {
  local file=$1
  shift
  "$@" >"$file" 2>&1
  local code=$?
  if [ $code -ne 0 ]; then
    failed=1
    echo "   FAILED ($code): $* (see $file)"
  fi
  return $code
}
# decoded ARGS...: one s5-image decode; prints "exit N" and the pixels' hash
# and mean, or the decoder's error. Frameworks' messages go to q5-stderr.txt,
# apart from the lines parsed.
decoded() {
  local lines code
  echo "# s5-image decode $*" >>"$out/q5-stderr.txt"
  lines=$("$image" decode "$@" 2>>"$out/q5-stderr.txt")
  code=$?
  printf 'exit %s %s' "$code" "$(printf '%s\n' "$lines" |
    awk -F'\t' '$1 == "decode" { printf "hash %s mean %s", $11, $12 } $1 == "error" { printf "error %s", $4 }')"
}

# --- S5-Q1: shaping ---------------------------------------------------------
step "S5-Q1 shaping"
check "$out/q1-shape-none.txt" "$text" shape --sheets "$out/sheets-q1"
check "$out/q1-shape-none-detail.txt" "$text" shape --detail on
check "$out/q1-shape-s4-text.txt" "$text" shape --profile text
check "$out/q1-shape-s5-text.txt" "$text" shape --profile text-s5
check "$out/q1-shape-harfrust-tracking.txt" "$text" shape --tracking harfrust
# CoreText's font and glyphs (by hash) for each run, keyed by line, size and
# UTF-16 range, against the unsandboxed run.
ct_compare() { # label reference other
  awk -F'\t' -v label="$1" '
    NR == FNR { if ($1 == "shape") ref[$2 "\t" $3 "\t" $5] = $6 "\t" $16; next }
    $1 == "shape" {
      key = $2 "\t" $3 "\t" $5; n++
      if (!(key in ref)) { d++; print "  only here: " key "\t" $6; next }
      if (ref[key] != $6 "\t" $16) { d++; split(ref[key], r, "\t"); print "  differs: " key "\t" r[1] " -> " $6 }
    }
    END { printf "%s: %d runs, %d whose CoreText font or glyphs differ from unsandboxed\n", label, n, d }' "$2" "$3"
}
{
  ct_compare "S4's text profile" "$out/q1-shape-none.txt" "$out/q1-shape-s4-text.txt"
  ct_compare "S5's text profile" "$out/q1-shape-none.txt" "$out/q1-shape-s5-text.txt"
} >"$out/q1-sandbox-compare.txt"

# --- S5-Q2: rasterization ---------------------------------------------------
step "S5-Q2 rasterization"
check "$out/q2-raster-none.txt" "$text" raster --sheets "$out/sheets-q2"
check "$out/q2-raster-s5-text.txt" "$text" raster --profile text-s5
untimed() { grep -v -e '^# timing' -e '^# sandboxed' "$1"; }
if diff <(untimed "$out/q2-raster-none.txt") <(untimed "$out/q2-raster-s5-text.txt") >"$out/q2-sandbox-diff.txt"; then
  echo "identical: every comparison row under S5's text profile matches unsandboxed" >>"$out/q2-sandbox-diff.txt"
fi

# --- S5-Q3: web fonts -------------------------------------------------------
step "S5-Q3 web fonts"
# manifest PROFILE: every display and reject file, one process each.
manifest() {
  local file sha expected lines code outcome got result
  while read -r file sha expected; do
    case $file in "" | "#"* | commit) continue ;; esac
    case $expected in display | reject) ;; *) continue ;; esac
    lines=$("$webfont" check --profile "$1" "$W/$file" 2>&1)
    code=$?
    outcome=$(printf '%s\n' "$lines" | grep -v '^#' | cut -f2)
    case "$code/$outcome" in
      0/accepted) got=display ;;
      0/rejected) got=reject ;;
      124/*) got=HANG ;;
      *) got="CRASH($code)" ;;
    esac
    if [ "$got" = "$expected" ]; then result=OK; else result=MISMATCH; fi
    printf '%s\t%s\t%s\t%s\t%s\n' "$result" "$expected" "$got" "$file" \
      "$(printf '%s\n' "$lines" | grep -v '^#' | cut -f3 | sed 's/; decode .*//')"
  done <spikes/s5/corpus/webfonts.txt
}
manifest renderer >"$out/q3-wpt-renderer.txt"
manifest none >"$out/q3-wpt-none.txt"
if diff "$out/q3-wpt-none.txt" "$out/q3-wpt-renderer.txt" >"$out/q3-wpt-sandbox-diff.txt"; then
  echo "identical: every outcome and render hash under the renderer profile matches unsandboxed" >>"$out/q3-wpt-sandbox-diff.txt"
fi
seeds="css/WOFF2/support/valid-005.woff2
fonts/Scheherazade-Regular.woff
fonts/noto/NotoNaskhArabic-regular.woff2
fonts/noto/NotoSansMongolian-regular.woff2
fonts/hasubi-mono/HasubiMono-Regular.woff2
fonts/noto/NotoSansDevanagari-Regular.ttf
fonts/noto/cjk/NotoSansCJKjp-Regular-subset-halt.otf
fonts/noto/color-emoji/NotoColorEmoji-subset.ttf"
: >"$out/q3-mutants.txt"
index=0
for seed in $seeds; do
  index=$((index + 1))
  dir="$out/mutants/$(basename "$seed")"
  "$webfont" mutate --seed "$index" --count "$mutants" --out "$dir" "$W/$seed" || failed=1
  for mutant in "$dir"/*; do
    lines=$("$webfont" check --profile renderer --timeout-ms 10000 "$mutant" 2>&1)
    code=$?
    outcome=$(printf '%s\n' "$lines" | grep -v '^#' | cut -f2)
    case "$code/$outcome" in
      0/accepted | 0/rejected)
        printf '%s\t%s\t%s\n' "$outcome" "$(basename "$seed")" "$(basename "$mutant")"
        rm -f "$mutant"
        ;;
      124/*) printf 'HANG\t%s\t%s\n' "$(basename "$seed")" "$(basename "$mutant")" ;;
      *) printf 'CRASH(%s)\t%s\t%s\t%s\n' "$code" "$(basename "$seed")" "$(basename "$mutant")" "$(printf '%s' "$lines" | tail -3 | tr '\n' ' ')" ;;
    esac
  done >>"$out/q3-mutants.txt"
  rmdir "$dir" 2>/dev/null
done
rmdir "$out/mutants" 2>/dev/null

# --- S5-Q4: footprint and size ----------------------------------------------
step "S5-Q4 footprint and size"
check "$out/q4-footprint.txt" "$text" footprint --runs "$helpers"
{
  echo "# size	binary	release-bytes	stripped-bytes"
  stripped="$out/stripped"
  mkdir -p "$stripped"
  for binary in s5-size-base s5-size-harfrust s5-size-raster s5-size-icu-rules s5-size-icu-auto s5-size-icu-all s5-webfont s5-text s5-image; do
    strip -o "$stripped/$binary" "target/release/$binary" 2>/dev/null
    printf 'size\t%s\t%s\t%s\n' "$binary" "$(stat -f %z "target/release/$binary")" "$(stat -f %z "$stripped/$binary")"
  done
  rm -rf "$stripped"
} >"$out/q4-sizes.txt"

# --- S5-Q5: images, untimed -------------------------------------------------
step "S5-Q5 images: pixels and profiles"
check "$out/q5-compare.txt" "$image" compare "$I"/photo.* "$I"/landscape.* "$I"/pattern.*
{
  echo "# sandbox	file	decoder	profile	result"
  for name in photo landscape pattern; do
    for ext in png jpg gif webp avif; do
      for config in "imageio none" "imageio renderer+imageio" "imageio renderer+avif" "rust none" "rust renderer"; do
        set -- $config
        [ "$1" = rust ] && [ "$ext" = avif ] && continue
        printf 'sandbox\t%s\t%s\t%s\t%s\n' "$name.$ext" "$1" "$2" \
          "$(decoded --decoder "$1" --profile "$2" --repeat 1 "$I/$name.$ext")"
      done
    done
  done
} >"$out/q5-sandbox.txt"
# Does a decoder give the same pixels every time? Ten processes for each large
# JPEG, decoder and profile.
{
  echo "# determinism	file	decoder	profile	hashes (first four hex digits), one per process"
  for file in landscape.jpg pattern.jpg; do
    for config in "imageio none" "imageio renderer+imageio" "imageio renderer+avif" "rust none" "rust renderer"; do
      set -- $config
      hashes=""
      for i in 1 2 3 4 5 6 7 8 9 10; do
        hashes="$hashes $(decoded --decoder "$1" --profile "$2" --repeat 1 "$I/$file" | awk '{ for (i = 1; i < NF; i++) if ($i == "hash") print substr($(i + 1), 1, 4) }')"
      done
      printf 'determinism\t%s\t%s\t%s\t%s\n' "$file" "$1" "$2" "$hashes"
    done
  done
} >"$out/q5-determinism.txt"
if [ -n "$crash" ]; then
  # S4's renderer profile plus R1-R6, then each rule removed in turn, then S4's
  # renderer profile alone.
  mkdir -p "$out/profiles"
  rules=$(cat spikes/s5/image/profiles/imageio.sb spikes/s5/image/profiles/avif.sb | grep '^(allow')
  count=$(printf '%s\n' "$rules" | wc -l | tr -d ' ')
  {
    echo "# removal	profile	file	result"
    for i in $(seq 0 "$count"); do
      if [ "$i" = 0 ]; then
        profile="$out/profiles/renderer+R1-R$count.sb" kept=$rules
      else
        profile="$out/profiles/renderer+R1-R$count-without-R$i.sb"
        kept=$(printf '%s\n' "$rules" | sed "${i}d")
      fi
      { cat spikes/s4/sandbox/profiles/renderer.sb; printf '%s\n' "$kept"; } >"$profile"
      for file in photo.avif photo.png landscape.jpg; do
        printf 'removal\t%s\t%s\t%s\n' "$(basename "$profile")" "$file" \
          "$(decoded --decoder imageio --profile "$profile" --repeat 1 "$I/$file")"
      done
    done
    for file in photo.avif photo.png landscape.jpg; do
      printf 'removal\t%s\t%s\t%s\n' "renderer (S4, unchanged)" "$file" \
        "$(decoded --decoder imageio --profile renderer --repeat 1 "$I/$file")"
    done
  } >"$out/q5-removal.txt"
  # CoreText drawing colour emoji under S4's unchanged text profile.
  "$text" work --config coretext --profile text >"$out/q1-crash-check-s4-text.txt" 2>&1
  echo "# exit $?" >>"$out/q1-crash-check-s4-text.txt"
fi

# --- Timed runs -------------------------------------------------------------
: >"$out/q2-timing.txt"
: >"$out/q4-speed.txt"
echo "# phase	run	decode	file	decoder	format	size	median-us	us-per-megapixel	decode-us	convert-us	peak-added-bytes	hash	mean-rgba" >"$out/q5-decode.txt"
for run in $(seq 1 "$runs"); do
  step "timed run $run of $runs, after a ${cooldown}s cool-down"
  sleep "$cooldown"
  echo "# run $run" >>"$out/q2-timing.txt"
  "$text" raster-timing --repeat "$repeat" >>"$out/q2-timing.txt" 2>&1 || failed=1
  echo "# run $run" >>"$out/q4-speed.txt"
  "$text" speed --repeat "$repeat" >>"$out/q4-speed.txt" 2>&1 || failed=1
  for name in photo landscape pattern; do
    for ext in png jpg gif webp avif; do
      for engine in rust imageio imageio-lazy; do
        [ "$engine" = rust ] && [ "$ext" = avif ] && continue
        # cold: a fresh process; warm: after a warm-up decode; sandboxed: warm,
        # inside the profile a renderer would use for that decoder.
        for phase in cold warm sandboxed; do
          options=""
          [ "$phase" != cold ] && options="--warmup $I/tiny.$ext"
          if [ "$phase" = sandboxed ]; then
            case "$engine/$ext" in
              rust/*) options="$options --profile renderer" ;;
              */avif) options="$options --profile renderer+avif" ;;
              *) options="$options --profile renderer+imageio" ;;
            esac
          fi
          echo "# s5-image decode --decoder $engine $options $I/$name.$ext" >>"$out/q5-stderr.txt"
          # shellcheck disable=SC2086
          "$image" decode --decoder "$engine" --repeat "$repeat" $options "$I/$name.$ext" 2>>"$out/q5-stderr.txt" |
            grep -v '^#' | sed "s/^/$phase	$run	/" >>"$out/q5-decode.txt" || failed=1
        done
      done
    done
  done
done

# --- Crash reports, denials and summary -------------------------------------
if [ -n "$crash" ]; then
  sleep 5 # crash reports are written asynchronously
  mkdir -p "$out/crash-reports"
  # The system later moves reports into Retired/.
  find "$HOME/Library/Logs/DiagnosticReports" -maxdepth 2 -name 's5-*.ips' -newer "$out/environment.txt" \
    -exec mv {} "$out/crash-reports/" \;
fi
/usr/bin/log show --start "$start" --style compact \
  --predicate 'sender == "Sandbox" AND (eventMessage CONTAINS "s5-text" OR eventMessage CONTAINS "s5-image" OR eventMessage CONTAINS "s5-webfont")' \
  >"$out/sandbox-denials.txt" 2>&1 || true

# For q5-decode.txt: the median over runs of each process's median total time,
# and of the peak memory it added, for each phase, image and decoder.
q5_medians() {
  awk -F'\t' '
    function median(values, key, count,   i, j, v, tmp) {
      for (i = 1; i <= count; i++) v[i] = values[key, i] + 0
      for (i = 2; i <= count; i++) {
        tmp = v[i]; j = i - 1
        while (j >= 1 && v[j] > tmp) { v[j + 1] = v[j]; j-- }
        v[j + 1] = tmp
      }
      return v[int((count + 1) / 2)]
    }
    $1 !~ /^#/ && $3 == "decode" {
      file = $4; sub(".*/", "", file)
      key = $1 "\t" file "\t" $5
      n[key]++; time[key, n[key]] = $8; peak[key, n[key]] = $12
    }
    END { for (key in n) printf "%s\t%d us\t%d bytes\n", key, median(time, key, n[key]), median(peak, key, n[key]) }' \
    "$out/q5-decode.txt" | LC_ALL=C sort
}
{
  echo "S5 results, $start ($(grep '^commit:' "$out/environment.txt"), at the start)"
  echo
  echo "S5-Q1 shaping (verdicts per run, Axiom's tracking unless noted)"
  for f in none s4-text s5-text harfrust-tracking; do
    printf '  %-18s %s\n' "$f" "$(grep '^# summary' "$out/q1-shape-$f.txt" | cut -c12-)"
  done
  grep -v -e '^  differs' -e '^  only here' "$out/q1-sandbox-compare.txt" | sed 's/^/  /'
  [ -f "$out/q1-crash-check-s4-text.txt" ] && echo "  CoreText drawing colour emoji under S4's text profile: $(tail -1 "$out/q1-crash-check-s4-text.txt")"
  echo
  echo "S5-Q2 rasterization"
  echo "  under S5's text profile: $(tail -1 "$out/q2-sandbox-diff.txt" | cut -c1-90)"
  grep '^# mean' "$out/q2-raster-none.txt" | grep -e 'zeno vs CoreText	' -e 'vello_cpu vs CoreText	' | sed 's/^# mean /  /'
  grep '^# colour' "$out/q2-raster-none.txt" | sed 's/^# /  /' | cut -f1-2,4
  grep '^# timing' "$out/q2-timing.txt" | sed 's/^# /  /'
  echo
  echo "S5-Q3 web fonts"
  for p in renderer none; do
    printf '  WPT, %-9s %s\n' "$p:" "$(cut -f1 "$out/q3-wpt-$p.txt" | sort | uniq -c | tr -s ' ' | tr '\n' ' ')"
  done
  echo "  renderer against unsandboxed: $(tail -1 "$out/q3-wpt-sandbox-diff.txt" | cut -c1-90)"
  echo "  mutants: $(cut -f1 "$out/q3-mutants.txt" | sed 's/(.*//' | sort | uniq -c | tr -s ' ' | tr '\n' ' ')"
  echo
  echo "S5-Q4 footprint (median of $helpers helpers: current, then lifetime maximum, bytes)"
  for config in none coretext-latin harfrust-coretext-latin rust-latin coretext harfrust-coretext rust rust-icu; do
    printf '  %-18s %s\n' "$config" "$(awk -F'\t' -v c="$config" '$1 == "footprint" && $2 == c { print $4 "\t" $5 }' "$out/q4-footprint.txt" |
      sort -n | awk -F'\t' '{ a[NR] = $1; b[NR] = $2 } END { if (NR) print a[int((NR + 1) / 2)] "  " b[int((NR + 1) / 2)] }')"
  done
  grep '^# speed' "$out/q4-speed.txt" | sed 's/^# speed/ /'
  grep '^size' "$out/q4-sizes.txt" | sed 's/^size/ /'
  echo
  echo "S5-Q5 images: median over runs of each process's median total time, and of the peak memory it added"
  q5_medians | sed 's/^/  /'
  echo "  determinism (hashes over ten processes):"
  grep -v '^#' "$out/q5-determinism.txt" | sed 's/^determinism/   /'
  echo "  sandboxed decodes:"
  grep -v '^#' "$out/q5-sandbox.txt" | sed 's/^sandbox/   /'
  [ -f "$out/q5-removal.txt" ] && grep -v '^#' "$out/q5-removal.txt" | sed 's/^removal/   /'
  echo
  echo "sandbox denials logged: $(grep -c 'deny' "$out/sandbox-denials.txt")"
  [ -d "$out/crash-reports" ] && echo "crash reports moved into the results: $(find "$out/crash-reports" -name '*.ips' | wc -l | tr -d ' ')"
  echo "load average at end: $(sysctl -n vm.loadavg)"
} >"$out/summary.txt"

echo "results in $out"
exit "$failed"
