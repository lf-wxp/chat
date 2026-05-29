/**
 * Blacklist (block/unblock) E2E tests.
 *
 * Maps to: Requirement 9.2 (Blacklist), Requirement 9.17 (Auto-decline with delay).
 *
 * Coverage:
 *   1. User can block another user via the user info card.
 *   2. Blocked user's invitation is auto-declined (with random delay).
 *   3. User can unblock a previously blocked user via settings drawer.
 *   4. Blacklist persists across page reloads (localStorage).
 */

import { sel } from '../utils/selectors.ts';
import { registerAndLogin } from '../fixtures/helpers.ts';
import { waitForOnlineUser } from '../utils/wait-helpers.ts';
import { expect, test } from '../fixtures/test-base.ts';

const settingsPageSelector = '[data-testid="settings-page"]';
const blacklistPanel = '[data-testid="blacklist-panel"]';
const blacklistRow = '[data-testid="blacklist-row"]';

test.describe('blacklist — block/unblock users', () => {
  test('user can block another user via user info card', async ({ pageA, pageB, server }) => {
    await registerAndLogin(pageA, server, { hint: 'blk_a' });
    const b = await registerAndLogin(pageB, server, { hint: 'blk_b' });

    // A opens B's user info card.
    const userBRow = await waitForOnlineUser(pageA, b.username);
    await userBRow.click();

    const userInfoCard = pageA.locator(sel.userInfoCard);
    await expect(userInfoCard).toBeVisible({ timeout: 5_000 });

    // Click the block button.
    const blockBtn = pageA.locator(sel.userInfoBlock);
    await expect(blockBtn).toBeVisible({ timeout: 5_000 });
    await expect(blockBtn).not.toHaveClass(/is-blocked/);
    await blockBtn.click();

    // The user-info-card closes after a successful block.
    await expect(userInfoCard).toBeHidden({ timeout: 10_000 });

    // Re-open the card — the block button should now have is-blocked class
    // and the invite button should be disabled.
    const bRow2 = await waitForOnlineUser(pageA, b.username);
    await bRow2.click();
    await expect(userInfoCard).toBeVisible({ timeout: 10_000 });

    await expect(pageA.locator(sel.userInfoBlock)).toHaveClass(/is-blocked/, {
      timeout: 5_000,
    });
    await expect(pageA.locator(sel.userInfoInvite)).toBeDisabled();
  });

  test('blocked user invitation is auto-declined', async ({ pageA, pageB, server }) => {
    const a = await registerAndLogin(pageA, server, { hint: 'blkd_a' });
    const b = await registerAndLogin(pageB, server, { hint: 'blkd_b' });

    // A blocks B.
    const userBRow = await waitForOnlineUser(pageA, b.username);
    await userBRow.click();
    await expect(pageA.locator(sel.userInfoCard)).toBeVisible({ timeout: 5_000 });
    await pageA.locator(sel.userInfoBlock).click();
    // Card auto-closes after block.
    await expect(pageA.locator(sel.userInfoCard)).toBeHidden({ timeout: 10_000 });

    // B sends an invitation to A.
    const aRow = await waitForOnlineUser(pageB, a.username);
    await aRow.click();
    await expect(pageB.locator(sel.userInfoCard)).toBeVisible({ timeout: 5_000 });
    await pageB.locator(sel.userInfoInvite).click();

    // Close B's user info card.
    await pageB.keyboard.press('Escape');
    await pageB.locator(sel.userInfoBackdrop).waitFor({ state: 'hidden', timeout: 10_000 });

    // A should NOT see an incoming invite modal (it's auto-declined after
    // a random 30-60s delay per Req 9.17).
    await pageA.waitForTimeout(5_000);
    await expect(pageA.locator(sel.incomingInviteModal)).toHaveCount(0);

    // B should eventually see the invite declined (timeout or explicit decline).
    // The auto-decline fires after 30-60s, so we wait up to 65s.
    await expect(pageB.locator(sel.userInfoConnecting)).toBeHidden({ timeout: 65_000 });
  });

  test('user can unblock via settings drawer blacklist panel', async ({
    pageA,
    pageB,
    server,
  }) => {
    await registerAndLogin(pageA, server, { hint: 'unblk_a' });
    const b = await registerAndLogin(pageB, server, { hint: 'unblk_b' });

    // A blocks B.
    const userBRow = await waitForOnlineUser(pageA, b.username);
    await userBRow.click();
    await expect(pageA.locator(sel.userInfoCard)).toBeVisible({ timeout: 5_000 });
    await pageA.locator(sel.userInfoBlock).click();
    await expect(pageA.locator(sel.userInfoCard)).toBeHidden({ timeout: 10_000 });

    // Open the settings drawer.
    await pageA.locator(sel.sidebarSettingsBtn).click();
    await expect(pageA.locator(settingsPageSelector)).toBeVisible({ timeout: 5_000 });

    // The blacklist panel should show B's entry.
    const panel = pageA.locator(blacklistPanel);
    await expect(panel).toBeVisible({ timeout: 5_000 });
    const row = panel.locator(blacklistRow);
    await expect(row).toHaveCount(1, { timeout: 5_000 });
    await expect(row).toContainText(b.username);

    // Click the Unblock button on the row.
    await row.locator('button.blacklist-panel__unblock').click();
    await expect(panel.locator(blacklistRow)).toHaveCount(0, { timeout: 5_000 });

    // Close settings.
    await pageA.keyboard.press('Escape');

    // Re-open B's info card — should no longer be blocked.
    const bRow2 = await waitForOnlineUser(pageA, b.username);
    await bRow2.click();
    await expect(pageA.locator(sel.userInfoCard)).toBeVisible({ timeout: 10_000 });
    await expect(pageA.locator(sel.userInfoBlock)).not.toHaveClass(/is-blocked/, {
      timeout: 5_000,
    });
    await expect(pageA.locator(sel.userInfoInvite)).toBeEnabled();
  });

  test('blacklist persists across page reloads', async ({ pageA, pageB, server }) => {
    await registerAndLogin(pageA, server, { hint: 'blkp_a' });
    const b = await registerAndLogin(pageB, server, { hint: 'blkp_b' });

    // A blocks B.
    const userBRow = await waitForOnlineUser(pageA, b.username);
    await userBRow.click();
    await expect(pageA.locator(sel.userInfoCard)).toBeVisible({ timeout: 5_000 });
    await pageA.locator(sel.userInfoBlock).click();
    await expect(pageA.locator(sel.userInfoCard)).toBeHidden({ timeout: 10_000 });

    // Verify blacklist is in localStorage (stored as JSON array).
    await expect
      .poll(
        async () =>
          pageA.evaluate(() => {
            const raw = localStorage.getItem('blacklist');
            if (!raw) return 0;
            try {
              const parsed = JSON.parse(raw) as Array<unknown>;
              return Array.isArray(parsed) ? parsed.length : 0;
            } catch {
              return 0;
            }
          }),
        { timeout: 5_000 },
      )
      .toBeGreaterThan(0);

    // Reload the page.
    await pageA.reload();
    await pageA.locator(sel.sidebar).waitFor({ state: 'visible', timeout: 20_000 });

    // Re-open B's info card — should still show blocked state (is-blocked class).
    const userBRowAfter = await waitForOnlineUser(pageA, b.username);
    await userBRowAfter.click();
    await expect(pageA.locator(sel.userInfoCard)).toBeVisible({ timeout: 5_000 });
    await expect(pageA.locator(sel.userInfoBlock)).toHaveClass(/is-blocked/, {
      timeout: 5_000,
    });
    await expect(pageA.locator(sel.userInfoInvite)).toBeDisabled();
  });
});
