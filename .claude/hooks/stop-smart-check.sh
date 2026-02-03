#!/usr/bin/env bash
# L2: Smart lint on Stop — check only modified layers
set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

CHANGED=$(git diff --name-only HEAD 2>/dev/null; git diff --name-only --cached 2>/dev/null; git ls-files --others --exclude-standard 2>/dev/null)

if [[ -z "$CHANGED" ]]; then
  exit 0
fi

HAS_RS=false
HAS_FE=false

while IFS= read -r file; do
  case "$file" in
    *.rs) HAS_RS=true ;;
    *.ts | *.tsx | *.css) HAS_FE=true ;;
  esac
done <<< "$CHANGED"

ERRORS=""

if $HAS_RS; then
  if ! task backend:fmt:check 2>&1; then
    ERRORS+="Rust format check failed. "
  fi
  if ! task backend:lint 2>&1; then
    ERRORS+="Rust lint (clippy) failed. "
  fi
fi

if $HAS_FE; then
  if ! task frontend:fmt:check 2>&1; then
    ERRORS+="Frontend format check failed. "
  fi
  if ! task frontend:lint 2>&1; then
    ERRORS+="Frontend lint (eslint) failed. "
  fi
fi

if [[ -n "$ERRORS" ]]; then
  echo '{"systemMessage":"[Quality Check] '"$ERRORS"'Run `task fmt` to auto-fix formatting."}'
fi

exit 0
