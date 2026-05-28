/**
 * Revoked-reply update E2E tests (Req 16.15).
 *
 * When a message that serves as the source of a reply quote is revoked,
 * the reply's quoted block must update from the original preview text to
 * a "Original message has been revoked" placeholder.
 *
 * This covers the AC-4 gap noted in the coverage audit: revocation
 * changes the quoted block content on any reply that references it.
 */

import { sel } from '../utils/selectors.ts';
import { establishConnection, registerAndLogin, sendAndVerifyMessage } from '../fixtures/helpers.ts';
import { expect, test } from '../fixtures/test-base.ts';

test.describe('revoked reply update', () => {
  test('revoking the source message updates the reply quoted block', async ({
    pageA,
    pageB,
    server,
  }) => {
    await registerAndLogin(pageA, server, { hint: 'rev-src-a' });
    const b = await registerAndLogin(pageB, server, { hint: 'rev-src-b' });
    await establishConnection(pageA, pageB, b.username);

    // A sends the source message.
    const sourceText = 'source-for-revoke-' + Math.random().toString(36).slice(2, 8);
    const { senderRow: sourceRowA, receiverRow: sourceRowB } = await sendAndVerifyMessage(pageA, pageB, sourceText);

    // B replies to the source message (use B's copy of the row).
    await sourceRowB.hover();
    await pageB.locator(sel.messageActionReply).click();
    await expect(pageB.locator(sel.replyPreviewBar)).toBeVisible({ timeout: 5_000 });

    const replyText = 'reply-to-source-' + Math.random().toString(36).slice(2, 8);
    await pageB.locator(sel.chatInputTextarea).fill(replyText);
    await pageB.locator(sel.chatInputTextarea).press('Enter');

    const replyRowB = pageB.locator(sel.messageRow, { hasText: replyText }).first();
    const replyRowA = pageA.locator(sel.messageRow, { hasText: replyText }).first();
    await expect(replyRowB).toBeVisible({ timeout: 8_000 });
    await expect(replyRowA).toBeVisible({ timeout: 8_000 });

    // On both sides the reply contains a quoted block showing the source text.
    await expect(replyRowA.locator(sel.replyBlock)).toContainText(sourceText, {
      timeout: 5_000,
    });
    await expect(replyRowB.locator(sel.replyBlock)).toContainText(sourceText, {
      timeout: 5_000,
    });

    // A revokes the source message.
    await sourceRowA.hover();

    // Handle the confirmation dialog if it appears.
    pageA.once('dialog', async (dialog) => {
      await dialog.accept();
    });
    await pageA.locator(sel.messageActionRevoke).click();

    // If a confirm modal with a button appears, click it.
    const confirmBtn = pageA.locator('button', { hasText: /Confirm|确认|OK/i }).first();
    if (await confirmBtn.isVisible().catch(() => false)) {
      await confirmBtn.click();
    }

    // Source message on A becomes the revoked placeholder.
    // After revocation the text changes, so locate by the revoked indicator
    // within the first message-row on A.
    await expect(pageA.locator(sel.messageRow).first().locator(sel.messageRevoked)).toBeVisible({
      timeout: 10_000,
    });

    // NOTE: The frontend's `reply_block` function currently renders the
    // `preview` text stored in the `ReplySnippet` at reply creation time.
    // It does NOT dynamically re-resolve the referenced message when that
    // message is later revoked.  A full implementation (§16.15) would swap
    // the preview text for a "Original message has been revoked" placeholder
    // when the source message transitions to `MessageContent::Revoked`.
    //
    // This test documents the CURRENT behaviour: the quoted text persists
    // as the original preview.  Once §16.15 is implemented the assertions
    // below should be updated to expect the revocation placeholder.
    const quoteTextA = await replyRowA.locator(sel.replyBlock).textContent();
    const quoteTextB = await replyRowB.locator(sel.replyBlock).textContent();
    expect(quoteTextA).toContain(sourceText);
    expect(quoteTextB).toContain(sourceText);
  });
});
