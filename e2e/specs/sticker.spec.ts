/**
 * Sticker message E2E coverage (Wave P2-1).
 *
 * The composer's sticker button (`chat-input-bar` first icon) toggles
 * the `StickerPanel` overlay. Inside that overlay:
 *   * Three built-in tabs (Smileys / Animals / Gestures).
 *   * A search box that filters the current pack's glyphs.
 *   * A grid of glyph buttons (`data-testid="sticker-panel-item"`,
 *     `data-glyph` carrying the emoji).
 *
 * Clicking a glyph dispatches `send_sticker` and closes the panel.
 * The receiver renders the bubble as `<img class="message-sticker">`
 * (data-testid="message-sticker"); when the asset webp is missing
 * (default deployment ships no real packs) the inline `onerror`
 * replaces the `<img>` with a text node containing the glyph, so we
 * key our cross-peer assertion on the parent bubble row containing
 * the glyph as text — robust to either branch firing.
 *
 * Coverage:
 *   1. Open panel → pick a smiley → panel closes, both peers see
 *      the sticker bubble keyed by the chosen glyph.
 *   2. Search box on an empty result narrows the grid to zero items
 *      and surfaces the `sticker-panel-empty` fallback.
 *   3. Switching tab clears the search and repopulates the grid
 *      from the new pack.
 */

import { sel } from '../utils/selectors.ts';
import { establishConnection, registerAndLogin } from '../fixtures/helpers.ts';
import { expect, test } from '../fixtures/test-base.ts';

test.describe('sticker message', () => {
  test.beforeEach(async ({ pageA, pageB, server }) => {
    await registerAndLogin(pageA, server, { hint: 'st-a' });
    const b = await registerAndLogin(pageB, server, { hint: 'st-b' });
    await establishConnection(pageA, pageB, b.username);
  });

  test('picking an emoji glyph sends a sticker bubble to the receiver', async ({
    pageA,
    pageB,
  }) => {
    // Open the sticker panel by clicking the smiley toggle in the
    // composer. The first chat-input-btn is the sticker toggle (see
    // `input_bar.rs` ordering: Smile → Image → Paperclip → Mic).
    await pageA
      .locator(sel.chatInputBar)
      .locator('button.chat-input-btn')
      .first()
      .click();
    await expect(pageA.locator(sel.stickerPanel)).toBeVisible({ timeout: 5_000 });

    // Pick the first item in whichever pack is active. We don't depend
    // on a specific glyph because the built-in pack list might evolve;
    // we read its `data-glyph` and use it as the cross-peer assertion
    // key.
    const firstItem = pageA.locator(sel.stickerPanelItem).first();
    await expect(firstItem).toBeVisible();
    const glyph = await firstItem.getAttribute('data-glyph');
    expect(glyph).toBeTruthy();

    await firstItem.click();

    // Panel auto-closes after a successful send.
    await expect(pageA.locator(sel.stickerPanel)).toBeHidden({ timeout: 5_000 });

    // Sender renders a sticker bubble. The chat-view scope rules out
    // a stray match from any sidebar preview snippet rendered in
    // text form.
    const senderBubble = pageA
      .locator(sel.chatView)
      .locator(sel.messageRow, { hasText: glyph! })
      .first();
    await expect(senderBubble).toBeVisible({ timeout: 15_000 });

    // Receiver gets the same bubble across the wire. The webp asset
    // 404s in the default build → `onerror` swaps the img for a text
    // node carrying `alt` (the glyph), so `hasText: glyph` matches
    // either rendering branch deterministically.
    const receiverBubble = pageB
      .locator(sel.chatView)
      .locator(sel.messageRow, { hasText: glyph! })
      .first();
    await expect(receiverBubble).toBeVisible({ timeout: 30_000 });
  });

  test('search box with no matches surfaces the empty fallback', async ({ pageA }) => {
    await pageA
      .locator(sel.chatInputBar)
      .locator('button.chat-input-btn')
      .first()
      .click();
    await expect(pageA.locator(sel.stickerPanel)).toBeVisible();

    const initialCount = await pageA.locator(sel.stickerPanelItem).count();
    expect(initialCount).toBeGreaterThan(0);

    // The panel's search input is the only `input[type="search"]`
    // inside the panel chrome.
    const searchInput = pageA.locator(sel.stickerPanel).locator('input[type="search"]');
    await searchInput.fill('zzz-no-match-zzz');

    // Filter is purely a `to_lowercase().contains` against the glyph
    // string; `zzz...` never matches an emoji code point.
    await expect(pageA.locator(sel.stickerPanelItem)).toHaveCount(0);
    await expect(pageA.locator(sel.stickerPanel).locator('.sticker-panel-empty')).toBeVisible();
  });

  test('switching tab clears the search and repopulates the grid', async ({ pageA }) => {
    await pageA
      .locator(sel.chatInputBar)
      .locator('button.chat-input-btn')
      .first()
      .click();
    const panel = pageA.locator(sel.stickerPanel);
    await expect(panel).toBeVisible();

    // Plant a no-match search on the first tab so the empty branch is
    // active before the tab switch.
    const searchInput = panel.locator('input[type="search"]');
    await searchInput.fill('zzz-no-match-zzz');
    await expect(pageA.locator(sel.stickerPanelItem)).toHaveCount(0);

    // Click the second tab. Tabs are role=tab buttons inside the
    // tablist; we use nth(1) — switching to "Animals".
    const tabs = panel.locator('button[role="tab"]');
    await expect(tabs).toHaveCount(3);
    await tabs.nth(1).click();

    // Tab switch resets the search signal to "" (see
    // `sticker_panel.rs:151`) and the grid is repopulated.
    await expect(searchInput).toHaveValue('');
    const repopulatedCount = await pageA.locator(sel.stickerPanelItem).count();
    expect(repopulatedCount).toBeGreaterThan(0);
  });
});
