import { writeFileSync } from "node:fs";
import { resolve } from "node:path";
import {
  buildUpdaterReleaseConfig,
  readUpdaterReleaseEnvironment,
} from "./updater-release-config.mjs";

// Write a tauri.release.conf.json with release-only overrides.
//
// Tauri's --config flag merges the provided JSON on top of the base
// tauri.conf.json, so this file must contain ONLY the delta fields —
// not a copy of the base config.
//
// For OSS release builds this script emits:
// 1. bundle.macOS.minimumSystemVersion = "10.15" for broad compatibility.
// 2. bundle.createUpdaterArtifacts = true so Tauri produces the .tar.gz
//    archive and .sig signature during the build.
// 3. plugins.updater with the public key and endpoint from env vars.
//    Both BUZZ_UPDATER_PUBLIC_KEY and BUZZ_UPDATER_ENDPOINT are required -
//    the script fails if either is missing (OSS builds always ship with updater).
//
// Apple code signing and notarization happen post-build via
// block/apple-codesign-action in release.yml, so no signingIdentity is
// emitted here and the Tauri build is invoked with --no-sign.

const outputConfigPath = resolve(
  process.cwd(),
  "src-tauri/tauri.release.conf.json",
);

const { pubkey, endpoint } = readUpdaterReleaseEnvironment();
const releaseConfig = buildUpdaterReleaseConfig(
  {},
  { pubkey, endpoint, minimumMacOSVersion: "10.15" },
);

console.log(`Updater enabled -> ${endpoint}`);

writeFileSync(outputConfigPath, `${JSON.stringify(releaseConfig, null, 2)}\n`);
console.log(`Wrote ${outputConfigPath}`);
