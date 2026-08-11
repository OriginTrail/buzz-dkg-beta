import assert from "node:assert/strict";
import {
  chmodSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";
import test from "node:test";

import { dkgBetaAssets } from "./dkg-beta-assets.mjs";

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
const promotionScript = join(
  repoRoot,
  "scripts/promote-dkg-beta-desktop-release.sh",
);

const GH_STUB = `#!/usr/bin/env node
const fs = require("node:fs");
const path = require("node:path");
const args = process.argv.slice(2);
const state = process.env.MOCK_GH_STATE;
const candidateTag = process.env.MOCK_CANDIDATE_TAG;
const rollingTag = "buzz-dkg-beta-latest";
const option = (name) => args[args.indexOf(name) + 1];
const options = (name) => args.flatMap((arg, index) =>
  arg === name ? [args[index + 1]] : [],
);
if (args[0] === "release" && args[1] === "view") {
  const tag = args[2];
  if (args.includes("--json")) {
    process.stdout.write(fs.readFileSync(path.join(state, "release.json"), "utf8"));
    process.exit(0);
  }
  process.exit(tag === rollingTag && fs.existsSync(path.join(state, "latest.json")) ? 0 : 1);
}
if (args[0] === "api") {
  const resource = args[1];
  const sha = resource.endsWith(\`/commits/\${candidateTag}\`)
    ? process.env.MOCK_TAG_SHA
    : process.env.MOCK_TARGET_SHA;
  process.stdout.write(\`\${sha}\\n\`);
  process.exit(0);
}
if (args[0] === "release" && args[1] === "download") {
  const tag = args[2];
  const destination = option("--dir");
  if (tag === candidateTag) {
    for (const pattern of options("--pattern")) {
      const source = pattern === "updater-manifest.json"
        ? path.join(state, "candidate.json")
        : path.join(state, "assets", pattern);
      if (!fs.existsSync(source)) process.exit(1);
      fs.copyFileSync(source, path.join(destination, pattern));
    }
    process.exit(0);
  }
  const latest = path.join(state, "latest.json");
  if (tag === rollingTag && fs.existsSync(latest)) {
    fs.copyFileSync(latest, path.join(destination, "latest.json"));
    process.exit(0);
  }
  process.exit(1);
}
if (args[0] === "release" && args[1] === "create") process.exit(0);
if (args[0] === "release" && args[1] === "upload") {
  fs.copyFileSync(args[3], path.join(state, "latest.json"));
  fs.appendFileSync(path.join(state, "uploads"), "upload\\n");
  process.exit(0);
}
console.error("unexpected gh invocation", args.join(" "));
process.exit(2);
`;

const MINISIGN_STUB = `#!/usr/bin/env node
const args = process.argv.slice(2);
const option = (name) => args[args.indexOf(name) + 1];
if (process.env.MOCK_MINISIGN_INVALID === "true") process.exit(1);
const archive = require("node:fs").readFileSync(option("-m"), "utf8");
const signature = require("node:fs").readFileSync(option("-x"), "utf8");
process.exit(signature.slice(4) === archive.slice(8) ? 0 : 1);
`;

function updaterSignature(platform) {
  return Buffer.from(`sig-${platform}`).toString("base64");
}

function manifest(version, mutate = (value) => value) {
  const model = dkgBetaAssets(version);
  const base = `https://github.com/OriginTrail/buzz-dkg-beta/releases/download/v${version}/`;
  return mutate({
    version,
    notes: `Buzz DKG Beta ${version}`,
    platforms: Object.fromEntries(
      Object.entries(model.platforms).map(([platform, { updaterArchive }]) => [
        platform,
        {
          signature: updaterSignature(platform),
          url: `${base}${updaterArchive}`,
        },
      ]),
    ),
  });
}

function releaseAssetNames(version) {
  return [
    "updater-manifest.json",
    ...Object.values(dkgBetaAssets(version).platforms).flatMap(
      ({ updaterArchive }) => [updaterArchive, `${updaterArchive}.sig`],
    ),
  ];
}

function runPromotion({
  version = "0.5.7-dkg-beta.4",
  candidate = manifest(version),
  current,
  releaseAssets,
  repository = "OriginTrail/buzz-dkg-beta",
  targetCommitish = "candidate-target",
  tagSha = "candidate-sha",
  targetSha = tagSha,
  invalidSignature = false,
} = {}) {
  const root = mkdtempSync(join(tmpdir(), "buzz-dkg-promotion-"));
  const bin = join(root, "bin");
  const state = join(root, "state");
  mkdirSync(bin);
  mkdirSync(state);
  mkdirSync(join(state, "assets"));
  const gh = join(bin, "gh");
  writeFileSync(gh, GH_STUB);
  chmodSync(gh, 0o755);
  const minisign = join(bin, "minisign");
  writeFileSync(minisign, MINISIGN_STUB);
  chmodSync(minisign, 0o755);
  writeFileSync(join(state, "candidate.json"), JSON.stringify(candidate));
  if (current)
    writeFileSync(join(state, "latest.json"), JSON.stringify(current));
  for (const [platform, { updaterArchive }] of Object.entries(
    dkgBetaAssets(version).platforms,
  )) {
    writeFileSync(join(state, "assets", updaterArchive), `archive-${platform}`);
    writeFileSync(
      join(state, "assets", `${updaterArchive}.sig`),
      updaterSignature(platform),
    );
  }
  const assets = releaseAssets ?? releaseAssetNames(version);
  writeFileSync(
    join(state, "release.json"),
    JSON.stringify({
      isDraft: false,
      isPrerelease: true,
      targetCommitish,
      assets: assets.map((name) => ({ name })),
    }),
  );
  const result = spawnSync("bash", [promotionScript, version], {
    cwd: repoRoot,
    encoding: "utf8",
    env: {
      ...process.env,
      PATH: `${bin}:${process.env.PATH}`,
      GITHUB_REPOSITORY: repository,
      GITHUB_STEP_SUMMARY: join(root, "summary.md"),
      MOCK_GH_STATE: state,
      MOCK_CANDIDATE_TAG: `v${version}`,
      MOCK_TAG_SHA: tagSha,
      MOCK_TARGET_SHA: targetSha,
      MOCK_MINISIGN_INVALID: String(invalidSignature),
      BUZZ_UPDATER_PUBLIC_KEY: Buffer.from(
        "untrusted comment: minisign public key: mock\\nRWTmock",
      ).toString("base64"),
    },
  });
  return {
    ...result,
    uploaded: existsSync(join(state, "uploads")),
    rolling: existsSync(join(state, "latest.json"))
      ? readFileSync(join(state, "latest.json"), "utf8")
      : null,
  };
}

test("promotion rejects downgrade and same-version manifest replacement", () => {
  const downgrade = runPromotion({
    current: manifest("0.5.7-dkg-beta.5"),
  });
  assert.notEqual(downgrade.status, 0);
  assert.match(downgrade.stderr, /refusing downgrade/);
  assert.equal(downgrade.uploaded, false);

  const replacement = runPromotion({
    current: manifest("0.5.7-dkg-beta.4", (value) => ({
      ...value,
      notes: "different immutable content",
    })),
  });
  assert.notEqual(replacement.status, 0);
  assert.match(replacement.stderr, /different manifest content/);
  assert.equal(replacement.uploaded, false);
});

test("promotion rejects incomplete manifests and missing release assets", () => {
  const incomplete = runPromotion({
    candidate: manifest("0.5.7-dkg-beta.4", (value) => {
      delete value.platforms["windows-x86_64"];
      return value;
    }),
  });
  assert.notEqual(incomplete.status, 0);
  assert.match(
    incomplete.stderr,
    /failed version, platform, signature, or URL/,
  );

  const missing = runPromotion({
    releaseAssets: releaseAssetNames("0.5.7-dkg-beta.4").filter(
      (name) => !name.endsWith("setup.exe"),
    ),
  });
  assert.notEqual(missing.status, 0);
  assert.match(missing.stderr, /references missing release asset/);
});

test("promotion verifies manifest signatures against every updater archive", () => {
  const mismatch = runPromotion({
    candidate: manifest("0.5.7-dkg-beta.4", (value) => {
      value.platforms["darwin-aarch64"].signature = "not-the-release-signature";
      return value;
    }),
  });
  assert.notEqual(mismatch.status, 0);
  assert.match(mismatch.stderr, /signature does not match/);
  assert.equal(mismatch.uploaded, false);

  const invalid = runPromotion({ invalidSignature: true });
  assert.notEqual(invalid.status, 0);
  assert.match(invalid.stderr, /does not verify/);
  assert.equal(invalid.uploaded, false);
});

test("promotion rejects a release target that differs from its immutable tag", () => {
  const result = runPromotion({
    tagSha: "tag-sha",
    targetSha: "other-sha",
  });
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /do not resolve to the same commit/);
  assert.equal(result.uploaded, false);
});

test("successful promotion uploads the exact validated candidate", () => {
  const candidate = manifest("0.5.7-dkg-beta.4");
  const result = runPromotion({
    candidate,
    current: manifest("0.5.7-dkg-beta.3"),
  });
  assert.equal(result.status, 0, result.stderr);
  assert.equal(result.uploaded, true);
  assert.deepEqual(JSON.parse(result.rolling), candidate);
});

test("promotion remains repository-bound", () => {
  const result = runPromotion({ repository: "attacker/fork" });
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /promotion is restricted/);
  assert.equal(result.uploaded, false);
});
