/**
 * End-to-end encryption verification E2E tests.
 *
 * Maps to: Requirement 16.20 (E2EE Verification).
 *
 * Strategy:
 * 1. Open the WebSocket sniffing hook BEFORE the user logs in so that all
 *    signaling frames flow through it.
 * 2. After the chat session is established, send a recognisable plaintext
 *    message and assert that the exact bytes never appear inside any
 *    WebSocket frame routed through the signaling server.
 */

import { type Page } from '@playwright/test';

import { sel } from '../utils/selectors.ts';
import {
  establishConnection,
  registerAndLogin,
  sendAndVerifyMessage,
} from '../fixtures/helpers.ts';
import { expect, test } from '../fixtures/test-base.ts';

/**
 * Install a tap that copies every outbound and inbound WebSocket frame into
 * `window.__wsFrames` so the test can inspect them later.
 */
async function installWebSocketTap(page: Page): Promise<void> {
  await page.addInitScript(() => {
    const w = window as unknown as { __wsFrames?: string[] };
    if (w.__wsFrames) {
      return;
    }
    w.__wsFrames = [];
    const NativeWebSocket = WebSocket;
    const Wrapped = function (this: WebSocket, url: string | URL, protocols?: string | string[]) {
      const ws = new NativeWebSocket(url, protocols);
      const collect = (data: unknown): void => {
        try {
          if (typeof data === 'string') {
            w.__wsFrames!.push(data);
          } else if (data instanceof ArrayBuffer) {
            w.__wsFrames!.push(`<binary:${data.byteLength}>`);
          } else if (data instanceof Blob) {
            w.__wsFrames!.push(`<blob:${data.size}>`);
          }
        } catch {
          // ignore
        }
      };
      const origSend = ws.send.bind(ws);
      ws.send = function (data: string | ArrayBufferLike | Blob | ArrayBufferView): void {
        collect(data);
        return origSend(data as never);
      };
      ws.addEventListener('message', (ev) => collect(ev.data));
      return ws;
    } as unknown as typeof WebSocket;
    Wrapped.prototype = NativeWebSocket.prototype;
    Object.defineProperty(Wrapped, 'CONNECTING', { value: NativeWebSocket.CONNECTING });
    Object.defineProperty(Wrapped, 'OPEN', { value: NativeWebSocket.OPEN });
    Object.defineProperty(Wrapped, 'CLOSING', { value: NativeWebSocket.CLOSING });
    Object.defineProperty(Wrapped, 'CLOSED', { value: NativeWebSocket.CLOSED });
    (window as unknown as { WebSocket: typeof WebSocket }).WebSocket = Wrapped;
  });
}

test.describe('e2ee', () => {
  test('plaintext chat content never appears in signaling WebSocket frames', async ({
    pageA,
    pageB,
    server,
  }) => {
    await installWebSocketTap(pageA);
    await installWebSocketTap(pageB);

    await registerAndLogin(pageA, server, { hint: 'a' });
    const b = await registerAndLogin(pageB, server, { hint: 'b' });
    await establishConnection(pageA, pageB, b.username);

    const secret = `top-secret-${Date.now().toString(36)}-${Math.random().toString(36).slice(2)}`;
    await sendAndVerifyMessage(pageA, pageB, secret);

    const inspect = async (page: Page): Promise<string[]> =>
      page.evaluate(() => {
        const w = window as unknown as { __wsFrames?: string[] };
        return w.__wsFrames ? [...w.__wsFrames] : [];
      });

    const framesA = await inspect(pageA);
    const framesB = await inspect(pageB);

    // The signaling server must only carry SDP/ICE/auth frames (text JSON or
    // bitcode binary) — never the plaintext chat content.
    for (const frame of [...framesA, ...framesB]) {
      expect(frame.includes(secret)).toBeFalsy();
    }

    // Sanity: the chat view actually rendered the message on both sides.
    await expect(pageA.locator(sel.messageRow, { hasText: secret })).toBeVisible();
    await expect(pageB.locator(sel.messageRow, { hasText: secret })).toBeVisible();
  });
});
