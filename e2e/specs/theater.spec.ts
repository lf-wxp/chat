/**
 * Theater room E2E coverage (Wave P2-5).
 *
 * The sidebar's "Create room" button opens `CreateRoomModal`. Picking
 * the Theater radio (`data-testid="room-type-theater"`) and submitting
 * sends a `CreateRoom { room_type: Theater }` to the signaling server.
 * The frontend then:
 *   * receives `RoomCreated` (handler stores the room and sets the
 *     creator as Owner),
 *   * sets `active_conversation = ConversationId::Room(room_id)`,
 *   * causes `HomePage` to render `<TheaterPage>` (because the active
 *     conversation's room_type is Theater).
 *
 * What is reliably observable on the owner-side without any
 * second-peer choreography:
 *   * `theater-page` is mounted with the room's name in its header.
 *   * `theater-source-picker` (the upload-local / share-screen / URL
 *     trio) is visible — the owner can choose a source.
 *   * The chat tab is wired and dispatching a `danmaku-input` /
 *     `theater-chat-input` send creates a `theater-chat-bubble`.
 *
 * Plan §3 P2-5 originally listed "owner selects local video", "viewer
 * joins", and "danmaku round-trip". The first sub-test would require
 * us to feed a video file to the `theater-source-local-input` and
 * wait for a synthetic `<video>` element to begin playing — which
 * needs codec-compatible test media (Chromium's fake video file).
 * Without that fixture we cannot deterministically assert "video
 * source is playing", so this spec covers what is robust today and
 * leaves the full media round-trip to a follow-up (G27 in the plan).
 */

import { sel } from '../utils/selectors.ts';
import { registerAndLogin } from '../fixtures/helpers.ts';
import { expect, test } from '../fixtures/test-base.ts';
import type { Page } from '@playwright/test';

/**
 * Create a Theater-type room as `page`'s logged-in user.
 *
 * Mirrors the `createRoom` helper but flips the radio to Theater and
 * waits for the page to auto-mount `<TheaterPage>` (the creator is
 * auto-switched into the room and the home page swaps ChatView →
 * TheaterPage based on `active_conversation`'s room type).
 */
async function createTheaterRoom(
  page: Page,
  options: { name?: string } = {},
): Promise<{ name: string }> {
  const name = options.name ?? `theater-${Math.random().toString(36).slice(2, 8)}`;

  await page.locator(sel.sidebarRoomCreateBtn).click();
  const modal = page.locator(sel.createRoomModal);
  await expect(modal).toBeVisible({ timeout: 10_000 });

  await modal.locator(sel.createRoomName).fill(name);
  // Flip the room type radio to Theater. The radio itself is hidden
  // behind a styled label; click the input element directly.
  await modal.locator(sel.createRoomTypeTheater).click();

  // The Theater-only hint mounts once the radio is on; serves as a
  // readiness oracle that the room-type signal has flipped.
  await expect(modal.locator('[data-testid="theater-extra-hint"]')).toBeVisible({
    timeout: 3_000,
  });

  await modal.locator(sel.createRoomSubmit).click();
  await expect(modal).toBeHidden({ timeout: 10_000 });

  // TheaterPage auto-mounts because `RoomCreated` sets
  // `active_conversation` to the new room id and the room_type is
  // Theater (see `home_page.rs`).
  await expect(page.locator(sel.theaterPage)).toBeVisible({ timeout: 15_000 });

  return { name };
}

test.describe('theater room', () => {
  test('creator lands on theater-page with the source picker visible', async ({
    pageA,
    server,
  }) => {
    await registerAndLogin(pageA, server, { hint: 'th-cr' });
    const { name } = await createTheaterRoom(pageA);

    const page = pageA.locator(sel.theaterPage);
    await expect(page).toBeVisible();
    // Header carries the room name (the only `h2` inside the
    // theater-page header).
    await expect(page.locator('header h2').first()).toContainText(name);

    // Source picker is rendered for the owner before any video is
    // chosen — confirms the player slot resolved to the empty state
    // rather than a stale ChatView.
    await expect(pageA.locator(sel.theaterSourcePicker)).toBeVisible({ timeout: 10_000 });

    // The picker's three primary affordances are all present.
    await expect(pageA.locator(sel.theaterSourceLocal)).toBeVisible();
    await expect(pageA.locator(sel.theaterSourceScreen)).toBeVisible();
    await expect(pageA.locator(sel.theaterSourceUrl)).toBeVisible();
  });

  test('chat tab dispatches a message and renders a theater-chat-bubble', async ({
    pageA,
    server,
  }) => {
    await registerAndLogin(pageA, server, { hint: 'th-chat' });
    await createTheaterRoom(pageA);

    // Activate the chat tab — `theater-tab-chat` is the default, but
    // click it anyway so the test is robust against future tab-state
    // changes that might land the user on Members first.
    await pageA.locator(sel.theaterTabChat).click();
    await expect(pageA.locator(sel.theaterChatPanel)).toBeVisible({ timeout: 5_000 });

    const tag = Date.now().toString(36);
    const message = `theater-msg-${tag}`;
    await pageA.locator(sel.theaterChatInput).fill(message);
    await pageA.locator(sel.theaterChatSend).click();

    // The owner's own send appears in the chat panel as a
    // theater-chat-bubble.
    const bubble = pageA
      .locator(sel.theaterChatPanel)
      .locator('[data-testid="theater-chat-bubble"]', { hasText: message })
      .first();
    await expect(bubble).toBeVisible({ timeout: 10_000 });
  });

  test('URL picker rejects non-http(s) URLs with a visible error (G27)', async ({
    pageA,
    server,
  }) => {
    await registerAndLogin(pageA, server, { hint: 'th-url' });
    await createTheaterRoom(pageA);

    // Toggle the URL form open.
    await pageA.locator(sel.theaterSourceUrl).click();
    const urlInput = pageA.locator('[data-testid="theater-source-url-input"]');
    await expect(urlInput).toBeVisible({ timeout: 5_000 });

    // The native `<input type="url">` validation accepts any
    // RFC 3986 URI (incl. `ftp://`), so a non-http(s) URL passes the
    // browser-level check and the form's `submit` event fires —
    // which lets the picker's own
    // `starts_with("http://") || starts_with("https://")` branch
    // surface its alert.
    await urlInput.fill('ftp://example.test');
    await pageA.locator('[data-testid="theater-source-url-submit"]').click();

    const error = pageA
      .locator(sel.theaterSourcePicker)
      .locator('.theater-source-picker__error');
    await expect(error).toBeVisible({ timeout: 3_000 });
    await expect(error).not.toBeEmpty();
    // Form value preserved so the user can correct the typo.
    await expect(urlInput).toHaveValue('ftp://example.test');

    // The full "happy-path URL → <video>.src binding" assertion is
    // deferred until a codec-compatible video fixture lands under
    // `e2e/assets/`. The player gates `<video>` mounting on
    // `loadedmetadata`, which never fires for a non-video URL, so
    // the picker's `url_input.set("")` reset happens but the
    // `<video>.src` cannot be read until the element materialises.
    // Tracked as a follow-up under the same G27 plan line.
  });
});
