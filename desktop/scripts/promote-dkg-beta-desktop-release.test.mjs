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
if (args[0] === "release" && args[1] === "view") {
  const tag = args[2];
  if (args.includes("--json")) {
    process.stdout.write(fs.readFileSync(path.join(state, "release.json"), "utf8"));
    process.exit(0);
  }
  process.exit(tag === rollingTag && fs.existsSync(path.join(state, "latest.json")) ? 0 : 1);
}
if (args[0] === "api") {
  process.stdout.write("candidate-sha\\n");
  process.exit(0);
}
if (args[0] === "release" && args[1] === "download") {
  const tag = args[2];
  const destination = option("--dir");
  if (tag === candidateTag) {
    fs.copyFileSync(path.join(state, "candidate.json"), path.join(destination, "updater-manifest.json"));
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

function manifest(version, mutate = (value) => value) {
  const model = dkgBetaAssets(version);
  const base = `https://github.com/OriginTrail/buzz-dkg-beta/releases/download/v${version}/`;
  return mutate({
    version,
    notes: `Buzz DKG Beta ${version}`,
    platforms: Object.fromEntries(
      Object.entries(model.platforms).map(([platform, { updaterArchive }]) => [
        platform,
        { signature: `sig-${platform}`, url: `${base}${updaterArchive}` },
      ]),
    ),
  });
}

function runPromotion({
  version = "0.5.7-dkg-beta.4",
  candidate = manifest(version),
  current,
  releaseAssets,
  repository = "OriginTrail/buzz-dkg-beta",
} = {}) {
  const root = mkdtempSync(join(tmpdir(), "buzz-dkg-promotion-"));
  const bin = join(root, "bin");
  const state = join(root, "state");
  mkdirSync(bin);
  mkdirSync(state);
  const gh = join(bin, "gh");
  writeFileSync(gh, GH_STUB);
  chmodSync(gh, 0o755);
  writeFileSync(join(state, "candidate.json"), JSON.stringify(candidate));
  if (current)
    writeFileSync(join(state, "latest.json"), JSON.stringify(current));
  const assets = releaseAssets ?? [
    "updater-manifest.json",
    ...Object.values(dkgBetaAssets(version).platforms).map(
      ({ updaterArchive }) => updaterArchive,
    ),
  ];
  writeFileSync(
    join(state, "release.json"),
    JSON.stringify({
      isDraft: false,
      isPrerelease: true,
      targetCommitish: "candidate-sha",
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

  const model = dkgBetaAssets("0.5.7-dkg-beta.4");
  const missing = runPromotion({
    releaseAssets: [
      "updater-manifest.json",
      ...Object.values(model.platforms)
        .map(({ updaterArchive }) => updaterArchive)
        .filter((name) => !name.endsWith("setup.exe")),
    ],
  });
  assert.notEqual(missing.status, 0);
  assert.match(missing.stderr, /references missing release asset/);
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
