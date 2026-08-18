#!/usr/bin/env bash
set -euo pipefail

SOURCE_REPOSITORY="${SOURCE_REPOSITORY:-https://github.com/OriginTrail/buzz.git}"
SOURCE_REF="${SOURCE_REF:?SOURCE_REF is required}"
EXPECTED_SOURCE_SHA="${EXPECTED_SOURCE_SHA:?EXPECTED_SOURCE_SHA is required}"

[[ "$EXPECTED_SOURCE_SHA" =~ ^[0-9a-f]{40}$ ]] || {
  echo "::error::EXPECTED_SOURCE_SHA must be a full lowercase Git commit SHA"
  exit 1
}

TOOLING_SHA=$(git rev-parse HEAD)
git fetch --no-tags --depth=1 "$SOURCE_REPOSITORY" "$SOURCE_REF"
SOURCE_SHA=$(git rev-parse FETCH_HEAD)
[[ "$SOURCE_SHA" == "$EXPECTED_SOURCE_SHA" ]] || {
  echo "::error::Source ref moved: expected $EXPECTED_SOURCE_SHA, resolved $SOURCE_SHA"
  exit 1
}

# Replace the application tree with the reviewed OriginTrail/buzz commit, then
# restore only the distribution harness. This keeps the beta repository a
# release channel, not a second source tree.
git read-tree --reset -u "$SOURCE_SHA"
git checkout "$TOOLING_SHA" -- \
  .github/workflows/dkg-beta-desktop.yml \
  .github/workflows/promote-dkg-beta-desktop.yml \
  docs/dkg-beta-desktop.md \
  scripts/materialize-canonical-buzz-source.sh \
  scripts/promote-dkg-beta-desktop-release.sh \
  desktop/.env.dkg-beta \
  desktop/src-tauri/Info.dkg-beta.plist \
  desktop/src-tauri/tauri.dkg-beta.conf.json \
  desktop/scripts/build-dkg-beta-frontend.sh \
  desktop/scripts/build-dkg-beta-linux.sh \
  desktop/scripts/build-dkg-beta-macos.sh \
  desktop/scripts/build-dkg-beta-release-config.mjs \
  desktop/scripts/build-dkg-beta-windows.sh \
  desktop/scripts/check-dkg-beta-build.mjs \
  desktop/scripts/dkg-beta-assets.mjs \
  desktop/scripts/fix-appimage.sh \
  desktop/scripts/generate-oss-latest-json.sh \
  desktop/scripts/promote-dkg-beta-desktop-release.test.mjs \
  desktop/scripts/set-dkg-beta-version.mjs \
  desktop/scripts/updater-release-config.mjs \
  desktop/scripts/updater-release-config.test.mjs

# The flavor adds scripts only; dependency resolution remains locked to the
# canonical source commit.
node <<'NODE'
const fs = require("node:fs");
const path = "desktop/package.json";
const pkg = JSON.parse(fs.readFileSync(path, "utf8"));
pkg.scripts["build:dkg-beta"] = "tsc && vite build --mode dkg-beta";
pkg.scripts["check:dkg-beta"] =
  "node ./scripts/check-dkg-beta-build.mjs && node --test ./scripts/promote-dkg-beta-desktop-release.test.mjs";
fs.writeFileSync(path, `${JSON.stringify(pkg, null, 2)}\n`);

const updaterPath = "desktop/src/features/settings/hooks/use-updater.ts";
let updater = fs.readFileSync(updaterPath, "utf8");
const oldConstant =
  'const GITHUB_RELEASES_URL = "https://github.com/block/buzz/releases/latest";';
const newConstant =
  'const DEFAULT_RELEASES_URL = "https://github.com/block/buzz/releases";\n' +
  'const RELEASES_URL =\n' +
  '  import.meta.env.VITE_BUZZ_RELEASES_URL?.trim() || DEFAULT_RELEASES_URL;';
if (!updater.includes(oldConstant) || !updater.includes("releaseUrl: GITHUB_RELEASES_URL")) {
  throw new Error("canonical updater source no longer matches the reviewed beta overlay");
}
updater = updater
  .replace(oldConstant, newConstant)
  .replace("releaseUrl: GITHUB_RELEASES_URL", "releaseUrl: RELEASES_URL");
fs.writeFileSync(updaterPath, updater);
NODE

echo "Canonical OriginTrail/buzz source materialized at $SOURCE_SHA"
