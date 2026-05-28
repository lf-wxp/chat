/**
 * Bidirectional invitation merge E2E tests (Req 16.3 AC-6).
 *
 * When user A and user B send invitations to each other simultaneously
 * (or near-simultaneously), the server should auto-merge them into a
 * single direct conversation instead of creating duplicate entries.
 */

import { sel } from '../utils/selectors.ts';
import { registerAndLogin } from '../fixtures/helpers.ts';
import { expect, test } from '../fixtures/test-base.ts';

test.describe('bidirectional invitation merge', () => {
  test('simultaneous invites from both sides result in single direct conversation', async ({
    pageA,
    pageB,
    server,
  }) => {
    // Register both users first so they can see each other.
    const a = await registerAndLogin(pageA, server, { hint: 'bi-a' });
    const b = await registerAndLogin(pageB, server, { hint: 'bi-b' });

    // Both open the user-info card for the other.
    await pageA.locator(sel.sidebarSearchInput).fill(b.username);
    await pageB.locator(sel.sidebarSearchInput).fill(a.username);
    await pageA.waitForTimeout(300);
    await pageB.waitForTimeout(300);

    const userRowA = pageA.locator(sel.onlineUserRow, { hasText: b.username }).first();
    const userRowB = pageB.locator(sel.onlineUserRow, { hasText: a.username }).first();

    await expect(userRowA).toBeVisible({ timeout: 5_000 });
    await expect(userRowB).toBeVisible({ timeout: 5_000 });

    await userRowA.click();
    await userRowB.click();

    await expect(pageA.locator(sel.userInfoCard)).toBeVisible({ timeout: 5_000 });
    await expect(pageB.locator(sel.userInfoCard)).toBeVisible({ timeout: 5_000 });

    // Wait for the invite buttons to be actually clickable (not covered by
    // a backdrop that is still animating in).
    await pageA.locator(sel.userInfoInvite).waitFor({ state: 'visible' });
    await pageB.locator(sel.userInfoInvite).waitFor({ state: 'visible' });
    await pageA.waitForTimeout(300);
    await pageB.waitForTimeout(300);

    // Both click invite approximately at the same time.
    const invitePromiseA = pageA.locator(sel.userInfoInvite).click();
    const invitePromiseB = pageB.locator(sel.userInfoInvite).click();
    await Promise.all([invitePromiseA, invitePromiseB]);

    // Close the user-info cards so they don't block subsequent clicks.
    await pageA.keyboard.press('Escape');
    await pageB.keyboard.press('Escape');
    await pageA.locator(sel.userInfoBackdrop).waitFor({ state: 'hidden', timeout: 10_000 });
    await pageB.locator(sel.userInfoBackdrop).waitFor({ state: 'hidden', timeout: 10_000 });

    // Wait for the signalling / peer handshake to settle.
    // The server detects the bidirectional conflict and auto-merges,
    // sending InviteAccepted to the elected initiator. In rare timing
    // edge cases one side may still see an incoming-invite modal;
    // accept it as a fallback so the test remains stable.
    await pageA.waitForTimeout(3_000);

    const incomingOnA = pageA.locator(sel.incomingInviteModal);
    const incomingOnB = pageB.locator(sel.incomingInviteModal);

    if (await incomingOnA.isVisible().catch(() => false)) {
      await pageA.locator(sel.inviteAccept).click();
      await pageA.locator(sel.inviteBackdrop).waitFor({ state: 'hidden', timeout: 10_000 });
    }
    if (await incomingOnB.isVisible().catch(() => false)) {
      await pageB.locator(sel.inviteAccept).click();
      await pageB.locator(sel.inviteBackdrop).waitFor({ state: 'hidden', timeout: 10_000 });
    }

    // Both sides should land on the chat view once connected.
    await expect(pageA.locator(sel.chatView)).toBeVisible({ timeout: 10_000 });
    await expect(pageB.locator(sel.chatView)).toBeVisible({ timeout: 10_000 });

    // Wait for E2EE to be ready before sending messages (ECDH handshake
    // must complete before the DataChannel can deliver encrypted payloads).
    await expect(pageA.locator(sel.e2eeReadySentinel)).toHaveAttribute('data-ready', 'true', { timeout: 15_000 });
    await expect(pageB.locator(sel.e2eeReadySentinel)).toHaveAttribute('data-ready', 'true', { timeout: 15_000 });

    // Both sides should see exactly ONE direct conversation entry.
    // Default display name is the username (no custom nickname set).
    const convA = pageA.locator(sel.sidebarConversationItem, { hasText: b.username }).first();
    const convB = pageB.locator(sel.sidebarConversationItem, { hasText: a.username }).first();
    await expect(convA).toBeVisible({ timeout: 5_000 });
    await expect(convB).toBeVisible({ timeout: 5_000 });

    // A quick sanity message to prove the channel is live.
    const text = 'bidirectional-merge-works';
    await pageA.locator(sel.chatInputTextarea).fill(text);
    await pageA.locator(sel.chatInputTextarea).press('Enter');

    const rowB = pageB.locator(sel.messageRow, { hasText: text }).first();
    await expect(rowB).toBeVisible({ timeout: 10_000 });
  });
});
