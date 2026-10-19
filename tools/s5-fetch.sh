#!/usr/bin/env bash
# Spike S5 (S5-Q3): download the test web fonts listed in spikes/s5/corpus/webfonts.txt
# from web-platform-tests at the commit the list pins, into target/s5/webfonts/.
# Each file is checked against its git blob hash, so a download that differs from the
# pinned commit fails. Files already present and correct are not downloaded again.
#
# Usage: tools/s5-fetch.sh [--out DIR]
#   --out DIR   where the files go (default: target/s5/webfonts)
#
# The fonts are test data, not part of the repository. The owner authorized "OFL/WPT
# test web fonts" at Step 5's Gate B (2026-10-18).
# Written for bash 3.2 (macOS /bin/bash).
set -euo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd)
OUT="$ROOT/target/s5/webfonts"
LIST="$ROOT/spikes/s5/corpus/webfonts.txt"

while [ $# -gt 0 ]; do
  case $1 in
    --out) OUT=$2; shift ;;
    -h | --help) sed -n '2,12p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
    *) echo "s5-fetch: unknown argument $1" >&2; exit 2 ;;
  esac
  shift
done

commit=$(awk '$1 == "commit" { print $2 }' "$LIST")
[ -n "$commit" ] || { echo "s5-fetch: no commit line in $LIST" >&2; exit 1; }

fetched=0 present=0 failed=0
while read -r path sha _expectation; do
  case $path in "" | "#"* | commit) continue ;; esac
  dest="$OUT/$path"
  if [ -f "$dest" ] && [ "$(git hash-object "$dest")" = "$sha" ]; then
    present=$((present + 1))
    continue
  fi
  mkdir -p "$(dirname "$dest")"
  if curl -sSfL --retry 3 -o "$dest.part" "https://raw.githubusercontent.com/web-platform-tests/wpt/$commit/$path" &&
    [ "$(git hash-object "$dest.part")" = "$sha" ]; then
    mv -f "$dest.part" "$dest"
    fetched=$((fetched + 1))
  else
    rm -f "$dest.part"
    echo "s5-fetch: $path did not download or does not match $sha" >&2
    failed=$((failed + 1))
  fi
done <"$LIST"

echo "s5-fetch: $fetched downloaded, $present already present, $failed failed, in $OUT (wpt $commit)"
[ "$failed" -eq 0 ]
