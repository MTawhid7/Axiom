#!/usr/bin/env bash
# Measure build cost on this machine: clean, no-op and incremental builds.
#
# Usage: tools/measure-build.sh [options] [-- <extra cargo build args>]
#   -p, --package NAME   package to build (repeatable; default: --workspace)
#   --release            build the release profile (default: dev)
#   --touch FILE         file touched before the incremental build (default:
#                        src/lib.rs, or src/main.rs, of the first package)
#   --runs N             repeat the clean/no-op/incremental cycle N times (default: 1)
#   --label TEXT         free-form label saved with the results
#   --spikes             measure the spikes' workspace (spikes/Cargo.toml)
#                        instead of the engine's at the root (default)
#   -h, --help
#
# For each phase it reports:
#   - wall time;
#   - the number of rustc compilations, and the largest per-compilation peak
#     RSS (with its crate);
#   - the number of links, and the largest linker peak RSS;
#   - the peak *aggregate* RSS of all concurrent rustc, cc and ld processes,
#     sampled every 200 ms. This is what actually pressures an 8 GB machine.
#
# Builds use an isolated target directory (target/measure/build), so they
# never disturb normal incremental state.
# Raw logs and summary.tsv are written to target/measure/<timestamp>/.
# Written for bash 3.2 (macOS /bin/bash).
set -uo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd)
cd "$ROOT" || exit 2

usage() { sed -n '2,26p' "$0" | sed 's/^# \{0,1\}//'; }

packages="" profile_flag="" touch_file="" runs=1 label="" extra=""
# The workspace measured: the engine's at the root, or the spikes' (since
# M1-CP2 a workspace of their own, ADR-0022 D12).
workspace_dir=crates manifest_flag=""
while [ $# -gt 0 ]; do
  case $1 in
    -p | --package) packages="$packages $2"; shift 2 ;;
    --release) profile_flag="--release"; shift ;;
    --touch) touch_file=$2; shift 2 ;;
    --runs) runs=$2; shift 2 ;;
    --label) label=$2; shift 2 ;;
    --spikes) workspace_dir=spikes manifest_flag="--manifest-path $ROOT/spikes/Cargo.toml"; shift ;;
    -h | --help) usage; exit 0 ;;
    --) shift; extra="$*"; break ;;
    *) echo "unknown argument: $1 (see --help)" >&2; exit 2 ;;
  esac
done

# Resolve a package name to its directory via the manifests (no jq dependency).
package_dir() { # name
  local manifest
  for manifest in $(find "$workspace_dir" -mindepth 2 -name Cargo.toml -not -path '*/target/*' 2>/dev/null | sort); do
    if grep -Eq "^name[[:space:]]*=[[:space:]]*\"$1\"" "$manifest"; then
      dirname "$manifest"
      return 0
    fi
  done
  return 1
}

cargo_args="build $manifest_flag $profile_flag"
if [ -n "$packages" ]; then
  for p in $packages; do cargo_args="$cargo_args -p $p"; done
  first_dir=$(package_dir $(echo $packages | awk '{print $1}')) || {
    echo "package not found: $packages" >&2
    exit 2
  }
else
  cargo_args="$cargo_args --workspace"
  first_dir=$(dirname "$(find "$workspace_dir" -mindepth 2 -name Cargo.toml -not -path '*/target/*' 2>/dev/null | sort | head -1)")
fi
cargo_args="$cargo_args $extra"
if [ -z "$touch_file" ]; then
  if [ -f "$first_dir/src/lib.rs" ]; then touch_file="$first_dir/src/lib.rs"; else touch_file="$first_dir/src/main.rs"; fi
fi
[ -f "$touch_file" ] || { echo "touch file not found: $touch_file" >&2; exit 2; }

stamp=$(date +%Y%m%d-%H%M%S)
run_dir="$ROOT/target/measure/$stamp"
mkdir -p "$run_dir"
export CARGO_TARGET_DIR="$ROOT/target/measure/build"
export RUSTC_WRAPPER="$ROOT/tools/rustc-rss-wrapper.sh"
export RUSTFLAGS="${RUSTFLAGS:-} -C linker=$ROOT/tools/linker-rss-wrapper.sh"

{
  echo "date:        $(date '+%Y-%m-%d %H:%M:%S %Z')"
  echo "label:       ${label:--}"
  echo "git:         $(git rev-parse --short HEAD 2>/dev/null || echo -)$([ -z "$(git status --porcelain 2>/dev/null)" ] || echo ' (uncommitted changes present)')"
  echo "rustc:       $(rustc -V)"
  echo "machine:     $(sysctl -n machdep.cpu.brand_string), $(($(sysctl -n hw.memsize) / 1073741824)) GiB, $(sw_vers -productName) $(sw_vers -productVersion)"
  echo "load avg:    $(sysctl -n vm.loadavg)"
  echo "thermal:     $(pmset -g therm 2>/dev/null | grep -Ei 'warning|limit' | tr -s ' \n' ' ' || echo -)"
  echo "command:     cargo $cargo_args"
  echo "incremental: touch $touch_file"
  echo "runs:        $runs"
} | tee "$run_dir/environment.txt"

# Peak aggregate RSS (KiB) of compiler and linker processes while $1 is alive.
sample_peak() { # watched-pid out-file
  local peak=0 cur
  while kill -0 "$1" 2>/dev/null; do
    cur=$(ps -axo rss=,comm= | awk '
      { name = $2; sub(".*/", "", name) }
      name ~ /^(rustc|cc|clang|ld|ld64)$/ { sum += $1 }
      END { print sum + 0 }')
    if [ "$cur" -gt "$peak" ]; then peak=$cur; fi
    sleep 0.2
  done
  echo "$peak" >"$2"
}

mib() { awk -v b="$1" 'BEGIN { printf "%.1f", b / 1048576 }'; }

summary="$run_dir/summary.tsv"
printf 'run\tphase\twall_s\trustc_n\trustc_max_mib\trustc_max_crate\tlink_n\tlink_max_mib\taggregate_peak_mib\texit\n' >"$summary"

run_phase() { # run-index phase-name
  local tag="$1-$2" build_pid sampler_pid status wall rustc_n rustc_max rustc_crate link_n link_max peak_kib
  export AXIOM_RUSTC_LOG="$run_dir/$tag.rustc.tsv"
  export AXIOM_LINK_LOG="$run_dir/$tag.link.tsv"
  : >"$AXIOM_RUSTC_LOG"
  : >"$AXIOM_LINK_LOG"

  # shellcheck disable=SC2086 # word splitting of cargo_args is intended
  /usr/bin/time -l -o "$run_dir/$tag.cargo.time" cargo $cargo_args >"$run_dir/$tag.cargo.log" 2>&1 &
  build_pid=$!
  sample_peak "$build_pid" "$run_dir/$tag.peak" &
  sampler_pid=$!
  wait "$build_pid"
  status=$?
  wait "$sampler_pid"

  wall=$(awk '/ real / { print $1 }' "$run_dir/$tag.cargo.time")
  rustc_n=$(wc -l <"$AXIOM_RUSTC_LOG" | tr -d ' ')
  rustc_max=$(sort -t "$(printf '\t')" -k4,4n "$AXIOM_RUSTC_LOG" | tail -1 | cut -f4)
  rustc_crate=$(sort -t "$(printf '\t')" -k4,4n "$AXIOM_RUSTC_LOG" | tail -1 | cut -f1)
  link_n=$(wc -l <"$AXIOM_LINK_LOG" | tr -d ' ')
  link_max=$(sort -t "$(printf '\t')" -k3,3n "$AXIOM_LINK_LOG" | tail -1 | cut -f3)
  peak_kib=$(cat "$run_dir/$tag.peak")
  printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' "$1" "$2" "$wall" "$rustc_n" \
    "$(mib "${rustc_max:-0}")" "${rustc_crate:--}" "$link_n" "$(mib "${link_max:-0}")" \
    "$(mib $((peak_kib * 1024)))" "$status" >>"$summary"
  if [ "$status" -ne 0 ]; then
    echo "build failed in phase $tag; see $run_dir/$tag.cargo.log" >&2
    tail -20 "$run_dir/$tag.cargo.log" >&2
    exit 1
  fi
}

i=1
while [ "$i" -le "$runs" ]; do
  rm -rf "$CARGO_TARGET_DIR"
  run_phase "$i" clean
  run_phase "$i" noop
  touch "$touch_file"
  run_phase "$i" incremental
  i=$((i + 1))
done

echo
column -t -s "$(printf '\t')" "$summary"
if [ "$runs" -gt 1 ]; then
  echo
  for phase in clean noop incremental; do
    awk -F '\t' -v p="$phase" '$2 == p { print $3 }' "$summary" | sort -n |
      awk -v p="$phase" '{ v[NR] = $1 } END { printf "median wall %-12s %s s (n=%d)\n", p, v[int((NR + 1) / 2)], NR }'
  done
fi
echo
echo "raw results: ${run_dir#"$ROOT"/}"
