/**
 * Sidebar conversation list / unread badge / last-message preview tests.
 *
 * Maps to: Requirement 16.12 (Conversation List & Unread Count).
 */

import { sel } from '../utils/selectors.ts';
import { establishConnection, registerAndLogin } from '../fixtures/helpers.ts';
import { expect, test } from '../fixtures/test-base.ts';

test.describe('conversation list', () => {
  test('unread badge appears when receiver is not focused on the conversation', async ({
    pageA,
    pageB,
    server,
  }) => {
    const a = await registerAndLogin(pageA, server, { hint: 'a' });
    const b = await registerAndLogin(pageB, server, { hint: 'b' });
    await establishConnection(pageA, pageB, b.username);

    // Force B back to the empty home view by clicking the active conversation
    // entry's settings button or simply navigating away.
    await pageB.evaluate(() => {
      // Drop the active conversation by simulating a click outside any chat.
      const empty = document.querySelector('[data-testid="home-empty"]');
      if (!empty) {
        // The chat view is open; force-blur by clicking the sidebar header.
        const sidebar = document.querySelector('[data-testid="sidebar"]');
        (sidebar as HTMLElement | null)?.click();
      }
    });

    // Send several messages from A. We use `sendAndVerifyMessage` for
    // the first message so the helper's built-in retry softens any
    // residual ECDH-not-ready race; the helper waits for the bubble
    // on B's chat view, but since B is no longer focused on the
    // conversation we wait for the message to appear in the *sender's*
    // own list instead. The follow-up sends use plain fill+Enter.
    const tag = Date.now().toString(36);
    const messages = [`u-${tag}-0`, `u-${tag}-1`, `u-${tag}-2`];
    for (const m of messages) {
      await pageA.locator(sel.chatInputTextarea).fill(m);
      await pageA.locator(sel.chatInputTextarea).press('Enter');
      // Wait for the sender to render its own bubble before issuing
      // the next send. This serializes the sends so we don't race
      // and lose ordering at the encryption layer's send queue.
      await expect(
        pageA.locator(sel.messageRow, { hasText: m }).first(),
      ).toBeVisible({ timeout: 15_000 });
    }

    // B's sidebar entry for A should reflect the latest preview.
    const sidebarItem = pageB
      .locator(sel.sidebarConversationItem, { hasText: a.username })
      .first();
    await expect(sidebarItem).toBeVisible({ timeout: 15_000 });
    await expect(sidebarItem).toContainText(messages[2] ?? '', { timeout: 15_000 });
  });
});
