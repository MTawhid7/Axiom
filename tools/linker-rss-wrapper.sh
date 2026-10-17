#!/usr/bin/env bash
# Linker wrapper (`-C linker=<this-script>`) that records each link's wall time
# and peak memory. It delegates to `cc` exactly as rustc does on macOS.
# `/usr/bin/time` includes waited-for children, so the measured peak covers
# both the `cc` driver and `ld`.
#
# If AXIOM_LINK_LOG is set, it appends one TSV line per link:
#   output_name  wall_s  max_rss_bytes  peak_footprint_bytes  exit_status
# The output name is "-" when rustc passes the arguments in an @response-file.
set -u

if [ -z "${AXIOM_LINK_LOG:-}" ]; then exec cc "$@"; fi

out="-" prev=""
for arg in "$@"; do
  if [ "$prev" = "-o" ]; then out=$(basename "$arg"); fi
  prev=$arg
done

tmp=$(mktemp "${TMPDIR:-/tmp}/link-rss.XXXXXX")
/usr/bin/time -l -o "$tmp" cc "$@"
status=$?
awk -v out="$out" -v status="$status" '
  / real /                      { wall = $1 }
  /maximum resident set size/   { rss = $1 }
  /peak memory footprint/       { peak = $1 }
  END { printf "%s\t%s\t%s\t%s\t%s\n", out, wall, rss + 0, peak + 0, status }
' "$tmp" >>"$AXIOM_LINK_LOG"
rm -f "$tmp"
exit "$status"
