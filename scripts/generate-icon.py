#!/usr/bin/env python3
"""Generate the 穷鬼 / 财记 app icon — full-bleed opaque square for macOS Dock.

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

BLUE_TOP = (59, 130, 246)
BLUE_BOTTOM = (29, 78, 216)
GHOST = (248, 250, 252)
GHOST_SHADOW = (219, 228, 240)
EYE = (30, 41, 59)
COIN = (251, 191, 36)
COIN_RING = (245, 158, 11)
COIN_TEXT = (120, 53, 15)


def lerp(a: int, b: int, t: float) -> int:
    return int(a + (b - a) * t)


def draw_vertical_gradient(size: int) -> Image.Image:
    img = Image.new("RGB", (size, size))
    px = img.load()
    for y in range(size):
        t = y / (size - 1)
        color = (
            lerp(BLUE_TOP[0], BLUE_BOTTOM[0], t),
            lerp(BLUE_TOP[1], BLUE_BOTTOM[1], t),
            lerp(BLUE_TOP[2], BLUE_BOTTOM[2], t),
        )
        for x in range(size):
            px[x, y] = color
    return img.convert("RGBA")


def draw_ghost(draw: ImageDraw.ImageDraw, cx: int, cy: int) -> None:
    head_r = 168
    head_box = (cx - head_r, cy - head_r - 36, cx + head_r, cy + head_r - 36)
    draw.ellipse(head_box, fill=GHOST_SHADOW)
    draw.ellipse(
        (head_box[0] + 6, head_box[1] + 6, head_box[2] - 6, head_box[3] - 6),
        fill=GHOST,
    )

    wave_y = cy + head_r - 36
    wave_w = head_r * 2
    left = cx - head_r
    bump_w = wave_w // 3
    for i in range(3):
        bump_left = left + i * bump_w
        bump_cx = bump_left + bump_w // 2
        draw.ellipse(
            (bump_cx - bump_w // 2 + 8, wave_y - 18, bump_cx + bump_w // 2 - 8, wave_y + 72),
            fill=GHOST,
        )
    draw.rectangle((left + 4, wave_y, left + wave_w - 4, wave_y + 36), fill=GHOST)

    eye_y = cy - 28
    eye_r = 20
    for ex in (cx - 52, cx + 52):
        draw.ellipse((ex - eye_r, eye_y - eye_r, ex + eye_r, eye_y + eye_r), fill=EYE)

    smile_y = cy + 18
    draw.arc((cx - 42, smile_y - 18, cx + 42, smile_y + 26), start=20, end=160, fill=EYE, width=7)


def draw_coin(draw: ImageDraw.ImageDraw, cx: int, cy: int) -> None:
    r = 78
    draw.ellipse((cx - r - 4, cy - r - 4, cx + r + 4, cy + r + 4), fill=COIN_RING)
    draw.ellipse((cx - r, cy - r, cx + r, cy + r), fill=COIN)

    try:
        font = ImageFont.truetype("/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf", 92)
    except OSError:
        font = ImageFont.load_default()
    draw.text((cx, cy), "¥", fill=COIN_TEXT, font=font, anchor="mm")


def main() -> None:
    plate = draw_vertical_gradient(SIZE)
    draw = ImageDraw.Draw(plate)
    draw_ghost(draw, SIZE // 2 - 52, SIZE // 2 + 8)
    draw_coin(draw, SIZE // 2 + 148, SIZE // 2 + 132)

    OUT.parent.mkdir(parents=True, exist_ok=True)
    plate.save(OUT, "PNG")
    print(f"Saved {OUT} (full-bleed opaque {SIZE}×{SIZE})")


if __name__ == "__main__":
    main()
