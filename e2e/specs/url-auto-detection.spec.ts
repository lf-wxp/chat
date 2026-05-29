/**
 * URL auto-detection E2E tests (Req 16.4 AC-3).
 *
 * Verifies that plain-text URLs typed by users are rendered as
 * clickable `<a>` elements with `target="_blank"` and `rel="noopener"`.
 */

import { establishConnection, registerAndLogin, sendAndVerifyMessage } from '../fixtures/helpers.ts';
import { expect, test } from '../fixtures/test-base.ts';

test.describe('URL auto-detection', () => {
  test('plain URL is rendered as clickable link', async ({ pageA, pageB, server }) => {
    await registerAndLogin(pageA, server, { hint: 'url-a' });
    const b = await registerAndLogin(pageB, server, { hint: 'url-b' });
    await establishConnection(pageA, pageB, b.username);

    const url = 'https://example.com/test-page';
    const { senderRow: rowA, receiverRow: rowB } = await sendAndVerifyMessage(pageA, pageB, url);

    // The message bubble must contain an <a> element pointing to the URL.
    const linkB = rowB.locator('a');
    const linkA = rowA.locator('a');

    await expect(linkB).toHaveAttribute('href', url, { timeout: 3_000 });
    await expect(linkA).toHaveAttribute('href', url, { timeout: 3_000 });

    // Security attributes.
    await expect(linkB).toHaveAttribute('target', '_blank');
    await expect(linkB).toHaveAttribute('rel', 'noopener noreferrer');
    await expect(linkA).toHaveAttribute('target', '_blank');
    await expect(linkA).toHaveAttribute('rel', 'noopener noreferrer');

    // Link text should be the full URL (not truncated or altered).
    await expect(linkB).toHaveText(url);
    await expect(linkA).toHaveText(url);
  });

  test('URL inside normal text is also linkified', async ({ pageA, pageB, server }) => {
    await registerAndLogin(pageA, server, { hint: 'url-mix-a' });
    const b = await registerAndLogin(pageB, server, { hint: 'url-mix-b' });
    await establishConnection(pageA, pageB, b.username);

    const message = 'Check out https://rust-lang.org for more info';
    const { senderRow: rowA, receiverRow: rowB } = await sendAndVerifyMessage(pageA, pageB, message);

    // There should be an <a> tag inside the message.
    const linkB = rowB.locator('a');
    const linkA = rowA.locator('a');
    await expect(linkB).toHaveCount(1);
    await expect(linkA).toHaveCount(1);

    await expect(linkB).toHaveAttribute('href', 'https://rust-lang.org');
    await expect(linkA).toHaveAttribute('href', 'https://rust-lang.org');
  });

  test('non-URL text does not produce anchor element', async ({ pageA, pageB, server }) => {
    await registerAndLogin(pageA, server, { hint: 'url-none-a' });
    const b = await registerAndLogin(pageB, server, { hint: 'url-none-b' });
    await establishConnection(pageA, pageB, b.username);

    const plainText = 'This is just plain text without any link';
    const { senderRow: rowA, receiverRow: rowB } = await sendAndVerifyMessage(pageA, pageB, plainText);

    // No <a> tags inside the message bubble.
    await expect(rowB.locator('a')).toHaveCount(0);
    await expect(rowA.locator('a')).toHaveCount(0);
  });
});
