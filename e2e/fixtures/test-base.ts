/**
 * Extended Playwright `test` fixture for the WebRTC Chat suite.
 *
 * Provides:
 * - `server`     — file-scoped signaling server instance (`worker` scope keeps
 *                  it alive across tests within the same spec file).
 * - `pageA` / `pageB` — independent browser contexts simulating two users with
 *                  isolated cookies, localStorage, and IndexedDB.
 * - `pageC`     — third context, lazily allocated for multi-user scenarios.
 */

import { type BrowserContext, type Page, test as base } from '@playwright/test';

import { type ServerInstance, startServer } from './server.ts';

interface WorkerFixtures {
  server: ServerInstance;
}

interface TestFixtures {
  contextA: BrowserContext;
  contextB: BrowserContext;
  pageA: Page;
  pageB: Page;
  /** Lazily provisioned third context for multi-user scenarios. */
  contextC: BrowserContext;
  pageC: Page;
}

export const test = base.extend<TestFixtures, WorkerFixtures>({
  // ---- Worker-scoped: one server per spec file ----
  server: [
    async ({}, use) => {
      const instance = await startServer();
      try {
        await use(instance);
      } finally {
        // Surface logs on failure for easier debugging by attaching them to
        // any test that ran against this worker server.
        await instance.stop();
      }
    },
    { scope: 'worker' },
  ],

  // ---- Test-scoped: two clean browser contexts per test ----
  contextA: async ({ browser }, use) => {
    const ctx = await browser.newContext({
      ignoreHTTPSErrors: true,
      // Permission grants are inherited from project config but listed here
      // explicitly so tests that override one fixture remain consistent.
      permissions: ['microphone', 'camera', 'clipboard-read', 'clipboard-write', 'notifications'],
      // Block PWA service-worker registration — see
      // `playwright.config.ts` for rationale. `browser.newContext`
      // does NOT inherit the top-level `use.serviceWorkers`, so the
      // option has to be repeated per custom context fixture.
      serviceWorkers: 'block',
    });
    await use(ctx);
    await ctx.close();
  },

  contextB: async ({ browser }, use) => {
    const ctx = await browser.newContext({
      ignoreHTTPSErrors: true,
      permissions: ['microphone', 'camera', 'clipboard-read', 'clipboard-write', 'notifications'],
      serviceWorkers: 'block',
    });
    await use(ctx);
    await ctx.close();
  },

  contextC: async ({ browser }, use) => {
    const ctx = await browser.newContext({
      ignoreHTTPSErrors: true,
      permissions: ['microphone', 'camera', 'clipboard-read', 'clipboard-write', 'notifications'],
      serviceWorkers: 'block',
    });
    await use(ctx);
    await ctx.close();
  },

  pageA: async ({ contextA }, use) => {
    const page = await contextA.newPage();
    await use(page);
  },

  pageB: async ({ contextB }, use) => {
    const page = await contextB.newPage();
    await use(page);
  },

  pageC: async ({ contextC }, use) => {
    const page = await contextC.newPage();
    await use(page);
  },
});

export { expect } from '@playwright/test';
