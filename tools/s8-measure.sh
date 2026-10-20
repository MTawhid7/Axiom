#!/usr/bin/env bash
# Spike S8, the corpus census: the automated census, on a formal run.
#
# Usage: tools/s8-measure.sh [--runs N] [--cooldown SECONDS] [--seconds S]
#                            [--pages LIST] [--allow-battery] [--quick]
#   --runs N            runs of each page (default 3)
#   --cooldown SECONDS  idle time before each page's run (default 60), which also
#                       spaces loads of one site at least that far apart (CO4)
#   --seconds S         length of the interaction and of the time without input
#                       (default 30)
#   --pages LIST        comma-separated pages (default: every page the corpus lets
#                       automation load: wikipedia, maps, mdn, threejs, vscode)
#   --allow-battery     run without AC power, or with Low Power Mode on
#   --quick             one run of each page, 10 s phases, 5 s cool-downs: a check
#                       of the tools against the live pages, which counts towards
#                       each site's limit of five loads per census (CO4)
#
# The method is docs/corpus.md's (CO3-CO5, CO9-CO11) and the Step 6 tracker's
# §10.1. It needs AC power, the Mac otherwise idle, and Chrome Canary's window
# visible at the top left of the screen: leave the desktop showing. Each page's
# robots.txt is read first, and a page it disallows is refused.
#
# Runs S8's offline tests, downloads Chromium's use-counter enums for the
# installed Canary if they're missing (s8.py prepare), then the census
# (s8.py auto). Writes target/s8-results/<timestamp>-auto/, with summary.txt.
# The census by hand needs the owner at the keyboard: see
# `python3 -I spikes/s8/s8.py --help` (signin, hand).
# Written for bash 3.2 (macOS /bin/bash).
set -uo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd)
cd "$ROOT" || exit 2

runs=3 cooldown=60 seconds=30 pages="" battery=""
while [ $# -gt 0 ]; do
  case $1 in
    --runs) runs=$2; shift 2 ;;
    --cooldown) cooldown=$2; shift 2 ;;
    --seconds) seconds=$2; shift 2 ;;
    --pages) pages=$2; shift 2 ;;
    --allow-battery) battery=--allow-battery; shift ;;
    --quick) runs=1 cooldown=5 seconds=10; shift ;;
    -h | --help) sed -n '2,31p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
    *) echo "unknown argument: $1 (see --help)" >&2; exit 2 ;;
  esac
done

python3 -I spikes/s8/test_s8.py >/dev/null 2>&1 || { echo "S8's offline tests failed: python3 -I spikes/s8/test_s8.py" >&2; exit 1; }
python3 -I spikes/s8/s8.py prepare || exit 1
set -- --runs "$runs" --cooldown "$cooldown" --seconds "$seconds"
[ -n "$pages" ] && set -- "$@" --pages "$pages"
[ -n "$battery" ] && set -- "$@" "$battery"
exec python3 -I spikes/s8/s8.py auto "$@"
