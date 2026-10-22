#!/usr/bin/env bash
# Spike S1 memory and CPU work, measured per process by the kernel.
#
# Usage: tools/s1-memory.sh [--runs N] [--cooldown SECONDS]
#   --runs N            repetitions of each measurement (default 3)
#   --cooldown SECONDS  idle time before the measurements start (default 30)
#
# Each input is written once as a script file (`s1-probe gen`). Then every
# measurement is a fresh process, `s1-probe run <none|a|b> <file>`, under
# /usr/bin/time -l, which reports:
#   - maximum resident set size;
#   - peak memory footprint (what Activity Monitor calls Memory);
#   - instructions retired and cycles elapsed. Instructions do not depend on
#     clock speed, so they are robust to thermal throttling.
# `none` only parses the script. Subtracting it leaves what running the
# scenario through a pattern's driver cost.
#
# Inputs:
#   - the four CP2 workloads at Large size;
#   - tree-keep at 25k, 100k and 400k elements: the tree generator with every
#     id kept, so every node stays live. The slope between sizes gives bytes
#     per node, free of fixed costs.
#
# Writes target/s1-results/memory-<timestamp>/:
#   environment.txt, inputs/*.script, raw.tsv, summary.md
# Written for bash 3.2 (macOS /bin/bash). The summary uses python3 -I
# (standard library only), which the Command Line Tools provide.
set -uo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd)
cd "$ROOT" || exit 2

runs=3 cooldown=30
while [ $# -gt 0 ]; do
  case $1 in
    --runs) runs=$2; shift 2 ;;
    --cooldown) cooldown=$2; shift 2 ;;
    -h | --help) sed -n '2,27p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
    *) echo "unknown argument: $1 (see --help)" >&2; exit 2 ;;
  esac
done

cargo build --manifest-path "$ROOT/spikes/Cargo.toml" --release -p s1-bench --bin s1-probe >/dev/null 2>&1 || {
  echo "build failed: cargo build --release -p s1-bench --bin s1-probe" >&2
  exit 1
}
probe="$ROOT/target/release/s1-probe"
out="$ROOT/target/s1-results/memory-$(date +%Y%m%d-%H%M%S)"
mkdir -p "$out/inputs"

{
  echo "date:     $(date '+%Y-%m-%d %H:%M:%S %Z')"
  echo "git:      $(git rev-parse --short HEAD)$([ -z "$(git status --porcelain)" ] || echo ' (uncommitted changes present)')"
  echo "rustc:    $(rustc -V)"
  echo "machine:  $(sysctl -n machdep.cpu.brand_string), $(($(sysctl -n hw.memsize) / 1073741824)) GiB, $(sw_vers -productName) $(sw_vers -productVersion)"
  echo "runs:     $runs"
} | tee "$out/environment.txt"

inputs="tree:large listeners:large churn:large pressure:large tree-keep:25000 tree-keep:100000 tree-keep:400000"
for input in $inputs; do
  name=${input%%:*} size=${input#*:}
  "$probe" gen "$name" "$size" >"$out/inputs/$name-$size.script" || exit 1
done

sleep "$cooldown"
echo "load $(sysctl -n vm.loadavg), thermal: $(pmset -g therm 2>/dev/null | grep -Ei 'warning|limit' | tr -s ' \n' ' ')" | tee -a "$out/environment.txt"

raw="$out/raw.tsv"
printf 'input\tpattern\trun\twall_s\tmax_rss\tpeak_footprint\tinstructions\tcycles\tprobe\n' >"$raw"
i=1
while [ "$i" -le "$runs" ]; do
  for input in $inputs; do
    name=${input%%:*} size=${input#*:}
    file="$out/inputs/$name-$size.script"
    for pattern in none a b; do
      timing="$out/time.tmp"
      line=$(/usr/bin/time -l -o "$timing" "$probe" run "$pattern" "$file") || {
        echo "failed: s1-probe run $pattern $file" >&2
        exit 1
      }
      awk -v input="$name-$size" -v pattern="$pattern" -v run="$i" -v probe="$line" '
        / real /                      { wall = $1 }
        /maximum resident set size/   { rss = $1 }
        /peak memory footprint/       { peak = $1 }
        /instructions retired/        { ins = $1 }
        /cycles elapsed/              { cyc = $1 }
        END {
          gsub("\t", " ", probe)
          printf "%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n", input, pattern, run, wall, rss, peak, ins, cyc, probe
        }' "$timing" >>"$raw"
    done
  done
  i=$((i + 1))
done
rm -f "$out/time.tmp"

python3 -I - "$raw" >"$out/summary.md" <<'PY'
import collections, statistics, sys

rows = collections.defaultdict(list)
probe = {}
with open(sys.argv[1], encoding="utf-8") as f:
    next(f)
    for line in f:
        input_, pattern, run, wall, rss, peak, ins, cyc, text = line.rstrip("\n").split("\t")
        rows[(input_, pattern)].append((float(wall), int(rss), int(peak), int(ins), int(cyc)))
        probe[(input_, pattern)] = dict(kv.split("=", 1) for kv in text.split()[1:] if "=" in kv)

def med(input_, pattern, i):
    return statistics.median(r[i] for r in rows[(input_, pattern)])

def spread(input_, pattern, i):
    values = [r[i] for r in rows[(input_, pattern)]]
    m = statistics.median(values)
    return (max(values) - min(values)) / m if m else 0.0

mib = lambda b: b / 1048576
inputs = list(dict.fromkeys(k[0] for k in rows))
runs = len(next(iter(rows.values())))
print(f"Runs per measurement: {runs}. Medians; `none` (parse only) is subtracted from A and B.\n")
print("| Input | Δ peak footprint A | Δ peak footprint B | B/A | Δ max RSS A | Δ max RSS B | B/A | Δ instructions A | Δ instructions B | B/A |")
print("|---|---|---|---|---|---|---|---|---|---|")
for input_ in inputs:
    cells = [input_]
    for i, unit in ((2, "MiB"), (1, "MiB"), (3, "G")):
        base = med(input_, "none", i)
        da, db = med(input_, "a", i) - base, med(input_, "b", i) - base
        fmt = (lambda v: f"{mib(v):.1f} MiB") if unit == "MiB" else (lambda v: f"{v / 1e9:.2f} G")
        cells += [fmt(da), fmt(db), f"{db / da:.2f}" if da else "—"]
    print("| " + " | ".join(cells) + " |")

print("\nRun-to-run spread (range / median) of the raw values, worst case per input:\n")
for input_ in inputs:
    worst = max(spread(input_, p, i) for p in ("none", "a", "b") for i in (1, 2, 3))
    print(f"- {input_}: {100 * worst:.1f}%")

print("\nBytes per live node (tree-keep; every node live):\n")
print("| Input | Nodes | A: Δ footprint / node | B: Δ footprint / node |")
print("|---|---|---|---|")
keep = sorted((k for k in inputs if k.startswith("tree-keep")), key=lambda k: int(k.split("-")[-1]))
per = {}
for input_ in keep:
    nodes = int(probe[(input_, "a")]["peak_live_nodes"])
    base = med(input_, "none", 2)
    da, db = med(input_, "a", 2) - base, med(input_, "b", 2) - base
    per[input_] = (nodes, da, db)
    print(f"| {input_} | {nodes} | {da / nodes:.0f} B | {db / nodes:.0f} B |")
if len(keep) >= 2:
    (n0, a0, b0), (n1, a1, b1) = per[keep[-2]], per[keep[-1]]
    print(f"\nSlope between {keep[-2]} and {keep[-1]} (fixed costs cancel): "
          f"A {(a1 - a0) / (n1 - n0):.0f} B/node, B {(b1 - b0) / (n1 - n0):.0f} B/node.")

print("\nWall time and the patterns' own accounting (median wall; probe facts from the last run):\n")
for input_ in inputs:
    for pattern in ("a", "b"):
        facts = probe[(input_, pattern)]
        print(f"- {input_} {pattern.upper()}: wall {med(input_, pattern, 0):.3f} s; "
              + ", ".join(f"{k}={v}" for k, v in facts.items() if k != "outcome"))
PY
cat "$out/summary.md"
echo
echo "results: ${out#"$ROOT"/}"
