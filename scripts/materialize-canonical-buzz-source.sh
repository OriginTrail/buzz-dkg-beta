#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$REPO_ROOT"

SOURCE_REPOSITORY="${SOURCE_REPOSITORY:-https://github.com/OriginTrail/buzz.git}"
SOURCE_REF="${SOURCE_REF:?SOURCE_REF is required}"
EXPECTED_SOURCE_SHA="${EXPECTED_SOURCE_SHA:?EXPECTED_SOURCE_SHA is required}"

[[ "$EXPECTED_SOURCE_SHA" =~ ^[0-9a-f]{40}$ ]] || {
  echo "::error::EXPECTED_SOURCE_SHA must be a full lowercase Git commit SHA"
  exit 1
}

TOOLING_SHA=$(git rev-parse HEAD)
OVERLAY_MANIFEST=$(mktemp)
trap 'rm -f "$OVERLAY_MANIFEST"' EXIT
git show "${TOOLING_SHA}:scripts/dkg-beta-overlay-files.txt" >"$OVERLAY_MANIFEST"

git fetch --no-tags --depth=1 "$SOURCE_REPOSITORY" "$SOURCE_REF"
SOURCE_SHA=$(git rev-parse FETCH_HEAD)
[[ "$SOURCE_SHA" == "$EXPECTED_SOURCE_SHA" ]] || {
  echo "::error::Source ref moved: expected $EXPECTED_SOURCE_SHA, resolved $SOURCE_SHA"
  exit 1
}

# Replace the application tree with the reviewed OriginTrail/buzz commit, then
# restore the declarative distribution overlay. This keeps the beta repository
# a release channel, not a second source tree.
git read-tree --reset -u "$SOURCE_SHA"
while IFS= read -r overlay_path; do
  [[ -n "$overlay_path" && "$overlay_path" != \#* ]] || continue
  [[ "$overlay_path" != /* && "$overlay_path" != *".."* ]] || {
    echo "::error::Unsafe beta overlay path: $overlay_path"
    exit 1
  }
  git checkout "$TOOLING_SHA" -- "$overlay_path"
done <"$OVERLAY_MANIFEST"

# Application-tree deltas are explicit, reviewable patches rather than hidden
# string rewrites. Dependency resolution remains locked to canonical source.
git apply scripts/dkg-beta-source.patch

echo "Canonical OriginTrail/buzz source materialized at $SOURCE_SHA"
