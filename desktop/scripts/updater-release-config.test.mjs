import assert from "node:assert/strict";
import test from "node:test";

import {
  buildUpdaterReleaseConfig,
  readUpdaterReleaseEnvironment,
} from "./updater-release-config.mjs";

test("release flavors share updater environment validation", () => {
  assert.throws(
    () => readUpdaterReleaseEnvironment({}),
    /BUZZ_UPDATER_PUBLIC_KEY, BUZZ_UPDATER_ENDPOINT/,
  );
  assert.deepEqual(
    readUpdaterReleaseEnvironment({
      BUZZ_UPDATER_PUBLIC_KEY: " key ",
      BUZZ_UPDATER_ENDPOINT: " https://example.test/latest.json ",
    }),
    { pubkey: "key", endpoint: "https://example.test/latest.json" },
  );
});

test("release updater config preserves flavor fields", () => {
  const result = buildUpdaterReleaseConfig(
    {
      productName: "Buzz DKG Beta",
      bundle: { macOS: { infoPlist: "Info.dkg-beta.plist" } },
      plugins: { "deep-link": { desktop: { schemes: ["buzz-dkg-beta"] } } },
    },
    {
      pubkey: "key",
      endpoint: "https://example.test/latest.json",
      minimumMacOSVersion: "10.15",
    },
  );
  assert.equal(result.productName, "Buzz DKG Beta");
  assert.deepEqual(result.bundle, {
    macOS: {
      infoPlist: "Info.dkg-beta.plist",
      minimumSystemVersion: "10.15",
    },
    createUpdaterArtifacts: true,
  });
  assert.deepEqual(result.plugins.updater, {
    pubkey: "key",
    endpoints: ["https://example.test/latest.json"],
  });
  assert.deepEqual(result.plugins["deep-link"].desktop.schemes, [
    "buzz-dkg-beta",
  ]);
});

test("release delta never overrides platform sidecars", () => {
  assert.throws(
    () =>
      buildUpdaterReleaseConfig(
        { bundle: { externalBin: [] } },
        { pubkey: "key", endpoint: "https://example.test/latest.json" },
      ),
    /must not define bundle\.externalBin/,
  );
});
