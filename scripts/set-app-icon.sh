#!/usr/bin/env bash
# 用本地原图生成应用 Logo（自动居中裁剪为正方形）
# 用法: ./scripts/set-app-icon.sh /path/to/your-image.png
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SRC="${1:-}"

if [[ -z "$SRC" || ! -f "$SRC" ]]; then
  echo "用法: $0 <图片路径>" >&2
  echo "示例: $0 ~/Downloads/logo.png" >&2
  exit 1
fi

SQUARE="$ROOT/assets/.app-icon-square.png"
OUT="$ROOT/assets/app-icon-source.png"

make_square_macos() {
  local w h side
  w=$(sips -g pixelWidth "$SRC" 2>/dev/null | awk '/pixelWidth:/{print $2}')
  h=$(sips -g pixelHeight "$SRC" 2>/dev/null | awk '/pixelHeight:/{print $2}')
  if [[ -z "$w" || -z "$h" ]]; then
    return 1
  fi
  if (( w < h )); then side=$w; else side=$h; fi
  cp "$SRC" "$SQUARE"
  # sips 从中心裁剪为正方形，再缩放到 1024
  sips -c "$side" "$side" "$SQUARE" >/dev/null
  sips -z 1024 1024 "$SQUARE" >/dev/null
}

make_square_ffmpeg() {
  if ! command -v ffmpeg >/dev/null 2>&1; then
    return 1
  fi
  ffmpeg -y -loglevel error -i "$SRC" \
    -vf "crop=min(iw\,ih):min(iw\,ih):(iw-min(iw\,ih))/2:(ih-min(iw\,ih))/2,scale=1024:1024" \
    "$SQUARE"
}

echo ">> 居中裁剪为 1024×1024 …"
if [[ "$(uname -s)" == "Darwin" ]] && command -v sips >/dev/null 2>&1; then
  make_square_macos || make_square_ffmpeg
elif make_square_ffmpeg; then
  :
else
  echo "需要 macOS 自带 sips 或 ffmpeg（brew install ffmpeg）" >&2
  exit 1
fi

cp "$SQUARE" "$OUT"
cp "$SQUARE" "$ROOT/public/app-logo.png"
rm -f "$SQUARE"

echo ">> 生成 Tauri 全套图标 …"
cd "$ROOT"
npm run icon:gen

echo ">> 完成。请提交: assets/app-icon-source.png public/app-logo.png src-tauri/icons/"
