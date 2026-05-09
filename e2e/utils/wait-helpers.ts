/**
 * Custom waiting utilities used across the E2E suite.
 *
 * These wrappers wait on application-meaningful conditions (DataChannel
 * established, online user appeared, message status flipped) rather than on
 * arbitrary timeouts. They build on Playwright's polling primitives.
 */

import { type Locator, type Page, expect } from '@playwright/test';

import { sel } from './selectors.ts';

/** Wait until the chat view (post-connection main shell) is visible. */
export async function waitForChatView(page: Page, timeoutMs = 20_000): Promise<void> {
  await page.locator(sel.chatView).waitFor({ state: 'visible', timeout: timeoutMs });
}

/** Wait until the main app shell is visible (sidebar after login). */
export async function waitForAppShell(page: Page, timeoutMs = 15_000): Promise<void> {
  await page.locator(sel.sidebar).waitFor({ state: 'visible', timeout: timeoutMs });
}

/** Wait until a user with the given username appears in the online users panel. */
export async function waitForOnlineUser(
  page: Page,
  username: string,
  timeoutMs = 30_000,
): Promise<Locator> {
  // First make sure the WebSocket is connected so the panel can be populated.
  await page
    .locator(`${sel.sidebarConnectionStatus}.sidebar-connection-status--connected`)
    .waitFor({ state: 'visible', timeout: Math.min(timeoutMs, 20_000) });

  const row = page.locator(sel.onlineUserRow, { hasText: username }).first();
  await row.waitFor({ state: 'visible', timeout: timeoutMs });
  return row;
}

/** Wait until any message bubble whose visible text contains `content` is rendered. */
export async function waitForMessageWithText(
  page: Page,
  content: string,
  timeoutMs = 15_000,
): Promise<Locator> {
  const row = page.locator(sel.messageRow, { hasText: content }).first();
  await row.waitFor({ state: 'visible', timeout: timeoutMs });
  return row;
}

/**
 * Poll until `expect(...)` resolves or the budget is exhausted.
 *
 * Useful for assertions that depend on async background work (signaling round
 * trips, IndexedDB persistence) where Playwright's auto-waiting locators are
 * not enough.
 */
export async function pollExpect(
  fn: () => Promise<void>,
  { timeoutMs = 10_000, intervalMs = 250 }: { timeoutMs?: number; intervalMs?: number } = {},
): Promise<void> {
  const deadline = Date.now() + timeoutMs;
  let lastError: unknown;
  while (Date.now() < deadline) {
    try {
      await fn();
      return;
    } catch (err) {
      lastError = err;
      await new Promise((r) => setTimeout(r, intervalMs));
    }
  }
  throw lastError instanceof Error ? lastError : new Error('pollExpect timed out');
}

/** Wait until at least one DataChannel reports `readyState === 'open'` in the page. */
export async function waitForOpenDataChannel(page: Page, timeoutMs = 20_000): Promise<void> {
  await expect
    .poll(
      async () =>
        page.evaluate(() => {
          // The application stores active peer connections on `window` for
          // debug introspection. If unavailable, fall back to checking the
          // chat view visibility (DataChannel open is a precondition for
          // it to render).
          const w = window as unknown as { __peers?: Map<unknown, RTCPeerConnection> };
          if (!w.__peers) {
            return document.querySelector('[data-testid="chat-view"]') !== null;
          }
          for (const pc of w.__peers.values()) {
            // Chrome exposes `getDataChannels()` as non-standard; fall back
            // to inspecting connection state.
            if (pc.connectionState === 'connected') {
              return true;
            }
          }
          return false;
        }),
      { timeout: timeoutMs, intervals: [250, 500, 1_000] },
    )
    .toBe(true);
}
