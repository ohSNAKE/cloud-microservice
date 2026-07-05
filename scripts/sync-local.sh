#!/usr/bin/env bash
# 在本机 Mac 运行此脚本，从 GitHub 同步最新代码
# 用法: ./scripts/sync-local.sh

set -euo pipefail

BRANCH="cursor/personal-assets-23c7"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"

cd "$ROOT"

echo "→ 拉取分支 $BRANCH ..."

# icon 生成物不再入库；丢弃本地改动以免 pull 被阻挡
ICON_DIR="src-tauri/icons"
if git status --porcelain -- "$ICON_DIR" 2>/dev/null | grep -q .; then
  echo ">> 丢弃 src-tauri/icons 本地改动 …"
  git restore --source=HEAD --staged --worktree -- "$ICON_DIR" 2>/dev/null || \
    git checkout HEAD -- "$ICON_DIR" 2>/dev/null || true
fi
rm -rf "$ICON_DIR"/*
mkdir -p "$ICON_DIR"
touch "$ICON_DIR/.gitkeep"

git fetch origin "$BRANCH"
git checkout "$BRANCH" 2>/dev/null || git checkout -b "$BRANCH" "origin/$BRANCH"
git pull --rebase origin "$BRANCH"

echo ">> 生成本地 Tauri 图标 …"
npm run icon:gen

echo "✓ 已同步到最新: $(git log -1 --oneline)"
echo ""
echo "自定义 Logo:"
echo "  npm run icon:brand"
echo "  ./scripts/apply-download-logo.sh"
echo ""
echo "启动: npm run tauri dev"
