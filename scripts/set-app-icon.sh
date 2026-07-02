#!/usr/bin/env bash
# 用本地原图生成应用 Logo（居中裁剪 + macOS 标准内边距）
# 用法: ./scripts/set-app-icon.sh /path/to/your-image.png
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SRC="${1:-}"

# macOS Dock 图标：内容约占画布 84%，留出系统圆角与安全边距
ICON_CANVAS=1024
ICON_CONTENT=864

if [[ -z "$SRC" || ! -f "$SRC" ]]; then
  echo "用法: $0 <图片路径>" >&2
  echo "示例: $0 ~/Downloads/logo.png" >&2
  exit 1
fi

SQUARE="$ROOT/assets/.app-icon-square.png"
OUT="$ROOT/assets/app-icon-source.png"

make_icon_ffmpeg() {
  if ! command -v ffmpeg >/dev/null 2>&1; then
    return 1
  fi
  ffmpeg -y -loglevel error -i "$SRC" \
    -vf "crop=min(iw\,ih):min(iw\,ih):(iw-min(iw\,ih))/2:(ih-min(iw\,ih))/2,scale=${ICON_CONTENT}:${ICON_CONTENT},pad=${ICON_CANVAS}:${ICON_CANVAS}:(ow-iw)/2:(oh-ih)/2:color=0x000000" \
    "$SQUARE"
}

make_icon_macos() {
  local w h side tmp
  w=$(sips -g pixelWidth "$SRC" 2>/dev/null | awk '/pixelWidth:/{print $2}')
  h=$(sips -g pixelHeight "$SRC" 2>/dev/null | awk '/pixelHeight:/{print $2}')
  if [[ -z "$w" || -z "$h" ]]; then
    return 1
  fi
  if (( w < h )); then side=$w; else side=$h; fi
  tmp="$ROOT/assets/.app-icon-crop.png"
  cp "$SRC" "$tmp"
  sips -c "$side" "$side" "$tmp" >/dev/null
  sips -z "$ICON_CONTENT" "$ICON_CONTENT" "$tmp" >/dev/null
  # 内边距需要 ffmpeg；仅 sips 时退化为满画布（偏大）
  if make_icon_ffmpeg_from "$tmp"; then
    rm -f "$tmp"
    return 0
  fi
  sips -z "$ICON_CANVAS" "$ICON_CANVAS" "$tmp" >/dev/null
  mv "$tmp" "$SQUARE"
  echo "提示: 安装 ffmpeg 可获得与 Dock 一致的内边距（brew install ffmpeg）" >&2
}

make_icon_ffmpeg_from() {
  local input="$1"
  if ! command -v ffmpeg >/dev/null 2>&1; then
    return 1
  fi
  ffmpeg -y -loglevel error -i "$input" \
    -vf "scale=${ICON_CONTENT}:${ICON_CONTENT},pad=${ICON_CANVAS}:${ICON_CANVAS}:(ow-iw)/2:(oh-ih)/2:color=0x000000" \
    "$SQUARE"
}

echo ">> 裁剪并添加 macOS 图标内边距（${ICON_CONTENT}/${ICON_CANVAS}）…"
if [[ "$(uname -s)" == "Darwin" ]] && command -v sips >/dev/null 2>&1; then
  make_icon_macos || make_icon_ffmpeg
elif make_icon_ffmpeg; then
  :
else
  echo "需要 macOS 自带 sips 或 ffmpeg（brew install ffmpeg）" >&2
  exit 1
fi

cp "$SQUARE" "$OUT"
cp "$SQUARE" "$ROOT/public/app-logo.png"
rm -f "$SQUARE" "$ROOT/assets/.app-icon-crop.png"

echo ">> 生成 Tauri 全套图标 …"
cd "$ROOT"
npm run icon:gen

echo ">> 完成。请提交并重新构建应用后，Dock 图标才会更新。"
echo "   若 Dock 仍显示旧图标，可执行: killall Dock"
