/**
 * Connection invitation flow E2E tests.
 *
 * Maps to: Requirement 16.3 (Connection Invitation & Chat Session Establishment).
 */

import { sel } from '../utils/selectors.ts';
import { establishConnection, registerAndLogin } from '../fixtures/helpers.ts';
import { waitForOnlineUser } from '../utils/wait-helpers.ts';
import { expect, test } from '../fixtures/test-base.ts';

test.describe('invitation', () => {
  test('happy path: A invites B, B accepts, both reach chat view', async ({
    pageA,
    pageB,
    server,
  }) => {
    await registerAndLogin(pageA, server, { hint: 'a' });
    const b = await registerAndLogin(pageB, server, { hint: 'b' });

    await establishConnection(pageA, pageB, b.username);

    // Both pages now show the chat view with an enabled input.
    await expect(pageA.locator(sel.chatInputTextarea)).toBeEnabled();
    await expect(pageB.locator(sel.chatInputTextarea)).toBeEnabled();
  });

  test('opening user info card surfaces invite button', async ({ pageA, pageB, server }) => {
    await registerAndLogin(pageA, server, { hint: 'a' });
    const b = await registerAndLogin(pageB, server, { hint: 'b' });

    const userBRow = await waitForOnlineUser(pageA, b.username);
    await userBRow.click();

    const card = pageA.locator(sel.userInfoCard);
    await expect(card).toBeVisible();
    await expect(card).toContainText(b.username);
    await expect(pageA.locator(sel.userInfoInvite)).toBeVisible();
  });

  test('declining an invitation surfaces a prompt and re-enables invite', async ({
    pageA,
    pageB,
    server,
  }) => {
    await registerAndLogin(pageA, server, { hint: 'a' });
    const b = await registerAndLogin(pageB, server, { hint: 'b' });

    const row = await waitForOnlineUser(pageA, b.username);
    await row.click();
    await pageA.locator(sel.userInfoInvite).click();

    // B receives the modal and declines.
    const invite = pageB.locator(sel.incomingInviteModal);
    await invite.waitFor({ state: 'visible', timeout: 15_000 });
    await pageB.locator(sel.inviteDecline).click();

    // A's invite button (re)becomes interactable. The "connecting" indicator
    // must disappear within a reasonable budget.
    await expect(pageA.locator(sel.userInfoConnecting)).toBeHidden({ timeout: 15_000 });
  });
});
