# Testing the Web of Trust memory prototype

Thanks for testing! ~2-minute setup, and **you do not need to run any server or DKG node** — the memory panel resolves from the relay.

## 1. Download the app

Go to the repo's **[Releases](../../releases)** page and download the build for your OS:

| OS | File |
|----|------|
| **macOS — Apple Silicon** (M1–M4) | `Buzz-Memory_*_aarch64.dmg` |
| **macOS — Intel** | `Buzz-Memory_*_x86_64.dmg` |
| **Windows** | `Buzz-Memory_*_windows-x64.exe` |

*(Not sure which Mac? Apple menu →  About This Mac → "Apple M…" = Apple Silicon.)*

## 2. Install (unsigned test build)

These are **unsigned** test builds, so your OS will warn you:

- **macOS:** open the `.dmg`, drag Buzz to Applications. macOS will say *"Buzz is damaged and can't be opened"* — that's the quarantine flag on an unsigned build, not a broken download, and **right-click → Open does NOT bypass this variant**. The fix is one Terminal command:
  ```
  xattr -cr /Applications/Buzz.app
  ```
  then open Buzz from Applications normally.
- **Windows:** run the `.exe`. SmartScreen will warn — click **"More info" → "Run anyway"**.

## 3. Connect to the community

1. Launch Buzz.
2. Add the community relay: **`wss://macbook-pro-8.tailb02f7e.ts.net`**
   - You must be on our **Tailscale tailnet** to reach it (ping Žiga for access if needed).
3. Sign in with your existing Nostr key.

## 4. Open the memory panel

1. Open the **Web of Trust** channel.
2. Click the floating **◈ Memory** chip (bottom-right of the channel).
3. The panel opens beside the chat.

## Do I need to run a DKG node?

**No.** The panel resolves the **full channel memory** — all layers, decisions, evidence trails, and per-participant sub-graphs — through the **community gateway** (the community's DKG node, reached over the tailnet). You'll see a blue banner: *"✓ Resolved via community gateway — full memory from the community's node."* You see exactly what the operator sees.

Running your own DKG edge node upgrades that banner to green — *"✓ verified through your node"* — meaning your machine independently verified the graph rather than trusting the gateway. Optional, **not required for testing**.

If both the gateway and a local node are unreachable, the panel degrades to **discovery mode** (amber, *"shown for discovery — unverified"*), reading only the relay receipts. If you see amber, your tailnet connection to the gateway is down — check Tailscale.

See **[dkg-memory.md](dkg-memory.md)** for what the feature does and the three modes.

## What to report

- Did the app **install and launch**? Any OS warnings/blocks?
- Did the **◈ Memory** chip appear in the Web of Trust channel?
- Did the panel **populate** in discovery mode? Screenshot either way.
- Is the *"discovery / unverified"* labeling **clear** or confusing?
- Anything that felt broken, slow, or unclear.

Reply in the channel or DM Žiga. Thank you! 🙏
