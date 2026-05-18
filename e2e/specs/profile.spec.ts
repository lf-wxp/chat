/**
 * Profile / blacklist coverage (Wave P2-7).
 *
 * Plan §3 P2-7 originally listed three sub-tests (upload avatar,
 * nickname edit round-trip, block/unblock). The first two are
 * gated on UI surfaces that do not yet exist:
 *
 *   * Avatar upload — no UI affordance anywhere (see G26).
 *   * Nickname edit — `NicknameEditor` component exists with full
 *     testids, but is never mounted in the live view tree (see
 *     G25). Settings drawer does not embed it; no other surface
 *     instantiates it either.
 *
 * What works today is the **block / unblock** flow, which is
 * actually wired end-to-end:
 *
 *   1. User opens an `user-info-card` from the online users panel
 *      and clicks the Block button.
 *   2. `BlacklistState` writes a `BlacklistEntry` for the target
 *      and persists the list as JSON under
 *      `localStorage["blacklist"]`.
 *   3. The `BlacklistManagementPanel` embedded in the settings
 *      drawer surfaces every entry; an Unblock button on each row
 *      removes it.
 *
 * This spec locks down that flow on three axes:
 *   * Block flips the user-info-card's invite button to `disabled`
 *     and the Block button to "Unblock" — the card-local state
 *     reflects the global signal.
 *   * The blacklist row shows up in the settings drawer's
 *     `blacklist-panel` and disappears after Unblock.
 *   * The blacklist persists across a page reload via the
 *     `blacklist` localStorage key.
 */

import { sel } from '../utils/selectors.ts';
import { registerAndLogin } from '../fixtures/helpers.ts';
import { expect, test } from '../fixtures/test-base.ts';
import {
  waitForAppShell,
  waitForOnlineUser,
} from '../utils/wait-helpers.ts';

const settingsPageSelector = '[data-testid="settings-page"]';
const blacklistPanel = '[data-testid="blacklist-panel"]';
const blacklistRow = '[data-testid="blacklist-row"]';

test.describe('profile / blacklist', () => {
  test('block flips invite button to disabled and the block button to Unblock', async ({
    pageA,
    pageB,
    server,
  }) => {
    await registerAndLogin(pageA, server, { hint: 'pf-blk-a' });
    const userB = await registerAndLogin(pageB, server, { hint: 'pf-blk-b' });

    // Open A's user-info-card for userB (the helper waits for B to
    // appear in A's online list before clicking).
    const bRow = await waitForOnlineUser(pageA, userB.username);
    await bRow.click();
    const card = pageA.locator(sel.userInfoCard);
    await expect(card).toBeVisible({ timeout: 10_000 });

    // Before block: invite button is enabled, block button label is
    // "Block". The label is i18n-translated; we key off the
    // class-derived `is-blocked` modifier instead so the test is
    // locale-independent.
    const invite = pageA.locator(sel.userInfoInvite);
    const block = pageA.locator(sel.userInfoBlock);
    await expect(invite).toBeEnabled();
    await expect(block).not.toHaveClass(/is-blocked/);

    await block.click();

    // The user-info-card closes after a successful block (`close()`
    // called from the click handler). When closed, neither button is
    // mounted — assert the dismiss instead.
    await expect(card).toBeHidden({ timeout: 10_000 });

    // Open the card a second time — userB should still be in the
    // online list (block doesn't kick anyone offline server-side).
    const bRow2 = await waitForOnlineUser(pageA, userB.username);
    await bRow2.click();
    await expect(card).toBeVisible({ timeout: 10_000 });

    // Now the card reflects the blocked state.
    await expect(pageA.locator(sel.userInfoBlock)).toHaveClass(/is-blocked/, {
      timeout: 5_000,
    });
    await expect(pageA.locator(sel.userInfoInvite)).toBeDisabled();
  });

  test('blacklist panel in settings drawer surfaces blocked rows and Unblock removes them', async ({
    pageA,
    pageB,
    server,
  }) => {
    await registerAndLogin(pageA, server, { hint: 'pf-pnl-a' });
    const userB = await registerAndLogin(pageB, server, { hint: 'pf-pnl-b' });

    // Step 1 — block userB.
    const bRow = await waitForOnlineUser(pageA, userB.username);
    await bRow.click();
    await expect(pageA.locator(sel.userInfoCard)).toBeVisible({ timeout: 10_000 });
    await pageA.locator(sel.userInfoBlock).click();
    await expect(pageA.locator(sel.userInfoCard)).toBeHidden({ timeout: 10_000 });

    // Step 2 — open the settings drawer; assert the blacklist row.
    await pageA.locator(sel.sidebarSettingsBtn).click();
    await expect(pageA.locator(settingsPageSelector)).toBeVisible({ timeout: 5_000 });

    const panel = pageA.locator(blacklistPanel);
    await expect(panel).toBeVisible({ timeout: 5_000 });
    const row = panel.locator(blacklistRow);
    await expect(row).toHaveCount(1, { timeout: 5_000 });
    // The row carries userB's display name (nickname falls back to
    // username for users without a custom nickname).
    await expect(row).toContainText(userB.username);

    // Step 3 — click the Unblock button on the row. The list re-
    // renders with zero entries.
    await row.locator('button.blacklist-panel__unblock').click();
    await expect(panel.locator(blacklistRow)).toHaveCount(0, { timeout: 5_000 });
  });

  test('blacklist persists across page reload via localStorage', async ({
    pageA,
    pageB,
    server,
  }) => {
    await registerAndLogin(pageA, server, { hint: 'pf-rld-a' });
    const userB = await registerAndLogin(pageB, server, { hint: 'pf-rld-b' });

    const bRow = await waitForOnlineUser(pageA, userB.username);
    await bRow.click();
    await expect(pageA.locator(sel.userInfoCard)).toBeVisible({ timeout: 10_000 });
    await pageA.locator(sel.userInfoBlock).click();
    await expect(pageA.locator(sel.userInfoCard)).toBeHidden({ timeout: 10_000 });

    // The blacklist persists synchronously after `block()` finishes;
    // poll the localStorage key so we don't race the write.
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
      .toBe(1);

    await pageA.reload();
    await waitForAppShell(pageA);

    // After reload, the blacklist panel is reachable via the
    // settings drawer; the previously-blocked entry survives.
    await pageA.locator(sel.sidebarSettingsBtn).click();
    await expect(pageA.locator(settingsPageSelector)).toBeVisible({ timeout: 5_000 });
    const panel = pageA.locator(blacklistPanel);
    await expect(panel).toBeVisible({ timeout: 5_000 });
    await expect(panel.locator(blacklistRow)).toHaveCount(1, { timeout: 5_000 });
    await expect(panel.locator(blacklistRow)).toContainText(userB.username);
  });
});
