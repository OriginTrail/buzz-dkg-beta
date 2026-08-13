import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { DKG_BETA_PLATFORM_ORDER, dkgBetaAssets } from "./dkg-beta-assets.mjs";

const desktopRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const repoRoot = resolve(desktopRoot, "..");
const read = (path) => readFileSync(resolve(repoRoot, path), "utf8");
const config = JSON.parse(read("desktop/src-tauri/tauri.dkg-beta.conf.json"));
const baseConfig = JSON.parse(read("desktop/src-tauri/tauri.conf.json"));
const packageJson = JSON.parse(read("desktop/package.json"));
const macosBuildScript = read("desktop/scripts/build-dkg-beta-macos.sh");
const linuxBuildScript = read("desktop/scripts/build-dkg-beta-linux.sh");
const windowsBuildScript = read("desktop/scripts/build-dkg-beta-windows.sh");
const frontendBuildScript = read("desktop/scripts/build-dkg-beta-frontend.sh");
const releaseConfigScript = read(
  "desktop/scripts/build-dkg-beta-release-config.mjs",
);
const sharedReleaseConfigScript = read(
  "desktop/scripts/updater-release-config.mjs",
);
const versionScript = read("desktop/scripts/set-dkg-beta-version.mjs");
const workflow = read(".github/workflows/dkg-beta-desktop.yml");
const promotionWorkflow = read(
  ".github/workflows/promote-dkg-beta-desktop.yml",
);
const promotionScript = read("scripts/promote-dkg-beta-desktop-release.sh");
const desktopGuide = read("docs/dkg-beta-desktop.md");
const betaEnv = read("desktop/.env.dkg-beta");
const acp = read("crates/buzz-acp/src/lib.rs");
const cliMemory = read("crates/buzz-cli/src/commands/memory.rs");
const coreKinds = read("crates/buzz-core/src/kind.rs");
const relayMemory = read("crates/buzz-relay/src/api/dkg_memory.rs");
const commandDiscovery = read(
  "desktop/src-tauri/src/managed_agents/discovery.rs",
);
const assetModel = dkgBetaAssets("0.5.7-dkg-beta.999");

function check(condition, message) {
  if (!condition) throw new Error(`DKG beta build contract: ${message}`);
}

check(
  config.productName === "Buzz DKG Beta",
  "product name must remain isolated",
);
check(
  Object.keys(assetModel.platforms).join(",") ===
    DKG_BETA_PLATFORM_ORDER.join(",") &&
    Object.values(assetModel.platforms).every(({ updaterArchive }) =>
      updaterArchive.startsWith("Buzz-DKG-Beta_0.5.7-dkg-beta.999_"),
    ),
  "the canonical platform asset model is incomplete",
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
  config.build?.beforeBuildCommand === "pnpm build:dkg-beta",
  "the cross-platform beta-specific frontend build must be used",
);
check(
  config.bundle?.macOS?.infoPlist === "Info.dkg-beta.plist",
  "the beta-specific Info.plist must be used",
);
check(
  config.bundle?.createUpdaterArtifacts === false,
  "local builds must not create release updater artifacts",
);
check(
  config.bundle?.targets === "all",
  "the beta config must support native packages on every desktop platform",
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
  frontendBuildScript.includes("vite build --mode dkg-beta") &&
    packageJson.scripts?.["build:dkg-beta"]?.includes(
      "vite build --mode dkg-beta",
    ),
  "the beta frontend build must load the dkg-beta mode",
);
check(
  betaEnv.includes("VITE_BUZZ_DKG_BETA=true"),
  "the beta disclosure flag must be enabled",
);
check(
  betaEnv.includes(
    "VITE_BUZZ_RELEASES_URL=https://github.com/OriginTrail/buzz-dkg-beta/releases",
  ),
  "the beta build flavor must select the OriginTrail releases channel",
);
check(
  macosBuildScript.includes("--no-default-features"),
  "the native build must compile out the default system-keyring feature",
);
check(
  macosBuildScript.includes('--target "$TARGET"') &&
    macosBuildScript.includes("aarch64-apple-darwin") &&
    macosBuildScript.includes("x86_64-apple-darwin"),
  "the macOS build must support both Apple Silicon and Intel targets",
);
check(
  !macosBuildScript.includes("--features system-keyring"),
  "the beta build must never explicitly enable system-keyring",
);
check(
  macosBuildScript.includes("--no-sign") &&
    macosBuildScript.includes(
      './node_modules/.bin/tauri signer sign "$UPDATER_ARCHIVE"',
    ),
  "the unsigned macOS app build must explicitly sign its updater archive",
);
for (const [platform, script, bundle] of [
  ["macOS", macosBuildScript, "--bundles app,dmg"],
  ["Linux", linuxBuildScript, "--bundles deb,appimage"],
  ["Windows", windowsBuildScript, "--bundles nsis"],
]) {
  check(
    script.includes("./node_modules/.bin/tauri build"),
    `${platform} must invoke the repository-pinned Tauri CLI`,
  );
  check(script.includes(bundle), `${platform} package selection is missing`);
  check(
    script.includes("BUZZ_DKG_BETA_TAURI_CONFIG"),
    `${platform} must accept the generated release config`,
  );
}
check(
  !linuxBuildScript.includes("--no-default-features") &&
    !windowsBuildScript.includes("--no-default-features"),
  "Windows and Linux must retain their native system-keyring feature",
);
check(
  versionScript.includes("src-tauri/tauri.dkg-beta.conf.json"),
  "the release version helper must update the beta config",
);
for (const required of [
  "macos-15",
  "macos-15-intel",
  "windows-latest",
  "ubuntu-latest",
  "build-dkg-beta-macos.sh",
  "build-dkg-beta-linux.sh",
  "build-dkg-beta-windows.sh",
  "build-dkg-beta-release-config.mjs",
  "dkg-beta-assets.mjs",
  "TAURI_SIGNING_PRIVATE_KEY",
  "updater-manifest.json",
  "buzz-dkg-beta-latest/latest.json",
  "gh release create",
]) {
  check(workflow.includes(required), `desktop workflow is missing ${required}`);
}
check(
  releaseConfigScript.includes("buildUpdaterReleaseConfig") &&
    sharedReleaseConfigScript.includes("createUpdaterArtifacts: true") &&
    sharedReleaseConfigScript.includes("BUZZ_UPDATER_PUBLIC_KEY") &&
    sharedReleaseConfigScript.includes("BUZZ_UPDATER_ENDPOINT"),
  "release config must enable signed updater artifacts and the rolling endpoint",
);
check(
  promotionScript.includes("OriginTrail/buzz-dkg-beta") &&
    promotionScript.includes('ROLLING_TAG="buzz-dkg-beta-latest"') &&
    promotionScript.includes("refusing downgrade") &&
    promotionScript.includes("dkg-beta-assets.mjs"),
  "promotion must be repository-bound, complete, and downgrade-safe",
);
check(
  promotionWorkflow.includes("promote-dkg-beta-desktop-release.sh") &&
    promotionWorkflow.includes("contents: write"),
  "the manual promotion workflow must be the only rolling-manifest writer",
);
check(
  !workflow.includes('gh release upload "buzz-dkg-beta-latest"'),
  "publishing an immutable release must not automatically promote it",
);
check(
  desktopGuide.includes("Windows Credential Manager") &&
    desktopGuide.includes("desktop Secret Service") &&
    desktopGuide.includes("Keychain-free") &&
    desktopGuide.includes("Restart to update"),
  "the desktop guide must disclose each platform's credential storage",
);
check(
  acp.includes("buzz memory propose") &&
    acp.includes("nip11_dkg_memory_schema"),
  "managed agents must receive the capability-gated DKG memory proposal instructions",
);
check(
  cliMemory.includes("pub async fn dispatch") &&
    cliMemory.includes("/api/dkg/memory"),
  "the bundled Buzz CLI must implement authenticated DKG memory proposals",
);
check(
  coreKinds.includes("KIND_DKG_MEMORY_PROPOSAL") &&
    relayMemory.includes("pub async fn propose"),
  "the relay must recognize and accept DKG memory proposal events",
);
check(
  commandDiscovery.indexOf("let mut dirs: Vec<_> = std::env::current_exe()") >= 0 &&
    commandDiscovery.indexOf("let mut dirs: Vec<_> = std::env::current_exe()") <
      commandDiscovery.indexOf(
        "dirs.extend(profile_target_dirs(&workspace_root_dir()))",
      ),
  "the running app bundle must win over compile-time workspace sidecars",
);

console.log("Buzz DKG Beta build contract is valid.");
