#!/usr/bin/env python3
"""Prepare a macOS-friendly app icon: center crop, inset artwork, transparent canvas.

macOS Dock applies a squircle mask automatically. Artwork should NOT fill the
full 1024×1024 canvas — leave transparent margins so the icon matches system apps.
"""

from __future__ import annotations

import sys
from pathlib import Path

from PIL import Image

CANVAS = 1024
# ~81% content area — similar visual weight to Apple system icons in the Dock
CONTENT = 832


def prepare_icon(src: Path, dest: Path) -> None:
    image = Image.open(src).convert("RGBA")
    width, height = image.size
    side = min(width, height)
    left = (width - side) // 2
    top = (height - side) // 2
    image = image.crop((left, top, left + side, top + side))
    image = image.resize((CONTENT, CONTENT), Image.Resampling.LANCZOS)

    canvas = Image.new("RGBA", (CANVAS, CANVAS), (0, 0, 0, 0))
    offset = (CANVAS - CONTENT) // 2
    canvas.paste(image, (offset, offset), image)
    dest.parent.mkdir(parents=True, exist_ok=True)
    canvas.save(dest, "PNG")
    margin = offset
    print(
        f"Saved {dest} ({CANVAS}×{CANVAS}, content {CONTENT}×{CONTENT}, "
        f"margin {margin}px, transparent padding)"
    )


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
