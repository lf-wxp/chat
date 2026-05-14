/**
 * E2EE wire-level coverage (Wave P1-6).
 *
 * Locks down the encryption contract that
 * `frontend/src/webrtc/encryption.rs` (AES-256-GCM, 12-byte nonce
 * prepended, ECDH-derived shared key via HKDF) implements at the
 * DataChannel layer.
 *
 *   1. **Wire frames are ciphertext** — every outbound DataChannel
 *      send after the ECDH handshake completes is asserted to NOT
 *      contain the plaintext message string. We monkey-patch
 *      `RTCDataChannel.prototype.send` from `addInitScript` and
 *      record every call into `window.__dcSends__`.
 *   2. **Tampered frame is dropped** — after the ECDH handshake we
 *      flip a single byte in the next outbound frame from the
 *      sender side; the receiver's decrypt step rejects it (AES-GCM
 *      auth tag fails) and the bubble never lands on B. The send
 *      reaches B as raw bytes (DataChannel still moves them) but
 *      the decrypt error is swallowed inside the WebRTC layer with
 *      a console warning, so the test asserts on the absence of the
 *      bubble at B AND the presence of the original (untouched)
 *      bubble on A.
 *
 * --- Scope notes ---
 * Plan §3 P1-6 originally listed three tests, the third being
 * "key rotation on re-invite". The current frontend does not
 * implement key rotation: each ECDH handshake runs once per
 * pair-of-peers per page lifetime, and a re-invite path that
 * forces a fresh handshake without a full WebSocket teardown does
 * not exist as a user-reachable flow. Tracking as feature gap G16.
 */

import { expect, test } from '../fixtures/test-base.ts';
import {
  establishConnection,
  registerAndLogin,
  sendAndVerifyMessage,
} from '../fixtures/helpers.ts';
import { sel } from '../utils/selectors.ts';
import type { Page } from '@playwright/test';

/**
 * Install a monkey-patch on `RTCDataChannel.prototype.send` that
 * records every outbound buffer into `window.__dcSends__` (an
 * `Array<Uint8Array>`). Must be called BEFORE any
 * `RTCPeerConnection` is constructed by the application — we use
 * `addInitScript` to inject it on every navigation.
 *
 * Optionally, when `corruptOnce` is true, the FIRST send larger
 * than `corruptThresholdBytes` after `corruptArmed` is set to true
 * has its 30th byte (well past the 12-byte AES-GCM nonce so it
 * lands inside the actual ciphertext) flipped before forwarding to
 * the real `send`. The flip is one-shot — subsequent sends pass
 * through untouched.
 */
async function installDataChannelHook(page: Page): Promise<void> {
  await page.addInitScript(() => {
    interface WindowWithHook extends Window {
      __dcSends__?: Uint8Array[];
      __dcSendOriginal__?: typeof RTCDataChannel.prototype.send;
      __dcCorruptArmed__?: boolean;
      __dcCorruptThreshold__?: number;
      __dcCorruptHits__?: number;
    }
    const w = window as WindowWithHook;
    if (w.__dcSendOriginal__) return;
    w.__dcSends__ = [];
    w.__dcCorruptArmed__ = false;
    // Skip small frames (heartbeat, MessageAck, MessageRead, typing
    // — all observed empirically at ≤ 32 bytes). The threshold
    // filters those out so the one-shot tamper lands on the actual
    // ChatText envelope (~70 bytes for a 10-character message).
    w.__dcCorruptThreshold__ = 50;
    w.__dcCorruptHits__ = 0;
    const original = RTCDataChannel.prototype.send;
    w.__dcSendOriginal__ = original;
    RTCDataChannel.prototype.send = function (
      this: RTCDataChannel,
      data: string | Blob | ArrayBuffer | ArrayBufferView,
    ): void {
      let recordedLen = -1;
      try {
        if (data instanceof ArrayBuffer) {
          w.__dcSends__!.push(new Uint8Array(data.slice(0)));
          recordedLen = data.byteLength;
        } else if (ArrayBuffer.isView(data)) {
          // Copy out so the test inspector sees a stable snapshot.
          const view = data as ArrayBufferView;
          const copy = new Uint8Array(view.byteLength);
          copy.set(new Uint8Array(view.buffer, view.byteOffset, view.byteLength));
          w.__dcSends__!.push(copy);
          recordedLen = view.byteLength;
        } else {
          // Diagnostic: log the type so we know what to handle.
          // eslint-disable-next-line no-console
          console.log('[hook] unhandled send type', typeof data, data);
        }
      } catch {
        /* hook recording failure must not break the app */
      }

      // Optional one-shot tamper.
      if (
        w.__dcCorruptArmed__ &&
        recordedLen > w.__dcCorruptThreshold__!
      ) {
        let buf: Uint8Array;
        if (data instanceof ArrayBuffer) {
          buf = new Uint8Array(data.slice(0));
        } else {
          const view = data as ArrayBufferView;
          buf = new Uint8Array(view.byteLength);
          buf.set(new Uint8Array(view.buffer, view.byteOffset, view.byteLength));
        }
        // Flip a bit deep in the payload (well after the 12-byte
        // GCM nonce) so the AES-GCM auth tag verification fails.
        buf[28] = buf[28] ^ 0xff;
        w.__dcCorruptArmed__ = false;
        w.__dcCorruptHits__! += 1;
        return (original as (data: ArrayBuffer) => void).call(this, buf.buffer as ArrayBuffer);
      }
      return (original as (data: string | Blob | ArrayBuffer | ArrayBufferView) => void).call(this, data);
    };
  });
}

async function getRecordedSends(page: Page): Promise<Uint8Array[]> {
  return page.evaluate(() => {
    interface WindowWithHook extends Window {
      __dcSends__?: Uint8Array[];
    }
    const arr = (window as WindowWithHook).__dcSends__ ?? [];
    // Convert to plain arrays for serialisation across the bridge,
    // then convert back on this side.
    return arr.map((u) => Array.from(u)) as unknown as Uint8Array[];
  });
}

async function armCorruption(page: Page): Promise<void> {
  await page.evaluate(() => {
    interface WindowWithHook extends Window {
      __dcCorruptArmed__?: boolean;
    }
    (window as WindowWithHook).__dcCorruptArmed__ = true;
  });
}

async function corruptionFired(page: Page): Promise<number> {
  return page.evaluate(() => {
    interface WindowWithHook extends Window {
      __dcCorruptHits__?: number;
    }
    return (window as WindowWithHook).__dcCorruptHits__ ?? 0;
  });
}

/** Search every recorded outbound buffer for the literal UTF-8
 *  encoding of `needle`. Returns `true` iff at least one frame
 *  contains the needle (i.e. the plaintext leaked). */
function anyFrameContains(frames: Uint8Array[], needle: string): boolean {
  const enc = new TextEncoder().encode(needle);
  for (const frame of frames) {
    const buf = frame instanceof Uint8Array ? frame : new Uint8Array(frame);
    outer: for (let i = 0; i + enc.length <= buf.length; i += 1) {
      for (let j = 0; j < enc.length; j += 1) {
        if (buf[i + j] !== enc[j]) continue outer;
      }
      return true;
    }
  }
  return false;
}

test.describe('E2EE wire-level guarantees', () => {
  test('outbound DataChannel frames are ciphertext (no plaintext leak)', async ({
    pageA,
    pageB,
    server,
  }) => {
    await installDataChannelHook(pageA);
    await registerAndLogin(pageA, server, { hint: 'e2ee-ct-a' });
    const userB = await registerAndLogin(pageB, server, { hint: 'e2ee-ct-b' });
    await establishConnection(pageA, pageB, userB.username);

    const secret = `pln-secret-${Math.random().toString(36).slice(2, 10)}`;
    await sendAndVerifyMessage(pageA, pageB, secret);

    // Inspect every byte ever sent over A's DataChannel since the
    // start of the page. The literal `secret` UTF-8 bytes must NOT
    // appear in any frame — they would only appear if the encrypt
    // path was bypassed (e.g. plaintext fallback) or if the
    // discriminator-byte framing leaked the body.
    const frames = await getRecordedSends(pageA);
    expect(frames.length).toBeGreaterThan(0);
    expect(anyFrameContains(frames, secret)).toBe(false);
  });

  test('a tampered DataChannel frame is rejected by the receiver', async ({
    pageA,
    pageB,
    server,
  }) => {
    await installDataChannelHook(pageA);
    await registerAndLogin(pageA, server, { hint: 'e2ee-tp-a' });
    const userB = await registerAndLogin(pageB, server, { hint: 'e2ee-tp-b' });
    await establishConnection(pageA, pageB, userB.username);

    // Baseline: a clean send round-trips, anchoring "the link
    // works at this point".
    await sendAndVerifyMessage(pageA, pageB, 'baseline-ok');

    // Arm the one-shot byte-flip; the next non-trivial outbound
    // chunk has a deep-payload byte XOR'd with 0xff, which makes
    // the AES-GCM auth tag verification fail on the receiver.
    await armCorruption(pageA);

    const tamperedText = `tamper-${Math.random().toString(36).slice(2, 10)}`;
    const senderRow = pageA.locator(sel.messageRow, { hasText: tamperedText }).first();
    const receiverRow = pageB.locator(sel.messageRow, { hasText: tamperedText }).first();
    const textarea = pageA.locator(sel.chatInputTextarea);
    await textarea.fill(tamperedText);
    await textarea.press('Enter');

    // The sender always renders its own bubble locally (via
    // `push_outgoing` — independent of wire success).
    await expect(senderRow).toBeVisible({ timeout: 5_000 });

    // Confirm the corruption hook actually fired at least once.
    await expect.poll(async () => corruptionFired(pageA), { timeout: 5_000 }).toBeGreaterThan(0);

    // The receiver MUST NOT render the bubble, because the
    // ciphertext failed AES-GCM authentication and was dropped.
    // Use a tight negative window — if the tampered frame were
    // accepted it would land inside the normal sub-second
    // delivery latency.
    await expect(receiverRow).toHaveCount(0, { timeout: 5_000 });

    // Recovery sanity check: a follow-up send (no corruption
    // armed) still gets through. This rules out the test silently
    // bricking the channel.
    await sendAndVerifyMessage(pageA, pageB, 'recovery-after-tamper');
  });
});
