#!/usr/bin/env python3
"""Check that every relative Markdown link in the repository resolves.

Usage: tools/check-doc-links.py [--root DIR]

Scans every .md file under the repository root, skipping target/ and hidden
directories. For each inline link [text](target) outside code:
  - a relative path must name an existing file or directory;
  - a #fragment on a .md target, or on the file itself (#fragment alone), must
    match one of that file's headings, slugged as GitHub does: lower case,
    punctuation other than '-' and '_' removed, spaces turned into hyphens.
Links with a scheme (https:, mailto:) are not checked.

Prints one line per problem and a summary. Exits 1 if there is any problem.
Uses only the Python standard library.
"""

import argparse
import os
import re
import sys

LINK = re.compile(r"\[[^\]]*\]\(([^)\s]+)\)")
FENCE = re.compile(r"^(```|~~~).*?^\1", re.S | re.M)
INLINE_CODE = re.compile(r"`[^`\n]*`")
HEADING = re.compile(r"^(#+)\s+(.*?)\s*#*\s*$")
SKIP_DIRS = {"target"}


def slug(heading):
    text = heading.strip().lower()
    text = re.sub(r"[^\w\- ]", "", text)
    return text.replace(" ", "-")


def anchors(path, cache={}):
    if path not in cache:
        found = set()
        with open(path, encoding="utf-8") as f:
            text = FENCE.sub("", f.read())
        for line in text.splitlines():
            m = HEADING.match(line)
            if m:
                found.add(slug(m.group(2)))
        cache[path] = found
    return cache[path]


def markdown_files(root):
    for dirpath, dirnames, filenames in os.walk(root):
        dirnames[:] = sorted(
            d for d in dirnames if d not in SKIP_DIRS and not d.startswith(".")
        )
        for name in sorted(filenames):
            if name.endswith(".md"):
                yield os.path.join(dirpath, name)


def check(root):
    problems = []
    for src in markdown_files(root):
        with open(src, encoding="utf-8") as f:
            text = f.read()
        text = INLINE_CODE.sub("", FENCE.sub("", text))
        rel_src = os.path.relpath(src, root)
        for target in LINK.findall(text):
            if re.match(r"^[a-zA-Z][a-zA-Z0-9+.-]*:", target):
                continue
            path, _, fragment = target.partition("#")
            full = os.path.normpath(os.path.join(os.path.dirname(src), path)) if path else src
            if not os.path.exists(full):
                problems.append(f"MISSING  {rel_src} -> {target}")
            elif fragment and full.endswith(".md") and fragment not in anchors(full):
                problems.append(f"ANCHOR   {rel_src} -> {target}")
    return problems


def main():
    default_root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    parser = argparse.ArgumentParser(
        description="Check that every relative Markdown link in the repository resolves."
    )
    parser.add_argument(
        "--root", default=default_root, help="repository root (default: this script's parent)"
    )
    args = parser.parse_args()
    problems = check(args.root)
    for problem in problems:
        print(problem)
    print(f"check-doc-links: {len(problems)} problem(s)")
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
