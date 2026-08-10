import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const desktopRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const repoRoot = resolve(desktopRoot, "..");
const read = (path) => readFileSync(resolve(repoRoot, path), "utf8");
const config = JSON.parse(read("desktop/src-tauri/tauri.dkg-beta.conf.json"));
const baseConfig = JSON.parse(read("desktop/src-tauri/tauri.conf.json"));
const packageJson = JSON.parse(read("desktop/package.json"));
const buildScript = read("desktop/scripts/build-dkg-beta-macos.sh");
const frontendBuildScript = read("desktop/scripts/build-dkg-beta-frontend.sh");
const betaEnv = read("desktop/.env.dkg-beta");

function check(condition, message) {
  if (!condition) throw new Error(`DKG beta build contract: ${message}`);
}

check(
  config.productName === "Buzz DKG Beta",
  "product name must remain isolated",
);
check(
  config.identifier === "io.origintrail.buzz.dkgbeta",
  "bundle identifier must remain isolated",
);
check(
  config.identifier !== baseConfig.identifier,
  "bundle identifier must differ from production Buzz",
);
check(
  config.build?.beforeBuildCommand === "./scripts/build-dkg-beta-frontend.sh",
  "the beta-specific frontend build must be used",
);
check(
  config.bundle?.macOS?.infoPlist === "Info.dkg-beta.plist",
  "the beta-specific Info.plist must be used",
);
check(
  config.bundle?.createUpdaterArtifacts === false,
  "auto-updater artifacts must stay disabled",
);
check(
  config.plugins?.["deep-link"]?.desktop?.schemes?.[0] === "buzz-dkg-beta",
  "the beta deep-link scheme must not claim production Buzz links",
);
check(
  packageJson.scripts?.["build:dkg-beta"] ===
    "tsc && vite build --mode dkg-beta",
  "package.json must expose the beta frontend build",
);
check(
  frontendBuildScript.includes("vite build --mode dkg-beta"),
  "the Tauri frontend hook must load the beta build mode",
);
check(
  betaEnv.includes("VITE_BUZZ_DKG_BETA=true"),
  "the beta disclosure flag must be enabled",
);
check(
  buildScript.includes("--no-default-features"),
  "the native build must compile out the default system-keyring feature",
);
check(
  buildScript.includes("./node_modules/.bin/tauri build"),
  "the native build must invoke the repository-pinned Tauri CLI",
);
check(
  !buildScript.includes("--features system-keyring"),
  "the beta build must never explicitly enable system-keyring",
);

console.log("Buzz DKG Beta build contract is valid.");
