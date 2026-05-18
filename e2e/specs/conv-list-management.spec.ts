/**
 * Conversation list management — pin / mute / archive context-menu actions.
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
 *
 * Plan §3 P2-4 originally also listed "search filter" and "delete
 * conversation" as fourth and fifth tests. Both of these UI surfaces
 * are missing from the current frontend (sidebar search input is
 * rendered but has no `on:input` binding, and there is no per-row
 * delete affordance anywhere). Those gaps are tracked in the plan
 * §2.1 table as feature gaps; the corresponding tests will land once
 * the surfaces ship.
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
});
