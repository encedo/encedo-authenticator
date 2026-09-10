#!/usr/bin/env python3
"""iOS app icons from a square, opaque source.

App Store icons may not carry transparency: the disc artwork that suits an
Android launcher leaves white corners on an iPhone. This builds a full-bleed
square (the same green ramp as the disc, with the head glyph centred) and
writes every size the Xcode asset catalogue asks for.

    scripts/ios-icons.py [path/to/gen/apple]
"""
import re
import sys
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parent.parent
GLYPH = Image.open(ROOT / 'assets' / 'notification-icon.png')     # white head, transparent
DISC = Image.open(ROOT / 'assets' / 'icon-1024.png').convert('RGBA')
SQUARE = ROOT / 'assets' / 'icon-ios-1024.png'


def build_square(side: int = 1024) -> Image.Image:
    """The disc's own gradient, stretched across the whole square."""
    left = DISC.getpixel((40, DISC.size[1] // 2))[:3]
    right = DISC.getpixel((DISC.size[0] - 40, DISC.size[1] // 2))[:3]
    ramp = Image.new('RGB', (side, 1))
    px = ramp.load()
    for x in range(side):
        t = x / (side - 1)
        px[x, 0] = tuple(round(left[i] + (right[i] - left[i]) * t) for i in range(3))
    im = ramp.resize((side, side))
    inner = round(side * 0.62)
    glyph = GLYPH.resize((inner, inner), Image.LANCZOS)
    im.paste(glyph, ((side - inner) // 2, (side - inner) // 2), glyph)
    return im


def main() -> None:
    gen = Path(sys.argv[1]) if len(sys.argv) > 1 else ROOT / 'src-tauri' / 'gen' / 'apple'
    square = build_square()
    square.save(SQUARE)
    targets = [ROOT / 'src-tauri' / 'icons' / 'ios', gen / 'Assets.xcassets' / 'AppIcon.appiconset']
    written = 0
    for folder in targets:
        if not folder.is_dir():
            continue
        for f in sorted(folder.glob('AppIcon-*.png')):
            m = re.match(r'AppIcon-([\d.]+)x[\d.]+@(\d)x(?:-\d)?\.png', f.name)
            if not m:
                continue
            side = round(float(m.group(1)) * int(m.group(2)))
            square.resize((side, side), Image.LANCZOS).save(f)
            written += 1
    print(f'wrote {written} icons from {SQUARE.name}')


if __name__ == '__main__':
    main()
