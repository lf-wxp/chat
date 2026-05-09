/**
 * Emoji reaction E2E tests.
 *
 * Maps to: Requirement 16.11 (Message Reaction).
 */

import { sel } from '../utils/selectors.ts';
import {
  establishConnection,
  registerAndLogin,
  sendAndVerifyMessage,
} from '../fixtures/helpers.ts';
import { expect, test } from '../fixtures/test-base.ts';

test.describe('reactions', () => {
  test('clicking the react action opens the emoji picker', async ({ pageA, pageB, server }) => {
    await registerAndLogin(pageA, server, { hint: 'a' });
    const b = await registerAndLogin(pageB, server, { hint: 'b' });
    await establishConnection(pageA, pageB, b.username);

    const tag = Date.now().toString(36);
    const text = `rx-${tag}`;
    await sendAndVerifyMessage(pageB, pageA, text);

    const row = pageA.locator(sel.messageRow, { hasText: text }).first();
    await row.hover();
    await row.locator(sel.messageActionReact).click();

    await expect(pageA.locator(sel.reactionPicker)).toBeVisible();
  });
});
