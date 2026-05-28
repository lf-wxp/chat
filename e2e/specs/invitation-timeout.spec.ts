/**
 * Invitation timeout E2E tests (Req 16.3 AC-5).
 *
 * When a user sends a connection invitation and the recipient does not
 * respond within the timeout window, the sender's UI should display an
 * "invitation timed out" indicator and the invite button should become
 * clickable again.
 *
 * NOTE: We cannot wait 60 s in a real E2E test.  The test environment
 * overrides `INVITATION_TIMEOUT_MS` to 5 s so the scenario still exercises
 * the timeout path without making the suite unbearably slow.
 */

import { sel } from '../utils/selectors.ts';
import { registerAndLogin } from '../fixtures/helpers.ts';
import { expect, test } from '../fixtures/test-base.ts';

// Override timeout for test speed.  This matches the dev-server
// override in `frontend/src/invite/manager.rs` when
// `cfg!(debug_assertions)` is enabled.
const TEST_TIMEOUT_MS = 5_000;

test.describe('invitation timeout', () => {
  test('unaccepted invite times out and re-enables invite button', async ({
    pageA,
    pageB,
    server,
  }) => {
    await registerAndLogin(pageA, server, { hint: 'inv-to-a' });
    const b = await registerAndLogin(pageB, server, { hint: 'inv-to-b' });

    // A opens the online-users panel and clicks invite on B.
    await pageA.locator(sel.sidebarSearchInput).fill(b.username);
    await pageA.waitForTimeout(300);
    const userRowA = pageA.locator(sel.onlineUserRow, { hasText: b.username }).first();
    await expect(userRowA).toBeVisible({ timeout: 5_000 });
    await userRowA.click();

    await expect(pageA.locator(sel.userInfoCard)).toBeVisible({ timeout: 5_000 });
    const inviteBtn = pageA.locator(sel.userInfoInvite);
    await inviteBtn.click();

    // The invite button should be disabled (pending state).
    await expect(inviteBtn).toBeDisabled({ timeout: 3_000 });

    // B does NOT accept or decline — we just wait for the timeout.
    // In the dev build the timeout is shortened to 5 s.
    await pageA.waitForTimeout(TEST_TIMEOUT_MS + 2_000);

    // After timeout the invite button should be enabled again.
    await expect(inviteBtn).toBeEnabled({ timeout: 5_000 });

    // A can now send a new invitation.
    await inviteBtn.click();
    await expect(inviteBtn).toBeDisabled({ timeout: 3_000 });
  });

  test('timed-out invite is not accepted after the deadline', async ({
    pageA,
    pageB,
    server,
  }) => {
    await registerAndLogin(pageA, server, { hint: 'inv-to2-a' });
    const b = await registerAndLogin(pageB, server, { hint: 'inv-to2-b' });

    // A sends an invite to B.
    await pageA.locator(sel.sidebarSearchInput).fill(b.username);
    await pageA.waitForTimeout(300);
    const userRowA = pageA.locator(sel.onlineUserRow, { hasText: b.username }).first();
    await userRowA.click();
    await expect(pageA.locator(sel.userInfoCard)).toBeVisible({ timeout: 5_000 });
    const inviteBtn = pageA.locator(sel.userInfoInvite);
    await inviteBtn.click();
    await expect(inviteBtn).toBeDisabled({ timeout: 3_000 });

    // Wait for timeout.
    await pageA.waitForTimeout(TEST_TIMEOUT_MS + 2_000);

    // B should NOT see an incoming invite modal (it expired locally too
    // since the server also enforces the same timeout window).
    await expect(pageB.locator(sel.incomingInviteModal)).not.toBeVisible();

    // A's invite button is clickable again.
    await expect(inviteBtn).toBeEnabled({ timeout: 5_000 });
  });
});
