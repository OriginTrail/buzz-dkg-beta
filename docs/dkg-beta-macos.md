# Buzz DKG Beta for macOS

Buzz DKG Beta is an isolated, unsigned macOS build for controlled DKG testing.
It installs alongside production Buzz and deliberately avoids macOS Keychain
access so an ad-hoc signature cannot cause repeated authorization prompts.

## User experience

1. Download the Apple Silicon DMG from the test release.
2. Drag **Buzz DKG Beta** to Applications.
3. On first launch, right-click the app, choose **Open**, then confirm **Open**.
4. Create a beta identity and back it up during onboarding.
5. Join `wss://buzz-dkg-relay.origintrail.io` or use an invitation supplied by
   the community operator.

The first-launch Gatekeeper confirmation is expected because this build is not
Apple-notarized. Keychain password prompts are not expected.

## Security boundary

This beta compiles without the `system-keyring` feature. Human identity keys
are stored in the app's private `identity.key` file and managed-agent keys are
stored in its private agent store. Buzz creates both with owner-only (`0600`)
permissions on macOS. They are not encrypted at rest, so this build is intended
only for controlled testing on a trusted, single-user Mac.

The beta uses:

- app name `Buzz DKG Beta`;
- bundle identifier `io.origintrail.buzz.dkgbeta`;
- deep-link scheme `buzz-dkg-beta://`;
- a separate application-data directory;
- no updater endpoints or updater artifacts.

It neither reads nor migrates production Buzz's Keychain entries or local app
profile.

## Build

Install the repository's Hermit toolchain and JavaScript dependencies, then run:

```bash
. ./bin/activate-hermit
just desktop-install-ci
desktop/scripts/build-dkg-beta-macos.sh
```

The script builds the managed-agent sidecars and invokes Tauri with
`-- --no-default-features`; the separator forwards the keychain-disabling flag
to Cargo. The optional Mesh LLM feature is not required for managed agents or
DKG memory and is omitted from this focused beta.

The DMG is written below `desktop/src-tauri/target/release/bundle/dmg/`. The
script also emits an app ZIP below `desktop/src-tauri/target/release/bundle/zip/`
as a fallback for restricted macOS environments where `hdiutil` cannot create a
disk image. The ZIP contains the same app bundle and has the same first-launch
Gatekeeper behavior.

## Release smoke test

- Confirm the installed name is **Buzz DKG Beta**, not **Buzz**.
- Confirm a normal Buzz installation remains present and unchanged.
- Complete onboarding without a Keychain authorization prompt.
- Quit and relaunch; confirm the public identity is unchanged.
- Confirm `identity.key` and `agents/managed-agents.json` are mode `0600` when
  present.
- Join the DKG relay, exchange a message, invoke a managed agent, and query the
  channel's DKG memory.
- Remove the app and its isolated profile after testing if the machine is not
  intended to retain plaintext-at-rest beta keys.
