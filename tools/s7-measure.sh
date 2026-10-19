#!/usr/bin/env bash
# Spike S7 measurements: every S7 question, against a signed release build.
#
# Usage: tools/s7-measure.sh [--runs N] [--cooldown SECONDS] [--count N]
#                            [--megabytes N] [--idle SECONDS] [--allow-battery] [--quick]
#   --runs N            runs of each measurement (default 3), each after a cool-down
#   --cooldown SECONDS  idle time before each run (default 30)
#   --count N           handshakes in each full and each resumed series (default 20)
#   --megabytes N       the download's size (default 50)
#   --idle SECONDS      S7-Q4's idle window (default 30)
#   --allow-battery     run without AC power, or with Low Power Mode on
#   --quick             a short pass for checking the script
#
# Network access, authorized at Step 5's Gate B (2026-10-18): read-only requests
# to public test hosts (badssl.com's test hosts, example.com, www.wikipedia.org,
# www.apple.com, www.cloudflare.com, github.com, speed.cloudflare.com and
# cloudflare-quic.com), and DNS queries through the system's resolver. With the
# defaults each run downloads about 2 x 50 MB, plus small pages. Nothing on the
# Mac is reconfigured, and no certificate is added to a keychain.
#
# Builds the release binary and signs a copy ad hoc with the hardened runtime,
# as Axiom's helpers would be (ADR-0004 D2). Each run, after a cool-down:
#   S7-Q1  verify: 18 hosts with rustls (verified through Security) and with
#          Network framework, under the network profile; rustls unsandboxed, for
#          comparison; probe
#   S7-Q2  handshakes: --count full and resumed, per stack, to
#          www.cloudflare.com and www.apple.com; download: --megabytes per
#          stack, after a warm-up; ready: the idle helper's footprint; startup:
#          one-time costs, stage by stage
#   S7-Q3  dns: addresses and HTTPS records
#   S7-Q4  one-wait: everything from one kevent64 wait, then --idle seconds idle
# Once, after the runs:
#   removal tests: each rule of profiles/network.sb taken out in turn, and the
#   failure it causes checked; then reporting runs under a profile that allows
#   everything and logs it, which list what the helper attempts, so that every
#   denial under the real profile can be explained (the kernel doesn't log some
#   denials, such as file-read-metadata ones).
# Writes target/s7-results/<timestamp>/, with summary.txt.
# Written for bash 3.2 (macOS /bin/bash).
set -uo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd)
cd "$ROOT" || exit 2

runs=3 cooldown=30 count=20 megabytes=50 idle=30 battery="" quick=""
while [ $# -gt 0 ]; do
  case $1 in
    --runs) runs=$2; shift 2 ;;
    --cooldown) cooldown=$2; shift 2 ;;
    --count) count=$2; shift 2 ;;
    --megabytes) megabytes=$2; shift 2 ;;
    --idle) idle=$2; shift 2 ;;
    --allow-battery) battery=yes; shift ;;
    --quick) quick=yes; shift ;;
    -h | --help) sed -n '2,39p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
    *) echo "unknown argument: $1 (see --help)" >&2; exit 2 ;;
  esac
done
if [ -n "$quick" ]; then runs=1 cooldown=0 count=3 megabytes=5 idle=3; fi

power=$(pmset -g batt | head -1)
lpm=$(pmset -g | awk '/lowpowermode/ { print $2 }')
if [ -z "$battery" ] && { ! printf '%s' "$power" | grep -q "AC Power" || [ "${lpm:-0}" != 0 ]; }; then
  echo "not on AC power with Low Power Mode off ($power; lowpowermode ${lpm:-?})." >&2
  echo "Timings would not be comparable; use --allow-battery to run anyway." >&2
  exit 3
fi

cargo build -q --release -p s7-net || { echo "the release build failed" >&2; exit 1; }
bin=target/s7/bin
mkdir -p "$bin"
cp -f target/release/s7-net "$bin/s7-net"
codesign --sign - --force --identifier org.axiom.s7.net --options runtime "$bin/s7-net" 2>/dev/null || {
  echo "signing s7-net failed" >&2
  exit 1
}
app=$bin/s7-net
profile=spikes/s7/net/profiles/network.sb

out="target/s7-results/$(date +%Y%m%d-%H%M%S)"
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
  echo "options: runs=$runs cooldown=${cooldown}s count=$count megabytes=$megabytes idle=${idle}s quick=${quick:-no}"
  flags=$(codesign -dv "$app" 2>&1 | grep -E '^CodeDirectory' | sed 's/.*flags=//; s/ hashes.*//')
  sdk=$(otool -l "$app" | awk '/LC_BUILD_VERSION/ { f = 1 } f && /sdk/ { print $2; exit }')
  echo "s7-net: flags=$flags sdk=$sdk"
  echo "s7-net links: $(otool -L "$app" | tail -n +2 | awk '{ print $1 }' | tr '\n' ' ')"
} >"$out/environment.txt"

failed=0
# run NAME ARGS...: one broker run; its output to NAME.txt.
run() {
  local name=$1
  shift
  if ! "$app" "$@" >"$out/$name.txt" 2>&1; then
    failed=1
    echo "   FAILED: $app $* (see $out/$name.txt)"
  fi
}

bytes=$((megabytes * 1000000))
for r in $(seq 1 "$runs"); do
  echo "== run $r of $runs, after a ${cooldown}s cool-down"
  sleep "$cooldown"
  run "q1-verify-rustls-run$r" verify --stack rustls
  run "q1-verify-nw-run$r" verify --stack nw
  run "q1-verify-unsandboxed-run$r" --profile none verify --stack rustls
  run "q1-probe-run$r" probe --home "$HOME"
  for stack in rustls nw; do
    run "q2-handshakes-$stack-cloudflare-run$r" handshakes --stack "$stack" \
      --host www.cloudflare.com --path /cdn-cgi/trace --count "$count"
    run "q2-handshakes-$stack-apple-run$r" handshakes --stack "$stack" --host www.apple.com --count "$count"
    run "q2-download-$stack-run$r" download --stack "$stack" \
      --url "https://speed.cloudflare.com/__down?bytes=$bytes"
  done
  run "q2-ready-run$r" ready
  run "q2-startup-run$r" startup
  run "q3-dns-run$r" dns
  run "q4-one-wait-run$r" one-wait --idle "$idle"
done
end_main=$(date '+%Y-%m-%d %H:%M:%S')

# --- Removal tests: each rule out in turn; the failure it should cause --------
echo "== removal tests"
mkdir -p "$out/profiles"
# variant NAME SED-SCRIPT: profiles/rm-NAME.sb, network.sb edited by SED-SCRIPT.
variant() { sed "$2" "$profile" >"$out/profiles/rm-$1.sb"; }
variant trustd-agent '/^(allow mach-lookup (global-name "com.apple.trustd.agent"))/d'
variant executable-dir '/^(allow file-read-metadata (literal (param "EXECUTABLE_DIR")))/d'
variant var '/^(allow file-read-metadata (literal "\/var"))/d'
variant mdnsresponder '/^(allow network-outbound (literal "\/private\/var\/run\/mDNSResponder"))/d'
variant tcp 's/^(allow network-outbound (remote tcp) (remote udp))/(allow network-outbound (remote udp))/'
variant udp 's/^(allow network-outbound (remote tcp) (remote udp))/(allow network-outbound (remote tcp))/'
variant data-dir '/^(allow file-read\* file-write\* (subpath (param "DATA_DIR")))/d'
# removal NAME EXPECTED ARGS...: run under rm-NAME.sb; PASS if EXPECTED appears.
removal() {
  local name=$1 expected=$2
  shift 2
  local file="$out/profiles/rm-$name.sb"
  if cmp -s "$file" "$profile"; then
    printf 'rm-%s\tNOT-REMOVED\tthe rule was not found in network.sb\n' "$name" >>"$out/q1-removal.txt"
    failed=1
    return
  fi
  "$app" --profile "$file" "$@" >"$out/q1-removal-$name.txt" 2>&1
  if grep -q -- "$expected" "$out/q1-removal-$name.txt"; then
    printf 'rm-%s\tPASS\t%s\n' "$name" "$(grep -m1 -- "$expected" "$out/q1-removal-$name.txt" | cut -c1-200)" >>"$out/q1-removal.txt"
  else
    printf 'rm-%s\tFAIL\texpected %s\n' "$name" "$expected" >>"$out/q1-removal.txt"
    failed=1
  fi
}
: >"$out/q1-removal.txt"
removal trustd-agent "-26276" verify example.com
removal executable-dir "SecPolicyCreateSSL returned NULL" verify example.com
removal var "DNSServiceCreateConnection: -65563" dns example.com
removal mdnsresponder "DNSServiceCreateConnection: -65563" dns example.com
removal tcp "connect: Operation not permitted" verify example.com
removal udp "udp: Operation not permitted" one-wait --idle 1 --messages 5
removal data-dir "data-directory write and read	denied" probe --home "$HOME"

# Denials logged under the real profile, during the runs and the removals.
end_removal=$(date '+%Y-%m-%d %H:%M:%S')
/usr/bin/log show --start "$start" --end "$end_main" --style compact \
  --predicate 'sender == "Sandbox" AND eventMessage CONTAINS "s7-net"' >"$out/sandbox-denials.txt" 2>&1 || true

# --- Reporting runs: everything allowed and logged ---------------------------
echo "== reporting runs"
printf '(version 1)\n(allow default (with report))\n' >"$out/profiles/report-all.sb"
report_start=$(date '+%Y-%m-%d %H:%M:%S')
"$app" --profile "$out/profiles/report-all.sb" verify --stack rustls >"$out/q1-report-verify-rustls.txt" 2>&1
"$app" --profile "$out/profiles/report-all.sb" verify --stack nw >"$out/q1-report-verify-nw.txt" 2>&1
"$app" --profile "$out/profiles/report-all.sb" dns >"$out/q1-report-dns.txt" 2>&1
"$app" --profile "$out/profiles/report-all.sb" one-wait --idle 2 >"$out/q1-report-one-wait.txt" 2>&1
sleep 3 # the log is written asynchronously
/usr/bin/log show --start "$report_start" --style compact \
  --predicate 'sender == "Sandbox" AND eventMessage CONTAINS "s7-net"' 2>&1 |
  grep ' allow ' | sed 's/.*Sandbox: s7-net([0-9]*) //' | sed "s|$HOME|~|g" |
  sort | uniq -c | sort -rn >"$out/q1-attempted.txt"

# Crash reports, if any helper crashed (the system writes them asynchronously).
sleep 2
mkdir -p "$out/crash-reports"
find "$HOME/Library/Logs/DiagnosticReports" -maxdepth 2 -name 's7-net*.ips' -newer "$out/environment.txt" \
  -exec mv {} "$out/crash-reports/" \;

# --- Summary -----------------------------------------------------------------
# median FILE... reads one number per line on standard input.
median() {
  sort -n | awk '{ v[NR] = $1 } END { if (NR == 0) print "-"; else if (NR % 2) print v[(NR + 1) / 2]; else printf "%.3f\n", (v[NR / 2] + v[NR / 2 + 1]) / 2 }'
}
{
  echo "S7 results, $start ($(grep '^commit:' "$out/environment.txt"), at the start)"
  echo
  echo "S7-Q1: verdicts per run (verified/rejected/failed; unexpected outcomes)"
  for f in "$out"/q1-verify-*-run*.txt; do
    printf '  %s\t%s/%s/%s\t%s\n' "$(basename "$f" .txt)" \
      "$(grep -c $'\tverified\t' "$f")" "$(grep -c $'\trejected\t' "$f")" "$(grep -c $'\tfailed\t' "$f")" \
      "$(grep -h '^# verify:' "$f" | sed 's/^# verify: //')"
  done
  echo "  trust evaluation, ms (median over every rustls host and run, sandboxed):" \
    "$(cat "$out"/q1-verify-rustls-run*.txt | awk -F'\t' '$1 == "verify" { print $7 }' | tr ',' '\n' | grep -v '^$' | median)"
  echo "  probe (run 1):"
  grep -h '^probe' "$out/q1-probe-run1.txt" | cut -f2,3 | sed 's/^/    /'
  echo "  removal tests:"
  sed 's/^/    /' "$out/q1-removal.txt"
  echo "  denials logged under the real profile, by kind:"
  grep -v '^Timestamp' "$out/sandbox-denials.txt" | sed 's/.*Sandbox: s7-net([0-9]*) //' | sed "s|$HOME|~|g" |
    sort | uniq -c | sort -rn | sed 's/^/    /'
  echo
  echo "S7-Q2: handshakes, medians over runs (excluding each process's first connection and the warm-up):"
  echo "  stack/host/kind: start-to-handshake-ms tls-ms cpu-us instructions"
  for stack in rustls nw; do
    for host in cloudflare apple; do
      for kind in full resumed; do
        files=$(ls "$out"/q2-handshakes-$stack-$host-run*.txt 2>/dev/null)
        [ -n "$files" ] || continue
        rows=$(cat $files | awk -F'\t' -v k="$kind" '$1 == "handshakes" && $4 == k && !(k == "full" && $5 == 0)')
        printf '  %s/%s/%s: %s %s %s %s\n' "$stack" "$host" "$kind" \
          "$(printf '%s\n' "$rows" | awk -F'\t' '$8 != "-" { print $8 }' | median)" \
          "$(printf '%s\n' "$rows" | awk -F'\t' '$9 != "-" { print $9 }' | median)" \
          "$(printf '%s\n' "$rows" | awk -F'\t' '{ print $13 }' | median)" \
          "$(printf '%s\n' "$rows" | awk -F'\t' '{ print $14 }' | median)"
      done
    done
  done
  echo "  rustls handshake kinds seen: $(cat "$out"/q2-handshakes-rustls-*-run*.txt | awk -F'\t' '$1 == "handshakes" { print $4 "=" $12 }' | sort | uniq -c | tr '\n' ' ')"
  echo "  downloads (bytes wall-ms cpu-ms cpu-ms-per-MB instructions/byte cycles/byte MB/s):"
  grep -h '^download	' "$out"/q2-download-*-run*.txt | cut -f2,4-10 | sed 's/^/    /'
  echo "  startup stages (cpu-us instructions cycles detail), run 1:"
  grep -h '^startup	' "$out/q2-startup-run1.txt" | cut -f2- | cut -c1-200 | sed 's/^/    /'
  echo "  ready footprint (bytes):"
  grep -h '^ready-usage' "$out"/q2-ready-run*.txt | sed 's/.*\tfootprint=\([0-9]*\).*/\1/' | sed 's/^/    /'
  echo
  echo "S7-Q3: per run"
  grep -h '^dns-loop' "$out"/q3-dns-run*.txt | sed 's/^/  /'
  echo "  first answers, ms (median over names and runs):" \
    "$(cat "$out"/q3-dns-run*.txt | awk -F'\t' '$1 == "dns" { print $4 }' | median)"
  echo
  echo "S7-Q4: per run"
  for f in "$out"/q4-one-wait-run*.txt; do
    echo "  $(basename "$f" .txt)"
    grep -h '^one-wait	' "$f" | cut -f2- | cut -c1-240 | sed 's/^/    /'
  done
  echo
  echo "Attempted operations (reporting runs, everything allowed): q1-attempted.txt"
  echo "Crash reports: $(find "$out/crash-reports" -name '*.ips' | wc -l | tr -d ' ')"
} >"$out/summary.txt"

echo "results in $out"
exit "$failed"
