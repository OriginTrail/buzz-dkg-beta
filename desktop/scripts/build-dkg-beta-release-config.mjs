import { readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";

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

const updaterPubkey = process.env.BUZZ_UPDATER_PUBLIC_KEY?.trim();
const updaterEndpoint = process.env.BUZZ_UPDATER_ENDPOINT?.trim();
const missing = [];
if (!updaterPubkey) missing.push("BUZZ_UPDATER_PUBLIC_KEY");
if (!updaterEndpoint) missing.push("BUZZ_UPDATER_ENDPOINT");
if (missing.length > 0) {
  console.error(
    `Error: required environment variable(s) missing: ${missing.join(", ")}`,
  );
  process.exit(1);
}

const config = JSON.parse(readFileSync(betaConfigPath, "utf8"));
const releaseConfig = {
  ...config,
  plugins: {
    ...config.plugins,
    updater: {
      pubkey: updaterPubkey,
      endpoints: [updaterEndpoint],
    },
  },
  bundle: {
    ...config.bundle,
    createUpdaterArtifacts: true,
  },
};

writeFileSync(outputConfigPath, `${JSON.stringify(releaseConfig, null, 2)}\n`);
console.log(`DKG beta updater enabled -> ${updaterEndpoint}`);
console.log(`Wrote ${outputConfigPath}`);
