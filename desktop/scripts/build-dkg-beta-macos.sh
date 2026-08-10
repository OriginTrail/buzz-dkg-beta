#!/usr/bin/env bash
set -euo pipefail

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "Buzz DKG Beta DMGs must be built on macOS." >&2
  exit 1
fi

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
REPO_ROOT=$(cd "${SCRIPT_DIR}/../.." && pwd)
HOST_TARGET=$(rustc -vV | sed -n 's|host: ||p')
TARGET=${1:-$HOST_TARGET}

if [[ -z "$HOST_TARGET" ]]; then
  echo "Unable to determine the Rust host target." >&2
  exit 1
fi

case "$TARGET" in
  aarch64-apple-darwin) ARCH="aarch64" ;;
  x86_64-apple-darwin) ARCH="x86_64" ;;
  *)
    echo "Unsupported macOS target: $TARGET" >&2
    exit 1
    ;;
esac

cd "$REPO_ROOT"

echo "==> Building Buzz sidecars for $TARGET"
cargo build --release --target "$TARGET" \
  -p buzz-acp \
  -p buzz-agent \
  -p buzz-backend-kubernetes \
  -p buzz-dev-mcp \
  -p git-credential-nostr \
  -p buzz-cli
./scripts/bundle-sidecars.sh "$TARGET"

echo "==> Building isolated, keychain-free Buzz DKG Beta DMG"
cd desktop
BUILD_MARKER=$(mktemp)
trap 'rm -f "$BUILD_MARKER"' EXIT
set +e
CMAKE_POLICY_VERSION_MINIMUM="${CMAKE_POLICY_VERSION_MINIMUM:-3.5}" \
MACOSX_DEPLOYMENT_TARGET="${MACOSX_DEPLOYMENT_TARGET:-10.15}" \
CMAKE_OSX_DEPLOYMENT_TARGET="${CMAKE_OSX_DEPLOYMENT_TARGET:-10.15}" \
TAURI_BUNDLER_DMG_IGNORE_CI="${TAURI_BUNDLER_DMG_IGNORE_CI:-true}" \
./node_modules/.bin/tauri build \
  --verbose \
  --no-sign \
  --target "$TARGET" \
  --bundles dmg \
  --config src-tauri/tauri.dkg-beta.conf.json \
  -- \
  --no-default-features
BUILD_STATUS=$?
set -e

BUNDLE_ROOT="src-tauri/target/${TARGET}/release/bundle"
APP="$BUNDLE_ROOT/macos/Buzz DKG Beta.app"
if [[ ! -d "$APP" || ! "$APP" -nt "$BUILD_MARKER" ]]; then
  echo "Buzz DKG Beta did not produce a fresh app bundle." >&2
  exit "$BUILD_STATUS"
fi

VERSION=$(node -e \
  'const fs = require("fs"); process.stdout.write(JSON.parse(fs.readFileSync("src-tauri/tauri.dkg-beta.conf.json", "utf8")).version)')
ZIP_DIR="$BUNDLE_ROOT/zip"
ZIP="$ZIP_DIR/Buzz DKG Beta_${VERSION}_${ARCH}.zip"
mkdir -p "$ZIP_DIR"
ditto -c -k --sequesterRsrc --keepParent "$APP" "$ZIP"
echo "==> App ZIP: $REPO_ROOT/desktop/$ZIP"

if [[ "$BUILD_STATUS" -ne 0 ]]; then
  echo "DMG packaging was unavailable, but the fresh app ZIP is ready." >&2
  exit 0
fi

DMG=$(find "$BUNDLE_ROOT/dmg" -name '*.dmg' -type f | head -1)
if [[ -z "$DMG" ]]; then
  echo "Buzz DKG Beta build completed without producing a DMG." >&2
  exit 1
fi

echo "==> DMG: $REPO_ROOT/desktop/$DMG"
