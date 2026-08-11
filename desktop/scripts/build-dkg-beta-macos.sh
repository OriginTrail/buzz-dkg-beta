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
TAURI_CONFIG=${BUZZ_DKG_BETA_TAURI_CONFIG:-src-tauri/tauri.dkg-beta.conf.json}

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
  --bundles app,dmg \
  --config "$TAURI_CONFIG" \
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
  'const fs = require("fs"); process.stdout.write(JSON.parse(fs.readFileSync(process.argv[1], "utf8")).version)' \
  "$TAURI_CONFIG")
ZIP_DIR="$BUNDLE_ROOT/zip"
ZIP="$ZIP_DIR/Buzz DKG Beta_${VERSION}_${ARCH}.zip"
mkdir -p "$ZIP_DIR"
ditto -c -k --sequesterRsrc --keepParent "$APP" "$ZIP"
echo "==> App ZIP: $REPO_ROOT/desktop/$ZIP"

UPDATER_ARCHIVE=$(find "$BUNDLE_ROOT/macos" -name '*.app.tar.gz' -type f | head -1 || true)
if [[ -n "$UPDATER_ARCHIVE" ]]; then
  UPDATER_SIG="${UPDATER_ARCHIVE}.sig"
  if [[ ! -f "$UPDATER_SIG" ]]; then
    if [[ -z "${TAURI_SIGNING_PRIVATE_KEY:-}" && -z "${TAURI_SIGNING_PRIVATE_KEY_PATH:-}" ]]; then
      echo "Updater archive was produced without a signature and no updater signing key is configured: $UPDATER_ARCHIVE" >&2
      exit 1
    fi

    # --no-sign keeps the beta app free of Apple publisher signing, but it also
    # suppresses Tauri updater signing. Sign only the updater archive here so
    # installed beta builds can still verify updates cryptographically.
    echo "==> Signing updater archive"
    ./node_modules/.bin/tauri signer sign "$UPDATER_ARCHIVE"
  fi
  if [[ ! -f "$UPDATER_SIG" ]]; then
    echo "Updater signature was not produced: $UPDATER_SIG" >&2
    exit 1
  fi
  RENAMED_ARCHIVE="$BUNDLE_ROOT/macos/Buzz-DKG-Beta_${VERSION}_${ARCH}.app.tar.gz"
  mv "$UPDATER_ARCHIVE" "$RENAMED_ARCHIVE"
  mv "$UPDATER_SIG" "${RENAMED_ARCHIVE}.sig"
  echo "==> Updater archive: $REPO_ROOT/desktop/$RENAMED_ARCHIVE"
fi

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
