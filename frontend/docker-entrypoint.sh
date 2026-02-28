#!/bin/sh
set -e

# Install dependencies if node_modules is missing or package.json has changed
if [ ! -d node_modules ] || [ package.json -nt node_modules/.package-lock.json ]; then
  echo "Installing dependencies..."
  npm ci
fi

exec "$@"
