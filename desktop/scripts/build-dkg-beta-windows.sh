#!/usr/bin/env bash
set -euo pipefail

case "$(uname -s)" in
  MINGW*|MSYS*|CYGWIN*) ;;
  *)
    echo "Buzz DKG Beta Windows packages must be built from Git Bash on Windows." >&2
    exit 1
    ;;
esac

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
REPO_ROOT=$(cd "${SCRIPT_DIR}/../.." && pwd)
TARGET=${1:-x86_64-pc-windows-msvc}

if [[ "$TARGET" != "x86_64-pc-windows-msvc" ]]; then
  echo "Unsupported Windows beta target: $TARGET" >&2
  exit 1
fi

cd "$REPO_ROOT"

echo "==> Building Buzz sidecars for $TARGET"
cargo build --release --target "$TARGET" \
  -p buzz-acp \
  -p buzz-agent \
  -p buzz-dev-mcp \
  -p git-credential-nostr \
  -p buzz-cli
./scripts/bundle-sidecars.sh "$TARGET"

echo "==> Building Buzz DKG Beta Windows installer"
cd desktop
CMAKE_POLICY_VERSION_MINIMUM="${CMAKE_POLICY_VERSION_MINIMUM:-3.5}" \
./node_modules/.bin/tauri build \
  --verbose \
  --target "$TARGET" \
  --bundles nsis \
  --config src-tauri/tauri.dkg-beta.conf.json

BUNDLE_ROOT="src-tauri/target/${TARGET}/release/bundle"
echo "==> NSIS installer: $REPO_ROOT/desktop/$BUNDLE_ROOT/nsis"
