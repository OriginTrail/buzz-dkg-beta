# Testing Buzz DKG Beta

Thanks for testing. You do not need to run a relay or DKG node: the app queries
the community's DKG through the authenticated Buzz relay.

## 1. Download the app

Open the repository's [Releases](../../releases) page and download the package
for your operating system:

| OS | Package |
|---|---|
| macOS Apple Silicon | `*aarch64.dmg` or `*aarch64.zip` |
| macOS Intel | `*x86_64.dmg` or `*x86_64.zip` |
| Windows x86-64 | `*.exe` |
| Linux x86-64 | `*.AppImage` or `*.deb` |

On a Mac, **About This Mac** reports an Apple M-series chip or an Intel
processor. Detailed unsigned-build installation steps are in
[dkg-beta-desktop.md](dkg-beta-desktop.md).

## 2. Create or import an identity

Buzz DKG Beta uses an isolated app profile and can run beside regular Buzz.

- To retain an existing relay membership, import the same Nostr identity
  locally. Never share its private `nsec`.
- For a new identity, create and export a backup, then send only its
  64-character public key to the community operator for relay admission.

On macOS, repeated Keychain prompts are a failure and should be reported. The
controlled beta deliberately uses local owner-only key files there.

## 3. Join the community

1. If the identity is new, wait for the tester coordinator to confirm that its
   public key has been admitted to the relay and authorized for the test
   channel.
2. Add `https://buzz-dkg-relay.origintrail.io` as the community URL.
3. Open the channel supplied by the tester coordinator.

No Tailscale connection or Cloudflare login is required.

## 4. Exercise DKG memory

1. Exchange a few messages and invoke a channel agent.
2. Open the floating **◈ Memory** chip.
3. Ask about a decision, contributor, or implementation topic discussed in the
   channel.
4. Follow an evidence or node-resolution link and confirm its source matches
   the conversation.

The normal beta path is the authenticated community provider. A personal DKG
Edge node remains an optional later upgrade for independent verification.

## What to report

- Operating system and CPU architecture.
- Whether installation and first launch succeeded.
- Any Keychain, SmartScreen, Gatekeeper, Secret Service, or permission prompts.
- Whether the app retained the same identity after restart.
- Whether the community and channel joined successfully.
- Whether **◈ Memory** populated and its evidence links were understandable.
- Any failed agent tools, stale memory, unexpected authorization, or slow query.
