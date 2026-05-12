/**
 * Persistence — extended coverage (Wave P1-2).
 *
 * Builds on `persistence.spec.ts` (which already covers the basic
 * "send → refresh → see history" round-trip) with three additional
 * regression guards around what survives a full page reload:
 *
 *   1. **Multi-conversation hydration** — A holds two parallel direct
 *      conversations (with B and with C) and after refresh both
 *      conversations are restored independently with their own
 *      message histories. The plain test exists so the IDB
 *      `messages` store's `by_conv_ts` index is exercised end-to-end
 *      under realistic conditions.
 *   2. **Conversation `pinned` flag persists** — A pins the
 *      conversation with B → reload → the row is rendered in the
 *      `sidebar-section-pinned` section AND `data-pinned="true"` on
 *      the row. This anchors the IDB `conversation_flags` store and
 *      the localStorage skeleton flush together.
 *   3. **Conversation `archived` flag persists** — A archives the
 *      conversation with B → reload → the row appears under
 *      `sidebar-section-archived` (collapsible) with
 *      `data-archived="true"`. The Archived section is collapsed by
 *      default so we have to expand it before asserting visibility.
 *
 * --- Scope note / deferred sub-features ---
 * Plan §3 P1-2 originally listed two more tests that turned out to
 * require product work first; they are tracked in §2.1 as G9/G10:
 *
 *   * Cross-tab sync via BroadcastChannel — the app does not use
 *     BroadcastChannel, StorageEvent, or any other cross-tab
 *     primitive. Two tabs of the same user diverge silently. This
 *     is feature gap G9.
 *   * Message-id deduplication on double-deliver —
 *     `chat::manager::inbound::push_incoming` does not check whether
 *     `msg.id` already exists in the conversation's `messages`
 *     `RwSignal<Vec<ChatMessage>>` before appending. The IDB write
 *     is a `put` so the persisted store is dedup-safe by primary key,
 *     but the in-memory list (and hence the rendered bubble list)
 *     would double the bubble. This is feature gap G10.
 *
 * Large-scroll-back after refresh — the original P1-2 ask — is left
 * to Wave P1-3 `scroll.spec.ts`, which is the natural home for
 * virtualized-scroll assertions.
 */

import { expect, test } from '../fixtures/test-base.ts';
import {
  establishConnection,
  registerAndLogin,
  sendAndVerifyMessage,
} from '../fixtures/helpers.ts';
import { sel } from '../utils/selectors.ts';
import { waitForAppShell } from '../utils/wait-helpers.ts';
import type  { Page } from '@playwright/test';

/**
 * Wait until the localStorage `conversations` skeleton has been
 * flushed for `page`. The chat app's `AppState` debounces conversation
 * metadata writes by 100 ms, so refreshing immediately after the last
 * action would race the writer. We poll the JSON until the expected
 * number of conversations is durable.
 */
async function waitForConversationsFlushed(page: Page, atLeast: number): Promise<void> {
  await page.waitForFunction(
    (n) => {
      const raw = window.localStorage.getItem('conversations');
      if (!raw) return false;
      try {
        const parsed = JSON.parse(raw) as Array<unknown>;
        return Array.isArray(parsed) && parsed.length >= n;
      } catch {
        return false;
      }
    },
    atLeast,
    { timeout: 10_000 },
  );
}

/**
 * Poll IndexedDB `chat_frontend > conversation_flags` from the browser
 * context until at least one row with the requested `flag` set to
 * `true` is durable. Guards against the 100 ms
 * `PERSIST_DEBOUNCE_MS` in `state/mod.rs`: pin / archive writes are
 * coalesced on the trailing edge of the debounce window, so a refresh
 * immediately after a click would race the writer and boot with the
 * pre-action state.
 *
 * We deliberately do NOT pass a version number to `indexedDB.open`.
 * Doing so without an `onupgradeneeded` handler would create empty
 * object stores. Instead we open with the implicit version (matching
 * whatever the application's own schema runner already established)
 * and skip the result if the `conversation_flags` store is not yet
 * present — this can happen briefly on a fresh reload when the app's
 * own schema upgrade hasn't completed yet.
 */
async function waitForConversationFlagSet(
  page: Page,
  flag: 'pinned' | 'archived' | 'muted',
): Promise<void> {
  const result = await page.evaluate(
    async (flagName: string) => {
      const deadline = Date.now() + 15_000;
      while (Date.now() < deadline) {
        const db = await new Promise<IDBDatabase | null>((resolve) => {
          const req = window.indexedDB.open('chat_frontend');
          req.onsuccess = () => resolve(req.result);
          req.onerror = () => resolve(null);
          req.onblocked = () => resolve(null);
        });
        if (db && Array.from(db.objectStoreNames).includes('conversation_flags')) {
          const rows = await new Promise<Record<string, unknown>[]>((resolve) => {
            try {
              const tx = db.transaction('conversation_flags', 'readonly');
              const req = tx.objectStore('conversation_flags').getAll();
              req.onsuccess = () => {
                resolve((req.result as Record<string, unknown>[]) ?? []);
              };
              req.onerror = () => resolve([]);
            } catch {
              resolve([]);
            }
          });
          db.close();
          if (rows.some((r) => r[flagName] === true)) {
            return { ok: true, rows };
          }
        } else if (db) {
          db.close();
        }
        await new Promise((r) => setTimeout(r, 150));
      }
      return { ok: false, rows: null };
    },
    flag,
  );
  if (!result.ok) {
    throw new Error(`IDB conversation_flags never saw ${flag}=true (rows=${JSON.stringify(result.rows)})`);
  }
}

test.describe('persistence — extended', () => {
  test('two parallel direct conversations are independently restored', async ({
    pageA,
    pageB,
    pageC,
    server,
  }) => {
    await registerAndLogin(pageA, server, { hint: 'pe-mult-a' });
    const userB = await registerAndLogin(pageB, server, { hint: 'pe-mult-b' });
    const userC = await registerAndLogin(pageC, server, { hint: 'pe-mult-c' });

    await establishConnection(pageA, pageB, userB.username);
    await establishConnection(pageA, pageC, userC.username);

    // Send a distinct message on each conversation so the sidebar
    // row carries a conversation-specific last-message preview we
    // can key our assertion on. `establishConnection` leaves A
    // active-focused on the just-connected peer, so we explicitly
    // switch back to B before the first send.
    const tag = Date.now().toString(36);
    const msgB = `hydrate-${tag}-talkB`;
    const msgC = `hydrate-${tag}-talkC`;

    await pageA
      .locator(`${sel.sidebarConversationItem}:has-text("${userB.username}")`)
      .first()
      .click();
    await sendAndVerifyMessage(pageA, pageB, msgB);

    await pageA
      .locator(`${sel.sidebarConversationItem}:has-text("${userC.username}")`)
      .first()
      .click();
    await sendAndVerifyMessage(pageA, pageC, msgC);

    await waitForConversationsFlushed(pageA, 2);

    await pageA.reload();
    await waitForAppShell(pageA);

    // Both conversations reappear as independent sidebar entries.
    const itemB = pageA
      .locator(`${sel.sidebarConversationItem}:has-text("${userB.username}")`)
      .first();
    const itemC = pageA
      .locator(`${sel.sidebarConversationItem}:has-text("${userC.username}")`)
      .first();
    await expect(itemB).toBeVisible({ timeout: 15_000 });
    await expect(itemC).toBeVisible({ timeout: 15_000 });

    // The two rows resolve to distinct sidebar items — checking the
    // conversation count rules out the row being a single shared
    // entry that happens to satisfy both `:has-text` predicates.
    const allItems = pageA.locator(sel.sidebarConversationItem);
    await expect(allItems).toHaveCount(2);

    // Open B's thread and confirm the IDB-backed history loader
    // hydrates msgB on conversation switch (Req 16.5.2).
    await itemB.click();
    await expect(pageA.locator(sel.chatView)).toBeVisible();
    await expect(pageA.locator(sel.messageRow, { hasText: msgB })).toBeVisible({
      timeout: 15_000,
    });
  });

  test('pinned flag is persisted and the row is rendered in the pinned section after reload', async ({
    pageA,
    pageB,
    server,
  }) => {
    await registerAndLogin(pageA, server, { hint: 'pe-pin-a' });
    const userB = await registerAndLogin(pageB, server, { hint: 'pe-pin-b' });
    await establishConnection(pageA, pageB, userB.username);

    // Send one message so the conversation is fully materialised
    // (sidebar entry + IDB row + localStorage skeleton).
    const tag = Date.now().toString(36);
    await sendAndVerifyMessage(pageA, pageB, `pin-${tag}`);

    const itemBeforePin = pageA
      .locator(`${sel.sidebarConversationItem}:has-text("${userB.username}")`)
      .first();
    await expect(itemBeforePin).toHaveAttribute('data-pinned', 'false');

    // Open the row's actions menu and click Pin.
    await itemBeforePin.locator(sel.sidebarConversationActions).click();
    await expect(pageA.locator(sel.sidebarConversationMenu)).toBeVisible();
    await pageA.locator(sel.sidebarConversationMenuPin).click();
    // Optimistic update: the row flips to pinned in-place.
    await expect(itemBeforePin).toHaveAttribute('data-pinned', 'true', {
      timeout: 5_000,
    });

    await waitForConversationsFlushed(pageA, 1);
    await waitForConversationFlagSet(pageA, 'pinned');

    await pageA.reload();
    await waitForAppShell(pageA);

    // Wait for the application's own schema runner to have re-opened
    // IDB after the reload AND for the reconcile pass to have written
    // pinned=true back into the conversation row. We poll IDB rather
    // than racing the synchronous render with `expect().toBeVisible`
    // — the reconcile is `spawn_local` so any visible-side check
    // would deadline-race the async update.
    await waitForConversationFlagSet(pageA, 'pinned');

    // After reload: the row is in the Pinned section AND keeps
    // `data-pinned="true"`.
    const pinnedSection = pageA.locator(sel.sidebarSectionPinned);
    await expect(pinnedSection).toBeVisible({ timeout: 15_000 });

    const restoredRow = pinnedSection
      .locator(`${sel.sidebarConversationItem}:has-text("${userB.username}")`)
      .first();
    await expect(restoredRow).toBeVisible({ timeout: 15_000 });
    await expect(restoredRow).toHaveAttribute('data-pinned', 'true');
  });

  test('archived flag is persisted and the row appears under the archived section after reload', async ({
    pageA,
    pageB,
    server,
  }) => {
    await registerAndLogin(pageA, server, { hint: 'pe-arc-a' });
    const userB = await registerAndLogin(pageB, server, { hint: 'pe-arc-b' });
    await establishConnection(pageA, pageB, userB.username);

    const tag = Date.now().toString(36);
    await sendAndVerifyMessage(pageA, pageB, `arc-${tag}`);

    const itemBeforeArchive = pageA
      .locator(`${sel.sidebarConversationItem}:has-text("${userB.username}")`)
      .first();
    await expect(itemBeforeArchive).toHaveAttribute('data-archived', 'false');

    // Toggle archive via the row's context menu.
    await itemBeforeArchive.locator(sel.sidebarConversationActions).click();
    await expect(pageA.locator(sel.sidebarConversationMenu)).toBeVisible();
    await pageA.locator(sel.sidebarConversationMenuArchive).click();

    // The Archived section is collapsible and starts collapsed: once
    // the conversation flips to archived, the `For` loop unmounts the
    // row from the live DOM (it now lives behind a `Show when=is_open`
    // gate). Don't try to read `data-archived` from the previous
    // locator — its element is gone. Instead poll IDB for the flush
    // and rely on it as the readiness oracle.
    await waitForConversationsFlushed(pageA, 1);
    await waitForConversationFlagSet(pageA, 'archived');

    await pageA.reload();
    await waitForAppShell(pageA);

    // Wait for the post-reload reconcile to have written archived=true
    // back to the conversation row before asserting the section.
    await waitForConversationFlagSet(pageA, 'archived');

    // After reload: the Archived section is rendered (it has a row)
    // but starts collapsed, so the row itself is not in the DOM until
    // we expand the section. We wait on the section toggle button
    // existing as the readiness oracle. The button can sit below the
    // fold of the sidebar-scroll container, which Chromium's hit-test
    // interprets as "not visible" — we therefore use `dispatchEvent`
    // to fire the click without any visibility / layout precondition.
    const archivedSection = pageA.locator(sel.sidebarSectionArchived);
    await expect(archivedSection).toHaveCount(1, { timeout: 15_000 });
    const toggleBtn = archivedSection.locator('button.sidebar-section-title--toggle');
    await expect(toggleBtn).toHaveAttribute('aria-expanded', 'false');
    await toggleBtn.dispatchEvent('click');
    await expect(toggleBtn).toHaveAttribute('aria-expanded', 'true');

    const restoredRow = archivedSection
      .locator(`${sel.sidebarConversationItem}:has-text("${userB.username}")`)
      .first();
    await expect(restoredRow).toBeVisible({ timeout: 10_000 });
    await expect(restoredRow).toHaveAttribute('data-archived', 'true');
  });
});
