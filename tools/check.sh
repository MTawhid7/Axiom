#!/usr/bin/env bash
# The repository's checks, in two tiers from one entry point (the engineering
# standards' ES25).
#
# Usage: tools/check.sh [--checkpoint] [--keep-going] [--help]
#
# The commit tier, before every commit:
#   the engine's workspace:  cargo fmt --check; cargo clippy -D warnings;
#                            cargo test (--locked)
#   the spikes' workspace:   the same three, while the spikes are kept
#   the repository's tools:  check-unsafe-policy.sh, check-doc-links.py,
#                            check-dependencies.py (CR16), check-file-size.py
#                            (ES1), each checker's self-test and the audit
#                            census's (unsafe-sites.py), the TODOs' form
#                            (ES17), and generated code fresh (ES19) once
#                            codegen exists
#   first-tier costs run optimized: trace's recording (OB25, OB27)
# Its time is budgeted, about 3 minutes for an incremental change (a
# hypothesis, measured at M1-CP2); past it, slow suites move to the
# checkpoint tier, never skipped.
#
# The checkpoint tier (--checkpoint) adds, before a checkpoint's last commit:
#   cargo doc, warning-free; Miri on the crates that hold `unsafe` (TS11);
#   cargo deny check and cargo vet (DP15, DP16); the TODO list (ES17);
#   and, as the checkpoints that bring them arrive, the conformance suites,
#   PF5's ratchet, loom, a short fuzzing run of each target and T1-T4, each
#   listed as pending until then.
#
# Every step runs even after one fails with --keep-going; by default the
# tier stops at the first failure. Each step's output goes to
# target/check/<timestamp>/<step>.log, and the summary gives each step's
# status and time, and the tier's total against its budget.
#
# Exit status: 0 every step passed, 1 a step failed, 2 usage error.
# Written for bash 3.2 (macOS /bin/bash).
set -uo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd)
cd "$ROOT" || exit 2

NIGHTLY=nightly-2026-10-21 # the plan's §2, item 6 (S12)
DENY_VERSION=0.20.2
VET_VERSION=0.10.2
COMMIT_BUDGET=180 # seconds: ES25's hypothesis
SPIKES=(--manifest-path "$ROOT/spikes/Cargo.toml")

tier=commit keep_going=no
for arg in "$@"; do
  case $arg in
    --checkpoint) tier=checkpoint ;;
    --keep-going) keep_going=yes ;;
    -h | --help) sed -n '2,33p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
    *) echo "unknown argument: $arg (see --help)" >&2; exit 2 ;;
  esac
done

LOGS="$ROOT/target/check/$(date +%Y%m%d-%H%M%S)"
mkdir -p "$LOGS"
# cargo-vet, and later cargo-fuzz, live under target/tools (tools/install-tools.sh).
export PATH="$ROOT/target/tools/bin:$PATH"

failed=0 summary="" started=$SECONDS

# step <name> <command...>: runs one step, logs it, records its status and time.
step() {
  local name=$1 began=$SECONDS status
  shift
  if [ "$failed" -gt 0 ] && [ "$keep_going" = no ]; then
    summary="$summary$(printf '  %-26s %s' "$name" "not run")
"
    return 0
  fi
  "$@" >"$LOGS/$name.log" 2>&1
  status=$?
  if [ $status -eq 0 ]; then
    summary="$summary$(printf '  %-26s %-8s %4ss' "$name" ok $((SECONDS - began)))
"
  else
    failed=$((failed + 1))
    summary="$summary$(printf '  %-26s %-8s %4ss  (target/check/%s/%s.log)' "$name" FAILED $((SECONDS - began)) "$(basename "$LOGS")" "$name")
"
    echo "== $name failed; the end of its log:" >&2
    tail -25 "$LOGS/$name.log" >&2
  fi
}

pending() { # name, the checkpoint that brings it
  summary="$summary$(printf '  %-26s %s' "$1" "pending ($2)")
"
}

# ES17: TODO(<milestone or checkpoint>, <rule or item>): <what>. No FIXME, XXX or HACK.
todo_form() {
  local bad
  bad=$(grep -rnE --include='*.rs' 'TODO|FIXME|XXX|HACK' crates fuzz 2>/dev/null |
    grep -vE 'TODO\(M[0-9]+(-CP[0-9]+)?, [A-Z][A-Za-z0-9.-]*\): [^ ]' || true)
  if [ -n "$bad" ]; then
    echo "ES17: write TODO(<milestone or checkpoint>, <rule or item>): <what>; FIXME, XXX and HACK aren't used:"
    echo "$bad"
    return 1
  fi
  echo "ES17: every TODO in crates/ and fuzz/ is in its form"
}

todo_list() {
  echo "The TODOs in crates/ and fuzz/ (ES17), by milestone or checkpoint:"
  grep -rnoE --include='*.rs' 'TODO\(M[0-9]+(-CP[0-9]+)?, [^)]*\): .*' crates fuzz 2>/dev/null | sort -t'(' -k2 || true
}

codegen_check() {
  if cargo metadata --format-version 1 --no-deps --locked 2>/dev/null | grep -q '"name":"codegen"'; then
    cargo run -q --locked -p codegen -- --check
  else
    echo "codegen doesn't exist yet"
  fi
}

# The crates allowed `unsafe` (tools/unsafe-policy.txt) that are engine crates.
unsafe_crates() {
  awk '$1 == "allow" && $2 ~ /^crates\// { sub("crates/", "", $2); print $2 }' tools/unsafe-policy.txt
}

miri() {
  local crates
  crates=$(unsafe_crates)
  if [ -z "$crates" ]; then
    echo "no engine crate holds unsafe"
    return 0
  fi
  local args=() crate
  for crate in $crates; do args+=(-p "$crate"); done
  # Miri's sysroot is built under target/, not in the user's caches.
  MIRI_SYSROOT="$ROOT/target/miri/sysroot" cargo +"$NIGHTLY" miri setup &&
    MIRI_SYSROOT="$ROOT/target/miri/sysroot" cargo +"$NIGHTLY" miri test --locked "${args[@]}"
}

tool_version() { # tool, version
  local found
  found=$("$1" --version 2>/dev/null | awk '{print $2}')
  if [ "$found" != "$2" ]; then
    echo "$1 $2 is needed; found ${found:-none}. Run tools/install-tools.sh."
    return 1
  fi
}

deny() { tool_version cargo-deny "$DENY_VERSION" && cargo deny --locked check; }
vet() { tool_version cargo-vet "$VET_VERSION" && cargo vet --locked --cache-dir "$ROOT/target/cargo-vet-cache"; }

echo "tools/check.sh: the $tier tier; logs in target/check/$(basename "$LOGS")/"

step fmt cargo fmt --all --check
step clippy cargo clippy --locked --workspace --all-targets -- -D warnings
step test cargo test --locked --workspace
step spikes-fmt cargo fmt "${SPIKES[@]}" --all --check
step spikes-clippy cargo clippy "${SPIKES[@]}" --locked --workspace --all-targets -- -D warnings
step spikes-test cargo test "${SPIKES[@]}" --locked --workspace
step unsafe-policy tools/check-unsafe-policy.sh
step doc-links python3 -I tools/check-doc-links.py
step dependencies python3 -I tools/check-dependencies.py
step file-size python3 -I tools/check-file-size.py
# OB25, OB27: recording's cost, which is only meaningful optimized.
step trace-cost cargo test --locked --release -p trace --test cost
step todo-form todo_form
step codegen codegen_check
step self-tests sh -c 'tools/check-unsafe-policy.sh --self-test &&
  python3 -I tools/check-dependencies.py --self-test && python3 -I tools/check-file-size.py --self-test &&
  python3 -I tools/unsafe-sites.py --self-test'

if [ "$tier" = checkpoint ]; then
  step doc env RUSTDOCFLAGS="-D warnings" cargo doc --locked --workspace --no-deps
  step miri miri
  step cargo-deny deny
  step cargo-vet vet
  step todo-list todo_list
  pending conformance M1-CP4
  pending ratchet "M1-CP4: PF5"
  pending loom "M1-CP3: TS11"
  pending fuzz-smoke "M1-CP3: the first target"
  pending build-thresholds "M1-CP4: T1-T4"
fi

elapsed=$((SECONDS - started))
printf '%s' "$summary"
if [ "$tier" = commit ]; then
  note="budget ${COMMIT_BUDGET}s"
  [ $elapsed -gt $COMMIT_BUDGET ] && note="$note: OVER BUDGET (ES25: move slow suites to the checkpoint tier)"
else
  note="the checkpoint tier has no budget"
fi
echo "tools/check.sh: $tier tier, ${elapsed}s ($note); $failed step(s) failed"
[ $failed -eq 0 ]
