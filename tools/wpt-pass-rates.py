#!/usr/bin/env python3
"""Pass rates of WPT directories, from wpt.fyi's run summaries.

    python3 -I tools/wpt-pass-rates.py LABEL=SUMMARY.json.gz [...] --dirs DIR [DIR ...]
                                       [--manifest MANIFEST.json --types reftest [...]]

Each SUMMARY is a run's `*-summary_v2.json.gz`, as wpt.fyi's runs API lists
it in `results_url`, downloaded beforehand, gzipped or not. For each
directory prefix (such as `/html/syntax/parsing/`), it prints the share of
results that pass in each run, counted as wpt.fyi counts them: a testharness
test's `c` field holds [passing, total] over its subtests; a reference or
crash test has `c` [0, 0], and counts once, by its status `s`. (Before
2026-10-20 this tool read only `c`, and so left reference tests out.) It also
prints the median of the runs labelled chrome,
firefox and safari, which the standards baseline's SB11 takes as a completed
area's target.

With --manifest (WPT's MANIFEST.json, as `./wpt manifest` writes it) and
--types, only tests of those types count: for example `--types reftest`
for an engine that can't run script yet.

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


def manifest_tests(path, types):
    """The URLs of a WPT manifest's tests of the given types (manifest version 8: a trie of path segments)."""
    with open(path, encoding="utf-8") as f:
        items = json.load(f).get("items", {})
    urls = set()
    for kind in types:
        stack = [("", items.get(kind, {}))]
        while stack:
            prefix, node = stack.pop()
            for name, value in node.items():
                path_here = f"{prefix}/{name}"
                if isinstance(value, dict):
                    stack.append((path_here, value))
                    continue
                for entry in value[1:]:  # [hash, [url or None, ...extras], ...]
                    url = entry[0] or path_here
                    urls.add(url if url.startswith("/") else "/" + url)
    return urls


def counts(entry):
    """[passing, total] for one test, as wpt.fyi counts it.

    A testharness test's `c` holds its subtests. Reference and crash tests
    have no subtests: their `c` is [0, 0], and their status `s` says whether
    they passed ("P", or "O" for a harness that ran clean).
    """
    c = entry.get("c") or [0, 0]
    if c[1]:
        return c
    return [1, 1] if entry.get("s") in ("P", "O") else [0, 1]


def rate(summary, prefix, only=None):
    passing = total = tests = 0
    for test, entry in summary.items():
        if test.startswith(prefix) and (only is None or test in only):
            p, t = counts(entry)
            passing += p
            total += t
            tests += 1
    return passing, total, tests


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0], epilog=__doc__.split("\n\n", 1)[1],
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("runs", nargs="+", help="LABEL=path to a summary_v2.json(.gz)")
    parser.add_argument("--dirs", nargs="+", required=True, help="directory prefixes, such as /html/syntax/parsing/")
    parser.add_argument("--manifest", help="WPT's MANIFEST.json, to count only the tests of --types")
    parser.add_argument("--types", nargs="+", default=["reftest"],
                        help="test types counted with --manifest (default: reftest)")
    args = parser.parse_args()
    only = manifest_tests(args.manifest, args.types) if args.manifest else None
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
            passing, total, count = rate(summary, prefix, only)
            tests = max(tests, count)
            share = 100 * passing / total if total else None
            cells.append(f"{share:.1f}%" if share is not None else "—")
            if label in SHIPPING and share is not None:
                shipping.append(share)
        median = f"{statistics.median(shipping):.1f}%" if len(shipping) == len(SHIPPING) else "—"
        print(f"| `{prefix}` | {tests} | " + " | ".join(cells) + f" | {median} |")


if __name__ == "__main__":
    main()
