/**
 * Voice message E2E coverage (Wave P2-2).
 *
 * The composer's mic button (`chat-input-bar` fourth icon — Smile,
 * Image, Paperclip, Mic) toggles `overlays.voice`. The
 * `VoiceRecorder` overlay then renders with a state machine:
 *
 *   Idle → click record → Starting → MediaRecorder onstart →
 *   Recording → click send → Stopping → MediaRecorder onstop →
 *   ChatManager.send_voice → overlay closes.
 *
 * Chromium runs with `--use-fake-device-for-media-stream` (see
 * `playwright.config.ts`) so `getUserMedia({audio:true})` returns a
 * synthetic audio stream — no real microphone required.
 *
 * Coverage:
 *   1. Mic button opens the recorder; cancel button dismisses it
 *      without dispatching any chat message.
 *   2. Recorder transitions through Starting → Recording (observable
 *      via the `data-state` attribute on `voice-recorder__status`).
 *   3. Stop-and-send produces a `message-voice` bubble on both the
 *      sender and the receiver — confirms the wire-level voice
 *      delivery path is intact.
 */

import { sel } from '../utils/selectors.ts';
import { establishConnection, registerAndLogin } from '../fixtures/helpers.ts';
import { expect, test } from '../fixtures/test-base.ts';
import type { Page } from '@playwright/test';

/** Click the mic button in the composer to open `voice-recorder`. */
async function openRecorder(page: Page): Promise<void> {
  // The composer renders four chat-input-btn icons in order:
  //   index 0 → sticker (Smile)
  //   index 1 → image (Image)
  //   index 2 → file (Paperclip)
  //   index 3 → voice (Mic)
  // Pick the mic toggle by index so we don't depend on i18n labels.
  await page
    .locator(sel.chatInputBar)
    .locator('button.chat-input-btn')
    .nth(3)
    .click();
  await expect(page.locator(sel.voiceRecorder)).toBeVisible({ timeout: 5_000 });
}

test.describe('voice message', () => {
  test.beforeEach(async ({ pageA, pageB, server }) => {
    await registerAndLogin(pageA, server, { hint: 'vm-a' });
    const b = await registerAndLogin(pageB, server, { hint: 'vm-b' });
    await establishConnection(pageA, pageB, b.username);
  });

  test('mic button opens the recorder and cancel dismisses it without sending', async ({
    pageA,
    pageB,
  }) => {
    // Snapshot pre-recorder receiver bubble count so we can prove
    // cancel does not produce any wire frame.
    const before = await pageB.locator(sel.messageRow).count();

    await openRecorder(pageA);

    // Idle state — record button is mounted, send button is not.
    await expect(pageA.locator(sel.voiceRecorderRecord)).toBeVisible();
    await expect(pageA.locator(sel.voiceRecorderSend)).toHaveCount(0);

    // Cancel — closes the overlay without dispatching anything.
    await pageA.locator(sel.voiceRecorderCancel).click();
    await expect(pageA.locator(sel.voiceRecorder)).toBeHidden({ timeout: 5_000 });

    // No wire frame produced; receiver's bubble count is unchanged
    // after a settle window.
    await pageA.waitForTimeout(800);
    await expect(pageB.locator(sel.messageRow)).toHaveCount(before);
  });

  test('record + stop dispatches a voice clip the receiver renders as message-voice', async ({
    pageA,
    pageB,
  }) => {
    await openRecorder(pageA);

    // The status element carries a `data-state` attribute (idle /
    // starting / recording / stopping) we can poll deterministically.
    const status = pageA.locator(sel.voiceRecorder).locator('.voice-recorder__status');
    await expect(status).toHaveAttribute('data-state', 'idle');

    // Click record. The state goes through Starting → Recording
    // once MediaRecorder.start fires its `onstart` event.
    await pageA.locator(sel.voiceRecorderRecord).click();
    await expect(status).toHaveAttribute('data-state', 'recording', {
      timeout: 10_000,
    });

    // Allow enough recording time for MediaRecorder to flush at
    // least one chunk + a non-empty waveform aggregate. The
    // recorder's internal RAF loop samples at ~30 Hz; 500 ms is
    // > 15 samples which is well past the empty-clip guard.
    await pageA.waitForTimeout(600);

    // Send → state flips to Stopping then the overlay closes once
    // the manager dispatches `send_voice`.
    await pageA.locator(sel.voiceRecorderSend).click();
    await expect(pageA.locator(sel.voiceRecorder)).toBeHidden({ timeout: 15_000 });

    // Sender's own bubble appears optimistically.
    const senderClip = pageA
      .locator(sel.chatView)
      .locator(sel.messageVoice)
      .first();
    await expect(senderClip).toBeVisible({ timeout: 15_000 });

    // Receiver renders the voice bubble across the wire. Voice clips
    // are bigger than text frames (Opus blob) so we give the
    // datachannel a longer window.
    const receiverClip = pageB
      .locator(sel.chatView)
      .locator(sel.messageVoice)
      .first();
    await expect(receiverClip).toBeVisible({ timeout: 30_000 });
  });
});
