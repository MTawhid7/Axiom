#!/usr/bin/env bash
# Fetch Milestone 1's pinned test data into target/test-data/ (MA3; the plan's
# §1 and §2, items 4 and 5).
#
# Usage: tools/fetch-test-data.sh [--verify] [wpt] [html5lib] [unicode] [corpus]
#
# With no set named, fetches wpt, html5lib and unicode. `corpus` must be named,
# since it loads two live pages.
#   wpt       web-platform-tests at 0f26972422 (2026-10-18), sparsely: the
#             reference tests M1 runs and what they share (css/CSS2,
#             css/reference, css/support, fonts, images, common, resources),
#             the tree-construction data (html/syntax/parsing), URL's test
#             data (url), and wptrunner's tools (tools, docs)
#   html5lib  html5lib-tests at c777c408 (2026-10-01), for the tokenizer tests
#   unicode   Unicode 17.0.0's conformance files: line, grapheme, word and
#             sentence breaking, the two bidi files, and UTS #46's IdnaTestV2
#   corpus    dated copies of the two documents-tier pages (CO12), the HTML
#             only: each fetched after robots.txt allows its path to every user
#             agent (CO3), with AxiomCensus's user agent, at most two loads a
#             site, and at least 60 s between loads of one site (CO4)
#   --verify  check what is present against the pins, without the network
#
# Checkouts are pinned by commit, which git verifies object by object; files
# by SHA-256, written here. A corpus copy is dated, and its SHA-256 recorded in
# its directory's manifest.tsv. Nothing here is committed: target/ is ignored.
# Written for bash 3.2 (macOS /bin/bash).
set -uo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd)
DATA="$ROOT/target/test-data"

WPT_REVISION=0f26972422b15f587e2afbd38dbb6ea2397fded0
WPT_DIRS="tools docs resources common fonts images css/CSS2 css/reference css/support html/syntax/parsing url"
HTML5LIB_REVISION=c777c408b61078ea2eb4acefc2535f54dbc8b28a
UNICODE_BASE=https://www.unicode.org/Public/17.0.0
# <SHA-256> <path under UNICODE_BASE>, computed on 2026-10-23 from unicode.org.
UNICODE_FILES="
e69884e0dde6a8724873f885d68c52dc14518abf9ae4ca9e2283b8773db3b752 ucd/auxiliary/LineBreakTest.txt
e2d134d2c52919bace503ebb6a551c1855fe1a1faec18478c78fff254a1793ec ucd/auxiliary/GraphemeBreakTest.txt
1de23a75f37904abc7d206239ee8d34f8fdf0fb4ab32a7174dfbabbde25419b2 ucd/auxiliary/WordBreakTest.txt
12cb47d028ded0c1cb8a28558f95479cbcd24559c46977015c82f3b50a1cc6e4 ucd/auxiliary/SentenceBreakTest.txt
888bdfc8090652272d1f859cdb00ae659e2dc6c26740be61ef1d03998a687620 ucd/BidiTest.txt
a3e6e905ab5afbe318a96df5401d0372a04cd73ef139ab5e3cf0ae241c255488 ucd/BidiCharacterTest.txt
beb5d0be20e896189b03209a82fdc34f06351502bbd4b8e2523583fc2954d9cf idna/IdnaTestV2.txt
"
# <name> <URL>: the corpus's documents tier (corpus §2, rows 5 and 9).
CORPUS_PAGES="
wikipedia-pi https://en.wikipedia.org/wiki/Pi
mdn-pixel-manipulation https://developer.mozilla.org/en-US/docs/Web/API/Canvas_API/Tutorial/Pixel_manipulation_with_canvas
"
CORPUS_AGENT="curl/$(curl --version | awk 'NR == 1 { print $2 }') AxiomCensus/0.1 (+https://github.com/MTawhid7/Axiom)"

verify=no sets=""
for arg in "$@"; do
  case $arg in
    --verify) verify=yes ;;
    wpt | html5lib | unicode | corpus) sets="$sets $arg" ;;
    -h | --help) sed -n '2,29p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
    *) echo "unknown argument: $arg (see --help)" >&2; exit 2 ;;
  esac
done
[ -n "$sets" ] || sets="wpt html5lib unicode"
failed=0
fail() { echo "FAILED: $*" >&2; failed=$((failed + 1)); }

# checkout <dir> <url> <revision> [sparse directories...]: a shallow, pinned checkout.
checkout() {
  local dir=$1 url=$2 revision=$3
  shift 3
  if [ "$verify" = no ]; then
    if [ ! -d "$dir/.git" ]; then
      mkdir -p "$dir" && git -C "$dir" init -q && git -C "$dir" remote add origin "$url" || return 1
    fi
    if [ $# -gt 0 ]; then
      git -C "$dir" sparse-checkout set "$@" || return 1
    fi
    if [ "$(git -C "$dir" rev-parse -q --verify HEAD 2>/dev/null)" != "$revision" ]; then
      git -C "$dir" fetch -q --filter=blob:none --depth 1 origin "$revision" &&
        git -C "$dir" checkout -q --detach "$revision" || return 1
    fi
  fi
  local head
  head=$(git -C "$dir" rev-parse -q --verify HEAD 2>/dev/null)
  [ "$head" = "$revision" ] || { fail "$dir is at ${head:-nothing}, not $revision"; return 1; }
  [ -z "$(git -C "$dir" status --porcelain 2>/dev/null)" ] || { fail "$dir has local changes"; return 1; }
  echo "$(basename "$dir"): at $revision, $(du -sh "$dir" | cut -f1)"
}

fetch_unicode() {
  local dir="$DATA/unicode-17.0.0" sum path file found
  mkdir -p "$dir"
  while read -r sum path; do
    [ -n "$sum" ] || continue
    file="$dir/$(basename "$path")"
    if [ "$verify" = no ] && [ ! -f "$file" ]; then
      curl -sSf -A "Axiom test data (https://github.com/MTawhid7/Axiom)" -o "$file.part" "$UNICODE_BASE/$path" &&
        mv "$file.part" "$file" || { fail "$UNICODE_BASE/$path"; continue; }
    fi
    found=$(shasum -a 256 "$file" 2>/dev/null | awk '{ print $1 }')
    [ "$found" = "$sum" ] || fail "$file: SHA-256 ${found:-missing}, pinned $sum"
  done <<EOF
$UNICODE_FILES
EOF
  echo "unicode-17.0.0: $(ls "$dir" | wc -l | tr -d ' ') files checked against their SHA-256"
}

# robots_allows <robots.txt file> <path>: RFC 9309 for the group of every user
# agent: the longest matching rule decides, Allow winning a tie, and no match
# allowing.
robots_allows() {
  python3 -I - "$1" "$2" <<'EOF'
import re, sys
text, path = open(sys.argv[1], encoding="utf-8", errors="replace").read(), sys.argv[2]
rules, agents, in_rules = [], [], False
for raw in text.splitlines():
    line = raw.split("#", 1)[0].strip()
    key, _, value = line.partition(":")
    key, value = key.strip().lower(), value.strip()
    if key == "user-agent":
        if in_rules:
            agents, in_rules = [], False
        agents.append(value)
    elif key in ("allow", "disallow"):
        in_rules = True
        if "*" in agents and value:
            rules.append((key, value))
def matches(pattern, path):
    regex = re.escape(pattern).replace(r"\*", ".*")
    regex = regex[:-2] + "$" if regex.endswith(r"\$") else regex
    return re.match(regex, path) is not None
best = max(((len(v), k == "allow") for k, v in rules if matches(v, path)), default=(0, True))
sys.exit(0 if best[1] else 1)
EOF
}

fetch_corpus() {
  local day dir name url host robots
  day=$(date +%Y-%m-%d)
  dir="$DATA/corpus/$day"
  mkdir -p "$dir"
  if [ "$verify" = no ]; then
    [ -f "$dir/manifest.tsv" ] || printf 'name\turl\tfetched\tstatus\tcontent-type\tlast-modified\tbytes\tsha256\n' >"$dir/manifest.tsv"
    # robots.txt first, for every site (CO3), then the pages, each at least 60 s
    # after its site's robots.txt (CO4).
    while read -r name url; do
      [ -n "$name" ] || continue
      [ -f "$dir/$name.html" ] && continue
      host=$(echo "$url" | awk -F/ '{ print $3 }')
      robots="$dir/$host.robots.txt"
      curl -sS -A "$CORPUS_AGENT" -o "$robots" "https://$host/robots.txt" || { fail "robots.txt of $host"; continue; }
      echo "robots.txt of $host read at $(date +%H:%M:%S)"
    done <<EOF
$CORPUS_PAGES
EOF
    local pending=no
    while read -r name url; do [ -n "$name" ] && [ ! -f "$dir/$name.html" ] && pending=yes; done <<EOF
$CORPUS_PAGES
EOF
    if [ "$pending" = yes ]; then
      echo "waiting 61 s, as CO4 spaces loads of one site"
      sleep 61
    fi
    while read -r name url; do
      [ -n "$name" ] || continue
      [ -f "$dir/$name.html" ] && continue
      host=$(echo "$url" | awk -F/ '{ print $3 }')
      if ! robots_allows "$dir/$host.robots.txt" "/${url#https://*/}"; then
        fail "$host's robots.txt disallows ${url#https://"$host"} to every user agent (CO3): not fetched"
        continue
      fi
      local headers="$dir/$name.headers"
      curl -sS -A "$CORPUS_AGENT" -D "$headers" -o "$dir/$name.html" "$url" || { fail "$url"; continue; }
      local status type modified
      status=$(awk 'toupper($1) ~ /^HTTP/ { code = $2 } END { print code }' "$headers")
      type=$(awk -F': ' 'tolower($1) == "content-type" { print $2 }' "$headers" | tr -d '\r' | tail -1)
      modified=$(awk -F': ' 'tolower($1) == "last-modified" { print $2 }' "$headers" | tr -d '\r' | tail -1)
      printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' "$name" "$url" "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$status" "$type" \
        "${modified:--}" "$(wc -c <"$dir/$name.html" | tr -d ' ')" "$(shasum -a 256 "$dir/$name.html" | awk '{ print $1 }')" \
        >>"$dir/manifest.tsv"
      [ "$status" = 200 ] || fail "$url answered $status"
    done <<EOF
$CORPUS_PAGES
EOF
  fi
  local copies=0 manifest
  for manifest in "$DATA"/corpus/*/manifest.tsv; do
    [ -f "$manifest" ] || continue
    while IFS="$(printf '\t')" read -r name url _ _ _ _ _ sum; do
      [ "$name" = name ] && continue
      local found
      found=$(shasum -a 256 "$(dirname "$manifest")/$name.html" 2>/dev/null | awk '{ print $1 }')
      [ "$found" = "$sum" ] || fail "$(dirname "$manifest")/$name.html: SHA-256 ${found:-missing}, recorded $sum"
      copies=$((copies + 1))
    done <"$manifest"
  done
  echo "corpus: $copies dated copies checked against their manifests"
}

for set in $sets; do
  case $set in
    wpt) checkout "$DATA/wpt" https://github.com/web-platform-tests/wpt.git "$WPT_REVISION" $WPT_DIRS || fail "wpt" ;;
    html5lib) checkout "$DATA/html5lib-tests" https://github.com/html5lib/html5lib-tests.git "$HTML5LIB_REVISION" || fail "html5lib" ;;
    unicode) fetch_unicode ;;
    corpus) fetch_corpus ;;
  esac
done
if [ $failed -gt 0 ]; then
  echo "fetch-test-data: $failed failure(s)"
  exit 1
fi
echo "fetch-test-data: every set at its pin"
