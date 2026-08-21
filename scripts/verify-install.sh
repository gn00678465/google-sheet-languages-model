#!/bin/sh
# Shared body of the verify-install CI jobs (native runners and alpine container).
# Usage: verify-install.sh <version>
# Requires: NODE_AUTH_TOKEN for GitHub Packages.
set -eu
VERSION="$1"
PKG_NAME="@gn00678465/google-sheet-languages-model"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

WORK="$(mktemp -d)"
cd "$WORK"
printf '@gn00678465:registry=https://npm.pkg.github.com\n//npm.pkg.github.com/:_authToken=${NODE_AUTH_TOKEN}\n' > .npmrc
npm init -y >/dev/null
npm install "${PKG_NAME}@${VERSION}"
node "$SCRIPT_DIR/verify-install.cjs" "$VERSION"

cp .npmrc "$HOME/.npmrc"
npm install -g "${PKG_NAME}@${VERSION}"
out="$(gslm --version)"
[ "$out" = "$VERSION" ] || { echo "✗ global gslm --version printed '$out', expected '$VERSION'"; exit 1; }
echo "✓ global gslm --version = $out"
