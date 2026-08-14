#!/usr/bin/env bash
# 确保 src-tauri/icons 与 assets/app-icon-source.png 一致（dev / build 共用）
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SRC="$ROOT/assets/app-icon-source.png"
ICNS="$ROOT/src-tauri/icons/icon.icns"

if [[ ! -f "$SRC" ]]; then
  echo "缺少图标源文件: $SRC" >&2
  exit 1
fi

mkdir -p "$ROOT/src-tauri/icons"

if [[ ! -f "$ICNS" ]] || [[ "$SRC" -nt "$ICNS" ]]; then
  echo ">> 从 assets/app-icon-source.png 生成 Tauri 图标 …"
  cd "$ROOT"
  npm run icon:gen --silent
fi
