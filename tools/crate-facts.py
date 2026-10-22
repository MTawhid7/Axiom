#!/usr/bin/env python3
"""Dated facts about crates, for the dependency register (DP6) and its reviews.

    python3 -I tools/crate-facts.py CRATE [CRATE ...] [--advisory-db DIR]
                                    [--audits FILE [...]] [--github]
                                    [--json FILE]

For each crate it reads crates.io's API: the newest version that isn't
yanked, and its date; the first version's date; how many versions there are;
all-time and recent downloads; and the repository. With --github, `gh api`
reads the repository's last push, whether it is archived, and its open
issues. With --advisory-db, a local checkout of RustSec's advisory database
gives the crate's advisories: their IDs, dates, kinds (a vulnerability, or an
informational notice: unmaintained, unsound), whether withdrawn, and the
versions that fix them. With --audits, each cargo-vet audits.toml named
(Google's, Mozilla's, Chromium's, the Bytecode Alliance's...) gives the
`ub-risk-N` grades it records for the crate, with the versions or deltas they
cover; give each as NAME=PATH to label it. It prints a Markdown table, and with
--json writes everything it read.

crates.io asks automated clients to identify themselves and to make at most
one request a second; the tool does both. What it downloads is data, never
run: use -I.

Used for the architecture review's Round 4 (the reuse audit, and the dated
facts of the crates on Axiom's path).
"""

import argparse
import json
import re
import subprocess
import sys
import time
import tomllib
import urllib.request
from pathlib import Path

AGENT = "Axiom architecture review (https://github.com/MTawhid7/Axiom; tools/crate-facts.py)"
GRADE = re.compile(r"ub-risk-\d")
FENCE = re.compile(r"```toml\n(.*?)```", re.S)


def crates_io(name):
    """crates.io's record for a crate, or an error string."""
    request = urllib.request.Request(f"https://crates.io/api/v1/crates/{name}", headers={"User-Agent": AGENT})
    try:
        with urllib.request.urlopen(request, timeout=30) as response:
            data = json.load(response)
    except Exception as error:  # noqa: BLE001: any failure is reported in the table
        return {"error": str(error)}
    crate = data.get("crate", {})
    versions = [v for v in data.get("versions", []) if isinstance(v, dict)]
    live = [v for v in versions if not v.get("yanked")]
    newest = max(live, key=lambda v: v.get("created_at", ""), default={})
    first = min(versions, key=lambda v: v.get("created_at", ""), default={})
    return {
        "newest": newest.get("num"),
        "max_stable": crate.get("max_stable_version"),
        "newest_date": (newest.get("created_at") or "")[:10],
        "first_date": (first.get("created_at") or "")[:10],
        "versions": len(versions),
        "downloads": crate.get("downloads"),
        "recent_downloads": crate.get("recent_downloads"),
        "repository": crate.get("repository") or "",
    }


def github(repository):
    """The repository's last push, archive flag and open issues, through `gh api`."""
    match = re.match(r"https?://github\.com/([^/]+)/([^/#?]+)", repository or "")
    if not match:
        return {}
    owner, repo = match.group(1), match.group(2).removesuffix(".git")
    try:
        output = subprocess.run(
            ["gh", "api", f"repos/{owner}/{repo}", "--jq", "{pushed_at, archived, open_issues_count}"],
            capture_output=True, text=True, timeout=60, check=False)
    except (OSError, subprocess.TimeoutExpired) as error:
        return {"error": str(error)}
    if output.returncode != 0:
        return {"error": output.stderr.strip()[:120]}
    facts = json.loads(output.stdout)
    facts["pushed_at"] = (facts.get("pushed_at") or "")[:10]
    return facts


def advisories(db, name):
    """RustSec's advisories for a crate, from a local checkout of advisory-db."""
    found = []
    for path in sorted((db / "crates" / name).glob("RUSTSEC-*.md")):
        match = FENCE.search(path.read_text(encoding="utf-8"))
        if not match:
            continue
        try:
            record = tomllib.loads(match.group(1))
        except tomllib.TOMLDecodeError:
            continue
        advisory, versions = record.get("advisory", {}), record.get("versions", {})
        found.append({
            "id": advisory.get("id", path.stem),
            "date": str(advisory.get("date", "")),
            "kind": advisory.get("informational") or "vulnerability",
            "withdrawn": bool(advisory.get("withdrawn")),
            "patched": versions.get("patched", []),
        })
    return found


def grades(audit_files, name):
    """The ub-risk grades each audits.toml records for a crate."""
    found = []
    for label, path in audit_files:
        try:
            data = tomllib.loads(Path(path).read_text(encoding="utf-8"))
        except (OSError, tomllib.TOMLDecodeError) as error:
            found.append({"source": label, "error": str(error)})
            continue
        for entry in data.get("audits", {}).get(name, []):
            criteria = entry.get("criteria", [])
            criteria = [criteria] if isinstance(criteria, str) else criteria
            for criterion in criteria:
                if GRADE.fullmatch(criterion):
                    found.append({"source": label, "grade": criterion,
                                  "version": entry.get("version") or entry.get("delta", "")})
    return found


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0], epilog=__doc__.split("\n\n", 1)[1],
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("crates", nargs="+", help="crate names")
    parser.add_argument("--advisory-db", type=Path, help="a checkout of github.com/rustsec/advisory-db")
    parser.add_argument("--audits", nargs="*", default=[], metavar="NAME=PATH", help="cargo-vet audits.toml files")
    parser.add_argument("--github", action="store_true", help="also read each repository through `gh api`")
    parser.add_argument("--json", type=Path, help="write everything read to this file")
    args = parser.parse_args()
    audit_files = [tuple(a.split("=", 1)) if "=" in a else (Path(a).stem, a) for a in args.audits]

    rows = []
    for index, name in enumerate(args.crates):
        if index:
            time.sleep(1.0)
        facts = {"crate": name, "crates_io": crates_io(name)}
        if args.github:
            facts["github"] = github(facts["crates_io"].get("repository"))
        if args.advisory_db:
            facts["advisories"] = advisories(args.advisory_db, name)
        if audit_files:
            facts["grades"] = grades(audit_files, name)
        rows.append(facts)
        print(f"read {name}", file=sys.stderr)

    print("| Crate | Newest | Stable | Published | First | Versions | Recent downloads | Repository pushed | Advisories | Grades |")
    print("|---|---|---|---|---|---|---|---|---|---|")
    for facts in rows:
        c, g = facts["crates_io"], facts.get("github", {})
        if "error" in c:
            print(f"| `{facts['crate']}` | crates.io: {c['error']} | | | | | | | | |")
            continue
        pushed = g.get("pushed_at", "") + (" (archived)" if g.get("archived") else "")
        adv = "; ".join(f"{a['id']} {a['kind']}{' (withdrawn)' if a['withdrawn'] else ''}"
                        for a in facts.get("advisories", [])) or ("none" if args.advisory_db else "")
        grade = "; ".join(f"{x.get('grade')} {x.get('version')} ({x['source']})"
                          for x in facts.get("grades", []) if "grade" in x) or ("none" if audit_files else "")
        print(f"| `{facts['crate']}` | {c['newest']} | {c['max_stable']} | {c['newest_date']} | {c['first_date']} | {c['versions']} "
              f"| {c['recent_downloads']} | {pushed} | {adv} | {grade} |")
    if args.json:
        args.json.write_text(json.dumps(rows, indent=1), encoding="utf-8")


if __name__ == "__main__":
    main()
