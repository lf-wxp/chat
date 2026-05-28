/**
 * Message persistence and connection-recovery E2E tests.
 *
 * Maps to: Requirement 16.5 (Message Persistence & Recovery).
 */

import { sel } from '../utils/selectors.ts';
import {
  establishConnection,
  registerAndLogin,
  sendAndVerifyMessage,
} from '../fixtures/helpers.ts';
import { waitForAppShell } from '../utils/wait-helpers.ts';
import { expect, test } from '../fixtures/test-base.ts';

test.describe('persistence', () => {
  test('chat history is restored from IndexedDB after page refresh', async ({
    pageA,
    pageB,
    server,
  }) => {
    await registerAndLogin(pageA, server, { hint: 'a' });
    const b = await registerAndLogin(pageB, server, { hint: 'b' });
    await establishConnection(pageA, pageB, b.username);

    const tag = Date.now().toString(36);
    const messages = [`hist-${tag}-0`, `hist-${tag}-1`, `hist-${tag}-2`];
    for (const m of messages) {
      await sendAndVerifyMessage(pageA, pageB, m);
    }

    // The app debounces conversation-metadata writes to localStorage
    // by 100 ms (see `PERSIST_DEBOUNCE_MS` in `state/mod.rs`). Reloading
    // immediately can drop the last write, leaving an empty sidebar on
    // boot. Wait for the localStorage skeleton to be flushed first.
    await pageA.waitForFunction(
      () => {
        const raw = window.localStorage.getItem('conversations');
        if (!raw) return false;
        try {
          const parsed = JSON.parse(raw) as Array<unknown>;
          return Array.isArray(parsed) && parsed.length > 0;
        } catch {
          return false;
        }
      },
      undefined,
      { timeout: 10_000 },
    );

    // Messages are persisted to IndexedDB via fire-and-forget
    // `spawn_local` calls. The localStorage write above only covers
    // the conversation skeleton — the actual message records may still
    // be in-flight. Wait until IndexedDB contains all 3 messages so
    // `load_history` after reload finds them reliably.
    await pageA.waitForFunction(
      (expectedCount: number) => {
        return new Promise<boolean>((resolve) => {
          const req = indexedDB.open('chat_frontend');
          req.onsuccess = () => {
            const db = req.result;
            if (!db.objectStoreNames.contains('messages')) {
              resolve(false);
              return;
            }
            const tx = db.transaction('messages', 'readonly');
            const store = tx.objectStore('messages');
            const countReq = store.count();
            countReq.onsuccess = () => {
              resolve(countReq.result >= expectedCount);
            };
            countReq.onerror = () => resolve(false);
          };
          req.onerror = () => resolve(false);
        });
      },
      messages.length,
      { timeout: 10_000 },
    );

    // Refresh A and assert the messages are restored.
    await pageA.reload();
    await waitForAppShell(pageA);

    // The sidebar conversation item materialises asynchronously after
    // the auth bootstrap restores the conversation list from
    // IndexedDB. Wait for it to appear, then click to open the chat
    // view. `load_history` (chat manager) hydrates the message list
    // on conversation switch.
    const sidebarItem = pageA.locator(sel.sidebarConversationItem).first();
    await expect(sidebarItem).toBeVisible({ timeout: 15_000 });
    await sidebarItem.click();
    await expect(pageA.locator(sel.chatView)).toBeVisible({ timeout: 15_000 });

    for (const m of messages) {
      await expect(pageA.locator(sel.messageRow, { hasText: m })).toBeVisible({
        timeout: 15_000,
      });
    }
  });
});
