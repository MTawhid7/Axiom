#!/usr/bin/env bash
# Spike S4: build the S4 binaries and sign copies ad hoc with the hardened
# runtime, as Axiom's helpers would be signed (ADR-0004 D2), with no paid
# developer account.
#
# Usage: tools/s4-sign.sh [--release] [--out DIR]
#   --release   build with the release profile (default: dev)
#   --out DIR   where the signed copies go (default: target/s4/bin)
#
# Signed copies:
#   s4-broker   hardened runtime, no entitlements
#   s4-helper   hardened runtime, no entitlements (the renderer stand-in)
#
# Then run the S4a scenarios against them:
#   target/s4/bin/s4-broker probe
set -euo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd)
PROFILE=debug
CARGO_FLAGS=()
OUT="$ROOT/target/s4/bin"

usage() { sed -n '2,17p' "$0" | sed 's/^# \{0,1\}//'; }

while [ $# -gt 0 ]; do
  case $1 in
    --release) PROFILE=release; CARGO_FLAGS+=(--release) ;;
    --out) OUT=$2; shift ;;
    -h | --help) usage; exit 0 ;;
    *) echo "s4-sign: unknown argument $1" >&2; usage >&2; exit 2 ;;
  esac
  shift
done

cd "$ROOT"
cargo build -p s4-sandbox ${CARGO_FLAGS[@]+"${CARGO_FLAGS[@]}"} --bins
mkdir -p "$OUT"

# sign <built-binary> <copy-name> <identifier> [entitlements-plist]
sign() {
  local source="target/$PROFILE/$1" copy="$OUT/$2" identifier=$3 entitlements=${4:-}
  cp -f "$source" "$copy"
  if [ -n "$entitlements" ]; then
    codesign --sign - --force --options runtime --identifier "$identifier" \
      --entitlements "$entitlements" "$copy"
  else
    codesign --sign - --force --options runtime --identifier "$identifier" "$copy"
  fi
  printf '%-14s %s\n' "$2" "$(codesign -dv "$copy" 2>&1 | grep -E '^CodeDirectory' | sed 's/.*flags=/flags=/; s/ hashes.*//')"
}

sign s4-broker s4-broker org.axiom.s4.broker
sign s4-helper s4-helper org.axiom.s4.helper

echo "signed copies in $OUT"
