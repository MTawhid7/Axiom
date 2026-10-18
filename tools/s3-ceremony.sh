#!/usr/bin/env bash
# Spike S3-Q5 ceremony: what generated setters for handle-valued fields (ADR-0008
# SH9) add to S1-Q2's count.
#
# Usage: tools/s3-ceremony.sh
#
# Compares S1's Pattern B DOM (spikes/s1/b-dom/src/dom.rs), where handle-valued
# fields are public and written by plain assignment, with its port to s3-gc
# (spikes/s3/bench/src/dom.rs), where they are private, read through getters
# and written through setters that s3_gc::setters! generates. Counts S1-Q2's
# GC-API tokens in both (tools/s1-ceremony.sh's method), the field writes and
# reads, and code size. Comments are excluded: lines that start with //, and
# anything after // on a line. Prints Markdown.
# Written for bash 3.2 (macOS /bin/bash).
set -uo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd)
cd "$ROOT" || exit 2
case ${1:-} in
  -h | --help) sed -n '2,15p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
esac
S1=spikes/s1/b-dom/src/dom.rs
S3=spikes/s3/bench/src/dom.rs
LINKS='(parent|first_child|last_child|prev_sibling|next_sibling)'

code() { # file
  grep -Ev '^[[:space:]]*//' "$1" | sed 's://.*$::'
}

count() { # file ere
  code "$1" | grep -oE "$2" | wc -l | tr -d ' '
}

sloc() { # file
  code "$1" | grep -cEv '^[[:space:]]*$'
}

# Non-blank, non-comment lines of a function, from its `fn` line to the brace
# that closes its body (tools/s1-ceremony.sh's method).
fn_sloc() { # file name
  awk -v name="$2" '
    !inside && $0 ~ "^[[:space:]]*(pub(\\([a-z]+\\))? )?fn " name "[<(]" { inside = 1; depth = 0; opened = 0; n = 0 }
    inside {
      line = $0
      sub("//.*$", "", line)
      if (line ~ /[^[:space:]]/) n++
      depth += gsub(/\{/, "{", line)
      if (depth > 0) opened = 1
      depth -= gsub(/\}/, "}", line)
      if (opened && depth == 0) { print n; exit }
    }' "$1"
}

row() { printf '| %s | %s | %s |\n' "$@"; }

# Occurrences of a Perl regex in a file's code.
pcount() { # file regex
  code "$1" | perl -ne 'BEGIN { $n = 0 } $n++ while /'"$2"'/g; END { print $n }'
}

echo "## Handle-valued fields (call sites; the Trace impls are not counted)"
echo
row "Concern" "S1 (plain fields)" "Port (setters)"
row "---" "---:" "---:"
row "Link writes: \`x.link = …\` / \`set_link(x, …)\`" \
  "$(pcount $S1 "(?<!self)\\.$LINKS\\s*=(?!=)")" "$(pcount $S3 "\\.set_$LINKS\\(")"
row "Listener-list writes: \`listeners.push\`, \`.remove\` / \`push_listener\`, \`remove_listener_at\`" \
  "$(pcount $S1 'listeners\.(push|remove)\(')" "$(pcount $S3 '\.(push_listener|remove_listener_at)\(')"
row "Link reads: \`x.link\` / \`x.link()\`" \
  "$(pcount $S1 "(?<!self)\\.$LINKS\\b(?!\\s*=(?!=))(?!\\()")" "$(pcount $S3 "\\.$LINKS\\(\\)")"
row "Accessor declarations" \
  "0" "$(count $S3 "pub fn $LINKS\\(") getters, $(count $S3 "set_$LINKS => ") setters, $(count $S3 'pub\(super\) fn new\(') constructors"
echo

echo "## S1-Q2's GC-API tokens"
echo
row "Token" "S1" "Port"
row "---" "---:" "---:"
for t in 'impl Trace for' 'impl Roots for' 'marker\.mark' 'forwarding\.relocate' 'GcScope<' '\.reborrow\(\)' '\.nogc\(\)' '\.into_nogc\(\)' '\.bind\(' '\.unbind\(\)' '\.frame\(\)' '\.scope\(&' '\.get\(\)' 'safepoint\('; do
  label=$(echo "$t" | sed 's/\\//g')
  row "\`$label\`" "$(count $S1 "$t")" "$(count $S3 "$t")"
done
echo

echo "## Code size (non-blank, non-comment lines)"
echo
row "Function" "S1" "Port"
row "---" "---:" "---:"
row "Whole file (\`dom.rs\`)" "$(sloc $S1)" "$(sloc $S3)"
for f in remove insert_before add_listener remove_listener dispatch run_closure tree_digest; do
  row "\`$f\`" "$(fn_sloc $S1 $f)" "$(fn_sloc $S3 $f)"
done
