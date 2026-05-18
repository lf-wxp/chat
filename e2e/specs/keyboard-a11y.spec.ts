/**
 * Keyboard accessibility for the sidebar + chat composer (Wave P2-8).
 *
 * Maps to: Requirement 16.18 keyboard-navigation gaps.
 *
 * The sidebar conversation list is a single roving-tabindex list
 * (Req 14.5.2). All conversation rows live under `aside.sidebar` and
 * carry `tabindex="0" role="button"`. Their keydown handler:
 *   * `Enter` / `Space` → activates the row (sets `aria-pressed`).
 *   * `ArrowDown` / `ArrowUp` → focus moves to the next / previous
 *     row, wrapping at both ends.
 *   * `Home` / `End` → focus jumps to the first / last row.
 *
 * The chat composer is a normal block of focusable form controls;
 * Tab order proceeds left-to-right through the four chat-input
 * buttons (Sticker → Image → Paperclip → Mic) → the textarea → the
 * Send button.
 *
 * These tests build a controlled multi-row sidebar by establishing
 * two parallel direct conversations on `pageA` (A↔B and A↔C). With
 * exactly two rows the wrap behaviour at both ends is observable
 * with a single ArrowUp/ArrowDown pair.
 */

import { sel } from '../utils/selectors.ts';
import { establishConnection, registerAndLogin } from '../fixtures/helpers.ts';
import { expect, test } from '../fixtures/test-base.ts';
import type { Page } from '@playwright/test';

/** Returns the `data-testid` (or null) of the currently-focused element. */
async function focusedTestid(page: Page): Promise<string | null> {
  return page.evaluate(() => document.activeElement?.getAttribute('data-testid') ?? null);
}

/** Returns the `aria-label` (or null) of the currently-focused element. */
async function focusedAriaLabel(page: Page): Promise<string | null> {
  return page.evaluate(() => document.activeElement?.getAttribute('aria-label') ?? null);
}

test.describe('keyboard a11y', () => {
  test('ArrowDown/ArrowUp wrap focus across sidebar conversation rows', async ({
    pageA,
    pageB,
    pageC,
    server,
  }) => {
    await registerAndLogin(pageA, server, { hint: 'kb-w-a' });
    const userB = await registerAndLogin(pageB, server, { hint: 'kb-w-b' });
    const userC = await registerAndLogin(pageC, server, { hint: 'kb-w-c' });
    await establishConnection(pageA, pageB, userB.username);
    await establishConnection(pageA, pageC, userC.username);

    // Wait for both rows to be in the sidebar.
    const rows = pageA.locator(sel.sidebarConversationItem);
    await expect(rows).toHaveCount(2, { timeout: 15_000 });

    // Move keyboard focus onto the first row programmatically so we
    // start from a deterministic anchor (the document focus default
    // can drift between renders).
    await rows.first().focus();
    await expect(
      pageA.locator(`${sel.sidebarConversationItem}:focus`),
    ).toHaveCount(1);

    // Read the aria-label of the first focused row, then ArrowDown
    // and assert focus has moved to a *different* sidebar row.
    const firstLabel = await focusedAriaLabel(pageA);
    expect(firstLabel).not.toBeNull();

    await pageA.keyboard.press('ArrowDown');
    await expect(
      pageA.locator(`${sel.sidebarConversationItem}:focus`),
    ).toHaveCount(1);
    const secondLabel = await focusedAriaLabel(pageA);
    expect(secondLabel).not.toBeNull();
    expect(secondLabel).not.toBe(firstLabel);

    // ArrowDown again wraps back to the first row.
    await pageA.keyboard.press('ArrowDown');
    expect(await focusedAriaLabel(pageA)).toBe(firstLabel);

    // ArrowUp from the first row wraps to the last row.
    await pageA.keyboard.press('ArrowUp');
    expect(await focusedAriaLabel(pageA)).toBe(secondLabel);
  });

  test('Home / End jump focus to the first / last sidebar row', async ({
    pageA,
    pageB,
    pageC,
    server,
  }) => {
    await registerAndLogin(pageA, server, { hint: 'kb-h-a' });
    const userB = await registerAndLogin(pageB, server, { hint: 'kb-h-b' });
    const userC = await registerAndLogin(pageC, server, { hint: 'kb-h-c' });
    await establishConnection(pageA, pageB, userB.username);
    await establishConnection(pageA, pageC, userC.username);

    const rows = pageA.locator(sel.sidebarConversationItem);
    await expect(rows).toHaveCount(2, { timeout: 15_000 });
    const firstLabel = await rows.first().getAttribute('aria-label');
    const lastLabel = await rows.last().getAttribute('aria-label');
    expect(firstLabel).not.toBeNull();
    expect(lastLabel).not.toBeNull();
    expect(firstLabel).not.toBe(lastLabel);

    // Anchor focus on the second row, then End → first row.
    await rows.last().focus();
    expect(await focusedAriaLabel(pageA)).toBe(lastLabel);

    await pageA.keyboard.press('Home');
    expect(await focusedAriaLabel(pageA)).toBe(firstLabel);

    await pageA.keyboard.press('End');
    expect(await focusedAriaLabel(pageA)).toBe(lastLabel);
  });

  test('Enter activates the focused conversation row (aria-pressed flips)', async ({
    pageA,
    pageB,
    pageC,
    server,
  }) => {
    await registerAndLogin(pageA, server, { hint: 'kb-en-a' });
    const userB = await registerAndLogin(pageB, server, { hint: 'kb-en-b' });
    const userC = await registerAndLogin(pageC, server, { hint: 'kb-en-c' });
    await establishConnection(pageA, pageB, userB.username);
    await establishConnection(pageA, pageC, userC.username);

    const rows = pageA.locator(sel.sidebarConversationItem);
    await expect(rows).toHaveCount(2, { timeout: 15_000 });

    // After establishConnection the active conversation is the most
    // recently connected (A↔C). Find the *other* row (A↔B) to activate
    // via the keyboard so we can observe a real flip.
    //
    // We pick the row whose aria-pressed is currently "false".
    const rowB = rows
      .filter({ has: pageA.locator('[aria-pressed="false"]') })
      .first();
    // Some Playwright versions resolve `filter({ has: <self-rule> })`
    // unexpectedly; fall back to scanning rows individually if the
    // chained locator can't narrow.
    const candidateCount = await rowB.count();
    let target = rowB;
    if (candidateCount === 0) {
      // Manual scan: find the first row whose own aria-pressed is "false".
      const all = await rows.all();
      for (const row of all) {
        const pressed = await row.getAttribute('aria-pressed');
        if (pressed === 'false') {
          target = row;
          break;
        }
      }
    }

    await target.focus();
    await expect(target).toHaveAttribute('aria-pressed', 'false');

    await pageA.keyboard.press('Enter');

    // The handler sets `active_conversation` and (on mobile) hides
    // the sidebar — but the desktop test viewport keeps the sidebar
    // visible. The activated row's aria-pressed flips to "true".
    await expect(target).toHaveAttribute('aria-pressed', 'true', { timeout: 5_000 });
  });

  test('Tab order from chat input flows through the send button', async ({
    pageA,
    pageB,
    server,
  }) => {
    await registerAndLogin(pageA, server, { hint: 'kb-tab-a' });
    const userB = await registerAndLogin(pageB, server, { hint: 'kb-tab-b' });
    await establishConnection(pageA, pageB, userB.username);

    const textarea = pageA.locator(sel.chatInputTextarea);
    await expect(textarea).toBeVisible();

    // Anchor focus on the textarea. The send button is its immediate
    // sibling in the chat-input-row, so a single Tab forward reaches
    // it. We also need at least one character typed: `can_send`
    // collapses to `false` for an empty draft, which renders the
    // button `disabled` — a `disabled` button is skipped by the Tab
    // sequence in Chromium. Type a glyph first so the button is
    // focusable.
    await textarea.focus();
    expect(await focusedTestid(pageA)).toBe('chat-input-textarea');
    await textarea.fill('hello');

    await pageA.keyboard.press('Tab');
    // The chat-input-counter sits AFTER the textarea but inside its
    // own div with no tabindex; the next focusable element is the
    // Send button (`data-testid="chat-input-send"`).
    await expect
      .poll(async () => focusedTestid(pageA), { timeout: 3_000 })
      .toBe('chat-input-send');

    // Reverse Tab returns to the textarea.
    await pageA.keyboard.press('Shift+Tab');
    await expect
      .poll(async () => focusedTestid(pageA), { timeout: 3_000 })
      .toBe('chat-input-textarea');
  });
});
