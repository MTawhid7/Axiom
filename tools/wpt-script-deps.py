#!/usr/bin/env python3
"""What a WPT directory's tests need to run: script, and the other capabilities a milestone may lack.

    python3 -I tools/wpt-script-deps.py WPT_ROOT MANIFEST.json --dir css/CSS2
                                        [--exclude css/CSS2/tables/ [...]]
                                        [--types reftest [...]] [--list]
                                        [--list-free FILE] [--tests-only]
                                        [--needs] [--list-without CAPS FILE]

For the tests of --types (default: reftest) under --dir, minus each --exclude
prefix, it reads every test file and, unless --tests-only, every reference
file the manifest gives it, following references that are themselves
reference tests. A test needs what any file of that graph needs. It counts the
tests that contain a <script> element; those whose inline <script> elements
hold code; those that use the `reftest-wait` class, which only script can
remove; and those that load only external scripts. It prints the counts by
subdirectory, then the highest share of the tests that an engine without
script could pass, if every test that contains script fails, as wpt.fyi counts
a test that doesn't run (SB10).

--needs adds a second table of the other capabilities each test's graph
needs, by subdirectory:
  xml        a test or reference that is an XML document (.xht, .xhtml, .xml,
             .svg), which WPT's server sends with an XML media type
  webfont    an @font-face rule other than /fonts/ahem.css's: a web font
             beyond the test fonts the harness registers
  svg        inline <svg>, or an .svg file named in src, href or url()
  forms      <input>, <select>, <textarea> or <button>
  print      the reftest-print class or an @page rule
A reference the checkout lacks is counted as missing, and its test's needs
are then a lower bound.

Counts are of test files, as the manifest's paths name them; a file with
variants counts once. The manifest is WPT's MANIFEST.json, as `./wpt manifest`
writes it (version 8, a trie of path segments).

--list-free writes the tests in scope whose graphs contain no script to FILE,
one `/path` per line, as wpt.fyi's summaries name them; `wpt-pass-rates.py
--tests FILE` then computes the shipping median over just those tests.
--list-without CAPS FILE writes the tests that need none of the
comma-separated capabilities CAPS (from script and the --needs list), for
the same use. --tests-only reads the test files alone, as this tool did
before Round 4, so its earlier counts can be reproduced.

Used for the architecture review's finding RV1-01, on Milestone 1's
`css/CSS2` target (MA4), and Round 4, which added references, their chains
and --needs, after Round 3 noted that references went unread. WPT's files are
untrusted data: run with -I, and keep the checkout in its own directory under
target/.
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
FONT_FACE = re.compile(rb"@font-face", re.I)
SVG = re.compile(rb"<svg[\s>]|(?:src|href)\s*=\s*[\"']?[^\"'\s>]*\.svg|url\(\s*[\"']?[^)\"']*\.svg", re.I)
FORMS = re.compile(rb"<(?:input|select|textarea|button)[\s>/]", re.I)
PRINT = re.compile(rb"reftest-print|@page", re.I)
XML_SUFFIXES = (".xht", ".xhtml", ".xml", ".svg")
NEEDS = ("xml", "webfont", "svg", "forms", "print")


def manifest_tests(path, types, directory):
    """The manifest's tests of the given types under a directory, each with its
    references; and every reference test in the manifest, for following chains."""
    with open(path, encoding="utf-8") as f:
        items = json.load(f).get("items", {})
    every = {}
    for kind in types:
        stack = [("", items.get(kind, {}))]
        while stack:
            prefix, node = stack.pop()
            for name, value in node.items():
                path_here = f"{prefix}/{name}" if prefix else name
                if isinstance(value, dict):
                    stack.append((path_here, value))
                else:
                    refs = []
                    for variant in value[1:]:
                        if len(variant) > 1 and isinstance(variant[1], list):
                            refs.extend(ref[0] for ref in variant[1])
                    every[path_here] = refs
    prefix = directory.strip("/") + "/"
    tests = {p: refs for p, refs in every.items() if p.startswith(prefix)}
    return tests, every


def ref_path(url):
    """A reference URL's path in the checkout, or None for about:blank."""
    if url.startswith("about:"):
        return None
    return url.split("#", 1)[0].split("?", 1)[0].lstrip("/")


def classify(data, name):
    """The facts of one file."""
    inline = any(body.strip() for body in INLINE.findall(data))
    font_faces = len(FONT_FACE.findall(data))
    return {
        "script": bool(SCRIPT.search(data)),
        "inline": inline,
        "external only": bool(EXTERNAL.search(data)) and not inline,
        "reftest-wait": bool(WAIT.search(data)),
        "xml": name.endswith(XML_SUFFIXES),
        "webfont": font_faces > 0 and not name.endswith("fonts/ahem.css"),
        "svg": bool(SVG.search(data)) or name.endswith(".svg"),
        "forms": bool(FORMS.search(data)),
        "print": bool(PRINT.search(data)),
    }


def graph(path, tests, every, root, follow):
    """The facts of a test and, if `follow`, of every reference it reaches;
    and how many references the checkout lacks."""
    facts, missing, seen = collections.Counter(), 0, set()
    frontier = [path]
    while frontier:
        nxt = []
        for p in frontier:
            if p in seen:
                continue
            seen.add(p)
            file = root / p
            if not file.is_file():
                missing += 1
                continue
            for fact, present in classify(file.read_bytes(), p).items():
                facts[fact] += present
            if follow:
                for url in (tests.get(p) if p == path else every.get(p)) or []:
                    ref = ref_path(url)
                    if ref is not None:
                        nxt.append(ref)
        frontier = nxt
    return {fact: count > 0 for fact, count in facts.items()}, missing


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
    parser.add_argument("--tests-only", action="store_true", help="read the test files alone, not their references")
    parser.add_argument("--needs", action="store_true", help="also count the other capabilities the tests need")
    parser.add_argument("--list-without", nargs=2, metavar=("CAPS", "FILE"),
                        help="write the tests that need none of CAPS (comma-separated) to FILE")
    args = parser.parse_args()

    root = Path(args.root)
    tests, every = manifest_tests(args.manifest, args.types, args.dir)
    paths = sorted(p for p in tests if not any(p.startswith(prefix.lstrip("/")) for prefix in args.exclude))
    if not paths:
        sys.exit(f"no tests of {args.types} under {args.dir}")

    totals = collections.Counter()
    by_dir = collections.defaultdict(collections.Counter)
    scripted, free, needs_of = [], [], {}
    for path in paths:
        if not (root / path).is_file():
            totals["missing"] += 1
            continue
        facts, missing_refs = graph(path, tests, every, root, not args.tests_only)
        rest = path[len(args.dir.strip("/")) + 1 :]
        sub = rest.split("/", 1)[0] if "/" in rest else "(top level)"
        by_dir[sub]["tests"] += 1
        totals["tests"] += 1
        if missing_refs:
            totals["tests with a missing reference"] += 1
            totals["missing references"] += missing_refs
        for fact, present in facts.items():
            if present:
                by_dir[sub][fact] += 1
                totals[fact] += 1
        needs_of[path] = {fact for fact, present in facts.items() if present}
        (scripted if facts["script"] else free).append(path)

    columns = ("tests", "script", "inline", "reftest-wait", "external only")
    print("| Subdirectory | " + " | ".join(columns) + " |")
    print("|---|" + "---|" * len(columns))
    for sub in sorted(by_dir, key=lambda s: (-by_dir[s]["script"], s)):
        print(f"| `{sub}` | " + " | ".join(str(by_dir[sub][c]) for c in columns) + " |")
    print("| **All** | " + " | ".join(f"**{totals[c]}**" for c in columns) + " |")
    if totals["missing"]:
        print(f"\n{totals['missing']} manifest paths have no file in the checkout, and are left out.")
    if totals["tests with a missing reference"]:
        print(f"\n{totals['tests with a missing reference']} tests have references the checkout lacks "
              f"({totals['missing references']} in all); their needs are a lower bound.")
    elif not args.tests_only:
        print("\nEvery reference these tests reach is in the checkout.")
    tests_n = totals["tests"]
    print(f"\nWithout script, at most {tests_n - totals['script']} of {tests_n} pass: "
          f"{100 * (tests_n - totals['script']) / tests_n:.2f}%.")
    if args.needs:
        print("\n| Subdirectory | tests | " + " | ".join(NEEDS) + " |")
        print("|---|---|" + "---|" * len(NEEDS))
        for sub in sorted(by_dir, key=lambda s: (-by_dir[s]["tests"], s)):
            print(f"| `{sub}` | {by_dir[sub]['tests']} | " + " | ".join(str(by_dir[sub][n]) for n in NEEDS) + " |")
        print(f"| **All** | **{tests_n}** | " + " | ".join(f"**{totals[n]}**" for n in NEEDS) + " |")
        free_set = set(free)
        for need in NEEDS:
            without = sum(1 for p in free_set if need not in needs_of[p])
            print(f"Without script or {need}: {without} of {tests_n} ({100 * without / tests_n:.2f}%).")
    if args.list:
        print()
        for path in scripted:
            print(path)
    if args.list_free:
        Path(args.list_free).write_text("".join(f"/{path}\n" for path in free), encoding="utf-8")
        print(f"\nWrote the {len(free)} tests without script to {args.list_free}.")
    if args.list_without:
        caps, file = args.list_without
        unwanted = {c.strip() for c in caps.split(",") if c.strip()}
        unknown = unwanted - {"script", *NEEDS}
        if unknown:
            sys.exit(f"unknown capabilities: {', '.join(sorted(unknown))}")
        chosen = [p for p in paths if p in needs_of and not (needs_of[p] & unwanted)]
        Path(file).write_text("".join(f"/{path}\n" for path in chosen), encoding="utf-8")
        print(f"\nWrote the {len(chosen)} tests that need none of {', '.join(sorted(unwanted))} to {file}.")


if __name__ == "__main__":
    main()
