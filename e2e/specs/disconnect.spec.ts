/**
 * Disconnect / reconnect E2E tests.
 *
 * Maps to: Requirement 16.16 (Chat Session Disconnect & Reconnect).
 */

import { sel } from '../utils/selectors.ts';
import { establishConnection, registerAndLogin } from '../fixtures/helpers.ts';
import { waitForOnlineUser } from '../utils/wait-helpers.ts';
import { expect, test } from '../fixtures/test-base.ts';

test.describe('disconnect', () => {
  test('A sees B drop out of the online list when B closes the page', async ({
    pageA,
    pageB,
    server,
  }) => {
    await registerAndLogin(pageA, server, { hint: 'a' });
    const b = await registerAndLogin(pageB, server, { hint: 'b' });
    await establishConnection(pageA, pageB, b.username);

    // Close B's page to simulate disconnect.
    await pageB.close();

    // B's row in A's online users panel must disappear (or change status to
    // offline). We accept either behaviour by polling for the absence of an
    // online row that lists B's username.
    await expect(
      pageA.locator(sel.onlineUserRow, { hasText: b.username }),
    ).toHaveCount(0, { timeout: 30_000 });
  });

  test('B reconnects on a fresh page and re-appears in A online list', async ({
    pageA,
    pageB,
    contextB,
    server,
  }) => {
    await registerAndLogin(pageA, server, { hint: 'a' });
    const b = await registerAndLogin(pageB, server, {
      hint: 'b',
    });
    await establishConnection(pageA, pageB, b.username);

    await pageB.close();
    await expect(
      pageA.locator(sel.onlineUserRow, { hasText: b.username }),
    ).toHaveCount(0, { timeout: 30_000 });

    // Reopen B in a brand-new page on the same context (cookies/localStorage
    // intact -> TokenAuth recovers the session).
    const reopened = await contextB.newPage();
    await reopened.goto(`${server.baseUrl}/`);
    await reopened.locator(sel.sidebar).waitFor({ state: 'visible', timeout: 20_000 });

    await waitForOnlineUser(pageA, b.username);
  });
});
