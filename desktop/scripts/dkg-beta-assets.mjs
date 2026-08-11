const VERSION_PATTERN = /^\d+\.\d+\.\d+-dkg-beta\.\d+$/;

import { pathToFileURL } from "node:url";

export const DKG_BETA_PLATFORM_ORDER = [
  "darwin-aarch64",
  "darwin-x86_64",
  "linux-x86_64",
  "windows-x86_64",
];

export function dkgBetaAssets(version) {
  if (!VERSION_PATTERN.test(version)) {
    throw new Error("version must match X.Y.Z-dkg-beta.N");
  }
  return {
    version,
    platforms: {
      "darwin-aarch64": {
        updaterArchive: `Buzz-DKG-Beta_${version}_aarch64.app.tar.gz`,
      },
      "darwin-x86_64": {
        updaterArchive: `Buzz-DKG-Beta_${version}_x86_64.app.tar.gz`,
      },
      "linux-x86_64": {
        updaterArchive: `Buzz-DKG-Beta_${version}_x86_64.AppImage`,
      },
      "windows-x86_64": {
        updaterArchive: `Buzz-DKG-Beta_${version}_x86_64-setup.exe`,
      },
    },
  };
}

if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(process.argv[1]).href
) {
  const [version, option, platform] = process.argv.slice(2);
  const model = dkgBetaAssets(version ?? "");
  if (option === "--asset") {
    const asset = model.platforms[platform]?.updaterArchive;
    if (!asset) throw new Error(`unsupported DKG beta platform: ${platform}`);
    process.stdout.write(asset);
  } else if (option) {
    throw new Error(`unsupported option: ${option}`);
  } else {
    process.stdout.write(JSON.stringify(model));
  }
}
