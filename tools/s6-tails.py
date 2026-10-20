#!/usr/bin/env python3
"""Count spike S6's frames over its thresholds, from a run's per-frame traces.

    python3 -I tools/s6-tails.py target/s6-results/<timestamp> [more directories]

For each run of S6-Q1 and S6-Q3, it reads the `.trace.tsv` files that
tools/s6-measure.sh writes and prints:
  - typical updates over 2 ms on the compositor thread, overall and by the
    number of tiles drawn, and over 4 ms on the GPU (S6-Q1's thresholds);
  - the surfaces allocated because no spare was free, against tiles drawn,
    for typical updates and full redraws;
  - scrolling frames that only moved layers, over 1 ms (S6-Q3's threshold),
    and the frames that drew a newly exposed row, with the frames that grew
    the surface pool;
  - animation frames over 1 ms.

Percentiles interpolate linearly between ranks. Times are in milliseconds.
"""

import argparse
import csv
import statistics
import sys
from pathlib import Path

# Trace columns: series, frame, tiles, allocated, gpu-ms, cpu-ms, wall-ms.
SERIES, TILES, ALLOCATED, GPU_MS, CPU_MS = 0, 2, 3, 4, 5


def load(path):
    with open(path, newline="") as f:
        return [r for r in csv.reader(f, delimiter="\t") if r and not r[0].startswith("#")]


def pct(xs, q):
    xs = sorted(xs)
    k = (len(xs) - 1) * q
    f = int(k)
    c = min(f + 1, len(xs) - 1)
    return xs[f] + (xs[c] - xs[f]) * (k - f)


def q1(directory, run):
    rows = load(directory / f"q1-run{run}.trace.tsv")
    updates = [r for r in rows if r[SERIES] == "typical-update"]
    full = [r for r in rows if r[SERIES] == "full-redraw"]
    cpu = [float(r[CPU_MS]) for r in updates]
    gpu = [float(r[GPU_MS]) for r in updates]
    by_tiles = {}
    for r in updates:
        by_tiles.setdefault(int(r[TILES]), []).append(float(r[CPU_MS]))
    print(
        f"  S6-Q1 run {run}: typical updates over 2 ms {sum(c > 2 for c in cpu)} of {len(cpu)}, "
        f"over 4 ms on the GPU {sum(g > 4 for g in gpu)}; compositor median {statistics.median(cpu):.3f}, "
        f"p95 {pct(cpu, 0.95):.3f}"
    )
    for tiles, times in sorted(by_tiles.items()):
        print(
            f"    {tiles} tiles: {len(times)} updates, median {statistics.median(times):.2f}, "
            f"over 2 ms {sum(t > 2 for t in times)}"
        )
    for name, series in (("typical updates", updates), ("full redraws", full)):
        drawn = sum(int(r[TILES]) for r in series)
        new = sum(int(r[ALLOCATED]) for r in series)
        print(f"    {name}: {new} new surfaces in {drawn} tile draws ({100 * new / max(drawn, 1):.1f}%)")


def q3(directory, run, motion):
    frames = [r for r in load(directory / f"q3-{motion}-run{run}.trace.tsv") if r[SERIES] == motion]
    layers = [float(r[CPU_MS]) for r in frames if int(r[TILES]) == 0]
    rows = [float(r[CPU_MS]) for r in frames if int(r[TILES]) > 0]
    growing = [float(r[CPU_MS]) for r in frames if int(r[ALLOCATED]) > 0]
    line = (
        f"  S6-Q3 {motion} run {run}: {len(layers)} frames moved layers only, over 1 ms "
        f"{sum(t > 1 for t in layers)}; median {statistics.median(layers):.3f}, p95 {pct(layers, 0.95):.3f}, "
        f"max {max(layers):.3f}"
    )
    print(line)
    if rows:
        print(
            f"    {len(rows)} frames drew a newly exposed row: median {statistics.median(rows):.2f}, "
            f"p95 {pct(rows, 0.95):.2f}, max {max(rows):.2f}; "
            f"{len(growing)} grew the surface pool: {', '.join(f'{t:.2f}' for t in growing)}"
        )


def main():
    parser = argparse.ArgumentParser(
        description=__doc__.split("\n\n")[0],
        epilog=__doc__.split("\n\n", 1)[1],
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    parser.add_argument("directories", nargs="+", type=Path, help="S6 results directories")
    args = parser.parse_args()
    for directory in args.directories:
        if not (directory / "q1-run1.trace.tsv").is_file():
            sys.exit(f"{directory}: no S6 traces (q1-run1.trace.tsv) here")
        print(directory)
        runs = sorted(int(p.name[len("q1-run"):-len(".trace.tsv")]) for p in directory.glob("q1-run*.trace.tsv"))
        for run in runs:
            q1(directory, run)
        for run in runs:
            for motion in ("scroll", "animate"):
                q3(directory, run, motion)


if __name__ == "__main__":
    main()
