#!/usr/bin/env python3
"""Check the engine's Rust files against ES1's size limits.

Usage: tools/check-file-size.py [--root DIR] [--self-test]

Counts every line of each .rs file under crates/ and fuzz/ (the spikes were
written before the limits and are exempt). A file of 600 lines or more is
reported as a warning; one of 1,000 or more fails, unless it is exempt:
  - files under a crate's src/generated/, which codegen writes (ES19);
  - files listed in tools/file-size-allowlist.txt, each with the line
    ceiling it may not pass and a justification on the comment line above.
An allowlist entry fails too when its file is missing, has fallen below
1,000 lines, or has passed its ceiling, so the list stays honest.

The limits are the owner's (2026-10-23): "Enforce 600-line warning / 1,000-line
failure for files, and 100-line failure for functions. We will review
empirical distribution at M1-CP15." Functions are Clippy's
`too_many_lines` (ES2).

Prints each warning and failure, then a summary with the largest files.
Exit status: 0 within the limits, 1 a failure, 2 usage error. Uses only the
Python standard library.
"""

import argparse
import sys
import tempfile
from pathlib import Path

WARN, FAIL = 600, 1000
SCANNED = ("crates", "fuzz")


def parse_allowlist(text):
    """{path: (ceiling, justification)} and the problems found in the file itself."""
    entries, problems, comment = {}, [], ""
    for number, raw in enumerate(text.splitlines(), 1):
        line = raw.strip()
        if not line:
            comment = ""
        elif line.startswith("#"):
            comment = line.lstrip("# ")
        else:
            fields = line.split()
            if len(fields) != 2 or not fields[1].isdigit():
                problems.append(f"allowlist line {number}: expected `<path> <ceiling>`, got {line!r}")
            elif not comment:
                problems.append(f"allowlist line {number}: {fields[0]} has no justification on the line above")
            else:
                entries[fields[0]] = (int(fields[1]), comment)
            comment = ""
    return entries, problems


def rust_files(root):
    for top in SCANNED:
        base = root / top
        if base.is_dir():
            for path in sorted(base.rglob("*.rs")):
                if "target" not in path.relative_to(root).parts:
                    yield path


def check(root, allowlist_text):
    """Returns (failures, warnings, sizes)."""
    entries, failures = parse_allowlist(allowlist_text)
    warnings, sizes, seen = [], [], set()
    for path in rust_files(root):
        rel = path.relative_to(root).as_posix()
        with path.open("rb") as f:
            lines = sum(1 for _ in f)
        sizes.append((lines, rel))
        if "/src/generated/" in f"/{rel}":
            continue
        entry = entries.get(rel)
        if entry is not None:
            seen.add(rel)
            ceiling, _ = entry
            if lines >= ceiling:
                failures.append(f"FAIL  {rel}: {lines} lines, at or past its allowlisted ceiling of {ceiling}")
            elif lines < FAIL:
                failures.append(f"FAIL  {rel}: {lines} lines, under {FAIL}; remove its allowlist entry")
        elif lines >= FAIL:
            failures.append(f"FAIL  {rel}: {lines} lines (ES1 fails at {FAIL}; split it, or allowlist it with a reason)")
        elif lines >= WARN:
            warnings.append(f"WARN  {rel}: {lines} lines (ES1 warns at {WARN}; move its tests out first)")
    for rel in sorted(set(entries) - seen):
        failures.append(f"FAIL  {rel}: allowlisted, but no such file is scanned; remove its entry")
    return failures, warnings, sorted(sizes, reverse=True)


def self_test():
    with tempfile.TemporaryDirectory(prefix="check-file-size-") as tmp:
        root = Path(tmp)

        def write(rel, lines):
            path = root / rel
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text("//\n" * lines, encoding="utf-8")

        write("crates/a/src/small.rs", 10)
        write("crates/a/src/warned.rs", WARN)
        write("crates/a/src/generated/table.rs", 5000)
        write("crates/a/src/allowed.rs", 1200)
        write("spikes/s1/src/huge.rs", 5000)
        allow = "# A specification's table, one unit by its structure.\ncrates/a/src/allowed.rs 1500\n"
        failures, warnings, _ = check(root, allow)
        if failures or warnings != [f"WARN  crates/a/src/warned.rs: {WARN} lines (ES1 warns at {WARN}; move its tests out first)"]:
            print("self-test FAILED: the compliant tree:", failures, warnings)
            return 1
        write("crates/a/src/big.rs", FAIL)
        write("fuzz/fuzz_targets/t.rs", FAIL + 1)
        write("crates/a/src/allowed.rs", 1500)
        allow += "# Gone.\ncrates/a/src/missing.rs 2000\n# Shrunk.\ncrates/a/src/small.rs 2000\nno-justification.rs 10\n"
        failures, _, _ = check(root, allow)
        expected = ["crates/a/src/big.rs: 1000 lines", "fuzz/fuzz_targets/t.rs: 1001 lines",
                    "crates/a/src/allowed.rs: 1500 lines, at or past", "crates/a/src/missing.rs: allowlisted, but",
                    "crates/a/src/small.rs: 10 lines, under", "no-justification.rs has no justification"]
        missing = [e for e in expected if not any(e in f for f in failures)]
        if missing or len(failures) != len(expected):
            print("self-test FAILED: expected", expected, "got", failures)
            return 1
    print(f"check-file-size self-test: OK (compliant tree accepted; {len(expected)}/{len(expected)} seeded failures reported)")
    return 0


def main():
    default_root = Path(__file__).resolve().parent.parent
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0], epilog=__doc__.split("\n\n", 1)[1],
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--root", type=Path, default=default_root, help="repository root (default: this script's parent)")
    parser.add_argument("--self-test", action="store_true", help="check the checker on a synthetic tree")
    args = parser.parse_args()
    if args.self_test:
        return self_test()
    root = args.root.resolve()
    try:
        allow = (root / "tools/file-size-allowlist.txt").read_text(encoding="utf-8")
    except OSError as error:
        print(f"check-file-size: {error}", file=sys.stderr)
        return 2
    failures, warnings, sizes = check(root, allow)
    for line in warnings + failures:
        print(line)
    largest = ", ".join(f"{rel} ({lines})" for lines, rel in sizes[:3]) or "none"
    print(f"check-file-size: {len(sizes)} file(s), largest {largest}; {len(warnings)} warning(s), {len(failures)} failure(s)")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
