#!/usr/bin/env python3
"""Generate a minimal ghost icon for 穷鬼 app."""

from PIL import Image, ImageDraw

SIZE = 1024
OUT = "/workspace/src-tauri/icons/icon-source.png"

# macOS-style dark rounded square palette
BG = (28, 28, 32)
GHOST = (245, 245, 250)
EYE = (28, 28, 32)


def rounded_rect(draw, xy, radius, fill):
    x0, y0, x1, y1 = xy
    draw.rounded_rectangle(xy, radius=radius, fill=fill)


def main():
    img = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)

    pad = 64
    rounded_rect(draw, (pad, pad, SIZE - pad, SIZE - pad), radius=200, fill=BG)

    cx, cy = SIZE // 2, SIZE // 2 + 20

    # Ghost body: circle + wavy skirt
    head_r = 190
    draw.ellipse(
        (cx - head_r, cy - head_r - 40, cx + head_r, cy + head_r - 40),
        fill=GHOST,
    )

    # Wavy bottom
    wave_y = cy + head_r - 40
    wave_w = head_r * 2
    left = cx - head_r
    for i in range(3):
        bump_left = left + i * (wave_w // 3)
        bump_right = bump_left + wave_w // 3
        bump_cx = (bump_left + bump_right) // 2
        draw.ellipse(
            (bump_cx - wave_w // 6, wave_y - 30, bump_cx + wave_w // 6, wave_y + 90),
            fill=GHOST,
        )

    # Cover top of bumps to flatten the connection
    draw.rectangle((left, wave_y, left + wave_w, wave_y + 45), fill=GHOST)

    # Eyes
    eye_y = cy - 30
    eye_r = 22
    eye_offset = 55
    for ex in (cx - eye_offset, cx + eye_offset):
        draw.ellipse((ex - eye_r, eye_y - eye_r, ex + eye_r, eye_y + eye_r), fill=EYE)

    img.save(OUT)
    print(f"Saved {OUT}")


if __name__ == "__main__":
    main()
