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
      REL_PATH="$FILE_PATH"
      if [[ "$REL_PATH" == frontend/* ]]; then
        REL_PATH="${REL_PATH#frontend/}"
      fi
      cd frontend
      npx biome format --write "$REL_PATH" --log-level=off 2>/dev/null || true
    fi
    ;;
esac

exit 0
