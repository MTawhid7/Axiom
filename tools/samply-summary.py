#!/usr/bin/env python3
"""Summarize a samply profile from the terminal: where the time went.

Record with symbols saved alongside the profile:

    samply record --save-only --unstable-presymbolicate -o out.json.gz -- <command>

which writes out.json.gz and its sidecar out.json.syms.json. Then:

    python3 -I tools/samply-summary.py out.json.gz [--top N] [--match TEXT]
                                       [--lines BINARY] [--source-root DIR]
                                       [--share NAME=REGEX ...]

It prints, as a share of the selected samples:
  - self time by compiled function (after inlining);
  - inclusive time by compiled function;
  - with --lines: self time by source line, and self time by *our* source
    line. "Our" line is the innermost inlined frame whose file is under
    --source-root (default: this repository's spikes/ and crates/), so time
    spent in inlined standard-library code (Vec::push, RefCell::borrow, …)
    is charged to the line of our code that called it.

--match keeps only samples whose stack contains a compiled function whose
name includes TEXT, to focus on one phase of a run (for example the timed
loop rather than input generation).

--share NAME=REGEX (with --lines; repeatable) also prints the share of the
selected samples whose leaf address has, anywhere in its chain of inlined
frames, a frame whose repository-relative file path matches REGEX. Code
inlined from a module therefore counts toward it wherever it was inlined.
Time in a separate compiled function that such code calls (for example a
vector's out-of-line growth) does not.

--lines resolves the profiled executable's addresses with `atos -i`, which
reads a .dSYM bundle next to the binary; the script creates one with
`dsymutil` if it is missing. Both ship with the Command Line Tools. samply's
own sidecar is used for everything else; its inline frames are per function,
not per address, so it cannot attribute lines by itself.

Only the Python standard library is used.
"""

import argparse
import collections
import gzip
import json
import os
import re
import subprocess
import sys

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
TEXT_BASE = 0x100000000  # __TEXT vmaddr of arm64 macOS executables


def load_symbols(syms_path):
    """{lib debug name: {address: compiled symbol name}}"""
    with open(syms_path, encoding="utf-8") as f:
        syms = json.load(f)
    strings = syms["string_table"]
    by_lib = {}
    for lib in syms["data"]:
        table = lib["symbol_table"]
        by_lib[lib["debug_name"]] = {
            address: strings[table[index]["symbol"]] for address, index in lib.get("known_addresses", [])
        }
    return by_lib


def resolve_lines(binary, addresses):
    """{address: [(function, path, line), ...] innermost first}, via atos."""
    if not os.path.isdir(binary + ".dSYM"):
        subprocess.run(["dsymutil", binary], check=True, capture_output=True)
    addresses = sorted(addresses)
    result = {}
    for start in range(0, len(addresses), 500):
        batch = addresses[start : start + 500]
        out = subprocess.run(
            ["atos", "-o", binary, "-l", hex(TEXT_BASE), "-i", "-fullPath"] + [hex(TEXT_BASE + a) for a in batch],
            check=True,
            capture_output=True,
            text=True,
        ).stdout
        groups = [g for g in out.split("\n\n")]
        groups = [g for g in groups if g.strip()]
        if len(groups) != len(batch):
            sys.exit(f"atos returned {len(groups)} groups for {len(batch)} addresses")
        for address, group in zip(batch, groups):
            frames = []
            for line in group.strip().splitlines():
                # "<function> (in <image>) (<path>:<line>)" or "<symbol> (in <image>) + <offset>"
                function = line.split(" (in ", 1)[0]
                path, number = "", None
                if line.endswith(")") and line.rfind(" (") > line.find(" (in "):
                    location = line[line.rfind(" (") + 2 : -1]
                    path, _, number = location.rpartition(":")
                frames.append((function, path, number))
            result[address] = frames
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("profile")
    parser.add_argument("--top", type=int, default=20)
    parser.add_argument("--match", default=None)
    parser.add_argument("--lines", metavar="BINARY", default=None)
    parser.add_argument("--source-root", action="append", default=None)
    parser.add_argument("--share", action="append", default=[], metavar="NAME=REGEX")
    args = parser.parse_args()
    roots = [os.path.abspath(r) for r in (args.source_root or [os.path.join(REPO, "spikes"), os.path.join(REPO, "crates")])]

    if not args.profile.endswith(".json.gz"):
        sys.exit("expected a .json.gz profile written by samply --save-only")
    symbols = load_symbols(args.profile[: -len(".gz")] + ".syms.json")
    with gzip.open(args.profile, "rt", encoding="utf-8") as f:
        profile = json.load(f)
    libs = profile["libs"]
    binary_name = os.path.basename(args.lines) if args.lines else None

    # Pass 1: select samples, and collect the binary's leaf addresses.
    selected = []  # (weight, [(lib, address, symbol), ...] leaf first)
    for thread in profile["threads"]:
        frames, funcs, resources = thread["frameTable"], thread["funcTable"], thread["resourceTable"]
        stacks, strings = thread["stackTable"], thread["stringArray"]
        cache = {}

        def describe(frame_index):
            if frame_index not in cache:
                func = frames["func"][frame_index]
                resource = funcs["resource"][func]
                lib = libs[resources["lib"][resource]]["debugName"] if resource is not None and resource >= 0 else "?"
                address = frames["address"][frame_index]
                symbol = symbols.get(lib, {}).get(address) or f"{lib}!{strings[funcs['name'][func]]}"
                cache[frame_index] = (lib, address, symbol)
            return cache[frame_index]

        weights = thread["samples"].get("weight") or [1] * thread["samples"]["length"]
        for stack, weight in zip(thread["samples"]["stack"], weights):
            if stack is None:
                continue
            chain = []
            while stack is not None:
                chain.append(describe(stacks["frame"][stack]))
                stack = stacks["prefix"][stack]
            if args.match and not any(args.match in symbol for _, _, symbol in chain):
                continue
            selected.append((weight or 1, chain))

    total = sum(w for w, _ in selected)
    if total == 0:
        sys.exit("no samples" + (f" matching {args.match!r}" if args.match else ""))

    lines = {}
    if args.lines:
        leaves = {chain[0][1] for _, chain in selected if chain[0][0] == binary_name}
        lines = resolve_lines(args.lines, leaves)

    shares = []
    for spec in args.share:
        name, sep, pattern = spec.partition("=")
        if not sep:
            sys.exit(f"--share expects NAME=REGEX, got {spec!r}")
        shares.append((name, re.compile(pattern), [0]))

    self_compiled, inclusive = collections.Counter(), collections.Counter()
    self_line, self_ours = collections.Counter(), collections.Counter()
    for weight, chain in selected:
        lib, address, symbol = chain[0]
        self_compiled[symbol] += weight
        for s in {s for _, _, s in chain}:
            inclusive[s] += weight
        if args.lines:
            inline = lines.get(address) if lib == binary_name else None
            if inline:
                fn, path, number = inline[0]
                self_line[f"{fn}  {os.path.basename(path)}:{number}"] += weight
                ours = next(((f, p, n) for f, p, n in inline if any(p.startswith(r) for r in roots)), None)
                if ours:
                    f, p, n = ours
                    self_ours[f"{os.path.relpath(p, REPO)}:{n}  ({f})"] += weight
                else:
                    self_ours[f"(no frame of ours) {symbol}"] += weight
                for _, regex, count in shares:
                    if any(p and regex.search(os.path.relpath(p, REPO)) for _, p, _ in inline):
                        count[0] += weight
            else:
                self_line[symbol] += weight
                self_ours[f"(outside {binary_name}) {lib}"] += weight

    print(f"{total} samples" + (f" with {args.match!r} on the stack" if args.match else ""))
    sections = [
        ("Self time, by compiled function", self_compiled),
        ("Inclusive time, by compiled function", inclusive),
    ]
    if args.lines:
        sections += [
            ("Self time, by source line (innermost inlined frame)", self_line),
            ("Self time, by our source line (inlined library code charged to its caller)", self_ours),
        ]
    for title, counter in sections:
        print(f"\n{title}:")
        for name, count in counter.most_common(args.top):
            print(f"  {100 * count / total:5.1f}%  {name}")
    if shares:
        print()
        for name, _, count in shares:
            print(f"share {name}: {100 * count[0] / total:.1f}%")


if __name__ == "__main__":
    main()
