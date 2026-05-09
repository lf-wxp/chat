/**
 * Message actions: reply, revoke, forward affordances.
 *
 * Maps to: Requirement 16.9 (Message Context Menu Actions),
 *           Requirement 16.10 (Message Forward),
 *           Requirement 16.15 (Revoke with Reply Reference).
 */

import { sel } from '../utils/selectors.ts';
import {
  establishConnection,
  registerAndLogin,
  sendAndVerifyMessage,
} from '../fixtures/helpers.ts';
import { expect, test } from '../fixtures/test-base.ts';

test.describe('message actions', () => {
  test.beforeEach(async ({ pageA, pageB, server }) => {
    await registerAndLogin(pageA, server, { hint: 'a' });
    const b = await registerAndLogin(pageB, server, { hint: 'b' });
    await establishConnection(pageA, pageB, b.username);
  });

  test('reply preview bar appears after clicking reply on a peer message', async ({
    pageA,
    pageB,
  }) => {
    const tag = Date.now().toString(36);
    const text = `peer-${tag}`;
    await sendAndVerifyMessage(pageB, pageA, text);

    // Hover the message bubble on A's side and click reply.
    const row = pageA.locator(sel.messageRow, { hasText: text }).first();
    await row.hover();
    await row.locator(sel.messageActionReply).click();

    await expect(pageA.locator(sel.replyPreviewBar)).toBeVisible();
    await expect(pageA.locator(sel.replyPreviewBar)).toContainText(text);

    // Type a reply and send it.
    const reply = `reply-${tag}`;
    await pageA.locator(sel.chatInputTextarea).fill(reply);
    await pageA.locator(sel.chatInputTextarea).press('Enter');

    // Reply preview bar disappears after send.
    await expect(pageA.locator(sel.replyPreviewBar)).toBeHidden({ timeout: 10_000 });
    // Both pages render the reply text.
    await expect(pageA.locator(sel.messageRow, { hasText: reply })).toBeVisible({
      timeout: 10_000,
    });
    await expect(pageB.locator(sel.messageRow, { hasText: reply })).toBeVisible({
      timeout: 15_000,
    });
  });

  test('revoke replaces the bubble with a revoked placeholder on both sides', async ({
    pageA,
    pageB,
  }) => {
    const tag = Date.now().toString(36);
    const text = `revoke-${tag}`;
    await sendAndVerifyMessage(pageA, pageB, text);

    const row = pageA.locator(sel.messageRow, { hasText: text }).first();
    await row.hover();

    // Suppress the revoke confirmation dialog if any. Most builds open a
    // confirm modal; auto-accept JS dialogs in case it falls back to one.
    pageA.once('dialog', async (dialog) => {
      await dialog.accept();
    });
    await row.locator(sel.messageActionRevoke).click();

    // Either a confirm modal appears or the action runs directly. If a confirm
    // dialog is rendered with a "Confirm" button, click it.
    const confirmBtn = pageA.locator('button', { hasText: /Confirm|确认|OK/i }).first();
    if (await confirmBtn.isVisible().catch(() => false)) {
      await confirmBtn.click();
    }

    await expect(pageA.locator(sel.messageRevoked)).toBeVisible({ timeout: 10_000 });
    await expect(pageB.locator(sel.messageRevoked)).toBeVisible({ timeout: 15_000 });
  });

  test('forward modal opens when forward action is clicked', async ({ pageA, pageB }) => {
    const tag = Date.now().toString(36);
    const text = `fwd-src-${tag}`;
    await sendAndVerifyMessage(pageB, pageA, text);

    const row = pageA.locator(sel.messageRow, { hasText: text }).first();
    await row.hover();
    await row.locator(sel.messageActionForward).click();

    await expect(pageA.locator(sel.forwardModal)).toBeVisible();
  });
});
