// Fallback launch point: when the context graph has captured decisions but no
// per-participant sub-graphs (flat capture), the panel must still offer a way
// into the Traces overlay — the "All decisions" timeline lens. Provider
// responses are stubbed so the spec is deterministic and independent of the
// capture daemon's partitioning.
import { expect, test } from "@playwright/test";

import { waitForAnimations } from "../helpers/animations";
import { installMockBridge } from "../helpers/bridge";

const CHANNEL = "engineering";
const CG = "buzz-e2e-flat-capture-fixture";

const FLAT_MEMORY = {
  gate: "ok",
  cg: CG,
  layers: { WM: [], SWM: [{ graph: "g1", label: "g1" }], VM: [], SWMCount: 3 },
  decisions: [
    {
      uri: `did:dkg:context-graph:${CG}/assertion/0xabc/buzz-dkg-1`,
      name: "DECISION: adopt NIP-42 for WebSockets and NIP-98 for HTTP.",
      digest: "sha256:aaaa",
      at: "2026-08-10T12:00:00Z",
    },
    {
      uri: `did:dkg:context-graph:${CG}/assertion/0xabc/buzz-dkg-2`,
      name: "DECISION: community provider is the default read path.",
      digest: "sha256:bbbb",
      at: "2026-08-10T13:00:00Z",
    },
    {
      uri: `did:dkg:context-graph:${CG}/assertion/0xabc/buzz-dkg-3`,
      name: "DECISION: flat capture ships in beta.3.",
      digest: "sha256:cccc",
      at: "2026-08-10T14:00:00Z",
    },
  ],
  contributors: [],
  subgraphs: [], // ← the condition under test: no per-participant sub-graphs
};

async function waitForMockLiveSubscription(
  page: import("@playwright/test").Page,
  channelName: string,
) {
  await expect
    .poll(() =>
      page.evaluate(
        ({ channelName }) =>
          window.__BUZZ_E2E_HAS_MOCK_LIVE_SUBSCRIPTION__?.({ channelName }) ??
          false,
        { channelName },
      ),
    )
    .toBe(true);
}

test("flat capture: All-decisions lens opens the Traces timeline", async ({
  page,
}) => {
  await page.addInitScript((cg) => {
    window.localStorage.setItem("dkg-memory-cg-override", cg);
  }, CG);
  // Stub the local provider: memory has decisions but zero sub-graphs.
  await page.route("http://127.0.0.1:9295/**", (route) => {
    const url = route.request().url();
    if (url.includes("/api/channel-memory")) {
      return route.fulfill({ json: FLAT_MEMORY });
    }
    return route.fulfill({ json: { gate: "ok" } });
  });
  await installMockBridge(page);
  await page.goto("/", { waitUntil: "domcontentloaded" });
  await page.getByTestId("channel-engineering").click();
  await waitForMockLiveSubscription(page, CHANNEL);
  await page.getByTestId("dkg-memory-toggle").click();
  const panel = page.getByTestId("dkg-memory-panel");
  await expect(panel.getByText(/what this channel remembers/i)).toBeVisible({
    timeout: 20_000,
  });

  // No sub-graphs → the fallback chip renders (and no topic chips exist).
  const lens = page.getByTestId("dkg-subgraph-all-decisions");
  await expect(lens).toBeVisible();
  await lens.click();

  const overlay = page.getByTestId("dkg-graph-overlay");
  await expect(overlay).toBeVisible();
  await expect(overlay.getByText("All decisions")).toBeVisible();
  // All three decisions appear as Traces cards, full titles readable.
  await expect(overlay.getByTestId("traces-card")).toHaveCount(3, {
    timeout: 15_000,
  });
  await expect(
    overlay
      .getByTestId("traces-card")
      .filter({ hasText: "adopt NIP-42 for WebSockets" })
      .first(),
  ).toBeVisible();
  // Fallback lens is Traces-only: the hexagonal Graph toggle is absent.
  await expect(overlay.getByTestId("dkg-topology-toggle")).toHaveCount(0);
  await waitForAnimations(page);
});
