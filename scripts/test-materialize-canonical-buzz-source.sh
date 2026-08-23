#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
TMP_ROOT=$(mktemp -d)
trap 'rm -rf "$TMP_ROOT"' EXIT

SOURCE_REPO="$TMP_ROOT/source"
TOOLING_REPO="$TMP_ROOT/tooling"

git init -q -b main "$SOURCE_REPO"
git -C "$SOURCE_REPO" config user.name "Beta materialization test"
git -C "$SOURCE_REPO" config user.email "beta-materialization@example.invalid"
mkdir -p "$SOURCE_REPO/desktop/src/features/settings/hooks"
cp "$REPO_ROOT/desktop/package.json" "$SOURCE_REPO/desktop/package.json"
node - "$SOURCE_REPO/desktop/package.json" <<'NODE'
const fs = require("node:fs");
const path = process.argv[2];
const pkg = JSON.parse(fs.readFileSync(path, "utf8"));
delete pkg.scripts["build:dkg-beta"];
delete pkg.scripts["check:dkg-beta"];
pkg.scripts.check = "biome check . && pnpm check:px-text && pnpm check:pubkey-truncation";
fs.writeFileSync(path, `${JSON.stringify(pkg, null, 2)}\n`);
NODE
printf '%s\n' 'canonical-source-marker' >"$SOURCE_REPO/canonical-source-marker"
printf '%s\n' \
  'const RELEASES_URL = import.meta.env.VITE_BUZZ_RELEASES_URL;' \
  >"$SOURCE_REPO/desktop/src/features/settings/hooks/use-updater.ts"
git -C "$SOURCE_REPO" add .
git -C "$SOURCE_REPO" commit -q -m source
SOURCE_SHA=$(git -C "$SOURCE_REPO" rev-parse HEAD)

git init -q -b tooling "$TOOLING_REPO"
git -C "$TOOLING_REPO" config user.name "Beta materialization test"
git -C "$TOOLING_REPO" config user.email "beta-materialization@example.invalid"
OVERLAY_PATHS=$(awk '!/^($|#)/ { print }' "$REPO_ROOT/scripts/dkg-beta-overlay-files.txt")
# shellcheck disable=SC2086 # Paths are controlled by the newline-only manifest.
git -C "$REPO_ROOT" archive HEAD -- $OVERLAY_PATHS | tar -x -C "$TOOLING_REPO"
mkdir -p "$TOOLING_REPO/desktop"
printf '%s\n' 'stale beta application code' >"$TOOLING_REPO/desktop/stale-beta-code.ts"
git -C "$TOOLING_REPO" add .
git -C "$TOOLING_REPO" commit -q -m tooling
SOURCE_REPOSITORY="$SOURCE_REPO" SOURCE_REF=main EXPECTED_SOURCE_SHA="$SOURCE_SHA" \
  "$TOOLING_REPO/scripts/materialize-canonical-buzz-source.sh"

test -f "$TOOLING_REPO/canonical-source-marker"
test -f "$TOOLING_REPO/scripts/dkg-beta-overlay-files.txt"
test -f "$TOOLING_REPO/desktop/.env.dkg-beta"
test ! -e "$TOOLING_REPO/desktop/stale-beta-code.ts"
grep -Fq 'VITE_BUZZ_RELEASES_URL' \
  "$TOOLING_REPO/desktop/src/features/settings/hooks/use-updater.ts"
node -e '
  const pkg = require(process.argv[1]);
  if (!pkg.scripts["build:dkg-beta"] || !pkg.scripts["check:dkg-beta"]) process.exit(1);
' "$TOOLING_REPO/desktop/package.json"

if SOURCE_REPOSITORY="$SOURCE_REPO" SOURCE_REF=main \
  EXPECTED_SOURCE_SHA=0000000000000000000000000000000000000000 \
  "$TOOLING_REPO/scripts/materialize-canonical-buzz-source.sh"; then
  echo "materializer accepted a mismatched source SHA" >&2
  exit 1
fi
test -e "$TOOLING_REPO/canonical-source-marker"

echo "Canonical Buzz source materialization contract passed."
