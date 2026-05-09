/**
 * Playwright configuration for the WebRTC Chat E2E test suite.
 *
 * Design notes:
 * - Chromium only (per Browser Compatibility NFR; Firefox/Safari are not required).
 * - Headless by default; CI runs the same.
 * - `webServer` is intentionally NOT used here. Each test spec spins up its own
 *   signaling-server child process via the `serverFixture` so that suites can
 *   run in parallel on independent random ports without sharing in-memory state.
 * - Tests within a single spec file share the same server (sequential by file)
 *   while different spec files may execute in parallel.
 * - WebRTC + getUserMedia is enabled via fake media flags so that calls and
 *   voice/image picker flows do not require real hardware.
 */

import { defineConfig, devices } from '@playwright/test';

const isCI = !!process.env.CI;

export default defineConfig({
  testDir: './specs',
  // Each spec file owns its server instance; within a file run sequentially
  // because the server uses in-memory state shared across the file's tests.
  fullyParallel: false,
  // Different spec files can still run in parallel as separate workers.
  workers: isCI ? 2 : undefined,
  forbidOnly: isCI,
  retries: isCI ? 2 : 1,
  timeout: 60_000,
  expect: {
    timeout: 15_000,
  },
  reporter: isCI
    ? [['html', { open: 'never' }], ['list'], ['github']]
    : [['html', { open: 'never' }], ['list']],
  outputDir: './test-results',
  use: {
    baseURL: 'http://127.0.0.1:3000',
    trace: 'retain-on-failure',
    screenshot: 'only-on-failure',
    video: 'retain-on-failure',
    actionTimeout: 10_000,
    navigationTimeout: 15_000,
    // Block service-worker registration in every browser context.
    //
    // The application ships a PWA service worker (`public/sw.js`) that
    // uses `stale-while-revalidate` for navigations and `cache-first`
    // for static assets. Under E2E that behaviour is actively harmful:
    // tests that reload the page (persistence, theme-a11y,
    // auth-session-restored, …) would race the installed SW and
    // occasionally receive stale HTML / stale WASM after a fresh
    // `cargo make e2e-build`, and the "update available" banner could
    // appear mid-test and obstruct clicks. Blocking SW registration
    // at the context level eliminates those failure modes without
    // affecting production / dev, which never load this config.
    //
    // A dedicated PWA-offline spec (not in the Req-16 suite) can opt
    // back in by overriding this field via `test.use({ serviceWorkers:
    // 'allow' })` at the describe level.
    serviceWorkers: 'block',
  },
  projects: [
    {
      name: 'chromium',
      use: {
        ...devices['Desktop Chrome'],
        launchOptions: {
          args: [
            '--use-fake-device-for-media-stream',
            '--use-fake-ui-for-media-stream',
            '--allow-insecure-localhost',
            '--disable-features=WebRtcHideLocalIpsWithMdns',
          ],
        },
        permissions: ['microphone', 'camera', 'clipboard-read', 'clipboard-write', 'notifications'],
      },
    },
  ],
});
