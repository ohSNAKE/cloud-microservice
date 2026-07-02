#!/usr/bin/env bash
# 用本地原图生成应用 Logo（需正方形源图，脚本会自动居中裁剪）
# 用法: ./scripts/set-app-icon.sh /path/to/your-image.png
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SRC="${1:-}"

if [[ -z "$SRC" || ! -f "$SRC" ]]; then
  echo "用法: $0 <图片路径>" >&2
  echo "示例: $0 ~/Downloads/logo.png" >&2
  exit 1
fi

if ! command -v ffmpeg >/dev/null 2>&1; then
  echo "需要 ffmpeg，请先安装（macOS: brew install ffmpeg）" >&2
  exit 1
fi

SQUARE="$ROOT/assets/.app-icon-square.png"
OUT="$ROOT/assets/app-icon-source.png"

echo ">> 居中裁剪为 1024×1024 …"
ffmpeg -y -loglevel error -i "$SRC" \
  -vf "crop='min(iw,ih)':'min(iw,ih)','scale=1024:1024'" \
  "$SQUARE"

cp "$SQUARE" "$OUT"
cp "$SQUARE" "$ROOT/public/app-logo.png"
rm -f "$SQUARE"

echo ">> 生成 Tauri 全套图标 …"
cd "$ROOT"
npm run icon:gen

echo ">> 完成。请提交: assets/app-icon-source.png public/app-logo.png src-tauri/icons/"
