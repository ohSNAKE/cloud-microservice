#!/usr/bin/env bash
# 在本机 Mac 运行此脚本，从 GitHub 同步最新代码（会丢弃未提交的本地改动）
# 用法: ./scripts/sync-local.sh

set -euo pipefail

BRANCH="cursor/personal-assets-23c7"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"

cd "$ROOT"

echo "→ 同步分支 $BRANCH ..."

git fetch origin "$BRANCH"
git checkout "$BRANCH" 2>/dev/null || git checkout -b "$BRANCH" "origin/$BRANCH"

if ! git diff --quiet || ! git diff --cached --quiet || [ -n "$(git status --porcelain)" ]; then
  echo ">> 检测到未提交改动，重置为 origin/$BRANCH（本地改动将丢弃）"
  git reset --hard "origin/$BRANCH"
else
  git merge --ff-only "origin/$BRANCH"
fi

# 清理 icon 生成物并重新生成（icons 目录已 gitignore）
ICON_DIR="src-tauri/icons"
rm -rf "$ICON_DIR"/*
mkdir -p "$ICON_DIR"
touch "$ICON_DIR/.gitkeep"

echo ">> 生成本地 Tauri 图标 …"
npm run icon:gen

echo "✓ 已同步: $(git log -1 --oneline)"
echo ""
echo "自定义 Logo: npm run icon:brand"
echo "启动: npm run tauri dev"
