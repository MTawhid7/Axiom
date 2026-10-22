#!/usr/bin/env bash
# Spike S15: a stable local signing identity (the architecture review's
# RV1-17; docs/research/s15-results.md).
#
# Usage: tools/s15-measure.sh STEP...
#
# Steps, in the order they're meant to run:
#   setup        a throwaway keychain file, target/s15/keychain/s15.keychain-db,
#                with a random password, and in it a self-signed code-signing
#                certificate made by the system's LibreSSL, its key usable by
#                codesign without a prompt (its partition list). The user's
#                keychain search list is checked unchanged. Skipped if the
#                keychain exists. The login keychain is never touched
#   s4           S15-Q1: S4's binaries signed twice, ad hoc and with the
#                identity (tools/s4-sign.sh --identity), then S4's scenarios
#                against each set: the 18 probes, the JIT matrix, CoreText in
#                the text profile, start-up and IPC
#   keychain     S15-Q2: for each signing, a probe build tagged v1 adds a
#                password to the S15 keychain; a build tagged v2, signed the
#                same way and put at the same path, reads it. User interaction
#                is disallowed, so a refusal is a status, never a dialog
#   requirement  S15-Q2: whether each signing's v2 satisfies v1's designated
#                requirement, the identity macOS tracks code by (TN3127)
#   teardown     deletes the S15 keychain and its key files, and checks the
#                search list is as it was
#
# Results go to target/s15-results/<timestamp>/, which git ignores.
set -uo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd)
S15="$ROOT/target/s15"
K="$S15/keychain"
KC="$K/s15.keychain-db"
NAME="Axiom Local Code Signing (S15)"

usage() { sed -n '2,/^set -uo/p' "$0" | sed '$d' | sed 's/^# \{0,1\}//'; }

steps=()
while [ $# -gt 0 ]; do
  case $1 in
    --help | -h) usage; exit 0 ;;
    setup | s4 | keychain | requirement | teardown) steps+=("$1"); shift ;;
    *) echo "unknown argument: $1 (see --help)" >&2; exit 2 ;;
  esac
done
[ ${#steps[@]} -gt 0 ] || { usage; exit 2; }

OUT="$ROOT/target/s15-results/$(date +%Y%m%d-%H%M%S)"
mkdir -p "$OUT" "$S15"
log() { echo "[$(date +%H:%M:%S)] $*" | tee -a "$OUT/log.txt"; }

identity() { # the identity's SHA-1, from the S15 keychain
  security find-identity -p codesigning "$KC" 2>/dev/null | awk '/\)/ {print $2; exit}'
}

unlock() { security unlock-keychain -p "$(cat "$K/password")" "$KC"; }

step_setup() {
  if [ -e "$KC" ]; then log "setup: the S15 keychain exists; skipped"; return; fi
  mkdir -p "$K" && chmod 700 "$K"
  security list-keychains -d user > "$K/searchlist-before.txt"
  head -c 24 /dev/urandom | base64 | tr -d '/+=' > "$K/password"
  head -c 12 /dev/urandom | base64 | tr -d '/+=' > "$K/p12password"
  chmod 600 "$K/password" "$K/p12password"
  cat > "$K/cert.cnf" <<EOF
[req]
distinguished_name = dn
x509_extensions = ext
prompt = no
[dn]
CN = $NAME
O = Axiom spike S15
[ext]
basicConstraints = critical, CA:false
keyUsage = critical, digitalSignature
extendedKeyUsage = critical, codeSigning
subjectKeyIdentifier = hash
EOF
  /usr/bin/openssl req -x509 -newkey rsa:2048 -sha256 -days 3650 -nodes -config "$K/cert.cnf" \
    -keyout "$K/key.pem" -out "$K/cert.pem" 2> "$OUT/openssl.txt"
  chmod 600 "$K/key.pem"
  /usr/bin/openssl pkcs12 -export -inkey "$K/key.pem" -in "$K/cert.pem" -name "$NAME" \
    -out "$K/s15.p12" -passout "pass:$(cat "$K/p12password")"
  security create-keychain -p "$(cat "$K/password")" "$KC"
  security set-keychain-settings "$KC" # no automatic lock
  unlock
  security import "$K/s15.p12" -k "$KC" -P "$(cat "$K/p12password")" -T /usr/bin/codesign >> "$OUT/setup.txt" 2>&1
  security set-key-partition-list -S apple-tool:,apple: -s -k "$(cat "$K/password")" "$KC" > /dev/null 2>&1
  security find-identity -p codesigning "$KC" >> "$OUT/setup.txt" 2>&1
  security list-keychains -d user > "$K/searchlist-after.txt"
  if diff -q "$K/searchlist-before.txt" "$K/searchlist-after.txt" > /dev/null; then
    log "setup: identity $(identity); the search list is unchanged"
  else
    log "setup: identity $(identity); THE SEARCH LIST CHANGED, see $K"
  fi
}

broker_runs() { # dir label
  local dir=$1 label=$2 status
  for scenario in "probe" "jit" "text --runs 2" "startup --runs 5" "ipc --batches 2"; do
    # shellcheck disable=SC2086 # the scenario's options are separate words
    "$dir/s4-broker" $scenario > "$OUT/s4-$label-${scenario%% *}.txt" 2>&1
    status=$?
    log "s4: $label ${scenario%% *}: exit $status"
  done
}

step_s4() {
  unlock
  local id; id=$(identity)
  "$ROOT/tools/s4-sign.sh" --out "$S15/s4-adhoc" > "$OUT/sign-adhoc.txt" 2>&1
  "$ROOT/tools/s4-sign.sh" --out "$S15/s4-identity" --identity "$id" --keychain "$KC" > "$OUT/sign-identity.txt" 2>&1
  for set in adhoc identity; do
    codesign -d -r- "$S15/s4-$set/s4-helper" 2>&1 | grep designated > "$OUT/s4-$set-requirement.txt"
    broker_runs "$S15/s4-$set" "$set"
  done
  # The probes' verdicts, with every number (timings, process IDs) masked,
  # compared between the two sets.
  for set in adhoc identity; do
    grep -E 'denied|allowed|ok|fail|PASS|FAIL' "$OUT/s4-$set-probe.txt" | sed -E 's/[0-9]+/N/g' > "$OUT/s4-$set-probe-verdicts.txt"
  done
  if diff -q "$OUT/s4-adhoc-probe-verdicts.txt" "$OUT/s4-identity-probe-verdicts.txt" > /dev/null; then
    log "s4: the probes' verdicts are the same under both signings"
  else
    log "s4: the probes' verdicts DIFFER (diff in s4-probe-verdicts.diff)"
    diff "$OUT/s4-adhoc-probe-verdicts.txt" "$OUT/s4-identity-probe-verdicts.txt" > "$OUT/s4-probe-verdicts.diff"
  fi
}

build_probe() { # tag destination
  (cd "$ROOT" && S15_BUILD_TAG=$1 cargo build -q -p s15-probe) && cp -f "$ROOT/target/debug/s15-probe" "$2"
}

sign_probe() { # signing path
  if [ "$1" = adhoc ]; then
    codesign --force --sign - --identifier org.axiom.s15.probe --options runtime "$2" 2>/dev/null
  else
    codesign --force --sign "$(identity)" --keychain "$KC" --identifier org.axiom.s15.probe --options runtime "$2" 2>/dev/null
  fi
}

step_keychain() {
  unlock
  local service
  for signing in adhoc identity; do
    service="s15-$signing-$(date +%s)"
    mkdir -p "$S15/run/$signing"
    local probe="$S15/run/$signing/s15-probe"
    build_probe v1 "$probe" && sign_probe "$signing" "$probe"
    codesign -d -r- "$probe" 2>&1 | grep designated > "$OUT/keychain-$signing-v1-requirement.txt"
    "$probe" write "$KC" "$service" > "$OUT/keychain-$signing.txt" 2>&1
    "$probe" read "$KC" "$service" >> "$OUT/keychain-$signing.txt" 2>&1
    build_probe v2 "$probe" && sign_probe "$signing" "$probe"
    codesign -d -r- "$probe" 2>&1 | grep designated > "$OUT/keychain-$signing-v2-requirement.txt"
    "$probe" read "$KC" "$service" >> "$OUT/keychain-$signing.txt" 2>&1
    log "keychain: $signing: $(tr '\n' ';' < "$OUT/keychain-$signing.txt")"
    security dump-keychain -a "$KC" 2>/dev/null | grep -A3 "description: $service\$" | grep requirement > "$OUT/keychain-$signing-acl.txt"
    log "keychain: $signing: the item trusts $(sed 's/^ *//' "$OUT/keychain-$signing-acl.txt" | head -1)"
  done
}

step_requirement() {
  unlock
  local dir v1_requirement
  for signing in adhoc identity; do
    dir="$S15/req/$signing"
    mkdir -p "$dir"
    build_probe v1 "$dir/v1" && sign_probe "$signing" "$dir/v1"
    build_probe v2 "$dir/v2" && sign_probe "$signing" "$dir/v2"
    v1_requirement=$(codesign -d -r- "$dir/v1" 2>&1 | sed -n 's/^# *designated => //p; s/^designated => //p' | head -1)
    echo "v1: $v1_requirement" > "$OUT/requirement-$signing.txt"
    if codesign --verify -R "=$v1_requirement" "$dir/v2" >> "$OUT/requirement-$signing.txt" 2>&1; then
      log "requirement: $signing: v2 satisfies v1's designated requirement"
    else
      log "requirement: $signing: v2 does NOT satisfy v1's designated requirement"
    fi
  done
}

step_teardown() {
  if [ -e "$KC" ]; then
    security delete-keychain "$KC" && log "teardown: the S15 keychain is deleted"
  fi
  rm -f "$K/key.pem" "$K/s15.p12" "$K/password" "$K/p12password"
  security list-keychains -d user > "$OUT/searchlist-after-teardown.txt"
  if [ -e "$K/searchlist-before.txt" ] && diff -q "$K/searchlist-before.txt" "$OUT/searchlist-after-teardown.txt" > /dev/null; then
    log "teardown: the search list is as it was"
  else
    log "teardown: compare the search list with $K/searchlist-before.txt"
  fi
}

for step in "${steps[@]}"; do
  case $step in
    setup) step_setup ;;
    s4) step_s4 ;;
    keychain) step_keychain ;;
    requirement) step_requirement ;;
    teardown) step_teardown ;;
  esac
done
log "done: $OUT"
