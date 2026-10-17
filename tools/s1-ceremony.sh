#!/usr/bin/env bash
# Spike S1 ceremony metrics (S1-Q2): what each GC pattern asks of the code
# that uses it.
#
# Usage: tools/s1-ceremony.sh
#
# Counts, in the two DOM crates (spikes/s1/a-dom, spikes/s1/b-dom), the
# GC-API tokens each pattern requires, and the code size of each file and of
# each DOM operation. Comments are excluded: lines that start with //, and
# anything after // on a line (the DOM crates have no string literals
# containing //). Prints Markdown.
# Written for bash 3.2 (macOS /bin/bash).
set -uo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd)
cd "$ROOT" || exit 2
A=spikes/s1/a-dom/src
B=spikes/s1/b-dom/src

# Source of a crate with comments removed.
code() { # dir
  cat "$1"/*.rs | grep -Ev '^[[:space:]]*//' | sed 's://.*$::'
}

# Occurrences of an extended regex in a crate's code.
count() { # dir ere
  code "$1" | grep -oE "$2" | wc -l | tr -d ' '
}

# Non-blank, non-comment lines of a file.
sloc() { # file
  grep -Ev '^[[:space:]]*(//|$)' "$1" | wc -l | tr -d ' '
}

# Non-blank, non-comment lines of a function: from its `fn` line to the brace
# that closes its body.
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

row() { printf '| %s | %s | %s | %s | %s |\n' "$@"; }

echo "## GC-API tokens in the DOM crates"
echo
row "Concern" "Pattern A token" "A" "Pattern B token" "B"
row "---" "---" "---:" "---" "---:"
row "Write barrier" '`Gc::write(…)`' "$(count $A 'Gc::write\(')" "—" 0
row "Field projection behind the barrier" '`unlock!(…)`' "$(count $A 'unlock!\(')" "—" 0
row "GC-pointer fields needing interior mutability" '`Lock<…>` / `RefLock<…>`' "$(count $A '(^|[^A-Za-z])Lock<') / $(count $A 'RefLock<')" "—" 0
row "Tracing: types" '`#[derive(…Trace…)]`' "$(code $A | grep -E 'derive\(' | grep -c 'Trace')" '`impl Trace for` / `impl Roots for`' "$(count $B 'impl Trace for') / $(count $B 'impl Roots for')"
row "Tracing: per-field work" '`#[trace(require_static)]`' "$(count $A 'require_static')" '`marker.mark…` / `forwarding.relocate…` calls' "$(count $B 'marker\.mark') / $(count $B 'forwarding\.relocate')"
row "Collection context passed to functions" '`&Mutation<…>` parameters' "$(count $A '&Mutation<')" '`GcScope<…>` parameters' "$(count $B 'GcScope<')"
row "Scope handling" '`mutate(` / `mutate_root(`' "$(count $A '\.mutate\(') / $(count $A '\.mutate_root\(')" '`.reborrow()` / `.nogc()` / `.into_nogc()`' "$(count $B '\.reborrow\(\)') / $(count $B '\.nogc\(\)') / $(count $B '\.into_nogc\(\)')"
row "Handle lifetime changes" "—" 0 '`.bind(…)` / `.unbind()`' "$(count $B '\.bind\(') / $(count $B '\.unbind\(\)')"
row "Rooting across a possible collection" "— (state lives in the root)" 0 '`.frame()` / `.scope(&…)` / `Scoped::get()`' "$(count $B '\.frame\(\)') / $(count $B '\.scope\(&') / $(count $B '\.get\(\)')"
row "Collection points" '`Step::Yielded` / `wants_collection` / `collect_debt(`' "$(count $A 'Step::Yielded') / $(count $A 'wants_collection') / $(count $A 'collect_debt\(')" '`safepoint(`' "$(count $B 'safepoint\(')"
echo

echo "## Code size (non-blank, non-comment lines)"
echo
row "File" "Pattern A" "lines" "Pattern B" "lines"
row "---" "---" "---:" "---" "---:"
row "Crate root" '`lib.rs`' "$(sloc $A/lib.rs)" '`lib.rs`' "$(sloc $B/lib.rs)"
row "Object model and operations" '`dom.rs`' "$(sloc $A/dom.rs)" '`dom.rs` (includes dispatch)' "$(sloc $B/dom.rs)"
row "Dispatch" '`dispatch.rs` (resumable state machine)' "$(sloc $A/dispatch.rs)" "(in \`dom.rs\`)" "—"
row "Script driver" '`driver.rs`' "$(sloc $A/driver.rs)" '`driver.rs`' "$(sloc $B/driver.rs)"
total_a=0; for f in $A/*.rs; do total_a=$((total_a + $(sloc "$f"))); done
total_b=0; for f in $B/*.rs; do total_b=$((total_b + $(sloc "$f"))); done
row "**Total**" "" "**$total_a**" "" "**$total_b**"
echo

echo "## Lines per DOM operation"
echo
row "Operation" "Pattern A" "lines" "Pattern B" "lines"
row "---" "---" "---:" "---" "---:"
for op in remove insert_before set_attribute add_listener remove_listener tree_digest; do
  row "\`$op\`" "\`dom::$op\`" "$(fn_sloc $A/dom.rs $op)" "\`dom::$op\`" "$(fn_sloc $B/dom.rs $op)"
done
a_dispatch=$(($(fn_sloc $A/dispatch.rs begin) + $(fn_sloc $A/dispatch.rs visit_count) + $(fn_sloc $A/dispatch.rs visit_at) + $(fn_sloc $A/dispatch.rs step) + $(fn_sloc $A/dispatch.rs run)))
b_dispatch=$(($(fn_sloc $B/dom.rs dispatch) + $(fn_sloc $B/dom.rs run_closure)))
row "dispatch (functions only)" '`Dispatch::{begin, visit_count, visit_at, step, run}`' "$a_dispatch" '`dispatch` + `run_closure`' "$b_dispatch"
row "dispatch (whole module)" '`dispatch.rs`' "$(sloc $A/dispatch.rs)" "—" "—"
