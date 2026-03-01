#!/bin/sh
set -e

PLATFORM="$(uname -s)-$(uname -m)"
MARKER="node_modules/.platform"

# Install dependencies if:
#   - node_modules is missing
#   - package manifest/lockfile has changed
#   - platform marker mismatches (e.g. host macOS modules mounted into Linux container)
if [ ! -d node_modules ] \
   || [ package.json -nt node_modules/.package-lock.json ] \
   || [ package-lock.json -nt node_modules/.package-lock.json ] \
   || [ ! -f "$MARKER" ] \
   || [ "$(cat "$MARKER")" != "$PLATFORM" ]; then
  echo "Installing dependencies (platform: $PLATFORM)..."
  npm ci
  touch -r package-lock.json node_modules/.package-lock.json
  echo "$PLATFORM" > "$MARKER"
fi

exec "$@"
