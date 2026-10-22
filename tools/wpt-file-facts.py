#!/usr/bin/env python3
"""Facts about a list of WPT tests that their scope depends on: encodings, Ahem and dynamic pseudo-classes.

    python3 -I tools/wpt-file-facts.py WPT_ROOT LIST [--list-legacy FILE]

LIST names the tests, one `/path` per line, as `wpt-script-deps.py
--list-free` writes it. For each test the tool reports:

  the document's encoding, in the order a browser decides it: a byte-order
      mark; a charset in the test's .headers file; the XML declaration (XML
      documents: .xht, .xhtml, .xml, .svg) or a <meta> charset (HTML); or,
      without any, UTF-8 for XML and the locale's default for HTML
      (windows-1252 in English locales). Each is counted with the files that
      hold bytes beyond ASCII, for which the choice matters;
  the encodings it declares with the files it loads: every encoding label
      in the test and in the files it names in href, src or @import (and in
      the stylesheets those import, two levels down), with their .headers
      files and byte-order marks. Labels are grouped as the Encoding
      Standard maps them: UTF-8; UTF-16; windows-1252, whose labels include
      US-ASCII and ISO-8859-1; and every other label, "legacy";
  whether it names the Ahem font, and whether it links /fonts/ahem.css;
  whether it uses :hover, :active or :focus, which a script-free run can't
      trigger.

--list-legacy writes the tests that declare a legacy label to FILE, one
`/path` per line, for `wpt-pass-rates.py --tests`. The label patterns are
textual, so a label in a comment counts too: the legacy count is an upper
bound on the tests a missing decoder can decide.

Used for the architecture review's Round 4 (RV4-08, on staging Milestone 1's
encodings, and the verification of Round 3's claims on fonts and
pseudo-classes). WPT's files are untrusted data: run with -I, and keep the
checkout in its own directory under target/.
"""

import argparse
import collections
import re
import sys
from pathlib import Path

XML_SUFFIXES = (".xht", ".xhtml", ".xml", ".svg")
XML_DECLARATION = re.compile(rb"<\?xml[^>]*encoding\s*=\s*[\"']([^\"']+)", re.I)
META = re.compile(rb"<meta[^>]*charset\s*=\s*[\"']?([A-Za-z0-9_.:-]+)", re.I)
HEADER = re.compile(rb"charset\s*=\s*([A-Za-z0-9_.:-]+)", re.I)
LABEL = re.compile(
    rb"(?:@charset\s*[\"']|charset\s*=\s*[\"']?|encoding\s*=\s*[\"'])([A-Za-z0-9_.:-]+)", re.I
)
LOADS = re.compile(
    rb"(?:href|src)\s*=\s*[\"']([^\"'#?]+)|@import\s+(?:url\()?\s*[\"']?([^\"')\s;]+)", re.I
)
DYNAMIC = re.compile(rb":(?:hover|active|focus)\b")
UTF8 = {"utf-8", "utf8", "unicode-1-1-utf-8", "unicode11utf8", "unicode20utf8", "x-unicode20utf8"}
UTF16 = {"utf-16", "utf-16le", "utf-16be", "unicode", "unicodefeff", "unicodefffe", "ucs-2", "iso-10646-ucs-2", "csunicode"}
WINDOWS_1252 = {
    "windows-1252", "us-ascii", "ascii", "iso-8859-1", "iso8859-1", "iso88591", "iso_8859-1",
    "iso_8859-1:1987", "iso-ir-100", "latin1", "l1", "cp1252", "x-cp1252", "cp819", "ibm819",
    "csisolatin1", "ansi_x3.4-1968", "us-ascii",
}
GROUPS = ("utf-8", "utf-16", "windows-1252", "legacy")


def group(label):
    """The Encoding Standard's encoding for a label, as one of GROUPS."""
    if label in UTF8:
        return "utf-8"
    if label in UTF16:
        return "utf-16"
    if label in WINDOWS_1252:
        return "windows-1252"
    return "legacy"


def declared(path):
    """The labels and byte-order mark a file declares, with its .headers file's."""
    try:
        data = path.read_bytes()
    except OSError:
        return set(), b""
    labels = {m.group(1).decode("ascii", "replace").lower() for m in LABEL.finditer(data)}
    if data.startswith(b"\xef\xbb\xbf"):
        labels.add("utf-8")
    elif data[:2] in (b"\xfe\xff", b"\xff\xfe"):
        labels.add("utf-16")
    headers = path.with_name(path.name + ".headers")
    if headers.is_file():
        labels |= {m.group(1).decode("ascii", "replace").lower() for m in LABEL.finditer(headers.read_bytes())}
    return labels, data


def document_encoding(path, data):
    """How the test document's own encoding is decided, as a browser would."""
    if data.startswith(b"\xef\xbb\xbf"):
        return "utf-8, by its byte-order mark"
    if data[:2] in (b"\xfe\xff", b"\xff\xfe"):
        return "utf-16, by its byte-order mark"
    headers = path.with_name(path.name + ".headers")
    if headers.is_file():
        m = HEADER.search(headers.read_bytes())
        if m:
            return f"{m.group(1).decode('ascii', 'replace').lower()}, by its .headers file"
    is_xml = path.suffix in XML_SUFFIXES
    m = (XML_DECLARATION if is_xml else META).search(data[:4096])
    if m:
        how = "its XML declaration" if is_xml else "a <meta>"
        return f"{m.group(1).decode('ascii', 'replace').lower()}, by {how}"
    return "utf-8, XML's default" if is_xml else "undeclared HTML: the locale's default"


def loaded_labels(root, path, data):
    """The labels declared by the files a test loads, two levels of stylesheets down."""
    labels = set()
    seen = {path}
    queue = [(path, data, 0)]
    while queue:
        base, body, depth = queue.pop()
        for m in LOADS.finditer(body):
            target = (m.group(1) or m.group(2)).decode("ascii", "replace")
            if "://" in target or target.startswith(("data:", "about:", "javascript:")):
                continue
            file = root / target.lstrip("/") if target.startswith("/") else base.parent / target
            if file in seen or not file.is_file():
                continue
            seen.add(file)
            more, more_data = declared(file)
            labels |= more
            if depth < 2 and file.suffix == ".css":
                queue.append((file, more_data, depth + 1))
    return labels


def main():
    parser = argparse.ArgumentParser(
        description=__doc__.splitlines()[0],
        epilog="\n".join(__doc__.splitlines()[1:]),
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    parser.add_argument("root", type=Path, help="the WPT checkout")
    parser.add_argument("list", type=Path, help="the tests, one /path per line")
    parser.add_argument("--list-legacy", type=Path, metavar="FILE",
                        help="write the tests that declare a legacy label to FILE")
    args = parser.parse_args()

    tests = [line.strip() for line in args.list.read_text(encoding="utf-8").splitlines() if line.strip()]
    documents = collections.Counter()
    non_ascii = collections.Counter()
    groups = collections.Counter()
    legacy_labels = collections.Counter()
    legacy = []
    ahem = ahem_css = dynamic = missing = 0
    for name in tests:
        path = args.root / name.lstrip("/")
        if not path.is_file():
            missing += 1
            continue
        labels, data = declared(path)
        how = document_encoding(path, data)
        documents[how] += 1
        non_ascii[how] += any(byte > 0x7F for byte in data)
        labels |= loaded_labels(args.root, path, data)
        found = {group(label) for label in labels}
        for name_of_group in found:
            groups[name_of_group] += 1
        if "legacy" in found:
            legacy.append(name)
            legacy_labels.update(label for label in labels if group(label) == "legacy")
        lower = data.lower()
        ahem += b"ahem" in lower
        ahem_css += b"/fonts/ahem.css" in lower
        dynamic += bool(DYNAMIC.search(data))

    print(f"{len(tests)} tests; {missing} missing from the checkout.\n")
    print("| The document's encoding | Tests | With bytes beyond ASCII |")
    print("|---|---|---|")
    for how, count in documents.most_common():
        print(f"| {how} | {count} | {non_ascii[how]} |")
    print("\n| Declared, with the files they load | Tests |")
    print("|---|---|")
    for name_of_group in GROUPS:
        print(f"| {name_of_group} | {groups[name_of_group]} |")
    print("\nLegacy labels: " + (", ".join(f"{label} {count}" for label, count in legacy_labels.most_common()) or "none"))
    by_directory = collections.Counter(name.lstrip("/").split("/")[2] if name.count("/") > 3 else "(top level)" for name in legacy)
    print("Tests with legacy labels, by directory: " + (", ".join(f"{d} {c}" for d, c in by_directory.most_common()) or "none"))
    print(f"\nName Ahem: {ahem}; link /fonts/ahem.css: {ahem_css}.")
    print(f"Use :hover, :active or :focus: {dynamic}.")
    if args.list_legacy:
        args.list_legacy.write_text("".join(f"{name}\n" for name in legacy), encoding="utf-8")
    return 0


if __name__ == "__main__":
    sys.exit(main())
