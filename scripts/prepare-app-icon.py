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


def sample_pad_color(image: Image.Image) -> tuple[int, int, int]:
    """Use the image's own edge colors for padding — never a hard-coded brand color."""
    rgba = image.convert("RGBA")
    width, height = rgba.size
    strip = max(1, min(width, height) // 64)
    pixels: list[tuple[int, int, int]] = []
    for y in range(height):
        for x in range(width):
            if x < strip or x >= width - strip or y < strip or y >= height - strip:
                red, green, blue, alpha = rgba.getpixel((x, y))
                if alpha > 128:
                    pixels.append((red, green, blue))
    if not pixels:
        red, green, blue, _ = rgba.getpixel((width // 2, height // 2))
        return (red, green, blue)
    return tuple(sum(channel[i] for channel in pixels) // len(pixels) for i in range(3))


def prepare_icon(src: Path, dest: Path) -> None:
    image = Image.open(src).convert("RGBA")
    width, height = image.size
    side = min(width, height)
    left = (width - side) // 2
    top = (height - side) // 2
    image = image.crop((left, top, left + side, top + side))
    pad_rgb = sample_pad_color(image)
    image = image.resize((CONTENT, CONTENT), Image.Resampling.LANCZOS)

    canvas = Image.new("RGB", (CANVAS, CANVAS), pad_rgb)
    offset = (CANVAS - CONTENT) // 2
    if image.mode == "RGBA":
        canvas.paste(image, (offset, offset), image)
    else:
        canvas.paste(image, (offset, offset))
    dest.parent.mkdir(parents=True, exist_ok=True)
    canvas.save(dest, "PNG")
    print(
        f"Saved {dest} ({CANVAS}×{CANVAS}, pad rgb={pad_rgb}, content {CONTENT}×{CONTENT})"
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
