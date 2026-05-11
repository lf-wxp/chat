/**
 * Emoji reaction E2E tests.
 *
 * Maps to: Requirement 16.11 (Message Reaction).
 *
 * Covers:
 *   1. Picker opens on click (the original baseline test).
 *   2. Adding a reaction on the sender shows a chip with count 1 on
 *      BOTH sender and receiver.
 *   3. Re-clicking the same emoji toggles it off — the chip
 *      disappears on both sides.
 *   4. Two different users react with the same emoji → one chip with
 *      count 2, aria-pressed reflects "me reacted" per side.
 *   5. Two different emojis by the same user → two chips, each
 *      count 1, rendered in insertion order.
 */

import { sel } from '../utils/selectors.ts';
import {
  establishConnection,
  registerAndLogin,
  sendAndVerifyMessage,
} from '../fixtures/helpers.ts';
import { expect, test, type Page } from '../fixtures/test-base.ts';
import type { Locator } from '@playwright/test';

/**
 * Click the react action on a message row and wait for the picker
 * to appear. Returns the picker locator scoped to the bubble.
 */
async function openReactionPicker(page: Page, row: Locator): Promise<Locator> {
  await row.hover();
  await row.locator(sel.messageActionReact).click();
  const picker = page.locator(sel.reactionPicker);
  await expect(picker).toBeVisible();
  return picker;
}

/**
 * Pick a specific emoji from an already-open picker. Uses the
 * `data-emoji` attribute instead of hasText because emoji often
 * span multiple code points and `hasText` matching is unreliable.
 */
async function pickEmoji(page: Page, emoji: string): Promise<void> {
  await page
    .locator(`${sel.reactionPickerEmoji}[data-emoji="${emoji}"]`)
    .first()
    .click();
}

/**
 * Convenience: open picker + pick emoji in one shot.
 */
async function toggleReaction(page: Page, row: Locator, emoji: string): Promise<void> {
  await openReactionPicker(page, row);
  await pickEmoji(page, emoji);
}

/**
 * Scoped locator for a reaction chip on the given message row.
 */
function chipFor(row: Locator, emoji: string): Locator {
  return row.locator(`${sel.reactionChip}[data-emoji="${emoji}"]`).first();
}

test.describe('reactions', () => {
  test('clicking the react action opens the emoji picker', async ({
    pageA,
    pageB,
    server,
  }) => {
    await registerAndLogin(pageA, server, { hint: 'a' });
    const b = await registerAndLogin(pageB, server, { hint: 'b' });
    await establishConnection(pageA, pageB, b.username);

    const tag = Date.now().toString(36);
    const text = `rx-${tag}`;
    await sendAndVerifyMessage(pageB, pageA, text);

    const row = pageA.locator(sel.messageRow, { hasText: text }).first();
    await openReactionPicker(pageA, row);
  });

  test('adding a reaction is visible on both sender and receiver', async ({
    pageA,
    pageB,
    server,
  }) => {
    await registerAndLogin(pageA, server, { hint: 'a' });
    const b = await registerAndLogin(pageB, server, { hint: 'b' });
    await establishConnection(pageA, pageB, b.username);

    // B sends → A reacts with 👍.
    const tag = Date.now().toString(36);
    const text = `rx-add-${tag}`;
    const { senderRow, receiverRow } = await sendAndVerifyMessage(pageB, pageA, text);

    await toggleReaction(pageA, receiverRow, '👍');

    // Chip appears on A with count=1, aria-pressed=true (A is me).
    await expect(chipFor(receiverRow, '👍')).toHaveAttribute('data-count', '1');
    await expect(chipFor(receiverRow, '👍')).toHaveAttribute('aria-pressed', 'true');

    // And propagates to B, with aria-pressed=false (B didn't react).
    await expect(chipFor(senderRow, '👍')).toHaveAttribute('data-count', '1', {
      timeout: 10_000,
    });
    await expect(chipFor(senderRow, '👍')).toHaveAttribute('aria-pressed', 'false');
  });

  test('re-clicking the same emoji removes the reaction on both sides', async ({
    pageA,
    pageB,
    server,
  }) => {
    await registerAndLogin(pageA, server, { hint: 'a' });
    const b = await registerAndLogin(pageB, server, { hint: 'b' });
    await establishConnection(pageA, pageB, b.username);

    const tag = Date.now().toString(36);
    const text = `rx-toggle-${tag}`;
    const { senderRow, receiverRow } = await sendAndVerifyMessage(pageB, pageA, text);

    // Add, confirm.
    await toggleReaction(pageA, receiverRow, '❤️');
    await expect(chipFor(receiverRow, '❤️')).toHaveAttribute('data-count', '1');
    await expect(chipFor(senderRow, '❤️')).toHaveAttribute('data-count', '1', {
      timeout: 10_000,
    });

    // Remove: click the chip itself (toggle via the rendered chip, not
    // via the picker, to exercise the chip's own on:click).
    await chipFor(receiverRow, '❤️').click();

    // Chip disappears on both sides.
    await expect(chipFor(receiverRow, '❤️')).toHaveCount(0, { timeout: 10_000 });
    await expect(chipFor(senderRow, '❤️')).toHaveCount(0, { timeout: 10_000 });
  });

  test('same emoji from two users aggregates to count=2', async ({
    pageA,
    pageB,
    server,
  }) => {
    await registerAndLogin(pageA, server, { hint: 'a' });
    const b = await registerAndLogin(pageB, server, { hint: 'b' });
    await establishConnection(pageA, pageB, b.username);

    // Send a message so BOTH sides have the same row to react against.
    const tag = Date.now().toString(36);
    const text = `rx-agg-${tag}`;
    const { senderRow, receiverRow } = await sendAndVerifyMessage(pageB, pageA, text);
    // Here `senderRow` is B's bubble (outgoing), `receiverRow` is A's.

    // A reacts 🎉.
    await toggleReaction(pageA, receiverRow, '🎉');
    await expect(chipFor(receiverRow, '🎉')).toHaveAttribute('data-count', '1');

    // B reacts 🎉 on its own outgoing bubble.
    await toggleReaction(pageB, senderRow, '🎉');

    // Both sides converge to count=2.
    await expect(chipFor(senderRow, '🎉')).toHaveAttribute('data-count', '2', {
      timeout: 10_000,
    });
    await expect(chipFor(receiverRow, '🎉')).toHaveAttribute('data-count', '2', {
      timeout: 10_000,
    });

    // Each side sees aria-pressed=true for itself only.
    await expect(chipFor(senderRow, '🎉')).toHaveAttribute('aria-pressed', 'true');
    await expect(chipFor(receiverRow, '🎉')).toHaveAttribute('aria-pressed', 'true');
  });

  test('two different emojis render as two distinct chips', async ({
    pageA,
    pageB,
    server,
  }) => {
    await registerAndLogin(pageA, server, { hint: 'a' });
    const b = await registerAndLogin(pageB, server, { hint: 'b' });
    await establishConnection(pageA, pageB, b.username);

    const tag = Date.now().toString(36);
    const text = `rx-multi-${tag}`;
    const { senderRow, receiverRow } = await sendAndVerifyMessage(pageB, pageA, text);

    // A reacts with two different emojis in sequence.
    await toggleReaction(pageA, receiverRow, '🔥');
    await toggleReaction(pageA, receiverRow, '🚀');

    // A sees both chips with count=1.
    await expect(chipFor(receiverRow, '🔥')).toHaveAttribute('data-count', '1');
    await expect(chipFor(receiverRow, '🚀')).toHaveAttribute('data-count', '1');
    await expect(receiverRow.locator(sel.reactionChip)).toHaveCount(2);

    // B converges too.
    await expect(chipFor(senderRow, '🔥')).toHaveAttribute('data-count', '1', {
      timeout: 10_000,
    });
    await expect(chipFor(senderRow, '🚀')).toHaveAttribute('data-count', '1', {
      timeout: 10_000,
    });
    await expect(senderRow.locator(sel.reactionChip)).toHaveCount(2);
  });
});
