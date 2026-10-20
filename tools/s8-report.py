#!/usr/bin/env python3
"""Tables for spike S8's results, from its records.

    python3 -I tools/s8-report.py target/s8-results/<timestamp>-auto [...] [--web-features data.json]

Reads the census's per-run records (auto-*.json, hand-*.json) and prints, as
Markdown tables, each page's medians with their ranges across runs (MB are
10^6 bytes, as B1 counts them):
  - memory: every Chrome process together (footprint's total, shared memory
    counted once), the renderers, the GPU and browser processes, the same
    browser on about:blank, and the change over blank, leaving out Chrome's
    component-extension renderers, which exit before the steady state;
  - load: navigation milestones, requests and bytes, DOM size;
  - the 30 s without input: CPU time, idle wakeups and energy impact;
  - features: S8-Q1's groups, and the page's web-features identifiers.
With --web-features (the web-features project's data.json), each identifier
gets its Baseline status: high (widely available), low (newly available) or
none (limited availability). Runs from several directories are pooled. A run
whose page wasn't visible throughout is left out and named, since a hidden
page isn't composited and doesn't take input.
"""

import argparse
import glob
import json
import os
import statistics
import sys

sys.dont_write_bytecode = True
sys.path.insert(0, os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "spikes", "s8"))

import usecounters  # noqa: E402

MB = 10**6  # as B1 and the S6 results count megabytes


def visible_throughout(record):
    """A run counts only if its page stayed visible: hidden pages aren't composited and don't take input."""
    if "valid" in record:
        return record["valid"]
    if "mode" in record:  # the census by hand has no DOM samples
        return True
    samples = [get(record, k, "visibility") for k in ("dom_loaded", "dom_after")]
    return all(v in (None, "visible") for v in samples)


def load(directories, include_hidden=False):
    pages, left_out = {}, []
    for directory in directories:
        for path in sorted(glob.glob(os.path.join(directory, "*-run*.json"))):
            with open(path) as f:
                record = json.load(f)
            if not include_hidden and not visible_throughout(record):
                left_out.append(os.path.relpath(path))
                continue
            pages.setdefault(record["page"], []).append(record)
    return pages, left_out


def get(record, *keys, default=None):
    for key in keys:
        if not isinstance(record, dict) or key not in record:
            return default
        record = record[key]
    return record


def cell(values, scale=1.0, digits=0):
    values = [v / scale for v in values if isinstance(v, (int, float))]
    if not values:
        return "—"
    median = statistics.median(values)
    if len(values) == 1:
        return f"{median:.{digits}f}"
    return f"{median:.{digits}f} ({min(values):.{digits}f}–{max(values):.{digits}f})"


def kind_sum(record, phase, kinds):
    by_kind = get(record, phase, "by_kind", default={})
    return sum(v.get("footprint", 0) for k, v in by_kind.items() if k in kinds)


def without_extensions(record, phase):
    """A phase's total without Chrome's component-extension renderers.

    They run on about:blank and exit about a minute after launch, before the
    steady state, so a page's cost is compared without them on both sides.
    footprint's total counts shared memory once; their own footprints are
    taken off it, an approximation that their private memory dominates.
    """
    return get(record, phase, "total_footprint", default=0) - kind_sum(record, phase, ("extension",))


def idle_sum(record, field):
    return sum(v.get(field, 0) for v in get(record, "without_input", "by_kind", default={}).values())


def memory_table(pages):
    print("| Page | All processes, loaded (MB) | After interaction | After 30 s without input | Renderers, loaded | GPU, loaded | Browser, loaded | On about:blank | Loaded − blank, without extension renderers | After 30 s − blank, without extension renderers | GPU after 30 s |")
    print("|---|---|---|---|---|---|---|---|---|---|---|")
    for page, runs in pages.items():
        total = lambda phase: [get(r, phase, "total_footprint") for r in runs]  # noqa: E731
        print("| " + " | ".join([
            page,
            cell(total("memory_loaded"), MB),
            cell(total("memory_after_interaction"), MB),
            cell(total("memory_after"), MB),
            cell([kind_sum(r, "memory_loaded", ("renderer",)) for r in runs], MB),
            cell([kind_sum(r, "memory_loaded", ("gpu",)) for r in runs], MB),
            cell([kind_sum(r, "memory_loaded", ("browser",)) for r in runs], MB),
            cell(total("memory_blank"), MB),
            cell([without_extensions(r, "memory_loaded") - without_extensions(r, "memory_blank")
                  for r in runs if get(r, "memory_blank")], MB),
            cell([without_extensions(r, "memory_after") - without_extensions(r, "memory_blank")
                  for r in runs if get(r, "memory_blank")], MB),
            cell([kind_sum(r, "memory_after", ("gpu",)) for r in runs], MB),
        ]) + " |")


def load_table(pages):
    print("| Page | First contentful paint (ms) | Largest contentful paint (ms) | Load event (ms) | Wait ended by | Requests | Transferred (MB) | Hosts | Elements | DOM depth | Frames (targets) |")
    print("|---|---|---|---|---|---|---|---|---|---|---|")
    for page, runs in pages.items():
        ended = sorted({get(r, "navigation", "ended_by", default="?") for r in runs})
        print("| " + " | ".join([
            page,
            cell([get(r, "timing", "firstContentfulPaint") for r in runs]),
            cell([get(r, "timing", "largestContentfulPaint") for r in runs]),
            cell([get(r, "timing", "load") for r in runs]),
            ", ".join(ended),
            cell([get(r, "network", "requests") for r in runs]),
            cell([get(r, "network", "bytes") for r in runs], MB, 2),
            cell([get(r, "network", "hosts") for r in runs]),
            cell([get(r, "dom_loaded", "elements") for r in runs]),
            cell([get(r, "dom_loaded", "depth") for r in runs]),
            cell([get(r, "targets", "iframe", default=0) for r in runs]),
        ]) + " |")


def wakeups_counted(record):
    """False when top reported no idle wakeup for any process while they used CPU: its counter failed."""
    return idle_sum(record, "idle_wakeups") > 0 or idle_sum(record, "cpu_seconds") < 0.2


def idle_table(pages):
    print("| Page | CPU time over 30 s (s) | Idle wakeups | Energy impact (sum of processes' means) | Renderers' energy impact |")
    print("|---|---|---|---|---|")
    for page, runs in pages.items():
        uncounted = [str(r.get("run")) for r in runs if not wakeups_counted(r)]
        wakeups = cell([idle_sum(r, "idle_wakeups") for r in runs if wakeups_counted(r)])
        if uncounted:
            wakeups += f"; run {', '.join(uncounted)} left out: no wakeup counted for any process"
        print("| " + " | ".join([
            page,
            cell([idle_sum(r, "cpu_seconds") for r in runs], digits=2),
            wakeups,
            cell([idle_sum(r, "energy_impact") for r in runs], digits=1),
            cell([get(r, "without_input", "by_kind", "renderer", "energy_impact") for r in runs], digits=1),
        ]) + " |")


def baseline_status(ident, features):
    entry = features.get(ident)
    while entry and entry.get("kind") == "moved":
        entry = features.get(entry.get("redirect_target"))
    if not entry or "status" not in entry:
        return "unknown"
    status = entry["status"].get("baseline")
    return {"high": "high", "low": "low"}.get(status, "none")


def feature_table(pages, features):
    print("| Page | Blink features | CSS properties | S8-Q1 groups | web-features IDs | Baseline high / low / none / unknown |")
    print("|---|---|---|---|---|---|")
    union = {}
    for page, runs in pages.items():
        blink = sorted({f for r in runs for f in r.get("features", [])})
        css = {c for r in runs for c in r.get("css_properties", [])}
        ids = sorted({i for r in runs for i in r.get("web_features", [])})
        for ident in ids:
            union.setdefault(ident, set()).add(page)
        counts = {"high": 0, "low": 0, "none": 0, "unknown": 0}
        for ident in ids:
            counts[baseline_status(ident, features) if features else "unknown"] += 1
        groups = ", ".join(f"{g} ({len(v)})" for g, v in usecounters.groups(blink).items())
        print(f"| {page} | {len(blink)} | {len(css)} | {groups or '—'} | {len(ids)} | "
              f"{counts['high']} / {counts['low']} / {counts['none']} / {counts['unknown']} |")
    return union


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0], epilog=__doc__.split("\n\n", 1)[1],
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("directories", nargs="+", help="S8 results directories, whose runs are pooled")
    parser.add_argument("--web-features", help="web-features' data.json, for Baseline status")
    parser.add_argument("--include-hidden", action="store_true", help="keep runs whose page wasn't visible throughout")
    args = parser.parse_args()
    pages, left_out = load(args.directories, args.include_hidden)
    if not pages:
        sys.exit("no S8 records (*-run*.json) in those directories")
    for path in left_out:
        print(f"Left out, the page wasn't visible throughout: `{path}`")
    print("\nRuns: " + ", ".join(f"{page} {len(runs)}" for page, runs in pages.items()))
    features = {}
    if args.web_features:
        with open(args.web_features) as f:
            features = json.load(f).get("features", {})
    for title, table in (("Memory", memory_table), ("Loading", load_table), ("Without input", idle_table)):
        print(f"\n### {title}\n")
        table(pages)
    print("\n### Features\n")
    union = feature_table(pages, features)
    print(f"\n### web-features identifiers used by these pages ({len(union)})\n")
    print("| Identifier | Baseline | Pages |")
    print("|---|---|---|")
    for ident in sorted(union, key=lambda i: (baseline_status(i, features) if features else "unknown", i)):
        print(f"| `{ident}` | {baseline_status(ident, features) if features else '—'} | {', '.join(sorted(union[ident]))} |")


if __name__ == "__main__":
    main()
