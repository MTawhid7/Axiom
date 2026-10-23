#!/usr/bin/env python3
"""The `unsafe` a Rust crate compiles in one configuration, for a soundness audit.

    python3 -I tools/unsafe-census.py CRATE_DIR [--root src/lib.rs]
                                      [--features alloc ...] [--arch aarch64]
                                      [--target-features neon ...] [--list]

It reads the crate's sources without building them. It evaluates `#[cfg(...)]`
attributes on items, and `cfg_if!` branches, for the configuration given:
the enabled features, the target architecture and features, little-endian,
macOS, 64-bit pointers; `test`, `miri`, `fuzzing` and `debug_assertions` are
off, so tests are left out. Modules the crate root gates out (`#[cfg(..)]
mod name;`) are skipped. For what remains, it counts `unsafe` blocks,
functions, impls and traits per file; counts the blocks with a safety comment
just before them or inside them; and sorts the blocks by the first operation
they contain (`get_unchecked`, `from_utf8_unchecked`, raw pointers,
`from_raw_parts`, `transmute`, `asm!`, other calls). `--list` prints every
site. Files under src/ only, one level deep, which suits most crates; check
the crate's module tree before trusting the totals.

A census guides a review; it doesn't replace one. The dependency policy's DP8
grades are Google's, and grading needs each unsafe line read. For an audit's
counts, prefer the compiler's census, tools/unsafe-sites.py, which follows
every cfg and macro; this tool needs no build. A cfg it can't evaluate is
reported and counted as compiled.

Used for the architecture review's RV1-08 (`encoding_rs`, 2026-10-21). Crate
sources are untrusted data: run with -I, and unpack each crate into its own
directory under target/.
"""

import argparse
import re
import sys
from pathlib import Path

CATEGORIES = [
    ("asm!", r"\basm!"),
    ("get_unchecked", r"get_unchecked"),
    ("from_utf8_unchecked", r"from_utf8_unchecked"),
    ("from_raw_parts", r"from_raw_parts"),
    ("transmute", r"transmute"),
    ("raw pointer", r"\.(read|write|add|offset|sub|cast)\s*[(:<]|ptr::(read|write|copy)|copy_nonoverlapping|as \*(const|mut)"),
    ("other call", r"[A-Za-z_]\s*[(!]"),
]


def balanced(text, i, open_, close):
    """The index of the bracket that closes the one at i."""
    depth = 0
    for j in range(i, len(text)):
        if text[j] == open_:
            depth += 1
        elif text[j] == close:
            depth -= 1
            if depth == 0:
                return j
    sys.exit(f"unbalanced {open_}{close} from offset {i}")


def blank_comments_and_strings(text):
    """Text of the same length with comments and string contents blanked, newlines kept."""
    out, i, n = list(text), 0, len(text)

    def blank(a, b):
        for k in range(a, b):
            if out[k] != "\n":
                out[k] = " "

    while i < n:
        if text.startswith("//", i):
            j = text.find("\n", i)
            j = n if j < 0 else j
            blank(i, j)
            i = j
        elif text.startswith("/*", i):
            j = text.find("*/", i + 2)
            j = n if j < 0 else j + 2
            blank(i, j)
            i = j
        elif text[i] == '"':
            j = i + 1
            while j < n and text[j] != '"':
                j += 2 if text[j] == "\\" else 1
            blank(i + 1, min(j, n))
            i = j + 1
        elif text[i] == "'" and re.match(r"'(\\.[^']*|[^\\'])'", text[i:]):
            j = i + re.match(r"'(\\.[^']*|[^\\'])'", text[i:]).end()
            blank(i + 1, j - 1)
            i = j
        else:
            i += 1
    return "".join(out)


class Config:
    def __init__(self, args):
        self.values = {
            "feature": set(args.features),
            "target_arch": {args.arch},
            "target_feature": set(args.target_features),
            "target_endian": {"little"},
            "target_os": {"macos"},
            "target_family": {"unix"},
            "target_pointer_width": {"64"},
        }

    def evaluate(self, expr):
        try:
            value, _ = self._parse(expr + " ", 0)
        except (AttributeError, IndexError):
            # Such as `cfg($($meta:meta),*)` inside a macro's definition: count it.
            print(f"unsafe-census: can't evaluate cfg({expr.strip()}); counted as compiled", file=sys.stderr)
            return True
        return value

    def _parse(self, s, i):
        while s[i].isspace() or s[i] == ",":
            i += 1
        m = re.match(r"(all|any|not)\s*\(", s[i:])
        if m:
            op, i, args = m.group(1), i + m.end(), []
            while True:
                while s[i].isspace() or s[i] == ",":
                    i += 1
                if s[i] == ")":
                    i += 1
                    break
                value, i = self._parse(s, i)
                args.append(value)
            if op == "all":
                return all(args), i
            if op == "any":
                return any(args), i
            return not args[0], i
        m = re.match(r'([A-Za-z_][A-Za-z0-9_]*)\s*(=\s*"([^"]*)")?', s[i:])
        key, value, i = m.group(1), m.group(3), i + m.end()
        if value is None:
            return False, i  # test, miri, fuzzing, debug_assertions, doc: off
        return value in self.values.get(key, set()), i


def item_end(code, i):
    """The end of the item that starts at i: its block, or a ';' before any '{'."""
    j, n = i, len(code)
    while j < n:
        if code[j] == ";":
            return j
        if code[j] == "{":
            return balanced(code, j, "{", "}")
        if code[j] in "([":
            j = balanced(code, j, code[j], ")" if code[j] == "(" else "]") + 1
            continue
        j += 1
    return n - 1


def compiled_mask(text, config):
    """(code, mask): code with comments and strings blanked; mask[k] is False where cfg removes text."""
    code = blank_comments_and_strings(text)
    mask = [True] * len(code)

    def drop(a, b):
        for k in range(a, b + 1):
            mask[k] = False

    for m in re.finditer(r"#\s*\[\s*cfg\s*\(", code):
        close = balanced(code, m.end() - 1, "(", ")")
        if not config.evaluate(text[m.end():close]):
            drop(m.start(), item_end(code, code.index("]", close) + 1))
    branch = re.compile(r"\s*(?:else\s+)?if\s+#\s*\[\s*cfg\s*\(")
    otherwise = re.compile(r"\s*else\s*\{")
    for m in re.finditer(r"cfg_if!\s*\{", code):
        end = balanced(code, m.end() - 1, "{", "}")
        i, taken = m.end(), False
        while i < end:
            b = branch.match(code, i)
            if b:
                close = balanced(code, b.end() - 1, "(", ")")
                start = code.index("{", close)
                stop = balanced(code, start, "{", "}")
                keep = not taken and config.evaluate(text[b.end():close])
                taken = taken or keep
                if not keep:
                    drop(start, stop)
                i = stop + 1
                continue
            e = otherwise.match(code, i)
            if e:
                stop = balanced(code, e.end() - 1, "{", "}")
                if taken:
                    drop(e.end() - 1, stop)
                i = stop + 1
                continue
            break
    return code, mask


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0], epilog=__doc__.split("\n\n", 1)[1],
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("crate", help="the unpacked crate's directory")
    parser.add_argument("--root", default="src/lib.rs", help="the crate root, relative to the crate (default: src/lib.rs)")
    parser.add_argument("--features", nargs="*", default=[], help="enabled features")
    parser.add_argument("--arch", default="aarch64", help="target_arch (default: aarch64)")
    parser.add_argument("--target-features", nargs="*", default=["neon"], help="target features (default: neon)")
    parser.add_argument("--list", action="store_true", help="print every unsafe site")
    args = parser.parse_args()

    config = Config(args)
    crate = Path(args.crate)
    root = crate / args.root
    root_code, root_mask = compiled_mask(root.read_text(errors="replace"), config)
    gated = {m.group(1) for m in re.finditer(r"\bmod\s+([a-z_0-9]+)\s*;", root_code) if not root_mask[m.start()]}

    totals = dict(blocks=0, documented=0, fns=0, impls=0, traits=0)
    categories, sites = {}, []
    print("| File | Blocks | With a safety comment | Functions | Impls | Traits |")
    print("|---|---|---|---|---|---|")
    for path in sorted(root.parent.glob("*.rs")):
        if path.stem in gated:
            continue
        text = path.read_text(errors="replace")
        code, mask = compiled_mask(text, config)
        lines = text.split("\n")
        counts = dict(blocks=0, documented=0, fns=0, impls=0, traits=0)
        for m in re.finditer(r"\bunsafe\b", code):
            if not mask[m.start()]:
                continue
            line = code.count("\n", 0, m.start())
            rest = code[m.end():m.end() + 40].lstrip()
            if rest.startswith("{"):
                start = code.index("{", m.end())
                body = text[start:balanced(code, start, "{", "}") + 1]
                before = "\n".join(lines[max(0, line - 6):line + 1])
                documented = bool(re.search(r"(?i)safety", before) or re.search(r"(?i)//\s*safety", body))
                kind = next(name for name, pattern in CATEGORIES if name == "other call" or re.search(pattern, body))
                counts["blocks"] += 1
                counts["documented"] += documented
                categories[kind] = categories.get(kind, 0) + 1
                sites.append(f"{path.name}:{line + 1}: block, {kind}{', documented' if documented else ''}")
            else:
                key = "fns" if rest.startswith(("fn", "extern")) else "impls" if rest.startswith("impl") else "traits"
                counts[key] += 1
                sites.append(f"{path.name}:{line + 1}: {key[:-1]}")
        if any(counts.values()):
            print(f"| `{path.name}` | {counts['blocks']} | {counts['documented']} | {counts['fns']} | {counts['impls']} | {counts['traits']} |")
        for key in totals:
            totals[key] += counts[key]
    print(f"| **All** | **{totals['blocks']}** | **{totals['documented']}** | **{totals['fns']}** | **{totals['impls']}** | **{totals['traits']}** |")
    print("\nBlocks by their first operation: " + ", ".join(f"{k} {v}" for k, v in sorted(categories.items(), key=lambda kv: -kv[1])))
    print("Modules the root gates out: " + (", ".join(sorted(gated)) or "none"))
    if args.list:
        print()
        print("\n".join(sites))


if __name__ == "__main__":
    main()
