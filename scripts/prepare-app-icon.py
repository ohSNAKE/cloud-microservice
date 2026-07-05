#!/usr/bin/env python3
"""Prepare a custom photo for macOS app icon.

Uses opaque padding (not transparent) — transparent corners become black in Dock.
"""

from __future__ import annotations

import sys
from pathlib import Path

from PIL import Image

CANVAS = 1024
CONTENT = 832
# Opaque pad color (matches brand icon top blue)
PAD_RGB = (59, 130, 246)


def prepare_icon(src: Path, dest: Path) -> None:
    image = Image.open(src).convert("RGBA")
    width, height = image.size
    side = min(width, height)
    left = (width - side) // 2
    top = (height - side) // 2
    image = image.crop((left, top, left + side, top + side))
    image = image.resize((CONTENT, CONTENT), Image.Resampling.LANCZOS)

    canvas = Image.new("RGB", (CANVAS, CANVAS), PAD_RGB)
    offset = (CANVAS - CONTENT) // 2
    if image.mode == "RGBA":
        canvas.paste(image, (offset, offset), image)
    else:
        canvas.paste(image, (offset, offset))
    dest.parent.mkdir(parents=True, exist_ok=True)
    canvas.save(dest, "PNG")
    print(f"Saved {dest} ({CANVAS}×{CANVAS}, opaque padding, content {CONTENT}×{CONTENT})")


def main() -> int:
    if len(sys.argv) != 3:
        print(f"Usage: {sys.argv[0]} <input-image> <output.png>", file=sys.stderr)
        return 1
    src = Path(sys.argv[1])
    dest = Path(sys.argv[2])
    if not src.is_file():
        print(f"Input not found: {src}", file=sys.stderr)
        return 1
    prepare_icon(src, dest)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
