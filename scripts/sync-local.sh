#!/usr/bin/env bash
# 在本机 Mac 运行此脚本，从 GitHub 同步最新代码
# 用法: ./scripts/sync-local.sh

set -euo pipefail

BRANCH="cursor/personal-assets-23c7"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"

cd "$ROOT"

echo "→ 拉取分支 $BRANCH ..."
git fetch origin "$BRANCH"
git checkout "$BRANCH" 2>/dev/null || git checkout -b "$BRANCH" "origin/$BRANCH"
git pull origin "$BRANCH"

echo "✓ 已同步到最新: $(git log -1 --oneline)"
echo ""
echo "启动应用: npm run tauri dev"
