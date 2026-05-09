/**
 * Smoke test: validates that the test infrastructure (server lifecycle, build
 * outputs, auth page rendering) works end-to-end.
 *
 * Maps to: Requirement 16.1 (Test Infrastructure & Setup).
 */

import { sel } from '../utils/selectors.ts';
import { expect, test } from '../fixtures/test-base.ts';

test.describe('smoke', () => {
  test('signaling server reports healthy', async ({ server, request }) => {
    const response = await request.get(`${server.baseUrl}/api/health`);
    expect(response.status()).toBe(200);
    const body = (await response.json()) as { status: string; service: string };
    expect(body.status).toBe('ok');
    expect(body.service).toBe('webrtc-chat-signaling');
  });

  test('frontend shell renders auth page on first load', async ({ pageA, server }) => {
    await pageA.goto(`${server.baseUrl}/`);
    await expect(pageA.locator(sel.authPage)).toBeVisible({ timeout: 20_000 });
  });
});
