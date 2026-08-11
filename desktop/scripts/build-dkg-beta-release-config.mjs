import { readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import {
  buildUpdaterReleaseConfig,
  readUpdaterReleaseEnvironment,
} from "./updater-release-config.mjs";

// Build the release-only Tauri delta from the checked-in DKG beta config.
// Local beta builds intentionally keep updater artifacts disabled; GitHub
// Actions opts into this generated config only when it has the permanent
// updater signing key available.

const betaConfigPath = resolve(
  process.cwd(),
  "src-tauri/tauri.dkg-beta.conf.json",
);
const outputConfigPath = resolve(
  process.cwd(),
  "src-tauri/tauri.dkg-beta.release.conf.json",
);

const config = JSON.parse(readFileSync(betaConfigPath, "utf8"));
const { pubkey, endpoint } = readUpdaterReleaseEnvironment();
const releaseConfig = buildUpdaterReleaseConfig(config, { pubkey, endpoint });

writeFileSync(outputConfigPath, `${JSON.stringify(releaseConfig, null, 2)}\n`);
console.log(`DKG beta updater enabled -> ${endpoint}`);
console.log(`Wrote ${outputConfigPath}`);
