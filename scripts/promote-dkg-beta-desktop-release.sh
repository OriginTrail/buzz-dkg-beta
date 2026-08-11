#!/usr/bin/env bash

set -euo pipefail
export LC_ALL=C

VERSION="${1:-}"
REPOSITORY="${GITHUB_REPOSITORY:-OriginTrail/buzz-dkg-beta}"
TAG="v${VERSION}"
CANDIDATE="updater-manifest.json"
ROLLING_TAG="buzz-dkg-beta-latest"
EXPECTED_PLATFORMS='["darwin-aarch64","darwin-x86_64","linux-x86_64","windows-x86_64"]'

fail() { echo "::error::$*" >&2; exit 1; }

[[ "$REPOSITORY" == "OriginTrail/buzz-dkg-beta" ]] || \
  fail "promotion is restricted to OriginTrail/buzz-dkg-beta"
[[ "$VERSION" =~ ^([0-9]+)\.([0-9]+)\.([0-9]+)-dkg-beta\.([0-9]+)$ ]] || \
  fail "version must match X.Y.Z-dkg-beta.N"
command -v gh >/dev/null || fail "gh is required"
command -v jq >/dev/null || fail "jq is required"

version_rank() {
  local version="$1"
  [[ "$version" =~ ^([0-9]+)\.([0-9]+)\.([0-9]+)-dkg-beta\.([0-9]+)$ ]] ||
    fail "invalid promoted beta version: $version"
  printf '%012d.%012d.%012d.%012d' \
    "$((10#${BASH_REMATCH[1]}))" \
    "$((10#${BASH_REMATCH[2]}))" \
    "$((10#${BASH_REMATCH[3]}))" \
    "$((10#${BASH_REMATCH[4]}))"
}

workdir="$(mktemp -d)"
trap 'rm -rf "$workdir"' EXIT
candidate="$workdir/$CANDIDATE"
current="$workdir/latest.json"

release_json="$(gh release view "$TAG" --repo "$REPOSITORY" --json isDraft,isPrerelease,targetCommitish,assets)"
[[ "$(jq -r .isDraft <<<"$release_json")" == false ]] || fail "$TAG is still a draft"
[[ "$(jq -r .isPrerelease <<<"$release_json")" == true ]] || fail "$TAG is not a beta prerelease"

tag_sha="$(gh api "repos/$REPOSITORY/commits/$TAG" --jq .sha)"
target="$(jq -r .targetCommitish <<<"$release_json")"
target_sha="$(gh api "repos/$REPOSITORY/commits/$target" --jq .sha)"
[[ -n "$tag_sha" && "$target_sha" == "$tag_sha" ]] || \
  fail "$TAG and its release target do not resolve to the same commit"

release_assets="$(jq -r '.assets[].name' <<<"$release_json")"
grep -Fxq "$CANDIDATE" <<<"$release_assets" || fail "$TAG has no $CANDIDATE asset"
gh release download "$TAG" --repo "$REPOSITORY" --pattern "$CANDIDATE" --dir "$workdir"

base_url="https://github.com/$REPOSITORY/releases/download/$TAG/"
jq -e \
  --arg version "$VERSION" \
  --arg base "$base_url" \
  --argjson expected "$EXPECTED_PLATFORMS" '
    .version == $version and
    (.platforms | keys == $expected) and
    ([.platforms[] | (.signature | type == "string" and length > 0)] | all) and
    ([.platforms[] | (.url | type == "string" and startswith($base))] | all)
  ' "$candidate" >/dev/null || \
  fail "$CANDIDATE failed version, platform, signature, or URL validation"

while IFS= read -r url; do
  asset="${url##*/}"
  [[ "$url" == "${base_url}${asset}" ]] || \
    fail "$CANDIDATE contains non-canonical updater URL: $url"
  grep -Fxq "$asset" <<<"$release_assets" || \
    fail "$CANDIDATE references missing release asset: $asset"
done < <(jq -r '.platforms[].url' "$candidate")

if ! gh release view "$ROLLING_TAG" --repo "$REPOSITORY" >/dev/null 2>&1; then
  gh release create "$ROLLING_TAG" \
    --repo "$REPOSITORY" \
    --target "$tag_sha" \
    --title "Buzz DKG Beta update channel" \
    --notes "Rolling signed updater manifest for Buzz DKG Beta." \
    --prerelease \
    --latest=false
fi

previous_version="none"
current_digest=""
if gh release download "$ROLLING_TAG" \
  --repo "$REPOSITORY" \
  --pattern latest.json \
  --dir "$workdir" >/dev/null 2>&1; then
  current_digest="$(sha256sum "$current" | awk '{print $1}')"
  previous_version="$(jq -er '.version | select(type == "string")' "$current")" || \
    fail "current latest.json has no version"
  version_rank "$previous_version" >/dev/null

  if [[ "$VERSION" == "$previous_version" ]]; then
    cmp -s "$candidate" "$current" || \
      fail "$VERSION is already promoted with different manifest content"
    echo "Version $VERSION is already promoted with identical manifest content."
    exit 0
  fi

  [[ "$(version_rank "$VERSION")" > "$(version_rank "$previous_version")" ]] || \
    fail "refusing downgrade from $previous_version to $VERSION"

  # Re-read immediately before the only write so a concurrent promotion cannot
  # silently be overwritten by stale validation.
  rm -f "$current"
  gh release download "$ROLLING_TAG" \
    --repo "$REPOSITORY" \
    --pattern latest.json \
    --dir "$workdir"
  [[ "$(sha256sum "$current" | awk '{print $1}')" == "$current_digest" ]] || \
    fail "current promotion changed during validation; retry"
fi

promotion="$workdir/latest.json"
cp "$candidate" "$promotion"
candidate_digest="$(sha256sum "$candidate" | awk '{print $1}')"
gh release upload "$ROLLING_TAG" "$promotion" \
  --repo "$REPOSITORY" \
  --clobber || fail "promotion upload failed; retry"

rm -f "$promotion"
gh release download "$ROLLING_TAG" \
  --repo "$REPOSITORY" \
  --pattern latest.json \
  --dir "$workdir" || fail "promoted latest.json could not be downloaded"
[[ "$(sha256sum "$promotion" | awk '{print $1}')" == "$candidate_digest" ]] || \
  fail "served latest.json does not match the promoted candidate"

{
  echo "### Buzz DKG Beta auto-update promoted"
  echo "- Version: \`$VERSION\`"
  echo "- Tag commit: \`$tag_sha\`"
  echo "- Previous version: \`$previous_version\`"
  echo "- Manifest SHA-256: \`$candidate_digest\`"
  echo "- Actor: \`${GITHUB_ACTOR:-unknown}\`"
  if [[ -n "${GITHUB_SERVER_URL:-}" && -n "${GITHUB_RUN_ID:-}" ]]; then
    echo "- Workflow: ${GITHUB_SERVER_URL}/${REPOSITORY}/actions/runs/${GITHUB_RUN_ID}"
  fi
} >> "${GITHUB_STEP_SUMMARY:-/dev/null}"
