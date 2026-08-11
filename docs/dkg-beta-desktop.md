# Buzz DKG Beta desktop builds

Buzz DKG Beta is an isolated, unsigned desktop distribution for controlled DKG
testing. It installs alongside production Buzz with a separate application
identity, deep-link scheme, and data profile.

## Supported packages

The `Buzz DKG Beta desktop` GitHub Actions workflow builds:

| Platform | Package |
|---|---|
| macOS Apple Silicon | DMG and app ZIP (`aarch64`) |
| macOS Intel | DMG and app ZIP (`x86_64`) |
| Windows x86-64 | NSIS installer (`.exe`) |
| Linux x86-64 | AppImage and Debian package (`.deb`) |

The packages are beta artifacts without Apple or Microsoft publisher signing.
Published builds do contain a dedicated, cryptographically signed DKG Beta
update channel. They never replace a regular Buzz installation.

## Credential-storage boundary

- **macOS:** the Keychain-free beta compiles without the `system-keyring`
  feature. This avoids repeated Keychain authorization prompts caused by an
  ad-hoc app signature.
  Human identity keys and managed-agent keys use owner-only local files. They
  are not encrypted at rest, so use the beta only on a trusted, single-user Mac
  and export an identity backup.
- **Windows:** the beta retains the default `system-keyring` feature and uses
  Windows Credential Manager.
- **Linux:** the beta retains the default `system-keyring` feature and uses the
  desktop Secret Service when available. Buzz's existing fallback behavior
  applies on systems without a usable Secret Service.

All platforms use:

- app name `Buzz DKG Beta`;
- bundle identifier `io.origintrail.buzz.dkgbeta`;
- deep-link scheme `buzz-dkg-beta://`;
- a separate application-data directory;
- an update channel isolated from production Buzz.

## Updates

The first updater-enabled build is a one-time manual installation. After that,
Buzz DKG Beta checks the public rolling manifest at launch and every six hours.
An available update downloads in the background and is installed only after the
user chooses **Restart to update**. Identities, communities, channels, and local
application state remain in the beta profile.

macOS, Windows NSIS, and Linux AppImage builds update in place. Linux `.deb`
installations link to the GitHub Releases page for a manual package upgrade.
Every in-app update is verified with the permanent Tauri updater public key;
this updater signature is independent from Apple or Microsoft publisher
signing.

## User installation

### macOS

Download the package matching **About This Mac**: Apple M-series processors use
`aarch64`; Intel processors use `x86_64`. Move **Buzz DKG Beta** to Applications,
then right-click it and choose **Open** on first launch. If macOS reports that
the unsigned app is damaged, run:

```bash
xattr -cr "/Applications/Buzz DKG Beta.app"
```

### Windows

Run the NSIS `.exe`. If SmartScreen appears, choose **More info**, verify that
the file came from the OriginTrail release, then choose **Run anyway**.

### Linux

Either install the Debian package:

```bash
sudo apt install ./buzz-dkg-beta*.deb
```

or mark the AppImage executable and run it:

```bash
chmod +x ./buzz-dkg-beta*.AppImage
./buzz-dkg-beta*.AppImage
```

## Community onboarding

1. Create a beta identity or import an existing Nostr identity locally. Never
   send an `nsec` to the community operator.
2. If the public key is not already a relay member, send only the 64-character
   public key to the operator and wait for admission.
3. Add `https://buzz-dkg-relay.origintrail.io` as the community URL.
4. Join an authorized channel and open the **◈ Memory** panel.

Users do not need Tailscale, a Cloudflare login, or a local DKG node. The app
authenticates to Buzz and uses the relay's authorization-aware DKG query proxy.

## Local builds

Install the repository's Hermit toolchain and JavaScript dependencies:

```bash
. ./bin/activate-hermit
just desktop-install-ci
```

Then run the script for the current host:

```bash
desktop/scripts/build-dkg-beta-macos.sh
desktop/scripts/build-dkg-beta-linux.sh
desktop/scripts/build-dkg-beta-windows.sh
```

The Windows script runs from Git Bash. The macOS script accepts
`aarch64-apple-darwin` or `x86_64-apple-darwin` as an optional target argument.
Local builds use the checked-in updater-disabled config because they do not have
access to the permanent release signing key.

## Release workflow

Run **Buzz DKG Beta desktop** from the Actions tab on `main`. Supply a unique
semver such as `0.5.7-dkg-beta.3`. With `publish` disabled, packages remain
14-day workflow artifacts. With `publish` enabled, the workflow waits for all
four native builds, creates updater signatures and checksums, and publishes the
complete set plus `updater-manifest.json` as one immutable GitHub prerelease.

Publishing does not automatically expose the release to installed apps. After
smoke-testing the immutable release, run **Promote Buzz DKG Beta desktop
update** with the same version. The promotion validates all platform URLs and
signatures, refuses downgrades, and replaces `latest.json` on the rolling
`buzz-dkg-beta-latest` release. Installed apps discover only promoted versions.

## Release smoke test

- Confirm the installed name is **Buzz DKG Beta**, not **Buzz**.
- Confirm a normal Buzz installation remains installed and unchanged.
- Quit and relaunch; confirm the public identity is unchanged.
- On macOS, complete onboarding without a Keychain authorization prompt.
- On Windows and Linux, confirm native credential storage works across restart.
- From the previous updater-enabled version, confirm the new version downloads,
  installs after **Restart to update**, and preserves the active identity.
- Join the public DKG relay, exchange a message, invoke a managed agent, and
  confirm its successful turn creates a signed kind-40009 memory proposal.
- Confirm the integration accepts that proposal and the channel's Shared
  Working Memory triple count increases before querying the new fact.
