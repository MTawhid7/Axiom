#!/bin/bash
# An independent count of a WPT directory's reference tests, by file contents alone.
usage() {
  cat <<'EOF'
An independent count of a WPT directory's reference tests, by file contents alone.

Usage: tools/wpt-recount.sh WPT_ROOT DIR [EXCLUDE_PREFIX ...]

  WPT_ROOT        the WPT checkout
  DIR             a directory under it, such as css/CSS2
  EXCLUDE_PREFIX  paths under DIR to leave out, such as tables/ or inline-svg-

It reads no manifest and shares no code with tools/wpt-script-deps.py, so it
can check that tool's counts, as the standards' SB11 asks of every target a
tool computes. A test is a file, outside support/ and reference/ directories,
whose text links a reference with rel="match" or rel="mismatch". It counts:
  the tests;
  those whose text contains a <script element;
  of the others, those that are XML documents (.xht, .xhtml, .xml, .svg),
  and those that are HTML but name an XML reference;
  so the script-free tests that need neither script nor XML.
Print reference tests, which WPT names with `-print` before the
extension, are a manifest type of their own and are skipped, with their
count printed. It reads test files only: references are checked by
wpt-script-deps.py, which follows them through the manifest. Counts can
still differ from the manifest's where a file links a reference but the
manifest types it otherwise (manual or visual tests); such a difference is
to be explained beside the counts. WPT's files are data; nothing here runs
them.

Used for the architecture review's Round 4 (RV4-01's recount, CP7).
EOF
}
case "${1:-}" in -h|--help|"") usage; exit 0 ;; esac
root=$1
dir=$2
shift 2
export LC_ALL=C
cd "$root" || exit 2
list=$(mktemp)
trap 'rm -f "$list"' EXIT
find "$dir" -type f \( -name '*.html' -o -name '*.htm' -o -name '*.xht' -o -name '*.xhtml' -o -name '*.xml' -o -name '*.svg' \) \
  -not -path '*/support/*' -not -path '*/reference/*' | sort >"$list.all"
: >"$list"
printed=0
while IFS= read -r f; do
  skip=
  for x in "$@"; do
    case "$f" in "$dir/$x"*) skip=1 ;; esac
  done
  [ -n "$skip" ] && continue
  case "$f" in *-print.*) printed=$((printed + 1)); continue ;; esac
  if grep -Eiq "rel=[\"']?(mis)?match" "$f"; then echo "$f" >>"$list"; fi
done <"$list.all"
rm -f "$list.all"
total=$(wc -l <"$list" | tr -d ' ')
script=$(xargs grep -il '<script' <"$list" | wc -l | tr -d ' ')
free=$(xargs grep -iL '<script' <"$list")
nfree=$(printf '%s\n' "$free" | grep -c .)
xmldoc=$(printf '%s\n' "$free" | grep -Ec '\.(xht|xhtml|xml|svg)$')
htmlxmlref=0
while IFS= read -r f; do
  case "$f" in *.xht|*.xhtml|*.xml|*.svg) continue ;; esac
  if grep -Eio "rel=[\"']?(mis)?match[\"']?[^>]*href=[\"'][^\"']+|href=[\"'][^\"']+[\"'][^>]*rel=[\"']?(mis)?match" "$f" |
     grep -Eio "href=[\"'][^\"']+" | grep -Eiq '\.(xht|xhtml|xml|svg)$'; then
    htmlxmlref=$((htmlxmlref + 1))
  fi
done < <(printf '%s\n' "$free")
neither=$((nfree - xmldoc - htmlxmlref))
echo "directory: $dir; excluded: ${*:-none}"
echo "print reference tests skipped by name: $printed"
echo "reference tests: $total"
echo "  with <script: $script"
echo "  without: $nfree"
echo "    XML documents: $xmldoc"
echo "    HTML naming an XML reference: $htmlxmlref"
echo "    needing neither script nor XML: $neither"
