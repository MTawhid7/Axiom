#!/usr/bin/env bash
# Spike S4: build the S4 binaries and sign copies ad hoc with the hardened
# runtime, as Axiom's helpers would be signed (ADR-0004 D2), with no paid
# developer account.
#
# Usage: tools/s4-sign.sh [--release] [--out DIR] [--identity ID --keychain PATH]
#   --release        build with the release profile (default: dev)
#   --out DIR        where the signed copies go (default: target/s4/bin)
#   --identity ID    sign with this identity (a certificate's SHA-1, or its
#                    name) instead of ad hoc: spike S15's stable local
#                    identity (the review's RV1-17)
#   --keychain PATH  the keychain that holds the identity (S15's own file)
#
# Signed copies:
#   s4-broker        hardened runtime, no entitlements
#   s4-helper        hardened runtime, no entitlements (the renderer stand-in)
#   s4-helper-jit    hardened runtime + com.apple.security.cs.allow-jit
#   s4-helper-jit1   hardened runtime + allow-jit + com.apple.security.cs.single-jit
#   s4-helper-plain  ad hoc only, without the hardened runtime
#   s4-text          hardened runtime, no entitlements (CoreText, S4-Q5)
#
# Then run the scenarios against them, for example:
#   target/s4/bin/s4-broker probe
#   target/s4/bin/s4-broker jit
set -euo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd)
PROFILE=debug
CARGO_FLAGS=()
OUT="$ROOT/target/s4/bin"

IDENTITY=-
KEYCHAIN=

usage() { sed -n '2,27p' "$0" | sed 's/^# \{0,1\}//'; }

while [ $# -gt 0 ]; do
  case $1 in
    --release) PROFILE=release; CARGO_FLAGS+=(--release) ;;
    --out) OUT=$2; shift ;;
    --identity) IDENTITY=$2; shift ;;
    --keychain) KEYCHAIN=$2; shift ;;
    -h | --help) usage; exit 0 ;;
    *) echo "s4-sign: unknown argument $1" >&2; usage >&2; exit 2 ;;
  esac
  shift
done

cd "$ROOT"
cargo build -p s4-sandbox -p s4-text ${CARGO_FLAGS[@]+"${CARGO_FLAGS[@]}"} --bins
mkdir -p "$OUT"

# sign <built-binary> <copy-name> <identifier> <runtime|plain> [entitlements-plist]
sign() {
  local source="target/$PROFILE/$1" copy="$OUT/$2" identifier=$3 kind=$4 entitlements=${5:-}
  local args=(--sign "$IDENTITY" --force --identifier "$identifier")
  [ -n "$KEYCHAIN" ] && args+=(--keychain "$KEYCHAIN")
  [ "$kind" = runtime ] && args+=(--options runtime)
  [ -n "$entitlements" ] && args+=(--entitlements "$entitlements")
  cp -f "$source" "$copy"
  codesign "${args[@]}" "$copy" 2>/dev/null
  printf '%-16s %s %s\n' "$2" \
    "$(codesign -dv "$copy" 2>&1 | grep -E '^CodeDirectory' | sed 's/.*flags=/flags=/; s/ hashes.*//')" \
    "$(codesign -d --entitlements - --xml "$copy" 2>/dev/null | grep -o 'com\.apple\.[a-z.-]*' | tr '\n' ' ')"
}

sign s4-broker s4-broker org.axiom.s4.broker runtime
sign s4-helper s4-helper org.axiom.s4.helper runtime
sign s4-helper s4-helper-jit org.axiom.s4.helper-jit runtime spikes/s4/sandbox/entitlements/allow-jit.plist
sign s4-helper s4-helper-jit1 org.axiom.s4.helper-jit1 runtime spikes/s4/sandbox/entitlements/allow-jit-single.plist
sign s4-helper s4-helper-plain org.axiom.s4.helper-plain plain
sign s4-text s4-text org.axiom.s4.text runtime

echo "signed copies in $OUT ($([ "$IDENTITY" = - ] && echo "ad hoc" || echo "identity $IDENTITY"))"
