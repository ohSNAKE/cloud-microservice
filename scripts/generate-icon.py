#!/usr/bin/env python3
"""Generate a flat minimalist icon for 穷鬼 / 财记.

macOS applies the squircle mask at display time. The source PNG must fill
the entire 1024×1024 canvas with opaque pixels — transparent corners become
a black border in the Dock / icns.
"""

from __future__ import annotations

from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

SIZE = 1024
ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "assets" / "app-icon-source.png"

# App brand palette (flat, no gradients)
BG = (22, 119, 255)  # #1677ff
WHITE = (255, 255, 255)
INK = (22, 119, 255)


def load_font(size: int) -> ImageFont.FreeTypeFont | ImageFont.ImageFont:
    candidates = [
        "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf",
        "/System/Library/Fonts/Supplemental/Arial Bold.ttf",
        "/Library/Fonts/Arial Bold.ttf",
    ]
    for path in candidates:
        try:
            return ImageFont.truetype(path, size)
        except OSError:
            continue
    return ImageFont.load_default()


def draw_yen_mark(draw: ImageDraw.ImageDraw, cx: int, cy: int, size: int) -> None:
    """Fallback ¥ mark drawn with primitives when font is unavailable."""
    arm = size * 0.34
    bar_w = max(8, size // 18)
    top = cy - size * 0.22
    mid = cy + size * 0.02
    bottom = cy + size * 0.24

    draw.line((cx - arm, top, cx + arm, top + arm * 1.15), fill=INK, width=bar_w)
    draw.line((cx + arm, top, cx - arm, top + arm * 1.15), fill=INK, width=bar_w)
    draw.line((cx - arm * 0.72, mid, cx + arm * 0.72, mid), fill=INK, width=bar_w)
    draw.line((cx - arm * 0.72, bottom, cx + arm * 0.72, bottom), fill=INK, width=bar_w)


def draw_flat_icon() -> Image.Image:
    canvas = Image.new("RGB", (SIZE, SIZE), BG)
    draw = ImageDraw.Draw(canvas)

    card_size = 560
    card_radius = 140
    left = (SIZE - card_size) // 2
    top = (SIZE - card_size) // 2
    card_box = (left, top, left + card_size, top + card_size)
    draw.rounded_rectangle(card_box, radius=card_radius, fill=WHITE)

    yen_cx = left + card_size // 2
    yen_cy = top + card_size // 2
    font = load_font(320)
    if isinstance(font, ImageFont.ImageFont) and not hasattr(font, "getbbox"):
        draw_yen_mark(draw, yen_cx, yen_cy, 280)
    else:
        draw.text((yen_cx, yen_cy), "¥", fill=INK, font=font, anchor="mm")

    return canvas


def main() -> None:
    icon = draw_flat_icon()
    OUT.parent.mkdir(parents=True, exist_ok=True)
    icon.save(OUT, "PNG")
    print(f"Saved {OUT} (flat minimalist {SIZE}×{SIZE})")


if __name__ == "__main__":
    main()
