#!/usr/bin/env python3
"""Region-accurate UI analysis of a Chronos-FM screenshot.

For each spec region (nav, title, tabs, address, sidebar, list, footer):
  avg colour, count of pixels that differ from the base bg by > tol,
  count of fg-like (#CDD6F4) pixels.

Also prints a text-density strip map (non-bg pixels per 10px row).
Usage: python3 t037_region_map.py <png>
"""
import sys

from PIL import Image

FN = sys.argv[1]
im = Image.open(FN).convert("RGB")
W, H = im.size
px = im.load()
BG = (30, 30, 46)  # #1e1e2e product bg
FG = (205, 214, 244)  # #CDD6F4 fg
TOL = 40
print(f"size {W}x{H}")


def stats(x0, y0, x1, y1):
    x1 = min(x1, W)
    y1 = min(y1, H)
    s = [0, 0, 0]
    non_bg = 0
    fg_like = 0
    n = 0
    for y in range(y0, y1):
        for x in range(x0, x1):
            r, g, b = px[x, y]
            s[0] += r
            s[1] += g
            s[2] += b
            n += 1
            if sum(abs(c - t) for c, t in zip((r, g, b), BG)) > TOL:
                non_bg += 1
            if all(abs(c - t) < TOL for c, t in zip((r, g, b), FG)):
                fg_like += 1
    avg = tuple(c // n for c in s) if n else (0, 0, 0)
    return avg, non_bg, fg_like, n


regions = [
    ("nav", 0, 0, 64, H),
    ("title", 64, 0, W, 36),
    ("tabs", 0, 36, W, 68),
    ("address", 0, 68, W, 110),
    ("sidebar", 64, 110, 276, H - 28),
    ("list", 276, 110, W, H - 28),
    ("footer", 0, H - 28, W, H),
]
print("=== regions ===")
for name, x0, y0, x1, y1 in regions:
    avg, non_bg, fg_like, n = stats(x0, y0, x1, y1)
    pct = 100.0 * non_bg / n
    print(f"  {name:8s} {x0:4d},{y0:4d} {x1:4d},{y1:4d}  avg={avg}  nonbg={non_bg:7d} ({pct:5.2f}%)  fg={fg_like}")

print("=== text-density map (non-bg px per 10px row, sampled x64..W) ===")
for y0 in range(0, H, 10):
    y1 = min(y0 + 10, H)
    c = 0
    for y in range(y0, y1):
        for x in range(64, W, 2):
            r, g, b = px[x, y]
            if sum(abs(v - t) for v, t in zip((r, g, b), BG)) > TOL:
                c += 1
    bar = "#" * min(60, c // 4)
    print(f"  y {y0:4d}: {c:5d} {bar}")
