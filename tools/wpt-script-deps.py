#!/usr/bin/env python3
"""How many of a WPT directory's tests need script to run.

    python3 -I tools/wpt-script-deps.py WPT_ROOT MANIFEST.json --dir css/CSS2
                                        [--exclude css/CSS2/tables/ [...]]
                                        [--types reftest [...]] [--list]
                                        [--list-free FILE]

For the tests of --types (default: reftest) under --dir, minus each --exclude
prefix, it reads every test file and counts those that contain a <script>
element; those whose inline <script> elements hold code; those that use the
`reftest-wait` class, which only script can remove; and those that load only
external scripts. It prints the counts by subdirectory, then the highest share
of the tests that an engine without script could pass, if every test that
contains script fails, as wpt.fyi counts a test that doesn't run (SB10).

Counts are of test files, as the manifest's paths name them; a file with
variants counts once. The manifest is WPT's MANIFEST.json, as `./wpt manifest`
writes it (version 8, a trie of path segments).

--list-free writes the tests in scope that contain no script to FILE, one
`/path` per line, as wpt.fyi's summaries name them; `wpt-pass-rates.py
--tests FILE` then computes the shipping median over just those tests.

Used for the architecture review's finding RV1-01, on Milestone 1's
`css/CSS2` target (MA4). WPT's files are untrusted data: run with -I, and keep
the checkout in its own directory under target/.
"""

import argparse
import collections
import json
import re
import sys
from pathlib import Path

SCRIPT = re.compile(rb"<script", re.I)
INLINE = re.compile(rb"<script(?![^>]*\ssrc\s*=)[^>]*>(.*?)</script>", re.I | re.S)
EXTERNAL = re.compile(rb"<script[^>]*\ssrc\s*=", re.I)
WAIT = re.compile(rb"reftest-wait")


def manifest_paths(path, types, directory):
    """The paths of a WPT manifest's tests of the given types under a directory."""
    with open(path, encoding="utf-8") as f:
        items = json.load(f).get("items", {})
    paths = set()
    for kind in types:
        node = items.get(kind, {})
        for segment in directory.strip("/").split("/"):
            node = node.get(segment, {}) if isinstance(node, dict) else {}
        stack = [(directory.strip("/"), node)]
        while stack:
            prefix, node = stack.pop()
            for name, value in node.items():
                path_here = f"{prefix}/{name}"
                if isinstance(value, dict):
                    stack.append((path_here, value))
                else:
                    paths.add(path_here)
    return paths


def classify(data):
    """The script-related facts of one test file."""
    inline = any(body.strip() for body in INLINE.findall(data))
    return {
        "script": bool(SCRIPT.search(data)),
        "inline": inline,
        "external only": bool(EXTERNAL.search(data)) and not inline,
        "reftest-wait": bool(WAIT.search(data)),
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0], epilog=__doc__.split("\n\n", 1)[1],
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("root", help="the WPT checkout")
    parser.add_argument("manifest", help="WPT's MANIFEST.json for that checkout")
    parser.add_argument("--dir", required=True, help="a directory, such as css/CSS2")
    parser.add_argument("--exclude", nargs="*", default=[], help="path prefixes to leave out, such as css/CSS2/tables/")
    parser.add_argument("--types", nargs="+", default=["reftest"], help="test types (default: reftest)")
    parser.add_argument("--list", action="store_true", help="also list the tests that contain script")
    parser.add_argument("--list-free", metavar="FILE", help="write the tests without script to FILE, one /path per line")
    args = parser.parse_args()

    root = Path(args.root)
    paths = sorted(p for p in manifest_paths(args.manifest, args.types, args.dir)
                   if not any(p.startswith(prefix.lstrip("/")) for prefix in args.exclude))
    if not paths:
        sys.exit(f"no tests of {args.types} under {args.dir}")

    totals = collections.Counter()
    by_dir = collections.defaultdict(collections.Counter)
    scripted, free = [], []
    for path in paths:
        file = root / path
        if not file.is_file():
            totals["missing"] += 1
            continue
        facts = classify(file.read_bytes())
        rest = path[len(args.dir.strip("/")) + 1 :]
        sub = rest.split("/", 1)[0] if "/" in rest else "(top level)"
        by_dir[sub]["tests"] += 1
        totals["tests"] += 1
        for fact, present in facts.items():
            if present:
                by_dir[sub][fact] += 1
                totals[fact] += 1
        (scripted if facts["script"] else free).append(path)

    columns = ("tests", "script", "inline", "reftest-wait", "external only")
    print("| Subdirectory | " + " | ".join(columns) + " |")
    print("|---|" + "---|" * len(columns))
    for sub in sorted(by_dir, key=lambda s: (-by_dir[s]["script"], s)):
        print(f"| `{sub}` | " + " | ".join(str(by_dir[sub][c]) for c in columns) + " |")
    print("| **All** | " + " | ".join(f"**{totals[c]}**" for c in columns) + " |")
    if totals["missing"]:
        print(f"\n{totals['missing']} manifest paths have no file in the checkout, and are left out.")
    tests = totals["tests"]
    print(f"\nWithout script, at most {tests - totals['script']} of {tests} pass: "
          f"{100 * (tests - totals['script']) / tests:.2f}%.")
    if args.list:
        print()
        for path in scripted:
            print(path)
    if args.list_free:
        Path(args.list_free).write_text("".join(f"/{path}\n" for path in free), encoding="utf-8")
        print(f"\nWrote the {len(free)} tests without script to {args.list_free}.")


if __name__ == "__main__":
    main()
