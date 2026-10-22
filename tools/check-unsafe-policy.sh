#!/usr/bin/env bash
# Enforce the unsafe-code policy (docs/technology/language-evaluation.md §15.3)
# and print an inventory of `unsafe` per crate.
#
# Rules, for every crate under spikes/ and crates/:
#   R1  Every target root declares #![forbid(unsafe_code)]. The exception is the
#       library/binary root of a crate allowlisted in tools/unsafe-policy.txt,
#       which declares #![deny(unsafe_code)] instead.
#   R2  `allow(unsafe_code)` appears only in allowlisted crates, and only in files
#       under that crate's allowed path prefixes.
#   R3  Macro crates that emit `unsafe` must be registered as `emits-unsafe`. This
#       is reported in the inventory; auditing them is a review obligation (§15.3
#       rule 7).
#
# Usage: tools/check-unsafe-policy.sh [--self-test] [--quiet]
# Exit status: 0 = compliant, 1 = violations found, 2 = usage error.
# Written for bash 3.2 (macOS /bin/bash): no associative arrays, no mapfile.
set -uo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd)
QUIET=no

# Does $1 contain an inner attribute #![<level>(... unsafe_code ...)]?
has_inner_lint() { # file level
  grep -E "^[[:space:]]*#!\[$2\(" "$1" 2>/dev/null | grep -q 'unsafe_code'
}

# Prints "<allowed-prefixes>" if the crate is allowlisted, otherwise nothing.
allow_entry() { # policy rel-crate-dir
  awk -v c="$2" '$1 == "allow" && $2 == c { print $3; exit }' "$1"
}

emits_unsafe() { # policy rel-crate-dir
  awk -v c="$2" '$1 == "emits-unsafe" && $2 == c { found = 1 } END { exit !found }' "$1"
}

starts_with_any() { # path comma-separated-prefixes
  local path=$1 IFS=,
  local prefix
  for prefix in $2; do
    case $path in "$prefix"*) return 0 ;; esac
  done
  return 1
}

# Count non-comment source lines matching an ERE, across a crate's Rust files.
count_lines() { # crate-dir ere
  find "$1" -name '*.rs' -not -path '*/target/*' -print0 2>/dev/null |
    xargs -0 cat 2>/dev/null |
    grep -Ev '^[[:space:]]*//' |
    grep -cE "$2" || true
}

# check_tree <root> <policy> → prints violations and the inventory; returns 0 iff compliant.
check_tree() {
  local root=$1 policy=$2 violations=0
  local manifests manifest crate_dir rel allowed f hit file relfile
  manifests=$(find "$root/spikes" "$root/crates" -name Cargo.toml -not -path '*/target/*' 2>/dev/null | sort)
  if [ -z "$manifests" ]; then
    echo "check-unsafe-policy: no crates under $root/{spikes,crates}"
    return 0
  fi

  local inventory=""
  for manifest in $manifests; do
    # A workspace's own manifest (spikes/Cargo.toml) declares no package.
    grep -q '^\[package\]' "$manifest" || continue
    crate_dir=$(dirname "$manifest")
    rel=${crate_dir#"$root"/}
    allowed=$(allow_entry "$policy" "$rel")

    # R1: crate-level lint declarations on every target root.
    for f in "$crate_dir"/src/lib.rs "$crate_dir"/src/main.rs "$crate_dir"/src/bin/*.rs \
      "$crate_dir"/tests/*.rs "$crate_dir"/benches/*.rs "$crate_dir"/examples/*.rs; do
      [ -f "$f" ] || continue
      case $f in
        "$crate_dir"/src/lib.rs | "$crate_dir"/src/main.rs)
          if [ -n "$allowed" ]; then
            if ! has_inner_lint "$f" deny; then
              echo "VIOLATION R1: ${f#"$root"/}: allowlisted crate root must declare #![deny(unsafe_code)]"
              violations=$((violations + 1))
            fi
            continue
          fi
          ;;
      esac
      if ! has_inner_lint "$f" forbid; then
        echo "VIOLATION R1: ${f#"$root"/}: missing #![forbid(unsafe_code)]"
        violations=$((violations + 1))
      fi
    done

    # R2: allow(unsafe_code) only inside allowlisted prefixes.
    while IFS= read -r hit; do
      [ -n "$hit" ] || continue
      file=${hit%%:*}
      relfile=${file#"$crate_dir"/}
      if [ -z "$allowed" ]; then
        echo "VIOLATION R2: ${file#"$root"/}: allow(unsafe_code) in a crate that is not allowlisted"
        violations=$((violations + 1))
      elif ! starts_with_any "$relfile" "$allowed"; then
        echo "VIOLATION R2: ${file#"$root"/}: allow(unsafe_code) outside allowed prefixes ($allowed)"
        violations=$((violations + 1))
      fi
    done <<EOF
$(grep -rnE --include='*.rs' 'allow\([^)]*unsafe_code' "$crate_dir" 2>/dev/null | grep -v '/target/' || true)
EOF

    # Inventory (informational).
    local blocks fns impls traits policy_col
    blocks=$(count_lines "$crate_dir" 'unsafe[[:space:]]*\{')
    fns=$(count_lines "$crate_dir" 'unsafe[[:space:]]+(extern[[:space:]]+"[^"]*"[[:space:]]+)?fn[[:space:]]')
    impls=$(count_lines "$crate_dir" 'unsafe[[:space:]]+impl')
    traits=$(count_lines "$crate_dir" 'unsafe[[:space:]]+trait')
    if [ -n "$allowed" ]; then policy_col="allow:$allowed"; else policy_col="forbid"; fi
    if emits_unsafe "$policy" "$rel"; then policy_col="$policy_col +emits-unsafe"; fi
    inventory="$inventory$(printf '%-28s %6s %6s %6s %6s  %s' "$rel" "$blocks" "$fns" "$impls" "$traits" "$policy_col")
"
  done

  if [ "$QUIET" = no ]; then
    printf '%-28s %6s %6s %6s %6s  %s\n' crate blocks fns impls traits policy
    printf '%s' "$inventory"
  fi
  if [ "$violations" -gt 0 ]; then
    echo "check-unsafe-policy: $violations violation(s)"
    return 1
  fi
  echo "check-unsafe-policy: OK"
  return 0
}

self_test() {
  local tmp status
  tmp=$(mktemp -d "${TMPDIR:-/tmp}/unsafe-policy-selftest.XXXXXX")
  self_test_in "$tmp"
  status=$?
  rm -rf "$tmp"
  return $status
}

self_test_in() { # empty-scratch-dir
  local tmp=$1 out

  mk() { # rel-crate-dir file content
    mkdir -p "$tmp/$1/$(dirname "$2")"
    [ -f "$tmp/$1/Cargo.toml" ] || printf '[package]\nname = "x"\n' >"$tmp/$1/Cargo.toml"
    printf '%s\n' "$3" >"$tmp/$1/$2"
  }
  mk spikes/good src/lib.rs '#![forbid(unsafe_code)]'
  mk spikes/good tests/it.rs '#![forbid(unsafe_code)]'
  mk spikes/allowed src/lib.rs '#![deny(unsafe_code)]'
  mk spikes/allowed src/raw/mod.rs '#![allow(unsafe_code)]'
  # A workspace manifest above the crates is not a crate (M1-CP2's split).
  printf '[workspace]\nmembers = ["good", "allowed"]\n' >"$tmp/spikes/Cargo.toml"
  cat >"$tmp/policy.txt" <<'EOF'
allow spikes/allowed src/raw/
allow spikes/nodeny src/raw/
emits-unsafe spikes/good
EOF

  # 1. A compliant tree passes.
  if ! out=$(check_tree "$tmp" "$tmp/policy.txt"); then
    echo "self-test FAILED: compliant tree rejected:"
    echo "$out"
    return 1
  fi

  # 2. Seed one violation per rule path; every one must be reported.
  mk spikes/missing src/lib.rs '// no lint declaration'
  mk spikes/missing tests/it.rs '#![forbid(unsafe_code)]'
  mk spikes/allowed src/other.rs '#[allow(unsafe_code)] fn f() {}'
  mk spikes/sneaky src/lib.rs '#![forbid(unsafe_code)]'
  mk spikes/sneaky src/x.rs '#[allow(clippy::all, unsafe_code)] fn g() {}'
  mk spikes/nodeny src/lib.rs '#![forbid(unsafe_code)]'
  mk spikes/good benches/b.rs '// bench root without forbid'
  if out=$(check_tree "$tmp" "$tmp/policy.txt"); then
    echo "self-test FAILED: seeded violations were not detected:"
    echo "$out"
    return 1
  fi
  local expect missing_hits=0
  for expect in \
    'R1: spikes/missing/src/lib.rs' \
    'R2: spikes/allowed/src/other.rs' \
    'R2: spikes/sneaky/src/x.rs' \
    'R1: spikes/nodeny/src/lib.rs' \
    'R1: spikes/good/benches/b.rs'; do
    if ! printf '%s\n' "$out" | grep -q "$expect"; then
      echo "self-test FAILED: expected violation not reported: $expect"
      missing_hits=$((missing_hits + 1))
    fi
  done
  if ! printf '%s\n' "$out" | grep -q '+emits-unsafe'; then
    echo "self-test FAILED: emits-unsafe registration not shown in inventory"
    missing_hits=$((missing_hits + 1))
  fi
  if [ "$missing_hits" -gt 0 ]; then
    echo "$out"
    return 1
  fi
  echo "check-unsafe-policy self-test: OK (compliant tree accepted; 5/5 seeded violations reported)"
}

main() {
  local mode=check arg
  for arg in "$@"; do
    case $arg in
      --self-test) mode=self-test ;;
      --quiet) QUIET=yes ;;
      -h | --help)
        sed -n '2,17p' "$0" | sed 's/^# \{0,1\}//'
        return 0
        ;;
      *)
        echo "unknown argument: $arg" >&2
        return 2
        ;;
    esac
  done
  case $mode in
    self-test) QUIET=no self_test ;;
    check) check_tree "$ROOT" "$ROOT/tools/unsafe-policy.txt" ;;
  esac
}

main "$@"
