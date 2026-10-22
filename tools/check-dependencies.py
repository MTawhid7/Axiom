#!/usr/bin/env python3
"""Check the engine's crate graph against the crate map (CR16; DP22's tool).

Usage: tools/check-dependencies.py [--root DIR] [--self-test]

It reads `cargo metadata` for the engine's workspace (the repository root),
resolved for aarch64-apple-darwin with --locked, and tools/crate-map.toml,
and checks:
  CR2   every workspace package has an entry in the crate map, and every
        entry is well formed;
  CR1   each Axiom dependency is in its crate's allowlist (`deps`, or
        `dev-deps` for development dependencies), and normal and build
        dependencies point to the same layer or a lower one;
  CR3   no cycle among Axiom's crates, development dependencies included;
  CR4   each shipping executable's closure of normal dependencies: every Axiom
        crate in it lists the executable's process, every third-party crate
        is in a group the process admits, and none is on the executable's
        `forbid` list or the global one;
  CR12  no build or test crate in a shipping crate's dependencies or in a
        shipping executable's closure;
  DP9   a crate that depends on a `sandboxed` or `test` crate is in that group
        too, and no shipping crate depends on a `build` crate;
  CR14  no package is named like a built-in crate or a package in the lock;
  CR6   the crates allowed `unsafe` are the platform layer's, `jit` and
        `test-support`, only in src/sys/, matching tools/unsafe-policy.txt;
  ES10  each crate root declares the deny lints of its panic class;
  ES19  no Axiom crate has a build script;
  ES24  every dependency of a member is inherited from the workspace, and
        every member inherits the workspace's lints;
  DP22  every third-party crate the engine builds has a `third-party` entry
        with its group, is named in docs/technology/dependency-register.md,
        and has its build script or procedural macro acknowledged (DP12);
  CR15  every third-party crate that the spikes' workspace (and fuzz/'s, once
        it exists) builds is named in the register (DP23);
  DP15  each member's policy in supply-chain/config.toml matches its group and
        panic class: safe-to-run for build and test code; for shipped code,
        safe-to-deploy and crypto-safe, with ub-risk-1 for a panic-free crate
        (it handles less trusted data in the browser, DP9, DP10), ub-risk-2
        for another trusted crate, ub-risk-3 for a sandboxed one (DP8).

The resolved graph unifies features across the workspace's members, so the
placement checks over-approximate one executable's build and err toward
refusing (CR4).

--self-test runs the checks on a synthetic workspace, compliant and then with
each rule's violation seeded alone, without running cargo.

Prints one line per violation and a summary. Exit status: 0 compliant,
1 violations, 2 usage or environment error. Uses only the Python standard
library.
"""

import argparse
import copy
import json
import re
import subprocess
import sys
import tempfile
import tomllib
from pathlib import Path

TARGET = "aarch64-apple-darwin"
BUILTIN_CRATES = {"core", "std", "alloc", "proc_macro", "test"}
PROCESSES = {"browser", "renderer", "network", "gpu", "utility"}
GROUPS = {"trusted", "sandboxed", "build", "test"}
# DP9: the browser admits `trusted` code only; the sandboxed helpers admit both.
ADMITS = {"browser": {"trusted"}, "renderer": {"trusted", "sandboxed"}, "network": {"trusted", "sandboxed"},
          "gpu": {"trusted", "sandboxed"}, "utility": {"trusted", "sandboxed"}}
# ES10: the deny lints each panic class's crate root declares.
PANIC_LINTS = {
    "panic-free": {"clippy::indexing_slicing", "clippy::unwrap_used", "clippy::expect_used",
                   "clippy::arithmetic_side_effects", "clippy::panic"},
    "checked-arithmetic": {"clippy::arithmetic_side_effects"},
    "ordinary": set(),
}
# CR6, ES11: besides the platform layer, only these may hold `unsafe`.
UNSAFE_OUTSIDE_PLATFORM = {"jit", "test-support"}
SHIPPING_LAYERS = set(range(7))
OUT_OF_LAYER = {"build", "test"}
ENTRY_KEYS = {"layer", "processes", "group", "deps", "dev-deps", "unsafe", "panic", "milestone", "ships-as", "forbid"}
DENY = re.compile(r"#!\[deny\(([^\]]*)\)\]", re.S)


def run_metadata(manifest):
    command = ["cargo", "metadata", "--format-version", "1", "--filter-platform", TARGET, "--locked",
               "--manifest-path", str(manifest)]
    result = subprocess.run(command, capture_output=True, text=True, check=False)
    if result.returncode != 0:
        print(f"check-dependencies: `{' '.join(command)}` failed:\n{result.stderr.strip()}", file=sys.stderr)
        sys.exit(2)
    return json.loads(result.stdout)


class Graph:
    """One workspace's resolved graph: packages, members and edges with their kinds."""

    def __init__(self, metadata):
        self.packages = {p["id"]: p for p in metadata["packages"]}
        self.member_ids = set(metadata["workspace_members"])
        self.members = {self.packages[i]["name"]: self.packages[i] for i in self.member_ids}
        self.edges = {}
        for node in metadata["resolve"]["nodes"]:
            self.edges[node["id"]] = [(dep["pkg"], {k.get("kind") or "normal" for k in dep["dep_kinds"]})
                                      for dep in node["deps"]]

    def name(self, package_id):
        return self.packages[package_id]["name"]

    def third_party(self):
        return sorted((p for i, p in self.packages.items() if i in self.edges and i not in self.member_ids),
                      key=lambda p: (p["name"], p["version"]))

    def normal_closure(self, root_id):
        seen, stack = set(), [root_id]
        while stack:
            for dep, kinds in self.edges.get(stack.pop(), []):
                if "normal" in kinds and dep not in seen:
                    seen.add(dep)
                    stack.append(dep)
        return seen


def named_in(text, crate):
    return re.search(r"(?<![\w-])" + re.escape(crate) + r"(?![\w-])", text) is not None


class Checker:
    """Every check over one workspace's graph, the crate map and the policy files."""

    def __init__(self, metadata, cmap, policy_text, register_text, root, vet=None):
        self.graph = Graph(metadata)
        self.vet_policies = (vet or {}).get("policy", {})
        self.crates = cmap.get("crate", {})
        self.third = cmap.get("third-party", {})
        self.shipping = cmap.get("shipping", {})
        self.policy = {f[1]: f[2] for f in (line.split() for line in policy_text.splitlines())
                       if len(f) >= 3 and f[0] == "allow"}
        self.register = register_text
        self.root = Path(root)
        self.lines = []

    def add(self, rule, message):
        self.lines.append(f"{rule:5} {message}")

    def group(self, package_id):
        name = self.graph.name(package_id)
        if package_id in self.graph.member_ids:
            return self.crates.get(name, {}).get("group")
        return self.third.get(name, {}).get("group")

    def run(self, others=()):
        self.validate_map()
        for name, package in sorted(self.graph.members.items()):
            self.check_member(name, package)
        self.check_cycles()
        self.check_placement()
        self.check_third_party()
        for label, metadata in others:
            for package in Graph(metadata).third_party():
                if not named_in(self.register, package["name"]):
                    self.add("CR15", f"{label}: third-party {package['name']} {package['version']}: not named in the register")
        return self.lines

    def validate_map(self):
        for name, entry in sorted(self.crates.items()):
            if set(entry) - ENTRY_KEYS:
                self.add("CR2", f"crate-map: {name}: unknown keys {sorted(set(entry) - ENTRY_KEYS)}")
            layer, group = entry.get("layer"), entry.get("group")
            if layer not in SHIPPING_LAYERS and layer not in OUT_OF_LAYER:
                self.add("CR2", f"crate-map: {name}: layer {layer!r} is not 0-6, \"build\" or \"test\"")
            if group not in GROUPS:
                self.add("CR2", f"crate-map: {name}: group {group!r} is not one of {sorted(GROUPS)}")
            elif layer in OUT_OF_LAYER and group != layer:
                self.add("CR2", f"crate-map: {name}: a {layer} crate is in the {layer} group")
            if entry.get("panic") not in PANIC_LINTS:
                self.add("CR2", f"crate-map: {name}: panic class {entry.get('panic')!r} is not one of {sorted(PANIC_LINTS)}")
            for process in entry.get("processes", []):
                if process not in PROCESSES:
                    self.add("CR2", f"crate-map: {name}: unknown process {process!r}")
            for key in ("deps", "dev-deps"):
                for dep in entry.get(key, []):
                    if dep not in self.crates:
                        self.add("CR2", f"crate-map: {name}: {key} names {dep!r}, which has no entry")
            ships = entry.get("ships-as")
            if ships is not None and (layer != 6 or ships not in PROCESSES or entry.get("processes") != [ships]):
                self.add("CR2", f"crate-map: {name}: a shipping executable is in layer 6 and links into its one process")
            if ships is None and entry.get("forbid"):
                self.add("CR2", f"crate-map: {name}: only a shipping executable has a forbid list")
        for name, entry in sorted(self.third.items()):
            if entry.get("group") not in GROUPS:
                self.add("DP9", f"crate-map: third-party {name}: group {entry.get('group')!r} is not one of {sorted(GROUPS)}")

    def check_member(self, name, package):
        """CR2, CR14, ES19, ES24, CR6, ES10, then each of the member's edges."""
        entry = self.crates.get(name)
        if entry is None:
            self.add("CR2", f"{name}: no entry in tools/crate-map.toml")
            return
        if name in BUILTIN_CRATES or name.replace("-", "_") in BUILTIN_CRATES:
            self.add("CR14", f"{name}: named like Rust's built-in crate")
        if name in {p["name"] for p in self.graph.third_party()}:
            self.add("CR14", f"{name}: a third-party package in the lock has the same name")
        if any("custom-build" in t["kind"] for t in package["targets"]):
            self.add("ES19", f"{name}: has a build script; generated code comes from codegen, committed")
        self.check_manifest(name, Path(package["manifest_path"]))
        self.check_unsafe(name, entry, Path(package["manifest_path"]).parent)
        self.check_panic_class(name, entry, package)
        self.check_vet_policy(name, entry)
        for dep_id, kinds in self.graph.edges.get(package["id"], []):
            self.check_edge(name, entry, dep_id, kinds)

    def check_edge(self, name, entry, dep_id, kinds):
        dep_name, dep_group = self.graph.name(dep_id), self.group(dep_id)
        if dep_id in self.graph.member_ids:
            dep_layer, layer = self.crates.get(dep_name, {}).get("layer"), entry.get("layer")
            if kinds - {"dev"} and dep_name not in entry.get("deps", []):
                self.add("CR1", f"{name} -> {dep_name}: not in {name}'s allowlist (`deps`)")
            if kinds == {"dev"} and dep_name not in entry.get("deps", []) + entry.get("dev-deps", []):
                self.add("CR1", f"{name} -> {dep_name} (dev): not in {name}'s `deps` or `dev-deps`")
            if kinds - {"dev"} and layer in SHIPPING_LAYERS:
                if dep_layer in OUT_OF_LAYER:
                    self.add("CR12", f"{name} -> {dep_name}: a shipping crate depends on a {dep_layer} crate")
                elif isinstance(dep_layer, int) and dep_layer > layer:
                    self.add("CR1", f"{name} (layer {layer}) -> {dep_name} (layer {dep_layer}): depends upward")
        if "normal" not in kinds:
            return
        group = entry.get("group")
        if dep_group == "sandboxed" and group not in {"sandboxed", "test"}:
            self.add("DP9", f"{name} ({group}) -> {dep_name} (sandboxed): a crate that depends on a sandboxed crate is sandboxed")
        if dep_group == "test" and group != "test":
            self.add("DP9", f"{name} ({group}) -> {dep_name} (test): only test code depends on a test crate")
        if dep_group == "build" and group not in {"build", "test"}:
            self.add("DP9", f"{name} ({group}) -> {dep_name} (build): a shipping crate links a build crate")

    def check_manifest(self, name, manifest):
        try:
            data = tomllib.loads(manifest.read_text(encoding="utf-8"))
        except (OSError, tomllib.TOMLDecodeError) as error:
            self.add("ES24", f"{name}: can't read {manifest}: {error}")
            return
        if data.get("lints", {}).get("workspace") is not True:
            self.add("ES24", f"{name}: [lints] doesn't inherit the workspace's (`workspace = true`)")
        for table in ("dependencies", "dev-dependencies", "build-dependencies"):
            for dep, spec in sorted(data.get(table, {}).items()):
                if not (isinstance(spec, dict) and spec.get("workspace") is True):
                    self.add("ES24", f"{name}: [{table}] {dep} isn't inherited from [workspace.dependencies]")

    def check_unsafe(self, name, entry, crate_dir):
        rel = crate_dir.relative_to(self.root).as_posix()
        flagged, listed = bool(entry.get("unsafe")), rel in self.policy
        if flagged != listed:
            self.add("CR6", f"{name}: crate-map says unsafe = {str(flagged).lower()}, but tools/unsafe-policy.txt "
                            f"{'lists' if listed else 'does not list'} {rel}")
        if flagged and entry.get("layer") != 1 and name not in UNSAFE_OUTSIDE_PLATFORM:
            self.add("CR6", f"{name}: only the platform layer, jit and test-support may hold unsafe")
        if listed and self.policy[rel] != "src/sys/":
            self.add("CR6", f"{name}: unsafe is allowed only in src/sys/, not {self.policy[rel]}")

    def check_panic_class(self, name, entry, package):
        required = PANIC_LINTS.get(entry.get("panic"), set())
        for target in package["targets"]:
            if not required or not {"lib", "bin"} & set(target["kind"]):
                continue
            path = Path(target["src_path"])
            try:
                text = path.read_text(encoding="utf-8")
            except OSError as error:
                self.add("ES10", f"{name}: can't read {path}: {error}")
                continue
            declared = {lint.strip() for group in DENY.findall(text) for lint in group.split(",")}
            if required - declared:
                self.add("ES10", f"{name}: {path.name} doesn't deny {', '.join(sorted(required - declared))}")

    def check_vet_policy(self, name, entry):
        group, panic = entry.get("group"), entry.get("panic")
        if group in OUT_OF_LAYER:
            expected = {"safe-to-run"}
        else:
            grade = "ub-risk-1" if panic == "panic-free" else "ub-risk-2" if group == "trusted" else "ub-risk-3"
            expected = {"safe-to-deploy", grade, "crypto-safe"}
        criteria = self.vet_policies.get(name, {}).get("criteria")
        found = {criteria} if isinstance(criteria, str) else set(criteria or [])
        if found != expected:
            self.add("DP15", f"{name}: supply-chain/config.toml's policy is {sorted(found) or 'missing'}, "
                             f"its group and panic class need {sorted(expected)}")

    def check_cycles(self):
        state, path = {}, []

        def visit(node):
            state[node] = "open"
            path.append(node)
            for dep, _ in self.graph.edges.get(node, []):
                if dep not in self.graph.member_ids:
                    continue
                if state.get(dep) == "open":
                    cycle = path[path.index(dep):] + [dep]
                    self.add("CR3", "cycle: " + " -> ".join(self.graph.name(n) for n in cycle))
                elif dep not in state:
                    visit(dep)
            path.pop()
            state[node] = "done"

        for node in sorted(self.graph.member_ids):
            if node not in state:
                visit(node)

    def check_placement(self):
        global_forbid = self.shipping.get("forbid", [])
        forbid_groups = set(self.shipping.get("forbid-groups", []))
        for name, entry in sorted(self.crates.items()):
            process = entry.get("ships-as")
            if process not in ADMITS or name not in self.graph.members:
                continue
            closure = self.graph.normal_closure(self.graph.members[name]["id"])
            for dep_id in sorted(closure, key=self.graph.name):
                dep_name, group = self.graph.name(dep_id), self.group(dep_id)
                if dep_name in entry.get("forbid", []):
                    self.add("CR4", f"{name}: links {dep_name}, which its forbid list excludes")
                if any(dep_name == p or (p.endswith("*") and dep_name.startswith(p[:-1])) for p in global_forbid):
                    self.add("CR4", f"{name}: links {dep_name}, which no shipping executable may link")
                if dep_id in self.graph.member_ids and process not in self.crates.get(dep_name, {}).get("processes", []):
                    self.add("CR4", f"{name} ({process}): links {dep_name}, which doesn't list {process}")
                if group in forbid_groups:
                    self.add("CR12", f"{name}: links {dep_name}, a {group} crate")
                elif group is not None and group not in ADMITS[process]:
                    self.add("CR4", f"{name} ({process}): links {dep_name}, a {group} crate the {process} doesn't admit")

    def check_third_party(self):
        for package in self.graph.third_party():
            name = package["name"]
            entry = self.third.get(name)
            if entry is None:
                self.add("DP22", f"third-party {name} {package['version']}: no `third-party` entry in tools/crate-map.toml")
                entry = {}
            if not named_in(self.register, name):
                self.add("DP22", f"third-party {name}: not named in docs/technology/dependency-register.md")
            kinds = {kind for target in package["targets"] for kind in target["kind"]}
            if "custom-build" in kinds and entry.get("build-script") is not True:
                self.add("DP12", f"third-party {name}: has a build script; acknowledge it with build-script = true")
            if "proc-macro" in kinds and entry.get("proc-macro") is not True:
                self.add("DP12", f"third-party {name}: is a procedural macro; acknowledge it with proc-macro = true")


# The self-test: a synthetic workspace written to a temporary directory.

def fixture():
    """A compliant workspace: (crates, third-party packages, crate map, policy, register, vet)."""
    crates = {
        # name: (kinds, root text, manifest dependencies, edges as (dependency, kinds))
        "base": (["lib"], "", {"tp-trusted"}, [("tp-trusted", {"normal"}), ("support", {"dev"})]),
        "helper": (["lib"], "#![deny(clippy::arithmetic_side_effects)]", {"base", "tp-sandboxed"},
                   [("base", {"normal"}), ("tp-sandboxed", {"normal"})]),
        "app": (["bin"], "", {"helper", "base"}, [("helper", {"normal"}), ("base", {"normal"})]),
        "brow": (["bin"], "", {"base"}, [("base", {"normal"})]),
        "support": (["lib"], "", set(), []),
    }
    third = {"tp-trusted": ["lib"], "tp-sandboxed": ["lib"]}
    cmap = {
        "crate": {
            "base": {"layer": 0, "processes": ["browser", "renderer"], "group": "trusted", "deps": [],
                     "dev-deps": ["support"], "unsafe": False, "panic": "ordinary"},
            "helper": {"layer": 3, "processes": ["renderer"], "group": "sandboxed", "deps": ["base"],
                       "unsafe": False, "panic": "checked-arithmetic"},
            "app": {"layer": 6, "processes": ["renderer"], "ships-as": "renderer", "group": "sandboxed",
                    "deps": ["helper", "base"], "unsafe": False, "panic": "ordinary"},
            "brow": {"layer": 6, "processes": ["browser"], "ships-as": "browser", "group": "trusted", "deps": ["base"],
                     "forbid": ["helper"], "unsafe": False, "panic": "ordinary"},
            "support": {"layer": "test", "processes": [], "group": "test", "deps": [], "unsafe": False,
                        "panic": "ordinary"},
        },
        "third-party": {"tp-trusted": {"group": "trusted"}, "tp-sandboxed": {"group": "sandboxed"}},
        "shipping": {"forbid-groups": ["build", "test"], "forbid": ["bindings-*"]},
    }
    shipped = ["safe-to-deploy", "crypto-safe"]
    vet = {"policy": {
        "base": {"criteria": [*shipped, "ub-risk-2"]},
        "helper": {"criteria": [*shipped, "ub-risk-3"]},
        "app": {"criteria": [*shipped, "ub-risk-3"]},
        "brow": {"criteria": [*shipped, "ub-risk-2"]},
        "support": {"criteria": "safe-to-run"},
    }}
    return crates, third, cmap, "", "Register: `tp-trusted` and `tp-sandboxed`; spikes: `tp-spike`.", vet


def materialize(tmp, crates, third):
    """Writes the crates' files under tmp and returns cargo-metadata-shaped JSON."""
    packages, nodes, members = [], [], []
    third_ids = {name: f"registry+https://github.com/rust-lang/crates.io-index#{name}@1.0.0" for name in third}
    ids = third_ids | {name: f"path+file://{tmp}/crates/{name}#0.0.0" for name in crates}
    for name, (kinds, root_text, manifest_deps, edges) in crates.items():
        crate_dir = Path(tmp, "crates", name)
        (crate_dir / "src").mkdir(parents=True, exist_ok=True)
        root = crate_dir / "src" / ("main.rs" if "bin" in kinds else "lib.rs")
        root.write_text(root_text + "\n", encoding="utf-8")
        deps = "".join(f"{d}.workspace = true\n" for d in sorted(manifest_deps))
        (crate_dir / "Cargo.toml").write_text(f"[package]\nname = \"{name}\"\n\n[dependencies]\n{deps}\n[lints]\n"
                                              "workspace = true\n", encoding="utf-8")
        targets = [{"kind": [k], "name": name, "src_path": str(root)} for k in kinds]
        packages.append({"id": ids[name], "name": name, "version": "0.0.0", "source": None,
                         "manifest_path": str(crate_dir / "Cargo.toml"), "targets": targets})
        members.append(ids[name])
        nodes.append({"id": ids[name], "deps": [{"pkg": ids[d], "dep_kinds": [
            {"kind": None if k == "normal" else k} for k in sorted(ks)]} for d, ks in edges]})
    for name, kinds in third.items():
        packages.append({"id": third_ids[name], "name": name, "version": "1.0.0", "source": "registry+crates-io",
                         "manifest_path": "/nonexistent/Cargo.toml",
                         "targets": [{"kind": [k], "name": name, "src_path": "/nonexistent"} for k in kinds]})
        nodes.append({"id": third_ids[name], "deps": []})
    return {"packages": packages, "workspace_members": members, "resolve": {"nodes": nodes}}


def seeded_cases():
    """Each case: (description, mutation, the text a reported line must contain)."""

    def edge(src, dst, kinds, manifest=True, allow=True):
        def mutate(crates, third, cmap, state):
            crates[src][3].append((dst, set(kinds)))
            if manifest:
                crates[src][2].add(dst)
            if allow and dst in cmap["crate"]:
                cmap["crate"][src].setdefault("deps" if kinds != {"dev"} else "dev-deps", []).append(dst)
        return mutate

    def entry(name, **fields):
        def mutate(crates, third, cmap, state):
            cmap["crate"][name].update(fields)
        return mutate

    def chain(*mutations):
        def mutate(crates, third, cmap, state):
            for m in mutations:
                m(crates, third, cmap, state)
        return mutate

    def new_crate(name, kinds=("lib",), root_text="", map_entry=None, edges=()):
        def mutate(crates, third, cmap, state):
            crates[name] = (list(kinds), root_text, set(), list(edges))
            if map_entry is not None:
                cmap["crate"][name] = map_entry
        return mutate

    def third_party(name, kinds=("lib",), group=None):
        def mutate(crates, third, cmap, state):
            third[name] = list(kinds)
            if group:
                cmap["third-party"][name] = {"group": group}
        return mutate

    def state(key, value):
        def mutate(crates, third, cmap, st):
            st[key] = value
        return mutate

    lib = {"layer": 3, "processes": ["renderer"], "group": "sandboxed", "deps": [], "unsafe": False, "panic": "ordinary"}
    return [
        ("a disallowed edge", edge("brow", "helper", {"normal"}, allow=False), "CR1   brow -> helper: not in brow's allowlist"),
        ("an upward edge", edge("base", "helper", {"normal"}), "depends upward"),
        ("a cycle through a development dependency", chain(edge("support", "base", {"normal"})), "CR3   cycle:"),
        ("a test crate in a shipping closure", edge("app", "support", {"normal"}), "CR12  app: links support, a test crate"),
        ("a crate outside its processes", entry("base", processes=["renderer"]), "links base, which doesn't list browser"),
        ("a forbidden crate", chain(entry("helper", processes=["renderer", "browser"], group="trusted"),
                                    state("helper-trusted", True), edge("brow", "helper", {"normal"})),
         "links helper, which its forbid list excludes"),
        ("a globally forbidden crate", chain(new_crate("bindings-dom", map_entry=dict(lib)),
                                             edge("app", "bindings-dom", {"normal"})), "which no shipping executable may link"),
        ("a sandboxed crate in the browser", edge("brow", "tp-sandboxed", {"normal"}), "a sandboxed crate the browser doesn't admit"),
        ("a trusted dependent of a sandboxed crate", entry("helper", group="trusted"), "DP9   helper (trusted) -> tp-sandboxed"),
        ("a package without an entry", new_crate("stray"), "CR2   stray: no entry"),
        ("a malformed entry", entry("base", layer=9), "layer 9 is not 0-6"),
        ("a built-in crate's name", new_crate("core", map_entry=dict(lib, layer=0, group="trusted")), "CR14  core"),
        ("a name taken in the lock", chain(new_crate("tp-trusted-x", map_entry=dict(lib)),
                                           third_party("tp-trusted-x", group="trusted")), "a third-party package in the lock"),
        ("unsafe flagged but not listed", entry("base", unsafe=True), "CR6   base: crate-map says unsafe = true"),
        ("unsafe outside the platform layer", chain(entry("helper", unsafe=True), state("policy", "allow crates/helper src/sys/\n")),
         "only the platform layer, jit and test-support"),
        ("unsafe outside src/sys/", chain(entry("base", unsafe=True, layer=1), state("policy", "allow crates/base src/raw/\n")),
         "only in src/sys/"),
        ("a missing panic-class lint", entry("app", panic="panic-free"), "ES10  app: main.rs doesn't deny"),
        ("a build script", new_crate("scripted", kinds=("lib", "custom-build"), map_entry=dict(lib)), "ES19  scripted"),
        ("an uninherited dependency", state("raw-dependency", True), "isn't inherited from [workspace.dependencies]"),
        ("an unmapped, unregistered third-party crate", chain(third_party("tp-new"), edge("base", "tp-new", {"normal"})),
         "DP22  third-party tp-new 1.0.0: no `third-party` entry"),
        ("an unacknowledged build script", third_party("tp-cc", kinds=("lib", "custom-build"), group="build"), "DP12  third-party tp-cc"),
        ("an unregistered crate in the spikes' lock", state("spike-crate", "tp-unnamed"), "CR15  spikes: third-party tp-unnamed"),
        ("a vet policy too weak for its crate", state("vet", {"helper": {"criteria": ["safe-to-deploy", "crypto-safe", "ub-risk-4"]}}),
         "DP15  helper: supply-chain/config.toml's policy"),
        ("a panic-free crate without ub-risk-1", entry("base", panic="panic-free"),
         "DP15  base: supply-chain/config.toml's policy"),
    ]


def run_case(module_check, mutate=None):
    crates, third, cmap, policy, register, vet = fixture()
    crates, third, cmap, vet = copy.deepcopy(crates), copy.deepcopy(third), copy.deepcopy(cmap), copy.deepcopy(vet)
    state = {}
    if mutate:
        mutate(crates, third, cmap, state)
    for name in [n for n in crates if n not in vet["policy"] and n in cmap["crate"]]:
        entry = cmap["crate"][name]
        grade = "ub-risk-2" if entry.get("group") == "trusted" else "ub-risk-3"
        vet["policy"][name] = {"criteria": ["safe-to-deploy", "crypto-safe", grade]}
    vet["policy"].update(state.get("vet", {}))
    with tempfile.TemporaryDirectory(prefix="check-dependencies-") as tmp:
        tmp = str(Path(tmp).resolve())
        metadata = materialize(tmp, crates, third)
        if state.get("raw-dependency"):
            manifest = Path(tmp, "crates/base/Cargo.toml")
            manifest.write_text(manifest.read_text(encoding="utf-8").replace("tp-trusted.workspace = true",
                                                                               "tp-trusted = \"1\""), encoding="utf-8")
        others = []
        if "spike-crate" in state:
            others.append(("spikes", materialize(tmp + "/spikes", {}, {state["spike-crate"]: ["lib"], "tp-spike": ["lib"]})))
        return module_check(metadata, cmap, state.get("policy", policy), register, tmp, others, vet)


def self_test():
    def check(metadata, cmap, policy, register, root, others, vet):
        return Checker(metadata, cmap, policy, register, root, vet).run(others)

    failures = 0
    compliant = run_case(check)
    if compliant:
        failures += 1
        print("self-test FAILED: the compliant workspace was refused:\n  " + "\n  ".join(compliant))
    cases = seeded_cases()
    for description, mutate, expected in cases:
        lines = run_case(check, mutate)
        if not any(expected in line for line in lines):
            failures += 1
            print(f"self-test FAILED: {description}: expected a line containing {expected!r}, got:\n  "
                  + ("\n  ".join(lines) or "(nothing)"))
    if failures:
        return 1
    print(f"check-dependencies self-test: OK (compliant workspace accepted; {len(cases)}/{len(cases)} seeded violations reported)")
    return 0


def main():
    default_root = Path(__file__).resolve().parent.parent
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0], epilog=__doc__.split("\n\n", 1)[1],
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--root", type=Path, default=default_root, help="repository root (default: this script's parent)")
    parser.add_argument("--self-test", action="store_true", help="check the checker on a synthetic workspace")
    args = parser.parse_args()
    if args.self_test:
        return self_test()
    root = args.root.resolve()
    try:
        cmap = tomllib.loads((root / "tools/crate-map.toml").read_text(encoding="utf-8"))
        policy = (root / "tools/unsafe-policy.txt").read_text(encoding="utf-8")
        register = (root / "docs/technology/dependency-register.md").read_text(encoding="utf-8")
        vet = tomllib.loads((root / "supply-chain/config.toml").read_text(encoding="utf-8"))
    except (OSError, tomllib.TOMLDecodeError) as error:
        print(f"check-dependencies: {error}", file=sys.stderr)
        return 2
    others = [(label, run_metadata(manifest)) for label, manifest in
              (("spikes", root / "spikes/Cargo.toml"), ("fuzz", root / "fuzz/Cargo.toml")) if manifest.exists()]
    lines = Checker(run_metadata(root / "Cargo.toml"), cmap, policy, register, root, vet).run(others)
    for line in lines:
        print(line)
    print(f"check-dependencies: {len(lines)} violation(s)" if lines else "check-dependencies: OK")
    return 1 if lines else 0


if __name__ == "__main__":
    sys.exit(main())
