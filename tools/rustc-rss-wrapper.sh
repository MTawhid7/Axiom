#!/usr/bin/env bash
# RUSTC_WRAPPER that records each rustc compilation's wall time and peak memory.
#
# Cargo invokes it as: <this-script> <path-to-rustc> <rustc-args...>
# If ARBOR_RUSTC_LOG is set, it appends one TSV line per compilation:
#   crate_name  crate_types  wall_s  max_rss_bytes  peak_footprint_bytes  exit_status
# Probes (`rustc -vV`, target-info queries) carry no --crate-name, so they are
# passed through unmeasured. rustc's own stdout and stderr are untouched: cargo
# parses its JSON diagnostics.
set -u

if [ -z "${ARBOR_RUSTC_LOG:-}" ]; then exec "$@"; fi

crate="" types="" prev=""
for arg in "$@"; do
  case $prev in
    --crate-name) crate=$arg ;;
    --crate-type) types="${types:+$types,}$arg" ;;
  esac
  prev=$arg
done
if [ -z "$crate" ]; then exec "$@"; fi

tmp=$(mktemp "${TMPDIR:-/tmp}/rustc-rss.XXXXXX")
/usr/bin/time -l -o "$tmp" "$@"
status=$?
awk -v crate="$crate" -v types="${types:--}" -v status="$status" '
  / real /                      { wall = $1 }
  /maximum resident set size/   { rss = $1 }
  /peak memory footprint/       { peak = $1 }
  END { printf "%s\t%s\t%s\t%s\t%s\t%s\n", crate, types, wall, rss + 0, peak + 0, status }
' "$tmp" >>"$ARBOR_RUSTC_LOG"
rm -f "$tmp"
exit "$status"
