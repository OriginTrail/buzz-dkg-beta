export function readUpdaterReleaseEnvironment(env = process.env) {
  const pubkey = env.BUZZ_UPDATER_PUBLIC_KEY?.trim();
  const endpoint = env.BUZZ_UPDATER_ENDPOINT?.trim();
  const missing = [];
  if (!pubkey) missing.push("BUZZ_UPDATER_PUBLIC_KEY");
  if (!endpoint) missing.push("BUZZ_UPDATER_ENDPOINT");
  if (missing.length > 0) {
    throw new Error(
      `required environment variable(s) missing: ${missing.join(", ")}`,
    );
  }
  return { pubkey, endpoint };
}

/**
 * Apply the one updater release contract to either a small production delta
 * or a complete flavor config. Flavor-specific fields remain untouched.
 */
export function buildUpdaterReleaseConfig(
  config,
  { endpoint, pubkey, minimumMacOSVersion },
) {
  const bundle = {
    ...config.bundle,
    createUpdaterArtifacts: true,
  };
  if (minimumMacOSVersion) {
    bundle.macOS = {
      ...config.bundle?.macOS,
      minimumSystemVersion: minimumMacOSVersion,
    };
  }
  if (Object.hasOwn(bundle, "externalBin")) {
    throw new Error(
      "Release config must not define bundle.externalBin; sidecars are platform-specific",
    );
  }
  return {
    ...config,
    bundle,
    plugins: {
      ...config.plugins,
      updater: {
        pubkey,
        endpoints: [endpoint],
      },
    },
  };
}
