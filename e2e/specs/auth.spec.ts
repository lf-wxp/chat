/**
 * User registration / login / session recovery E2E tests.
 *
 * Maps to: Requirement 16.2 (User Registration & Login Flow).
 *
 * Implementation notes:
 * - The current sidebar does NOT render the logged-in username (it shows the
 *   app brand, conversation lists, online users panel, and a settings button).
 *   We therefore assert auth success structurally — auth-page disappears,
 *   sidebar appears — and verify TokenAuth recovery via the persisted
 *   `auth_token` localStorage entry.
 */

import { sel } from '../utils/selectors.ts';
import { DEFAULT_PASSWORD, uniqueUsername } from '../utils/users.ts';
import { registerAndLogin } from '../fixtures/helpers.ts';
import { waitForAppShell, waitForOnlineUser } from '../utils/wait-helpers.ts';
import { expect, test } from '../fixtures/test-base.ts';

test.describe('auth', () => {
  test('registers a new user and reaches the main app shell', async ({ pageA, server }) => {
    await registerAndLogin(pageA, server, { hint: 'reg' });

    // Sidebar is visible and auth page has been replaced.
    await expect(pageA.locator(sel.sidebar)).toBeVisible();
    await expect(pageA.locator(sel.authPage)).toHaveCount(0);

    // The auth_token entry must have been persisted to localStorage so a
    // subsequent reload would TokenAuth-recover.
    const token = await pageA.evaluate(() => localStorage.getItem('auth_token'));
    expect(token).not.toBeNull();
    expect(token!.length).toBeGreaterThan(0);
  });

  test('rejects registration with a duplicate username', async ({ pageA, pageB, server }) => {
    const username = uniqueUsername('dup');

    // First registration succeeds.
    await registerAndLogin(pageA, server, { username });

    // Second attempt with the same username on a fresh context fails.
    await pageB.goto(`${server.baseUrl}/`);
    await pageB.locator(sel.authSwitchToRegister).click();
    await pageB.locator(sel.registerUsername).fill(username);
    await pageB.locator(sel.registerPassword).fill(DEFAULT_PASSWORD);
    await pageB.locator(sel.registerConfirmPassword).fill(DEFAULT_PASSWORD);
    await pageB.locator(sel.registerSubmit).click();

    await expect(pageB.locator(sel.registerError)).toBeVisible({ timeout: 10_000 });
    // Stays on the auth page (i.e. no redirect to the app shell).
    await expect(pageB.locator(sel.authPage)).toBeVisible();
  });

  test('two registered users see each other online', async ({ pageA, pageB, server }) => {
    const a = await registerAndLogin(pageA, server, { hint: 'aa' });
    const b = await registerAndLogin(pageB, server, { hint: 'bb' });

    await waitForOnlineUser(pageA, b.username);
    await waitForOnlineUser(pageB, a.username);
  });

  test('session is restored after a page refresh', async ({ pageA, server }) => {
    await registerAndLogin(pageA, server, { hint: 'rfr' });

    const tokenBefore = await pageA.evaluate(() => localStorage.getItem('auth_token'));
    expect(tokenBefore).not.toBeNull();

    await pageA.reload();
    await waitForAppShell(pageA);

    // Auth page must NOT be present after refresh — TokenAuth restored.
    await expect(pageA.locator(sel.authPage)).toHaveCount(0);

    // Token is still persisted (and identical, since TokenAuth re-uses it).
    const tokenAfter = await pageA.evaluate(() => localStorage.getItem('auth_token'));
    expect(tokenAfter).toBe(tokenBefore);
  });
});
