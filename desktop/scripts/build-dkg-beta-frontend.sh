#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
DESKTOP_ROOT=$(cd "${SCRIPT_DIR}/.." && pwd)

cd "$DESKTOP_ROOT"
./node_modules/.bin/tsc
./node_modules/.bin/vite build --mode dkg-beta
