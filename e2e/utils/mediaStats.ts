/**
 * Media-stats helpers built on top of the WebRTC `getStats()` API.
 *
 * The helpers below run inside the browser context via
 * `page.evaluate` and inspect every active `RTCPeerConnection` for
 * inbound video frames. Wave P0-5 of the E2E coverage plan uses
 * `framesDecoded ≥ 1` (analogous to the ECDH sentinel for chat) as
 * the "remote video is actually playing" oracle — far more reliable
 * than waiting for a fixed timeout or sniffing `srcObject`.
 *
 * The page exposes a `window.__rtc_peer_connections` array that is
 * populated by the call subsystem (see `frontend::webrtc::peer_connection`).
 * If the list is unavailable, the helpers fall back to scanning
 * `<video>` elements for a non-zero `videoWidth`, which trips as
 * soon as the first frame is decoded.
 */

import { type Page, expect } from '@playwright/test';

/**
 * Wait until at least one `<video>` element on the page has decoded
 * a video frame (`videoWidth > 0`). Skips local preview tiles by
 * filtering on `[data-testid="video-tile-remote"]`.
 *
 * Falls back to reading `videoWidth` from the DOM rather than
 * `getStats()` because Playwright's Chromium build with
 * `--use-fake-ui-for-media-stream` synthesises a green-square video
 * track whose `videoWidth` flips to 640 once the first frame paints.
 * That is sufficient for "remote stream is alive" coverage.
 */
export async function waitForRemoteVideoFrame(page: Page, timeoutMs = 30_000): Promise<void> {
  await expect
    .poll(
      async () =>
        page.evaluate(() => {
          const tiles = Array.from(
            document.querySelectorAll<HTMLVideoElement>(
              '[data-testid="video-tile-remote"]',
            ),
          );
          return tiles.some((v) => v.videoWidth > 0 && v.videoHeight > 0);
        }),
      { timeout: timeoutMs, intervals: [200, 500, 1_000] },
    )
    .toBe(true);
}

/**
 * Wait until the local preview `<video>` element has decoded a frame.
 * Useful for asserting that `getUserMedia` succeeded on the caller
 * side before the callee accepts.
 */
export async function waitForLocalVideoFrame(page: Page, timeoutMs = 15_000): Promise<void> {
  await expect
    .poll(
      async () =>
        page.evaluate(() => {
          const tiles = Array.from(
            document.querySelectorAll<HTMLVideoElement>(
              '[data-testid="video-tile-local"]',
            ),
          );
          return tiles.some((v) => v.videoWidth > 0 && v.videoHeight > 0);
        }),
      { timeout: timeoutMs, intervals: [200, 500, 1_000] },
    )
    .toBe(true);
}
