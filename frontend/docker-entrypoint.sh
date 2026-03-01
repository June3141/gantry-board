#!/bin/sh
set -e

# Install dependencies if node_modules is missing or package manifest/lockfile has changed
if [ ! -d node_modules ] || [ package.json -nt node_modules/.package-lock.json ] || [ package-lock.json -nt node_modules/.package-lock.json ]; then
  echo "Installing dependencies..."
  npm ci
fi

exec "$@"
