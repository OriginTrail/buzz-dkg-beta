#!/usr/bin/env bash
set -euo pipefail

if [[ "$(uname -s)" != "Linux" ]]; then
  echo "Buzz DKG Beta Linux packages must be built on Linux." >&2
  exit 1
fi

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
REPO_ROOT=$(cd "${SCRIPT_DIR}/../.." && pwd)
HOST_TARGET=$(rustc -vV | sed -n 's|host: ||p')
TARGET=${1:-$HOST_TARGET}
TAURI_CONFIG=${BUZZ_DKG_BETA_TAURI_CONFIG:-src-tauri/tauri.dkg-beta.conf.json}

case "$TARGET" in
  x86_64-unknown-linux-gnu) ;;
  *)
    echo "Unsupported Linux beta target: $TARGET" >&2
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

echo "==> Building Buzz DKG Beta AppImage and Debian package"
cd desktop
CMAKE_POLICY_VERSION_MINIMUM="${CMAKE_POLICY_VERSION_MINIMUM:-3.5}" \
./node_modules/.bin/tauri build \
  --verbose \
  --ci \
  --target "$TARGET" \
  --bundles deb,appimage \
  --config "$TAURI_CONFIG"

BUNDLE_ROOT="src-tauri/target/${TARGET}/release/bundle"
echo "==> Debian package: $REPO_ROOT/desktop/$BUNDLE_ROOT/deb"
echo "==> AppImage: $REPO_ROOT/desktop/$BUNDLE_ROOT/appimage"
