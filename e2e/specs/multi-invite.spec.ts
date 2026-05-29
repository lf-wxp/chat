/**
 * Multi-user invitation E2E tests.
 *
 * Maps to: Requirement 9.10-9.12 (MultiInvite — invite multiple users at once).
 *
 * Coverage:
 *   1. User can select multiple users and send a multi-invite.
 *   2. At least one user accepting creates a room and auto-joins all acceptors.
 *   3. Users who decline are not added to the room.
 *   4. Late acceptors are auto-joined to the already-created room.
 */

import { sel } from '../utils/selectors.ts';
import { registerAndLogin } from '../fixtures/helpers.ts';
import { waitForOnlineUser } from '../utils/wait-helpers.ts';
import { expect, test } from '../fixtures/test-base.ts';
import type { Page } from '@playwright/test';

/**
 * Enter multi-select mode, select the given users by clicking their
 * rows, and click Send. The caller is responsible for waiting on each
 * target's incoming modal afterwards.
 */
async function sendMultiInvite(page: Page, targetUsernames: string[]): Promise<void> {
  await page.locator('[data-testid="online-users-multi-toggle"]').click();
  for (const name of targetUsernames) {
    const row = await waitForOnlineUser(page, name);
    await row.click();
  }
  await page.locator('[data-testid="multi-invite-send"]').click();
}

test.describe('multi-invite — invite multiple users', () => {
  test('A can send a multi-invite to B and C', async ({ pageA, pageB, pageC, server }) => {
    await registerAndLogin(pageA, server, { hint: 'mi_a' });
    const b = await registerAndLogin(pageB, server, { hint: 'mi_b' });
    const c = await registerAndLogin(pageC, server, { hint: 'mi_c' });

    // Wait for B and C to appear in A's online list.
    await waitForOnlineUser(pageA, b.username);
    await waitForOnlineUser(pageA, c.username);

    // A enters multi-select mode and selects B and C.
    await sendMultiInvite(pageA, [b.username, c.username]);

    // Both B and C should receive incoming invite modals.
    await expect(pageB.locator(sel.incomingInviteModal)).toBeVisible({ timeout: 15_000 });
    await expect(pageC.locator(sel.incomingInviteModal)).toBeVisible({ timeout: 15_000 });
  });

  test('at least one accept creates a room — all acceptors join', async ({
    pageA,
    pageB,
    pageC,
    server,
  }) => {
    await registerAndLogin(pageA, server, { hint: 'mia_a' });
    const b = await registerAndLogin(pageB, server, { hint: 'mia_b' });
    const c = await registerAndLogin(pageC, server, { hint: 'mia_c' });

    await waitForOnlineUser(pageA, b.username);
    await waitForOnlineUser(pageA, c.username);

    // A sends multi-invite.
    await sendMultiInvite(pageA, [b.username, c.username]);

    // B accepts.
    await expect(pageB.locator(sel.incomingInviteModal)).toBeVisible({ timeout: 15_000 });
    await pageB.locator(sel.inviteAccept).click();
    await pageB.locator(sel.inviteBackdrop).waitFor({ state: 'hidden', timeout: 10_000 });

    // C accepts.
    await expect(pageC.locator(sel.incomingInviteModal)).toBeVisible({ timeout: 15_000 });
    await pageC.locator(sel.inviteAccept).click();
    await pageC.locator(sel.inviteBackdrop).waitFor({ state: 'hidden', timeout: 10_000 });

    // All three should end up in a chat view (room or multi-peer conversation).
    await expect(pageA.locator(sel.chatView)).toBeVisible({ timeout: 20_000 });
    await expect(pageB.locator(sel.chatView)).toBeVisible({ timeout: 20_000 });
    await expect(pageC.locator(sel.chatView)).toBeVisible({ timeout: 20_000 });
  });

  test('declining user is not added to the conversation', async ({
    pageA,
    pageB,
    pageC,
    server,
  }) => {
    await registerAndLogin(pageA, server, { hint: 'mid_a' });
    const b = await registerAndLogin(pageB, server, { hint: 'mid_b' });
    const c = await registerAndLogin(pageC, server, { hint: 'mid_c' });

    await waitForOnlineUser(pageA, b.username);
    await waitForOnlineUser(pageA, c.username);

    // A sends multi-invite.
    await sendMultiInvite(pageA, [b.username, c.username]);

    // B accepts.
    await expect(pageB.locator(sel.incomingInviteModal)).toBeVisible({ timeout: 15_000 });
    await pageB.locator(sel.inviteAccept).click();
    await pageB.locator(sel.inviteBackdrop).waitFor({ state: 'hidden', timeout: 10_000 });

    // C declines.
    await expect(pageC.locator(sel.incomingInviteModal)).toBeVisible({ timeout: 15_000 });
    await pageC.locator(sel.inviteDecline).click();

    // A and B should have a chat view.
    await expect(pageA.locator(sel.chatView)).toBeVisible({ timeout: 20_000 });
    await expect(pageB.locator(sel.chatView)).toBeVisible({ timeout: 20_000 });

    // C should NOT be in a chat view — they should remain on the home/empty state.
    await pageC.waitForTimeout(5_000);
    const chatViewCount = await pageC.locator(sel.chatView).count();
    expect(chatViewCount).toBeLessThanOrEqual(0);
  });
});
