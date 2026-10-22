#!/usr/bin/env bash
# Spike S5 (S5-Q5): make the test images in target/s5/images/, from images that
# ship with macOS and a generated pattern. Nothing is downloaded.
#
# Usage: tools/s5-images.sh [--out DIR]
#
# Sources:
#   photo      /Library/User Pictures/Animals/Zebra.heic, 512 x 512, a photograph
#   landscape  /System/Library/Desktop Pictures/Sonoma.heic, scaled to 3000 x 3000
#   pattern    a 2048 x 2048 image of gradients, edges and noise (s5-image pattern)
#   tiny       the same pattern at 16 x 16, for warm-up decodes (s5-image --warmup)
# Each is converted to sRGB, then written as PNG, JPEG (quality 85), GIF and AVIF
# with sips, and as lossless WebP with image-webp's encoder (macOS can read WebP
# but not write it). The decoders' outputs are then comparable without colour
# management.
# Written for bash 3.2 (macOS /bin/bash).
set -euo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd)
OUT="$ROOT/target/s5/images"
while [ $# -gt 0 ]; do
  case $1 in
    --out) OUT=$2; shift ;;
    -h | --help) sed -n '2,16p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
    *) echo "s5-images: unknown argument $1" >&2; exit 2 ;;
  esac
  shift
done

cd "$ROOT"
cargo build --manifest-path "$ROOT/spikes/Cargo.toml" -q --release -p s5-image
mkdir -p "$OUT/src"
SRGB="/System/Library/ColorSync/Profiles/sRGB Profile.icc"

sips -s format png -m "$SRGB" "/Library/User Pictures/Animals/Zebra.heic" --out "$OUT/src/photo.png" >/dev/null
sips -s format png -m "$SRGB" -z 3000 3000 "/System/Library/Desktop Pictures/Sonoma.heic" --out "$OUT/src/landscape.png" >/dev/null
target/release/s5-image pattern "$OUT/src/pattern.png" 2048 2048
target/release/s5-image pattern "$OUT/src/tiny.png" 16 16

for name in photo landscape pattern tiny; do
  src="$OUT/src/$name.png"
  cp -f "$src" "$OUT/$name.png"
  sips -s format jpeg -s formatOptions 85 "$src" --out "$OUT/$name.jpg" >/dev/null
  sips -s format gif "$src" --out "$OUT/$name.gif" >/dev/null
  sips -s format avif "$src" --out "$OUT/$name.avif" >/dev/null
  target/release/s5-image encode-webp "$src" "$OUT/$name.webp"
done
ls -l "$OUT" | awk 'NR > 1 && $NF !~ /^src$/ { printf "%10d  %s\n", $5, $NF }'
