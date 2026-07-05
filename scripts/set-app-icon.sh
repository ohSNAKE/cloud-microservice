#!/usr/bin/env bash
# 用本地原图生成应用 Logo（居中裁剪 + macOS 标准内边距 + 透明边距）
# 用法: ./scripts/set-app-icon.sh /path/to/your-image.png
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SRC="${1:-}"
PREPARE="$ROOT/scripts/prepare-app-icon.py"
OUT="$ROOT/assets/app-icon-source.png"

if [[ -z "$SRC" || ! -f "$SRC" ]]; then
  echo "用法: $0 <图片路径>" >&2
  echo "示例: $0 ~/Downloads/logo.png" >&2
  exit 1
fi

if ! python3 -c "import PIL" 2>/dev/null; then
  echo "需要 Pillow: pip install pillow  或  pip3 install pillow" >&2
  exit 1
fi

echo ">> 裁剪并添加 macOS Dock 内边距（透明边距，内容居中）…"
python3 "$PREPARE" "$SRC" "$OUT"

cp "$OUT" "$ROOT/public/app-logo.png"

echo ">> 生成 Tauri 全套图标 …"
cd "$ROOT"
npm run icon:gen

echo ">> 完成。请重新构建应用后 Dock 图标才会更新。"
echo "   若 Dock 仍显示旧图标: killall Dock"
