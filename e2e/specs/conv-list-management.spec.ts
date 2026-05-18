/**
 * Conversation list management — pin / mute / archive context-menu actions
 * and (new in G20) the sidebar search filter.
 *
 * Maps to: Wave P2-4. The persisted versions of these flags (pin /
 * archive surviving a reload via IDB `conversation_flags`) are
 * already covered by `persistence-extended.spec.ts`; this spec
 * locks down the interactive behaviour without a reload:
 *
 *   1. Pin → row reparents into the Pinned section, `data-pinned`
 *      flips to `"true"`. Unpin restores the row to the Active
 *      section.
 *   2. Mute → row carries `data-muted="true"` and the bell-off icon
 *      is visible. Unmute reverts both signals.
 *   3. Archive → row moves to the Archived (collapsible) section
 *      and is hidden by default; expanding the section reveals it
 *      with `data-archived="true"`.
 *   4. Search filter (G20) → typing in `sidebar-search-input` filters
 *      the conversation rows by case-insensitive substring on
 *      `display_name`; clearing the query restores every row.
 *
 * Plan §3 P2-4 originally also listed "delete conversation" as a
 * fifth test. That UI surface is missing from the current frontend
 * (no per-row delete affordance, no `AppState::delete_conversation`
 * method). The gap is tracked in the plan §2.1 table as G21; the
 * test will land once the surface ships.
 */

import { sel } from '../utils/selectors.ts';
import {
  establishConnection,
  registerAndLogin,
  sendAndVerifyMessage,
} from '../fixtures/helpers.ts';
import { expect, test } from '../fixtures/test-base.ts';

test.describe('conversation list management', () => {
  test.beforeEach(async ({ pageA, pageB, server }) => {
    await registerAndLogin(pageA, server, { hint: 'cl-a' });
    const userB = await registerAndLogin(pageB, server, { hint: 'cl-b' });
    await establishConnection(pageA, pageB, userB.username);
    // Send one message so the conversation row is fully realised
    // (sidebar entry + last-message preview). Use the helper so the
    // ECDH-not-ready race on the very first frame is softened.
    const tag = Date.now().toString(36);
    await sendAndVerifyMessage(pageA, pageB, `seed-${tag}`);
  });

  test('pin → row moves into the Pinned section, then unpin restores it', async ({
    pageA,
  }) => {
    const row = pageA.locator(sel.sidebarConversationItem).first();
    await expect(row).toHaveAttribute('data-pinned', 'false');

    // Open menu and click Pin.
    await row.locator(sel.sidebarConversationActions).click();
    await expect(pageA.locator(sel.sidebarConversationMenu)).toBeVisible();
    await pageA.locator(sel.sidebarConversationMenuPin).click();

    // The pinned section now contains the row with data-pinned="true".
    const pinnedSection = pageA.locator(sel.sidebarSectionPinned);
    await expect(pinnedSection).toBeVisible({ timeout: 5_000 });
    const pinnedRow = pinnedSection.locator(sel.sidebarConversationItem).first();
    await expect(pinnedRow).toHaveAttribute('data-pinned', 'true', { timeout: 5_000 });

    // The Active section no longer holds the row. The simplest
    // invariant that is robust against future "always render an
    // empty Active section" UI tweaks is: there is exactly one
    // sidebar conversation item in total (we only seeded one).
    await expect(pageA.locator(sel.sidebarConversationItem)).toHaveCount(1);

    // Unpin via the same menu — the row reparents back to Active.
    await pinnedRow.locator(sel.sidebarConversationActions).click();
    await expect(pageA.locator(sel.sidebarConversationMenu)).toBeVisible();
    await pageA.locator(sel.sidebarConversationMenuPin).click();

    const activeSection = pageA.locator(sel.sidebarSectionActive);
    const activeRow = activeSection.locator(sel.sidebarConversationItem).first();
    await expect(activeRow).toBeVisible({ timeout: 5_000 });
    await expect(activeRow).toHaveAttribute('data-pinned', 'false');
  });

  test('mute toggles data-muted and surfaces the bell-off indicator', async ({
    pageA,
  }) => {
    const row = pageA.locator(sel.sidebarConversationItem).first();
    await expect(row).toHaveAttribute('data-muted', 'false');
    // The decorative mute indicator is gated by `Show when=muted` —
    // before muting it must not be in the DOM.
    await expect(row.locator('.sidebar-conversation-mute-icon')).toHaveCount(0);

    // Mute via the menu.
    await row.locator(sel.sidebarConversationActions).click();
    await expect(pageA.locator(sel.sidebarConversationMenu)).toBeVisible();
    await pageA.locator(sel.sidebarConversationMenuMute).click();

    await expect(row).toHaveAttribute('data-muted', 'true', { timeout: 5_000 });
    await expect(row.locator('.sidebar-conversation-mute-icon')).toBeVisible();

    // Unmute via the menu — flag flips back, indicator is removed.
    await row.locator(sel.sidebarConversationActions).click();
    await expect(pageA.locator(sel.sidebarConversationMenu)).toBeVisible();
    await pageA.locator(sel.sidebarConversationMenuMute).click();

    await expect(row).toHaveAttribute('data-muted', 'false', { timeout: 5_000 });
    await expect(row.locator('.sidebar-conversation-mute-icon')).toHaveCount(0);
  });

  test('archive moves the row into the collapsed Archived section', async ({
    pageA,
  }) => {
    const row = pageA.locator(sel.sidebarConversationItem).first();
    await expect(row).toHaveAttribute('data-archived', 'false');

    await row.locator(sel.sidebarConversationActions).click();
    await expect(pageA.locator(sel.sidebarConversationMenu)).toBeVisible();
    await pageA.locator(sel.sidebarConversationMenuArchive).click();

    // The Archived section appears once it has at least one entry.
    const archivedSection = pageA.locator(sel.sidebarSectionArchived);
    await expect(archivedSection).toHaveCount(1, { timeout: 5_000 });

    // Active section is empty (we only seeded one conversation).
    await expect(
      pageA.locator(sel.sidebarSectionActive).locator(sel.sidebarConversationItem),
    ).toHaveCount(0, { timeout: 5_000 });

    // The Archived section starts collapsed: the toggle button is
    // present, `aria-expanded="false"`, and the row itself is NOT
    // mounted (the parent `Show when=is_open` gates the `<For>`).
    const toggleBtn = archivedSection.locator('button.sidebar-section-title--toggle');
    await expect(toggleBtn).toHaveAttribute('aria-expanded', 'false');
    await expect(archivedSection.locator(sel.sidebarConversationItem)).toHaveCount(0);

    // Expanding the section reveals the row with data-archived="true".
    // Use dispatchEvent to bypass any below-the-fold visibility
    // probe — the same pattern as in `persistence-extended.spec.ts`.
    await toggleBtn.dispatchEvent('click');
    await expect(toggleBtn).toHaveAttribute('aria-expanded', 'true');

    const archivedRow = archivedSection.locator(sel.sidebarConversationItem).first();
    await expect(archivedRow).toBeVisible({ timeout: 5_000 });
    await expect(archivedRow).toHaveAttribute('data-archived', 'true');
  });

  test('sidebar search filters conversation rows by display name (G20)', async ({
    pageA,
    pageB,
    pageC,
    server,
  }) => {
    // The standard beforeEach already establishes pageA <-> pageB
    // and seeds one direct conversation. Build a second one with
    // pageC so the search filter has meaningful work to do — with
    // a single row, clearing/applying the filter is indistinguishable
    // from "list is empty".
    const userC = await registerAndLogin(pageC, server, { hint: 'cl-search-c' });
    await establishConnection(pageA, pageC, userC.username);
    const tag = Date.now().toString(36);
    await sendAndVerifyMessage(pageA, pageC, `seed-c-${tag}`);

    // Both rows are now in the sidebar.
    const allRows = pageA.locator(sel.sidebarConversationItem);
    await expect(allRows).toHaveCount(2, { timeout: 15_000 });

    // Auto-generated usernames produced by `uniqueUsername` share a
    // common `t_<hint>_` prefix, so we cannot use a leading
    // substring to disambiguate. Instead we key the filter off the
    // *trailing* random hex of `userC.username`, which is unique to
    // userC. The filter is a case-insensitive substring match
    // against `display_name`, which mirrors the username for users
    // without a custom nickname.
    const userCSuffix = userC.username.slice(-4);
    expect(userCSuffix).toMatch(/^[0-9a-f]+$/);

    const search = pageA.locator(sel.sidebarSearchInput);
    await search.fill(userCSuffix);
    await expect(allRows).toHaveCount(1, { timeout: 5_000 });
    // The remaining row carries userC's username in its aria-label.
    await expect(allRows.first()).toHaveAttribute('aria-label', new RegExp(userC.username));

    // No-match query empties the visible list.
    await search.fill('zzz-no-match-zzz');
    await expect(allRows).toHaveCount(0, { timeout: 5_000 });

    // Clearing the input restores both rows.
    await search.fill('');
    await expect(allRows).toHaveCount(2, { timeout: 5_000 });
  });
});
