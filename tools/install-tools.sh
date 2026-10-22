#!/usr/bin/env bash
# Install the tools tools/check.sh needs, at pinned versions, under
# target/tools/ (MA10's tools; the plan's §1).
#
# Usage: tools/install-tools.sh [--help]
#
#   cargo-vet 0.10.2   audits (DP15), into target/tools/bin with --locked
#   miri               on nightly-2026-10-21, the plan's pinned nightly (S12),
#                      for the checkpoint tier's Miri runs (TS11)
#
# cargo-deny 0.20.2, the current release, was already installed in
# ~/.cargo/bin on this Mac before the project (observed at Step 6), so it isn't
# installed again; tools/check.sh checks its version. cargo-fuzz 0.13.2 joins
# this list with the first fuzz target (M1-CP3).
#
# Installs nothing that is already present at its version. target/ is ignored
# by git and wiped by `cargo clean`, after which this script installs again.
# Written for bash 3.2 (macOS /bin/bash).
set -euo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd)
case ${1:-} in
  -h | --help) sed -n '2,19p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
  "") ;;
  *) echo "unknown argument: $1 (see --help)" >&2; exit 2 ;;
esac

TOOLS="$ROOT/target/tools"
if [ "$("$TOOLS/bin/cargo-vet" --version 2>/dev/null | awk '{print $2}')" != 0.10.2 ]; then
  echo "installing cargo-vet 0.10.2 into target/tools"
  cargo install cargo-vet@0.10.2 --locked --root "$TOOLS"
fi
"$TOOLS/bin/cargo-vet" --version

if ! rustup component list --toolchain nightly-2026-10-21 --installed | grep -q '^miri'; then
  echo "adding miri to nightly-2026-10-21"
  rustup component add miri --toolchain nightly-2026-10-21
fi
echo "miri: $(rustup component list --toolchain nightly-2026-10-21 --installed | grep '^miri')"
cargo-deny --version
