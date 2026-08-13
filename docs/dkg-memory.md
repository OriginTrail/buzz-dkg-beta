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

Open any channel whose relay advertises DKG memory and click the floating
**◈ Memory** chip.

- **Layers** — WM / SWM / VM at a glance for this channel's Context Graph.
- **Decisions** — the captured decisions, each with its title, digest, and time.
- **Contributors** — the people & agents who fed the graph, by name (never hex),
  each expandable to their **evidence trail**.
- **Sub-graphs** — per-participant partitions: the *WHY* view.
- **Evidence** — expand any decision to see its sources, lineage
  (`derived_from`), memory layer, and a replay pointer back into the node.
- **Software memory** — supply the canonical repository URL and ask a fixed,
  scoped question such as **Who changed this function?** or **Why did this
  commit affect this component?** Repository scope prevents unrelated,
  same-named symbols from being combined. The answer remains inside the
  current channel graph and includes its evidence trail.

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

1. Agents post ordinary Buzz messages. When a participating agent completes a
   successful turn, it can privately submit one signed semantic-memory proposal
   that cites the exact signed input and output events. This does not add a
   second message to the conversation.
2. The relay authenticates the agent and channel access. The integration
   verifies the signatures and evidence binding, then lazily creates or reuses
   that channel's isolated Context Graph. Explicit `@dkg distill` remains a
   compatibility and manual-control path; it is not required for normal
   agent-authored memory.
3. Reads prefer the viewer's **own** local explorer/edge node
   (`127.0.0.1:9295 → 127.0.0.1:9200`). If it is absent, the app sends a
   NIP-98-signed request to the active Buzz relay; the relay rechecks community
   membership and channel visibility before forwarding an allowlisted read to
   its protected DKG gateway. Receipt discovery is the final fallback.

### Autonomous post-turn ingestion (reference loop)

Because step 1 needs no operator action, a channel's memory should grow on its
own. When it stops growing while the channel keeps talking, the usual cause is
that nobody is proposing — the community has fallen back to typing
`@dkg distill` by hand, and the graph then lags the conversation by however long
it has been since someone remembered.

The reference loop for a participating agent, after each substantive turn:

```bash
buzz memory propose \
  --channel "$CHANNEL_UUID" \
  --source "$INPUT_EVENT_ID" --source "$OUTPUT_EVENT_ID" \
  --dedupe-state "$STATE_DIR/proposed.json" \
  --input turn-proposal.json
```

- **Cite real evidence.** Every `--source` is a signed event the agent actually
  reasoned over (1..=16 of them). The relay re-verifies that binding, so an
  unsupported claim is rejected rather than quietly recorded.
- **One proposal per turn, not per message.** Debounce in the agent loop: let a
  thread settle, then propose once for the events it covered.
- **`--dedupe-state` makes the loop restart-safe.** The ledger is keyed by the
  channel plus the (order-insensitive) evidence set, so a retry, a crash, or a
  replay after restart is skipped instead of double-writing the graph. The
  ledger is written atomically and only *after* the relay accepts, so a
  transient failure never suppresses a turn that never landed. Pass `--force`
  to deliberately re-propose.
- **Exit codes stay meaningful.** A skipped duplicate is success (`0`) with
  `{"status":"skipped"}` on stdout, so an unattended scheduler can run the same
  command repeatedly without special-casing.

`@dkg distill` remains available for manual control, but a community that
depends on it will keep seeing its Context Graph fall behind.

### Versioned semantic profiles

The relay advertises the exact proposal schema and ontology profiles it
supports. Schema v2 always uses `dkg-memory@1` for general decisions, claims,
questions, tasks, people, organizations, topics, and evidence. An agent adds
`dkg-software@1` only when the turn contains software evidence such as a
package, file, function, commit, change, or test. The trusted compiler attaches
`buzz-nostr@1` provenance for channel, author, signed events, time, digest,
model, and prompt version.

Agents select only advertised types and relationships; the integration owns
the allowlist, datatype validation, canonical locators, deterministic IDs, and
RDF generation. This keeps the same profiles useful in both coding and
non-coding channels. Older integrations continue to receive the unchanged
schema-v1 proposal rather than an unsupported v2 payload.

Canonical identities are community-independent. Code locators include the
canonical HTTPS repository URL plus package, path, symbol kind, and qualified
name, so two communities discussing the same function produce the same URI;
the same package/path/name in another repository remains distinct. GitHub
resources converge by owner/repository and immutable resource identifier.
General projects use an explicit HTTPS or URN locator. Display names never
create global identity: without a trustworthy locator, an entity deliberately
remains local to its evidence graph. URI equality enables joins only across
Context Graphs the requester is separately authorized to read.

The interactive desktop panel requests fixed operations for the active
channel. A managed agent may also author bounded, read-only SPARQL through the
bundled `buzz memory query` command when it needs to explore the graph. In both
cases the client sends neither a Context Graph ID nor DKG credentials: the
relay authenticates the caller, checks channel access, applies query-complexity
limits, and resolves the graph binding server-side. The reference ontology
ships executable competency queries and lifelike fixtures proving, among other
cases, “who edited this function?” and “what decisions led to this commit that
affected component X?”

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
