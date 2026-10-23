#!/usr/bin/env python3
"""Every `unsafe` site the compiler builds, per crate: the census DP8's audits use.

Usage: tools/unsafe-sites.py MESSAGES.json... [--crate NAME...] [--root DIR] [--list]
       tools/unsafe-sites.py --self-test

Reads the JSON messages of a build made with the `unsafe_code` lint forced to
warn, and prints, for each crate and file, the `unsafe` blocks (with how many
have a safety comment), the `unsafe fn` declarations, the `unsafe` methods
declared and implemented, the `unsafe impl`s and the `unsafe trait`s.

Make the messages from a scratch manifest whose dependencies are unpacked
copies of the crates under audit, as path dependencies (`[patch.crates-io]`),
so cargo neither caps their lints nor reads them from the registry:

    RUSTFLAGS="--force-warn unsafe_code" cargo check --message-format=json \\
        --target aarch64-apple-darwin --target-dir <dir> > MESSAGES.json

rustc then reports every site it compiles in that configuration, with `cfg`s
evaluated and macros expanded. `--force-warn` matters: `-W unsafe_code` is
lowered by a crate's own `allow(unsafe_code)`, which peniko and color carry on
their `unsafe` modules (observed, 2026-10-23). Building runs the crates' build
scripts and procedural macros on this Mac (DP12), so audit those first or read
messages made elsewhere.

A site is counted once, at its primary span, however many times a macro
expands it; the expansion count is listed. A block has a safety comment when
"safety", in any case, appears in the six lines before it or in a `//` comment
inside it: the same heuristic as tools/unsafe-census.py, which reads sources
without building them and so can't follow every `cfg`. A census guides a
review; it doesn't replace one.

Exit status: 0, 1 when no `unsafe_code` message matched, 2 usage error. Uses
only the Python standard library; file names in the messages are resolved
against --root (default: the current directory) when they are relative.
"""

import argparse
import json
import re
import sys
import tempfile
from collections import Counter, defaultdict
from pathlib import Path

KINDS = [
    ("blocks", "usage of an `unsafe` block"),
    ("fns", "declaration of an `unsafe` function"),
    ("methods declared", "declaration of an `unsafe` method"),
    ("methods implemented", "implementation of an `unsafe` method"),
    ("impls", "implementation of an `unsafe` trait"),
    ("traits", "declaration of an `unsafe` trait"),
]


def package(package_id):
    """(`name@version`, the crate's directory for a path dependency, else None) from a cargo package ID."""
    if "#" in package_id:
        url, fragment = package_id.rsplit("#", 1)
        directory = url[len("path+file://"):] if url.startswith("path+file://") else None
        if "@" in fragment:
            return fragment, directory
        return f"{url.rstrip('/').rsplit('/', 1)[-1]}@{fragment}", directory
    parts = package_id.split()
    return (f"{parts[0]}@{parts[1]}" if len(parts) >= 2 else package_id), None


def short_name(file, directory):
    """The file relative to its crate: by the crate's directory when known, else from its last `src/`."""
    if directory and file.startswith(directory.rstrip("/") + "/"):
        return file[len(directory.rstrip("/")) + 1:]
    return file[file.rfind("/src/") + 1:] if "/src/" in file else Path(file).name


def kind_of(message):
    for kind, text in KINDS:
        if message.startswith(text):
            return kind
    return f"other: {message}"


def origin(span):
    """The innermost macro that expanded this span, as `name at file:line`."""
    expansion = span.get("expansion")
    if not expansion:
        return ""
    at = expansion["span"]
    return f"{expansion['macro_decl_name']} at {Path(at['file_name']).name}:{at['line_start']}"


def collect(lines, crates):
    """{(crate, file, line, column, kind): [end line, expansions, origin, crate directory]} from cargo's JSON lines."""
    sites = {}
    for raw in lines:
        try:
            message = json.loads(raw)
        except json.JSONDecodeError:
            continue
        if message.get("reason") != "compiler-message":
            continue
        diagnostic = message["message"]
        if (diagnostic.get("code") or {}).get("code") != "unsafe_code":
            continue
        crate, directory = package(message["package_id"])
        if crates and crate.split("@")[0] not in crates and crate not in crates:
            continue
        primary = next((s for s in diagnostic["spans"] if s["is_primary"]), None)
        if primary is None:
            continue
        key = (crate, primary["file_name"], primary["line_start"], primary["column_start"], kind_of(diagnostic["message"]))
        if key in sites:
            sites[key][1] += 1
        else:
            sites[key] = [primary["line_end"], 1, origin(primary), directory]
    return sites


def documented(path, start, end, cache):
    """Whether the block on lines start..end of path has a safety comment (the heuristic above)."""
    if path not in cache:
        try:
            cache[path] = path.read_text(errors="replace").split("\n")
        except OSError:
            cache[path] = None
    lines = cache[path]
    if lines is None:
        return None
    before = "\n".join(lines[max(0, start - 7):start])
    body = "\n".join(lines[start - 1:end])
    return bool(re.search(r"(?i)safety", before) or re.search(r"(?i)//\s*safety", body))


def report(sites, root, listing, out):
    cache, unread = {}, set()
    columns = [kind for kind, _ in KINDS]
    by_crate = defaultdict(lambda: defaultdict(Counter))
    rows = []
    for (crate, file, line, column, kind), (end, expansions, via, directory) in sorted(sites.items()):
        path = Path(file) if Path(file).is_absolute() else root / file
        short = short_name(file, directory)
        counts = by_crate[crate][short]
        counts[kind] += 1
        note = ""
        if kind == "blocks":
            has = documented(path, line, end, cache)
            if has is None:
                unread.add(file)
            counts["documented"] += bool(has)
            note = ", documented" if has else ""
        rows.append(f"{crate} {short}:{line}:{column}: {kind}{note}"
                    + (f", {expansions} expansions" if expansions > 1 else "") + (f", via {via}" if via else ""))
    others = sorted({k for files in by_crate.values() for c in files.values() for k in c if k.startswith("other")})
    head = ["Crate", "File", "Blocks", "With a safety comment"] + columns[1:] + others
    print("| " + " | ".join(head) + " |", file=out)
    print("|" + "---|" * len(head), file=out)
    for crate, files in sorted(by_crate.items()):
        total = Counter()
        for file, counts in sorted(files.items()):
            total.update(counts)
            cells = [crate, f"`{file}`", counts["blocks"], counts["documented"]] + [counts[k] for k in columns[1:] + others]
            print("| " + " | ".join(str(c) for c in cells) + " |", file=out)
        cells = [f"**{crate}**", "**all**", total["blocks"], total["documented"]] + [total[k] for k in columns[1:] + others]
        print("| " + " | ".join(f"**{c}**" if isinstance(c, int) else c for c in cells) + " |", file=out)
    if unread:
        print("\nNot found, so their blocks count as undocumented: " + ", ".join(sorted(unread)), file=out)
    if listing:
        print("\n" + "\n".join(rows), file=out)


def self_test():
    def message(crate_id, file, line, column, text, expansion=None):
        span = {"file_name": file, "line_start": line, "line_end": line + 2, "column_start": column, "is_primary": True}
        if expansion:
            span["expansion"] = {"macro_decl_name": "m!", "span": {"file_name": file, "line_start": 99}}
        return json.dumps({"reason": "compiler-message", "package_id": crate_id,
                           "message": {"code": {"code": "unsafe_code"}, "message": text, "spans": [span]}})

    with tempfile.TemporaryDirectory(prefix="unsafe-sites-") as tmp:
        root = Path(tmp)
        (root / "a/src").mkdir(parents=True)
        (root / "a/src/lib.rs").write_text("fn f() {\n    // SAFETY: in bounds.\n    unsafe {\n    }\n}\n"
                                           + "\n" * 10 + "fn g() {\n    unsafe {\n        x();\n    }\n}\n")
        new = f"path+file://{root}/a#a@0.1.0"
        old = "b 2.0.0 (registry+https://github.com/rust-lang/crates.io-index)"
        lines = [
            message(new, "a/src/lib.rs", 3, 5, "usage of an `unsafe` block"),
            message(new, "a/src/lib.rs", 17, 5, "usage of an `unsafe` block"),
            message(new, "a/src/lib.rs", 17, 5, "usage of an `unsafe` block"),  # a duplicate report
            message(old, "src/x.rs", 4, 1, "implementation of an `unsafe` trait", expansion=True),
            message(old, "src/x.rs", 4, 1, "implementation of an `unsafe` trait", expansion=True),
            message(old, "src/x.rs", 9, 1, "declaration of an `unsafe` function"),
            message(old, "src/x.rs", 12, 1, "usage of `#[unsafe(no_mangle)]`"),
            json.dumps({"reason": "compiler-message", "package_id": new,
                        "message": {"code": {"code": "dead_code"}, "message": "unused", "spans": []}}),
            "not json",
        ]
        sites = collect(lines, set())
        expected = {
            ("a@0.1.0", "a/src/lib.rs", 3, 5, "blocks"): 1,
            ("a@0.1.0", "a/src/lib.rs", 17, 5, "blocks"): 2,
            ("b@2.0.0", "src/x.rs", 4, 1, "impls"): 2,
            ("b@2.0.0", "src/x.rs", 9, 1, "fns"): 1,
            ("b@2.0.0", "src/x.rs", 12, 1, "other: usage of `#[unsafe(no_mangle)]`"): 1,
        }
        got = {key: value[1] for key, value in sites.items()}
        if got != expected:
            print("self-test FAILED: sites", got)
            return 1
        cache = {}
        if documented(root / "a/src/lib.rs", 3, 4, cache) is not True or documented(root / "a/src/lib.rs", 17, 19, cache):
            print("self-test FAILED: the safety-comment heuristic")
            return 1
        if set(collect(lines, {"b"})) != {k for k in expected if k[0] == "b@2.0.0"}:
            print("self-test FAILED: --crate")
            return 1
        names = (short_name(f"{root}/a/src/x/y.rs", f"{root}/a"), short_name("/r/registry/src/i/c-1.0/src/z.rs", None))
        if names != ("src/x/y.rs", "src/z.rs"):
            print("self-test FAILED: file names", names)
            return 1
    print(f"unsafe-sites self-test: OK ({len(expected)} sites from {len(lines)} lines, duplicates folded; "
          "the comment heuristic, --crate and file names checked)")
    return 0


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0], epilog=__doc__.split("\n\n", 1)[1],
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("messages", nargs="*", help="cargo's JSON messages ('-' for standard input)")
    parser.add_argument("--crate", action="append", default=[], help="only this crate (name or name@version)")
    parser.add_argument("--root", type=Path, default=Path.cwd(), help="where relative file names resolve")
    parser.add_argument("--list", action="store_true", help="print every site")
    parser.add_argument("--self-test", action="store_true", help="check the tool on synthetic messages")
    args = parser.parse_args()
    if args.self_test:
        return self_test()
    if not args.messages:
        parser.print_usage(sys.stderr)
        return 2
    lines = []
    for name in args.messages:
        try:
            lines.extend(sys.stdin if name == "-" else Path(name).read_text(errors="replace").splitlines())
        except OSError as error:
            print(f"unsafe-sites: {error}", file=sys.stderr)
            return 2
    sites = collect(lines, set(args.crate))
    if not sites:
        print("unsafe-sites: no unsafe_code message matched; was the build made with --force-warn unsafe_code?",
              file=sys.stderr)
        return 1
    report(sites, args.root, args.list, sys.stdout)
    return 0


if __name__ == "__main__":
    sys.exit(main())
