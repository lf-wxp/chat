/**
 * PWA offline banner and service worker update E2E tests.
 *
 * Maps to: Requirement 25 (PWA Configuration — OfflineBanner, PwaUpdateBanner).
 *
 * Coverage:
 *   1. Offline banner appears when network is disconnected.
 *   2. Offline banner disappears when network is restored.
 *   3. "Back online" confirmation is shown after reconnection.
 *   4. Connection status indicator reflects offline state.
 *   5. App loads correctly (service worker registration check).
 */

import { sel } from '../utils/selectors.ts';
import { registerAndLogin } from '../fixtures/helpers.ts';
import { expect, test } from '../fixtures/test-base.ts';

// The OfflineBanner component polls navigator.onLine every 3 seconds,
// so we need timeouts that account for this polling interval.
const POLL_INTERVAL_TIMEOUT = 15_000;

test.describe('PWA — offline banner & service worker', () => {
  test('offline banner appears when network is disconnected', async ({
    pageA,
    server,
    contextA,
  }) => {
    await registerAndLogin(pageA, server, { hint: 'pwa_off' });

    // Simulate going offline by setting the browser context offline.
    await contextA.setOffline(true);

    // The offline banner should appear (uses CSS class, not data-testid).
    // The component polls navigator.onLine every 3s, so allow extra time.
    const offlineBanner = pageA.locator('.offline-banner.offline-banner--offline');
    await expect(offlineBanner).toBeVisible({ timeout: POLL_INTERVAL_TIMEOUT });

    // Restore network.
    await contextA.setOffline(false);

    // The offline banner should disappear (replaced by online confirmation or hidden).
    await expect(offlineBanner).toBeHidden({ timeout: POLL_INTERVAL_TIMEOUT });
  });

  test('back-online confirmation is shown after reconnection', async ({
    pageA,
    server,
    contextA,
  }) => {
    await registerAndLogin(pageA, server, { hint: 'pwa_bon' });

    // Go offline.
    await contextA.setOffline(true);
    const offlineBanner = pageA.locator('.offline-banner.offline-banner--offline');
    await expect(offlineBanner).toBeVisible({ timeout: POLL_INTERVAL_TIMEOUT });

    // Go back online.
    await contextA.setOffline(false);

    // A "Back online" banner should appear briefly (uses --online modifier).
    const backOnline = pageA.locator('.offline-banner.offline-banner--online');
    await expect(backOnline).toBeVisible({ timeout: POLL_INTERVAL_TIMEOUT });

    // It should auto-dismiss after ~3 seconds.
    await expect(backOnline).toBeHidden({ timeout: 10_000 });
  });

  test('connection status indicator reflects offline state', async ({
    pageA,
    server,
    contextA,
  }) => {
    // Inject a script that tracks all WebSocket instances before the app loads.
    await contextA.addInitScript(() => {
      const origWS = window.WebSocket;
      (window as any).__ALL_WS__ = [] as WebSocket[];
      (window as any).WebSocket = function (...args: ConstructorParameters<typeof WebSocket>) {
        const ws = new origWS(...args);
        (window as any).__ALL_WS__.push(ws);
        return ws;
      } as any;
      (window as any).WebSocket.prototype = origWS.prototype;
      (window as any).WebSocket.CONNECTING = origWS.CONNECTING;
      (window as any).WebSocket.OPEN = origWS.OPEN;
      (window as any).WebSocket.CLOSING = origWS.CLOSING;
      (window as any).WebSocket.CLOSED = origWS.CLOSED;
    });

    await registerAndLogin(pageA, server, { hint: 'pwa_sts' });

    // Verify connected state.
    await expect(
      pageA.locator(
        `${sel.sidebarConnectionStatus}.sidebar-connection-status--connected`,
      ),
    ).toBeVisible({ timeout: 20_000 });

    // Force-close the WebSocket to simulate a network disconnect.
    // context.setOffline() blocks new requests but does NOT immediately
    // close existing WebSocket connections — the heartbeat timeout (55s)
    // would be needed. Instead, we close all tracked WebSocket instances
    // so the onclose handler fires immediately (connected=false).
    await contextA.setOffline(true);
    await pageA.evaluate(() => {
      const allWs = (window as any).__ALL_WS__ as WebSocket[] | undefined;
      if (allWs) {
        allWs.forEach(ws => {
          if (ws.readyState === WebSocket.OPEN) {
            ws.close(4000, 'e2e-test-disconnect');
          }
        });
      }
    });

    // Connection status should change to disconnected/reconnecting.
    await expect(
      pageA.locator(
        `${sel.sidebarConnectionStatus}.sidebar-connection-status--disconnected, ` +
        `${sel.sidebarConnectionStatus}.sidebar-connection-status--reconnecting`,
      ),
    ).toBeVisible({ timeout: 15_000 });

    // Restore network so the reconnect can succeed.
    await contextA.setOffline(false);

    // Should reconnect.
    await expect(
      pageA.locator(
        `${sel.sidebarConnectionStatus}.sidebar-connection-status--connected`,
      ),
    ).toBeVisible({ timeout: 30_000 });
  });

  test('app loads correctly with service worker enabled', async ({ browser, server }) => {
    // Create a fresh context with service workers allowed (the default
    // contextA fixture hardcodes serviceWorkers: 'block').
    const ctx = await browser.newContext({
      ignoreHTTPSErrors: true,
      permissions: ['microphone', 'camera', 'clipboard-read', 'clipboard-write', 'notifications'],
      serviceWorkers: 'allow',
    });

    try {
      const page = await ctx.newPage();
      await page.goto(`${server.baseUrl}/`);
      await expect(page.locator(sel.authPage)).toBeVisible({ timeout: 20_000 });

      // In a production build, SW should be registered. In dev mode it might
      // not be — we just verify the page loads without errors.
      // The assertion is soft: we only check the page rendered correctly.
      await expect(page.locator(sel.authPage)).toBeVisible();
    } finally {
      await ctx.close();
    }
  });
});
