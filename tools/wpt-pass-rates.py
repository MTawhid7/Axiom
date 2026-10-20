#!/usr/bin/env python3
"""Pass rates of WPT directories, from wpt.fyi's run summaries.

    python3 -I tools/wpt-pass-rates.py LABEL=SUMMARY.json.gz [...] --dirs DIR [DIR ...]

Each SUMMARY is a run's `*-summary_v2.json.gz`, as wpt.fyi's runs API lists
it in `results_url`, downloaded beforehand, gzipped or not. For each
directory prefix (such as `/html/syntax/parsing/`), it prints the share of
results that pass in each run, counted as wpt.fyi counts them: a test's `c` field holds [passing,
total], the subtests of a testharness test, or the test itself for reference
and crash tests. It also prints the median of the runs labelled chrome,
firefox and safari, which the standards baseline's SB11 takes as a completed
area's target.

Summaries are untrusted data: run with -I, and keep them in their own
directory under target/.
"""

import argparse
import gzip
import json
import statistics
import sys

SHIPPING = ("chrome", "firefox", "safari")


def load(path):
    # wpt.fyi stores the summaries gzipped, but its storage may decompress
    # them on the way, so the file's first bytes decide.
    with open(path, "rb") as f:
        compressed = f.read(2) == b"\x1f\x8b"
    opener = gzip.open if compressed else open
    with opener(path, "rt", encoding="utf-8") as f:
        data = json.load(f)
    return {k: v for k, v in data.items() if k.startswith("/") and isinstance(v, dict)}


def rate(summary, prefix):
    passing = total = tests = 0
    for test, entry in summary.items():
        if test.startswith(prefix):
            counts = entry.get("c") or [0, 0]
            passing += counts[0]
            total += counts[1]
            tests += 1
    return passing, total, tests


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0], epilog=__doc__.split("\n\n", 1)[1],
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("runs", nargs="+", help="LABEL=path to a summary_v2.json(.gz)")
    parser.add_argument("--dirs", nargs="+", required=True, help="directory prefixes, such as /html/syntax/parsing/")
    args = parser.parse_args()
    runs = []
    for spec in args.runs:
        label, _, path = spec.partition("=")
        if not path:
            sys.exit(f"bad run {spec!r}: expected LABEL=PATH")
        runs.append((label, load(path)))
    labels = [label for label, _ in runs]
    print("| Directory | Tests | " + " | ".join(labels) + " | Median of Chrome, Firefox, Safari |")
    print("|---|---|" + "---|" * len(labels) + "---|")
    for prefix in args.dirs:
        cells, shipping, tests = [], [], 0
        for label, summary in runs:
            passing, total, count = rate(summary, prefix)
            tests = max(tests, count)
            share = 100 * passing / total if total else None
            cells.append(f"{share:.1f}%" if share is not None else "—")
            if label in SHIPPING and share is not None:
                shipping.append(share)
        median = f"{statistics.median(shipping):.1f}%" if len(shipping) == len(SHIPPING) else "—"
        print(f"| `{prefix}` | {tests} | " + " | ".join(cells) + f" | {median} |")


if __name__ == "__main__":
    main()
