#!/usr/bin/env bash
# L1: Auto-format after Write/Edit
# Receives JSON on stdin with tool_input.file_path
set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

INPUT=$(cat)
FILE_PATH=$(echo "$INPUT" | jq -r '.tool_input.file_path // empty')

if [[ -z "$FILE_PATH" ]]; then
  exit 0
fi

# Skip non-project files
case "$FILE_PATH" in
  */.claude/* | */target/* | */node_modules/* | */dist/*)
    exit 0
    ;;
esac

case "$FILE_PATH" in
  *.rs)
    cargo fmt --quiet 2>/dev/null || true
    ;;
  *.ts | *.tsx | *.css)
    if [[ -f "$FILE_PATH" ]]; then
      cd frontend
      npx prettier --write "$FILE_PATH" --log-level silent 2>/dev/null || true
    fi
    ;;
esac

exit 0
