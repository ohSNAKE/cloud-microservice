#!/usr/bin/env bash
# 使用本机 Downloads/logo.png 作为应用 Logo（可在 Mac 上直接运行）
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
LOGO="${1:-$HOME/Downloads/logo.png}"
exec "$ROOT/scripts/set-app-icon.sh" "$LOGO"
