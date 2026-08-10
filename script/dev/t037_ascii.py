#!/usr/bin/env python3
"""Render a PNG as an ASCII brightness map (cols x rows).

Usage: python3 t037_ascii.py <png> [cols] [rows]
Dark pixels -> ' ', bright -> '#'. Also prints dominant hue hints per cell
by mapping (r>g>b)=purple-ish to p, gray to ' '.
"""
import sys

from PIL import Image

FN = sys.argv[1]
COLS = int(sys.argv[2]) if len(sys.argv) > 2 else 100
ROWS = int(sys.argv[3]) if len(sys.argv) > 3 else 44
im = Image.open(FN).convert("RGB")
W, H = im.size
px = im.load()
cw = W / COLS
ch = H / ROWS
chars = " .:-=+*#%@"
print(f"--- {FN} {W}x{H} -> {COLS}x{ROWS} ---")
for ry in range(ROWS):
    line = []
    for rx in range(COLS):
        x0 = int(rx * cw)
        x1 = max(x0 + 1, int((rx + 1) * cw))
        y0 = int(ry * ch)
        y1 = max(y0 + 1, int((ry + 1) * ch))
        s = [0, 0, 0]
        n = 0
        for y in range(y0, y1):
            for x in range(x0, x1):
                r, g, b = px[x, y]
                s[0] += r
                s[1] += g
                s[2] += b
                n += 1
        r, g, b = (c // n for c in s)
        lum = (r * 3 + g * 6 + b) // 10
        idx = min(len(chars) - 1, (lum * (len(chars) - 1)) // 255)
        c = chars[idx]
        # hue hints: purple-ish tint (nav/sidebar in this theme) -> lowercase 'p'
        if r > 38 and r > g + 6 and r > b + 6 and lum < 70:
            c = "p"
        elif b > r + 6 and b > 10 and lum < 90:
            c = "q"
        line.append(c)
    print("".join(line))
