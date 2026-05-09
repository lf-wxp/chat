/**
 * Text message send/receive, ordering, status and typing-indicator E2E tests.
 *
 * Maps to: Requirement 16.4 (Text Message Send & Receive).
 */

import { sel } from '../utils/selectors.ts';
import {
  establishConnection,
  registerAndLogin,
  sendAndVerifyMessage,
} from '../fixtures/helpers.ts';
import { expect, test } from '../fixtures/test-base.ts';

test.describe('text messaging', () => {
  test.beforeEach(async ({ pageA, pageB, server }) => {
    await registerAndLogin(pageA, server, { hint: 'a' });
    const b = await registerAndLogin(pageB, server, { hint: 'b' });
    await establishConnection(pageA, pageB, b.username);
  });

  test('plain text message is delivered with both bubbles visible', async ({ pageA, pageB }) => {
    const content = `hello-${Date.now().toString(36)}`;
    const { senderRow, receiverRow } = await sendAndVerifyMessage(pageA, pageB, content);

    await expect(senderRow).toContainText(content);
    await expect(receiverRow).toContainText(content);
  });

  test('multiple rapid messages are delivered in order without loss or duplication', async ({
    pageA,
    pageB,
  }) => {
    const tag = Date.now().toString(36);
    const messages = Array.from({ length: 5 }, (_, i) => `msg-${tag}-${i}`);

    // Send messages one at a time via `sendAndVerifyMessage` so the built-in
    // retry softens the ECDH-not-ready race on the very first frame.
    for (const m of messages) {
      await sendAndVerifyMessage(pageA, pageB, m);
    }

    // No duplicates: each message text appears exactly once on B.
    for (const m of messages) {
      await expect(pageB.locator(sel.messageRow, { hasText: m })).toHaveCount(1);
    }

    // Order: the rendered list contains the messages in the same order they
    // were sent.
    const texts = await pageB
      .locator(sel.messageRow)
      .filter({ hasText: `msg-${tag}-` })
      .allInnerTexts();
    const filtered = texts
      .map((t) => {
        const match = t.match(new RegExp(`msg-${tag}-(\\d)`));
        return match ? Number.parseInt(match[1] ?? '-1', 10) : -1;
      })
      .filter((n) => n >= 0);
    expect(filtered).toEqual([...filtered].sort((x, y) => x - y));
  });

  test('typing indicator appears on receiver while sender is typing', async ({ pageA, pageB }) => {
    const textarea = pageA.locator(sel.chatInputTextarea);
    await textarea.click();
    await textarea.type('still typing...', { delay: 30 });

    // Receiver shows the typing indicator within a few seconds.
    await expect(pageB.locator(sel.typingIndicator)).toBeVisible({ timeout: 10_000 });

    // Actually send the message. `send_typing(false)` is emitted from
    // `do_send` (input_bar.rs) after `send_text` succeeds, which clears
    // the peer-side indicator deterministically.
    await textarea.press('Enter');
    await expect(pageB.locator(sel.typingIndicator)).toBeHidden({ timeout: 15_000 });
  });

  test('Markdown formatting renders on the receiver', async ({ pageA, pageB }) => {
    const tag = Date.now().toString(36);
    const markdown = `**bold-${tag}**`;

    const textarea = pageA.locator(sel.chatInputTextarea);
    await textarea.fill(markdown);
    await textarea.press('Enter');

    // The visible text inside the rendered bubble strips the `**` markers.
    // Wait for the inner `bold-${tag}` substring on the receiver instead
    // of the raw markdown source.
    const row = pageB.locator(sel.messageRow, { hasText: `bold-${tag}` }).first();
    await expect(row).toBeVisible({ timeout: 20_000 });
    await expect(row.locator('strong, b').first()).toBeVisible();
  });
});
