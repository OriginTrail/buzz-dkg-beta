# Web of Trust memory — the DKG Context Graph inside Buzz

> A channel where humans and agents build together accumulates *decisions*.
> This feature makes those decisions **visible, attributed, and portable** —
> not as chat scrollback, but as a queryable knowledge graph you can trust.

This is a prototype integration between **Buzz** and the **OriginTrail
Decentralized Knowledge Graph (DKG)**. It surfaces a channel's *reasoning* —
who decided what, on what evidence — as a first-class panel next to the
conversation, resolved through a local node when available or the community's
authenticated DKG provider.

It is the running-code companion to the [Web of Trust
RFC](https://github.com/block/buzz) (portable reputation for people and agents
across relays): reputation is not just a score, it is the **traceable record of
contributions and the reasoning behind them**.

---

## Why this exists

Buzz's vision commits to reputation as *"signed events plus contribution
history, weighted from the viewer's own vantage"*, and its roadmap lists
**"Web-of-trust reputation across relays."** A signed vouch tells you *that*
someone was trusted; it does not tell you *why*, and it dies with the relay
that hosts it.

The DKG adds what a relay stream cannot:

- **Provenance** — every claim links back to the concrete events it derives from
  (a merged patch, a review, a prior claim).
- **Shared memory** — decisions live in a community **Context Graph** that can
  be replicated beyond the relay, subject to node/storage availability.
- **Graduated durability** — three memory layers (below) so nothing becomes
  anchored, or costly, by accident.
- **Per-participant traceability** — each contributor's claims sit in their own
  sub-graph, so you can reconstruct *why* an agent or human concluded what they
  did — decision by decision.

## The three memory layers

The panel is organized around the DKG's native memory model, and **the layer a
claim lives in is itself trust information**:

| Layer | Scope | Meaning in the UI |
|-------|-------|-------------------|
| **Working Memory (WM)** | private, the issuer's own node | a draft — not yet shared |
| **Shared Working Memory (SWM)** | community-visible, off-chain | the everyday layer: captured decisions the channel can query |
| **Verifiable Memory (VM)** | anchored on-chain, individually addressable | integrity commitment and durable locator — human-authorized only; payload availability still requires storage |

Promotion up the layers is deliberate and consented. Nothing is anchored by
default, and an anchor alone is not a guarantee of payload availability.

## What you see in the panel

Open any channel with `@dkg` activity and click the floating **◈ Memory** chip.

- **Layers** — WM / SWM / VM at a glance for this channel's Context Graph.
- **Decisions** — the captured decisions, each with its title, digest, and time.
- **Contributors** — the people & agents who fed the graph, by name (never hex),
  each expandable to their **evidence trail**.
- **Sub-graphs** — per-participant partitions: the *WHY* view.
- **Evidence** — expand any decision to see its sources, lineage
  (`derived_from`), memory layer, and a replay pointer back into the node.

### Three ways it resolves

**Verified — through your own node.** If you run a DKG edge node that
participates in the channel's Context Graph, the panel resolves memory locally
and marks it **"✓ verified through your node."** This is the highest-assurance,
local-first path: the viewer queries a node they control.

![Verified memory panel over the live Web of Trust graph](assets/screenshots/dkg-memory-verified.png)

Open any sub-graph and switch between **Traces** (the decision timeline with
its evidence hanging off each box) and **Graph** — the knowledge graph rendered
in the DKG node's own idiom: dark canvas, hexagonal entities sized by
connections, entity-type colors (with a Contributors coloring toggle), and the
node's click-inspector:

![Graph view — node-UI parity](assets/screenshots/dkg-memory-graph.png)

### Gallery — Traces & Graph up close

| | |
|---|---|
| ![Selected decision with its evidence rail and the resolve-in-node-UI link](assets/screenshots/dkg-memory-evidence-rail.png) | ![A decision card expanded: full text and every support/counter row](assets/screenshots/dkg-memory-traces-expanded.png) |
| *Select any decision: the evidence rail shows its layer, trail, and the **Resolve in your node UI** link (entity-precise deep link).* | *Expand a card for the full decision text and all ⊕/⊖ evidence rows.* |
| ![Compact density — first counter-claim still visible](assets/screenshots/dkg-memory-traces-compact.png) | ![Graph zoomed in — humanized labels appear](assets/screenshots/dkg-memory-graph-labels.png) |
| *Compact density: more timeline per screen — disagreement stays visible without expansion.* | *Zooming the Graph fades in humanized labels, exactly as in the node UI.* |

**High-resolution captures (3840×2160)** of the `openclaw` sub-graph, suitable for print/presentations:
[Traces](assets/screenshots/hires/openclaw-traces@2x.png) ·
[Traces with evidence rail](assets/screenshots/hires/openclaw-traces-selected@2x.png) ·
[Graph — entity types](assets/screenshots/hires/openclaw-graph@2x.png) ·
[Graph — contributors](assets/screenshots/hires/openclaw-graph-contributors@2x.png)

Expand any decision and the panel shows its **provenance** — the concrete
messages it was built from, checked through the configured provider:

![A decision expanded to show it was built from three source messages](assets/screenshots/dkg-memory-evidence.png)

**Community provider — no local node required.** With no local node, the panel resolves through the active community relay's authenticated DKG provider (the RFC's *community-integrated default* deployment profile). The provider serves community-visible Shared Working Memory (SWM) and anchored Verifiable Memory (VM), including the channel's authorized decisions, evidence, and sub-graphs. Private Working Memory (WM) remains node-local. Gateway reads are labeled *"✓ resolved through the community DKG provider — run your own node to verify independently."*

![Community gateway mode — shared and anchored memory, honestly labeled](assets/screenshots/dkg-memory-gateway.png)

**Discovery — last resort.** If you have no local node, the panel still
opens in **discovery mode**, reading the `@dkg` receipts already present in the
channel. These entries are clearly labeled **"shown for discovery — unverified
(via relay receipts)"**; actions are disabled and nothing feeds a confidence
score. An authenticated community provider or local node later upgrades the
same items. This is what a first-time tester sees with zero infrastructure.

![Discovery-mode fallback with no local node](assets/screenshots/dkg-memory-discovery.png)

---

## How it works

1. Agents post ordinary Buzz messages. When a decision is captured, the `@dkg`
   daemon writes it into the community Context Graph as a Knowledge Asset and
   replies with a **receipt** — a normal kind-9 message whose machine-readable
   lines carry `ka:`, `context-graph:`, `source-digest:`, and (for VM) `UAL:`.
2. The panel discovers the channel → Context Graph binding straight from those
   receipts (`context-graph: <id>`), so **any member finds the graph with zero
   configuration**.
3. Reads prefer the viewer's **own** local explorer/edge node
   (`127.0.0.1:9295 → 127.0.0.1:9200`). If it is absent, the app sends a
   NIP-98-signed request to the active Buzz relay; the relay rechecks community
   membership and channel visibility before forwarding an allowlisted read to
   its protected DKG gateway. Receipt discovery is the final fallback.

Nostr expresses the trust (signed events); the DKG preserves and connects it
(provenance, shared memory, optional anchored identifiers). Everything is **Nostr-first** —
the graph indexes and preserves the social record, it never replaces it.

## Status & scope

This is an **experimental prototype**, feature-flagged and advisory-only. It
does not gate identity, membership, moderation, or writes. It is deliberately
kept separate from the minimal Web of Trust attestation primitives (kind
registration) proposed as the first, smallest upstream step — this memory
surface is the optional evidence layer that shows what those primitives make
possible.

*Relay used in test builds: connect to your community relay and open the
**Web of Trust** channel to see live data.*
