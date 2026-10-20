#!/usr/bin/env python3
"""Pixel values of regions of a PNG, and the colour profile that says how to read them.

    python3 -I tools/png-region-stats.py IMAGE.png NAME:X0,Y0,X1,Y1 [...]

Each region is a half-open rectangle of pixels: columns X0 to X1 - 1 and rows
Y0 to Y1 - 1. For each one it prints the mean colour and the distinct values
(up to four). Then it prints how the values are encoded: the PNG's cICP chunk
(colour primaries, transfer function, matrix, range, as in ITU-T H.273), and
the embedded ICC profile's description, colorants (rXYZ, gXYZ and bXYZ, which
v2 and v4 profiles adapt to D50), white point and tone curves' types.

Only 8-bit RGB or RGBA, non-interlaced PNGs are read, with the standard
library alone (zlib and PNG's five scanline filters), and only down to the
last row a region needs. Used for S6-Q5's Display P3 check on a screenshot of
the test pattern.
"""

import argparse
import struct
import sys
import zlib

SIGNATURE = b"\x89PNG\r\n\x1a\n"


def chunks(data):
    if data[:8] != SIGNATURE:
        sys.exit("not a PNG file")
    pos = 8
    while pos + 8 <= len(data):
        (length,) = struct.unpack(">I", data[pos : pos + 4])
        kind = data[pos + 4 : pos + 8]
        yield kind, data[pos + 8 : pos + 8 + length]
        pos += 12 + length
        if kind == b"IEND":
            return


def paeth(a, b, c):
    p = a + b - c
    pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
    if pa <= pb and pa <= pc:
        return a
    return b if pb <= pc else c


def decode_rows(header, idat, rows):
    """The first `rows` scanlines, unfiltered, as bytearrays of RGB(A) bytes."""
    width, bpp = header["width"], header["bpp"]
    stride = width * bpp
    inflate = zlib.decompressobj()
    raw = bytearray()
    for chunk in idat:
        raw += inflate.decompress(chunk)
        if len(raw) >= (stride + 1) * rows:
            break
    if len(raw) < (stride + 1) * rows:
        sys.exit("the image data ends before the rows asked for")
    previous = bytearray(stride)
    lines = []
    for y in range(rows):
        start = y * (stride + 1)
        kind = raw[start]
        line = bytearray(raw[start + 1 : start + 1 + stride])
        if kind == 1:
            for i in range(bpp, stride):
                line[i] = (line[i] + line[i - bpp]) & 255
        elif kind == 2:
            for i in range(stride):
                line[i] = (line[i] + previous[i]) & 255
        elif kind == 3:
            for i in range(stride):
                left = line[i - bpp] if i >= bpp else 0
                line[i] = (line[i] + ((left + previous[i]) >> 1)) & 255
        elif kind == 4:
            for i in range(stride):
                left = line[i - bpp] if i >= bpp else 0
                up_left = previous[i - bpp] if i >= bpp else 0
                line[i] = (line[i] + paeth(left, previous[i], up_left)) & 255
        elif kind != 0:
            sys.exit(f"unknown scanline filter {kind} in row {y}")
        lines.append(line)
        previous = line
    return lines


def describe_icc(icc):
    def s15(b):
        return struct.unpack(">i", b)[0] / 65536

    print(f"ICC profile: {len(icc)} bytes, version {icc[8]}.{icc[9] >> 4}, class {icc[12:16].decode('latin-1')}, "
          f"colour space {icc[16:20].decode('latin-1')}")
    (count,) = struct.unpack(">I", icc[128:132])
    tags = {}
    for i in range(count):
        signature, offset, size = struct.unpack(">4sII", icc[132 + 12 * i : 144 + 12 * i])
        tags[signature.decode("latin-1")] = icc[offset : offset + size]
    description = tags.get("desc", b"")
    if description[:4] == b"mluc":
        length, offset = struct.unpack(">II", description[20:28])
        print(f"  description: {description[offset : offset + length].decode('utf-16-be')}")
    elif description[:4] == b"desc":
        (length,) = struct.unpack(">I", description[8:12])
        print(f"  description: {description[12 : 12 + length - 1].decode('latin-1')}")
    for tag in ("rXYZ", "gXYZ", "bXYZ", "wtpt"):
        if tag in tags:
            values = [round(s15(tags[tag][8 + 4 * k : 12 + 4 * k]), 5) for k in range(3)]
            print(f"  {tag}: {values}")
    for tag in ("rTRC", "gTRC", "bTRC"):
        if tag in tags:
            print(f"  {tag}: {tags[tag][:4].decode('latin-1')}")


def main():
    parser = argparse.ArgumentParser(
        description=__doc__.split("\n\n")[0],
        epilog=__doc__.split("\n\n", 1)[1],
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    parser.add_argument("image", help="an 8-bit RGB or RGBA PNG")
    parser.add_argument("regions", nargs="+", help="NAME:X0,Y0,X1,Y1")
    args = parser.parse_args()

    regions = []
    for spec in args.regions:
        try:
            name, coordinates = spec.split(":")
            x0, y0, x1, y1 = (int(v) for v in coordinates.split(","))
        except ValueError:
            sys.exit(f"bad region {spec!r}: expected NAME:X0,Y0,X1,Y1")
        if not (0 <= x0 < x1 and 0 <= y0 < y1):
            sys.exit(f"bad region {spec!r}: empty or negative")
        regions.append((name, x0, y0, x1, y1))

    with open(args.image, "rb") as f:
        data = f.read()
    header, idat, icc, cicp = {}, [], None, None
    for kind, body in chunks(data):
        if kind == b"IHDR":
            width, height, depth, colour_type, _, _, interlace = struct.unpack(">IIBBBBB", body)
            if depth != 8 or colour_type not in (2, 6) or interlace != 0:
                sys.exit("only 8-bit RGB or RGBA, non-interlaced PNGs are read")
            header = {"width": width, "height": height, "bpp": 4 if colour_type == 6 else 3}
        elif kind == b"iCCP":
            _, rest = body.split(b"\0", 1)
            icc = zlib.decompress(rest[1:])
        elif kind == b"cICP":
            cicp = tuple(body)
        elif kind == b"IDAT":
            idat.append(body)
    if not header:
        sys.exit("no IHDR chunk")
    print(f"{args.image}: {header['width']} x {header['height']}, {header['bpp']} bytes a pixel")
    for name, x0, y0, x1, y1 in regions:
        if x1 > header["width"] or y1 > header["height"]:
            sys.exit(f"region {name} lies outside the image")

    lines = decode_rows(header, idat, max(r[4] for r in regions))
    bpp = header["bpp"]
    for name, x0, y0, x1, y1 in regions:
        pixels = [tuple(lines[y][x * bpp : x * bpp + 3]) for y in range(y0, y1) for x in range(x0, x1)]
        mean = tuple(round(sum(p[c] for p in pixels) / len(pixels), 2) for c in range(3))
        distinct = sorted(set(pixels))
        shown = ", ".join(str(p) for p in distinct[:4]) + (", ..." if len(distinct) > 4 else "")
        print(f"{name}: {len(pixels)} pixels, mean {mean}, {len(distinct)} distinct: {shown}")

    if cicp:
        print(f"cICP: primaries {cicp[0]}, transfer {cicp[1]}, matrix {cicp[2]}, full range {cicp[3]}")
    if icc:
        describe_icc(icc)


if __name__ == "__main__":
    main()
