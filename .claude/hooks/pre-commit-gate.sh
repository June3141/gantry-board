#!/usr/bin/env bash
# L3: Commit gate — validate message format, commit size, and run full checks
set -euo pipefail

# Load nix if available (needed for devbox)
if [[ -f /nix/var/nix/profiles/default/etc/profile.d/nix-daemon.sh ]]; then
  . /nix/var/nix/profiles/default/etc/profile.d/nix-daemon.sh
fi

cd "$(git rev-parse --show-toplevel)"

INPUT=$(cat)
COMMAND=$(echo "$INPUT" | jq -r '.tool_input.command // empty')

# Only intercept git commit commands
if ! echo "$COMMAND" | grep -qE '\bgit\s+commit\b'; then
  exit 0
fi

# --- 1. Commit message validation ---
# Extract -m "message" from the command
COMMIT_MSG=$(echo "$COMMAND" | grep -oP '(?<=-m\s")[^"]*' || echo "$COMMAND" | grep -oP "(?<=-m\s')[^']*" || true)

# Also handle heredoc format: -m "$(cat <<'EOF' ... EOF )"
if [[ -z "$COMMIT_MSG" ]]; then
  COMMIT_MSG=$(echo "$COMMAND" | sed -n "s/.*<<'EOF'[[:space:]]*//p" | sed '/^EOF/d' | head -1 || true)
fi

if [[ -n "$COMMIT_MSG" ]]; then
  # Extract first line (subject)
  SUBJECT=$(echo "$COMMIT_MSG" | head -1 | sed 's/^[[:space:]]*//')

  # Validate gitmoji + scope format
  GITMOJI_FILE=".claude/hooks/gitmoji-pattern.txt"
  if [[ -f "$GITMOJI_FILE" ]]; then
    EMOJIS=$(cat "$GITMOJI_FILE")
  else
    EMOJIS='✨|🐛|📝|✅|♻️|🔧|🎨|⚡️|🔥|💥|🚀|🚧|🔒|⬆️|🗃️|🎉'
  fi
  TYPES='feat|fix|docs|test|refactor|chore|style|perf'
  GITMOJI_PATTERN="^($TYPES): ($EMOJIS) .+$"
  if ! echo "$SUBJECT" | grep -qP "$GITMOJI_PATTERN"; then
    echo '{"decision":"block","reason":"Commit message must match type: emoji subject format: e.g. feat: ✨ add health check endpoint"}'
    exit 2
  fi

  # Reject Japanese characters in subject
  if echo "$SUBJECT" | grep -qP '[\p{Hiragana}\p{Katakana}\p{Han}]'; then
    echo '{"decision":"block","reason":"Commit message subject must be in English (no Japanese characters)"}'
    exit 2
  fi
fi

# --- 2. Commit size validation ---
STAGED_FILES=$(git diff --cached --name-only 2>/dev/null | grep -v -E '(api/generated/|\.test\.|_test\.rs|tests/)' | wc -l | tr -d ' ')
STAGED_LINES=$(git diff --cached --stat 2>/dev/null | tail -1 | grep -oP '\d+(?= insertion)' || echo "0")
STAGED_DELETIONS=$(git diff --cached --stat 2>/dev/null | tail -1 | grep -oP '\d+(?= deletion)' || echo "0")
TOTAL_LINES=$((STAGED_LINES + STAGED_DELETIONS))

SIZE_WARN=""
if [[ "$STAGED_FILES" -gt 10 ]]; then
  SIZE_WARN+="Staged files ($STAGED_FILES) exceed limit of 10. "
fi
if [[ "$TOTAL_LINES" -gt 300 ]]; then
  SIZE_WARN+="Changed lines ($TOTAL_LINES) exceed limit of 300. "
fi

if [[ -n "$SIZE_WARN" ]]; then
  echo '{"decision":"block","reason":"Commit too large: '"$SIZE_WARN"'Split into smaller commits."}'
  exit 2
fi

# --- 3. Full quality check ---
if command -v devbox > /dev/null 2>&1 && [[ -f devbox.json ]]; then
  CHECK_CMD="devbox run -- task check"
else
  CHECK_CMD="task check"
fi
if ! $CHECK_CMD 2>&1; then
  echo '{"decision":"block","reason":"Quality checks failed. Run `task check` to see details."}'
  exit 2
fi

exit 0
