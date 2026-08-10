import { readFileSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const version = process.argv[2];

if (!version) {
  console.error("Usage: node scripts/set-dkg-beta-version.mjs <version>");
  process.exit(1);
}

if (!/^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/.test(version)) {
  console.error(`Invalid Buzz DKG Beta version: ${version}`);
  process.exit(1);
}

const desktopRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const configPath = resolve(desktopRoot, "src-tauri/tauri.dkg-beta.conf.json");
const config = JSON.parse(readFileSync(configPath, "utf8"));
config.version = version;
writeFileSync(configPath, `${JSON.stringify(config, null, 2)}\n`);
console.log(`Set Buzz DKG Beta version to ${version}`);
