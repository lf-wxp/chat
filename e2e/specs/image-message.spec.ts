/**
 * Image message E2E coverage (Wave P2-3).
 *
 * The composer's image button (`chat-input-bar` second icon) flips
 * `overlays.image` to `true`; the `ImagePicker` component watches
 * that signal and `.click()`s the hidden `<input type="file">` which
 * is permanently mounted in the DOM with
 * `data-testid="image-picker-input"`. We bypass the synthetic click
 * round-trip and feed bytes directly to the input via Playwright's
 * `setInputFiles`, matching the strategy used by
 * `file-transfer.spec.ts`.
 *
 * The receiver renders the bubble as `<img class="message-image"
 * data-testid="message-image">`. `width` / `height` are emitted from
 * the image's natural dimensions resolved before send — we ship a
 * 2x2 PNG (`assets/tiny.png`) so those attributes carry the expected
 * values on the cross-peer side, and we assert on them as a wire-
 * level integrity guard (the dims would be `0/0` if the natural-
 * dimension resolution path were skipped).
 *
 * Coverage:
 *   1. Send via file input → receiver renders `message-image` with
 *      the expected `data-image-width` / `data-image-height`.
 *   2. Receiver clicking the rendered thumbnail opens the
 *      `image-preview` overlay; pressing Escape closes it.
 *   3. Sender's own bubble also carries the image (round-trip
 *      symmetry — confirms the local optimistic append works).
 */

import * as path from 'node:path';
import { fileURLToPath } from 'node:url';

import { sel } from '../utils/selectors.ts';
import { establishConnection, registerAndLogin } from '../fixtures/helpers.ts';
import { expect, test } from '../fixtures/test-base.ts';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const ASSETS_DIR = path.resolve(__dirname, '..', 'assets');
const TINY_PNG = path.join(ASSETS_DIR, 'tiny.png');

test.describe('image message', () => {
  test.beforeEach(async ({ pageA, pageB, server }) => {
    await registerAndLogin(pageA, server, { hint: 'im-a' });
    const b = await registerAndLogin(pageB, server, { hint: 'im-b' });
    await establishConnection(pageA, pageB, b.username);
  });

  test('A sends a PNG, both sides render message-image with the natural dimensions', async ({
    pageA,
    pageB,
  }) => {
    // The hidden `<input type="file" accept="image/*">` is always
    // attached (it lives behind `style="display:none"` regardless of
    // overlay visibility). Feeding it directly bypasses the toolbar
    // toggle without losing the underlying `change` event the
    // component is wired to.
    const imageInput = pageA.locator(sel.imagePickerInput);
    await imageInput.waitFor({ state: 'attached', timeout: 5_000 });
    await imageInput.setInputFiles(TINY_PNG);

    // Sender's optimistic bubble appears first.
    const senderImg = pageA
      .locator(sel.chatView)
      .locator(sel.messageImage)
      .first();
    await expect(senderImg).toBeVisible({ timeout: 20_000 });

    // Receiver renders the same bubble across the wire. The wire
    // path is: image bytes via DataChannel → `send_image` →
    // `ImageRef { width, height, ... }` rebuilt on the receiver →
    // `<img data-image-width data-image-height>`. For our 2x2 PNG
    // both sides must agree on `2`.
    const receiverImg = pageB
      .locator(sel.chatView)
      .locator(sel.messageImage)
      .first();
    await expect(receiverImg).toBeVisible({ timeout: 30_000 });

    await expect(receiverImg).toHaveAttribute('data-image-width', '2');
    await expect(receiverImg).toHaveAttribute('data-image-height', '2');
    // The sender's own bubble should also carry the resolved dims —
    // catches a regression where the local optimistic path skips
    // `HtmlImageElement::natural_width` resolution.
    await expect(senderImg).toHaveAttribute('data-image-width', '2');
    await expect(senderImg).toHaveAttribute('data-image-height', '2');
  });

  test('clicking the received image opens the preview overlay; Escape dismisses it', async ({
    pageA,
    pageB,
  }) => {
    const imageInput = pageA.locator(sel.imagePickerInput);
    await imageInput.waitFor({ state: 'attached', timeout: 5_000 });
    await imageInput.setInputFiles(TINY_PNG);

    const receiverImg = pageB
      .locator(sel.chatView)
      .locator(sel.messageImage)
      .first();
    await expect(receiverImg).toBeVisible({ timeout: 30_000 });

    // The preview overlay starts hidden — `<Show when=url.is_some()>`
    // gates it on a signal initialised to `None`.
    await expect(pageB.locator(sel.imagePreview)).toHaveCount(0);

    await receiverImg.click();

    // The overlay mounts once `cbs.open_image` runs (the bubble's
    // on:click flips the parent `ChatView`'s preview signal).
    await expect(pageB.locator(sel.imagePreview)).toBeVisible({ timeout: 5_000 });

    // Escape dismisses the overlay (the component listens on its own
    // keydown handler).
    await pageB.locator(sel.imagePreview).press('Escape');
    await expect(pageB.locator(sel.imagePreview)).toHaveCount(0, { timeout: 5_000 });
  });
});
