/**
 * User logout and single-device login policy E2E tests.
 *
 * Maps to: Requirement 10.4 (UserLogout), Requirement 10.7 (Multi-device policy).
 *
 * Coverage:
 *   1. User logs out — auth page is shown, token is cleared, user disappears
 *      from other users' online lists.
 *   2. Single-device policy — logging in from a second context invalidates
 *      the first session (SessionInvalidated message).
 */

import { sel } from '../utils/selectors.ts';
import { registerAndLogin } from '../fixtures/helpers.ts';
import { waitForAppShell, waitForOnlineUser } from '../utils/wait-helpers.ts';
import { expect, test } from '../fixtures/test-base.ts';

const settingsPage = '[data-testid="settings-page"]';
const logoutBtn = '[data-testid="settings-logout"]';

test.describe('auth — logout & single-device policy', () => {
  test('user can log out and is redirected to auth page', async ({ pageA, server }) => {
    await registerAndLogin(pageA, server, { hint: 'logout' });

    // Verify we are in the app shell.
    await expect(pageA.locator(sel.sidebar)).toBeVisible();

    // Open settings and find the logout button.
    await pageA.locator(sel.sidebarSettingsBtn).click();
    await expect(pageA.locator(settingsPage)).toBeVisible({ timeout: 5_000 });

    // Click logout button (data-testid="settings-logout").
    await expect(pageA.locator(logoutBtn)).toBeVisible({ timeout: 5_000 });
    await pageA.locator(logoutBtn).click();

    // Should be redirected to auth page.
    await expect(pageA.locator(sel.authPage)).toBeVisible({ timeout: 10_000 });

    // Token should be cleared from localStorage.
    const token = await pageA.evaluate(() => localStorage.getItem('auth_token'));
    expect(token).toBeNull();
  });

  test('logged-out user disappears from other users online list', async ({
    pageA,
    pageB,
    server,
  }) => {
    const a = await registerAndLogin(pageA, server, { hint: 'lo_a' });
    const b = await registerAndLogin(pageB, server, { hint: 'lo_b' });

    // Both see each other online.
    await waitForOnlineUser(pageA, b.username);
    await waitForOnlineUser(pageB, a.username);

    // A logs out.
    await pageA.locator(sel.sidebarSettingsBtn).click();
    await expect(pageA.locator(settingsPage)).toBeVisible({ timeout: 5_000 });
    await expect(pageA.locator(logoutBtn)).toBeVisible({ timeout: 5_000 });
    await pageA.locator(logoutBtn).click();

    // B should no longer see A in the online list.
    await expect(
      pageB.locator(sel.onlineUserRow, { hasText: a.username }),
    ).toHaveCount(0, { timeout: 30_000 });
  });

  test('single-device policy — second login invalidates first session', async ({
    pageA,
    pageB,
    server,
  }) => {
    // Register user on pageA.
    const user = await registerAndLogin(pageA, server, { hint: 'sdev' });

    // Verify A is in the app shell.
    await expect(pageA.locator(sel.sidebar)).toBeVisible();

    // Login the same user from pageB (second device).
    await pageB.goto(`${server.baseUrl}/`);
    await pageB.locator(sel.authPage).waitFor({ state: 'visible' });

    // The auth page shows login form by default.
    await pageB.locator(sel.loginUsername).fill(user.username);
    await pageB.locator(sel.loginPassword).fill(user.password);
    await pageB.locator(sel.loginSubmit).click();

    // pageB should reach the app shell.
    await waitForAppShell(pageB);

    // pageA should be kicked back to the auth page (SessionInvalidated).
    // The application shows a notification/toast and redirects to login.
    await expect(pageA.locator(sel.authPage)).toBeVisible({ timeout: 30_000 });

    // Token on pageA should be cleared.
    const tokenA = await pageA.evaluate(() => localStorage.getItem('auth_token'));
    expect(tokenA).toBeNull();
  });

  test('user can re-login after logout', async ({ pageA, server }) => {
    const user = await registerAndLogin(pageA, server, { hint: 'relog' });

    // Logout.
    await pageA.locator(sel.sidebarSettingsBtn).click();
    await expect(pageA.locator(settingsPage)).toBeVisible({ timeout: 5_000 });
    await pageA.locator(logoutBtn).click();

    await expect(pageA.locator(sel.authPage)).toBeVisible({ timeout: 10_000 });

    // Re-login with the same credentials.
    await pageA.locator(sel.loginUsername).fill(user.username);
    await pageA.locator(sel.loginPassword).fill(user.password);
    await pageA.locator(sel.loginSubmit).click();

    // Should reach the app shell again.
    await waitForAppShell(pageA);
    await expect(pageA.locator(sel.sidebar)).toBeVisible();
  });
});
