/**
 * Profile / blacklist coverage (Wave P2-7).
 *
 * Plan §3 P2-7 originally listed three sub-tests (upload avatar,
 * nickname edit round-trip, block/unblock). The avatar upload path
 * is gated on a UI surface that does not yet exist (see G26 — no
 * picker, no `AvatarChange` signaling protocol field).
 *
 * What works today:
 *   * Block / unblock — wired end-to-end via `BlacklistState`,
 *     `localStorage["blacklist"]` and the settings-drawer
 *     `BlacklistManagementPanel`.
 *   * Nickname edit — `NicknameEditor` is mounted in the settings
 *     drawer's Account section (G25). The component validates with
 *     `message::error::validation::validate_nickname`, persists the
 *     new value into the auth state's `auth_nickname` localStorage
 *     key, and broadcasts a `NicknameChange` signaling message.
 *     **Server-side persistence (G28) lands the new nickname on
 *     the global UserStore so reloads no longer clobber the
 *     client mirror.**
 *
 * This spec locks down:
 *
 *   1. Block flips the user-info-card's invite button to `disabled`
 *      and the Block button to "Unblock" — the card-local state
 *      reflects the global signal.
 *   2. The blacklist row shows up in the settings drawer's
 *      `blacklist-panel` and disappears after Unblock.
 *   3. The blacklist persists across a page reload via the
 *      `blacklist` localStorage key.
 *   4. In-session nickname edit (G25) — opening the settings drawer
 *      surfaces the `nickname-editor` component; entering a new
 *      value enables Save; clicking Save persists into
 *      `localStorage[auth_nickname]` and survives a drawer reopen.
 *   5. Cross-reload nickname (G28) — the server's `AuthSuccess`
 *      after `page.reload()` carries the new nickname, so the
 *      localStorage mirror keeps the user's edit.
 */

import { sel } from '../utils/selectors.ts';
import { registerAndLogin } from '../fixtures/helpers.ts';
import { expect, test } from '../fixtures/test-base.ts';
import {
  waitForAppShell,
  waitForOnlineUser,
} from '../utils/wait-helpers.ts';

const settingsPageSelector = '[data-testid="settings-page"]';
const blacklistPanel = '[data-testid="blacklist-panel"]';
const blacklistRow = '[data-testid="blacklist-row"]';

test.describe('profile / blacklist', () => {
  test('block flips invite button to disabled and the block button to Unblock', async ({
    pageA,
    pageB,
    server,
  }) => {
    await registerAndLogin(pageA, server, { hint: 'pf-blk-a' });
    const userB = await registerAndLogin(pageB, server, { hint: 'pf-blk-b' });

    // Open A's user-info-card for userB (the helper waits for B to
    // appear in A's online list before clicking).
    const bRow = await waitForOnlineUser(pageA, userB.username);
    await bRow.click();
    const card = pageA.locator(sel.userInfoCard);
    await expect(card).toBeVisible({ timeout: 10_000 });

    // Before block: invite button is enabled, block button label is
    // "Block". The label is i18n-translated; we key off the
    // class-derived `is-blocked` modifier instead so the test is
    // locale-independent.
    const invite = pageA.locator(sel.userInfoInvite);
    const block = pageA.locator(sel.userInfoBlock);
    await expect(invite).toBeEnabled();
    await expect(block).not.toHaveClass(/is-blocked/);

    await block.click();

    // The user-info-card closes after a successful block (`close()`
    // called from the click handler). When closed, neither button is
    // mounted — assert the dismiss instead.
    await expect(card).toBeHidden({ timeout: 10_000 });

    // Open the card a second time — userB should still be in the
    // online list (block doesn't kick anyone offline server-side).
    const bRow2 = await waitForOnlineUser(pageA, userB.username);
    await bRow2.click();
    await expect(card).toBeVisible({ timeout: 10_000 });

    // Now the card reflects the blocked state.
    await expect(pageA.locator(sel.userInfoBlock)).toHaveClass(/is-blocked/, {
      timeout: 5_000,
    });
    await expect(pageA.locator(sel.userInfoInvite)).toBeDisabled();
  });

  test('blacklist panel in settings drawer surfaces blocked rows and Unblock removes them', async ({
    pageA,
    pageB,
    server,
  }) => {
    await registerAndLogin(pageA, server, { hint: 'pf-pnl-a' });
    const userB = await registerAndLogin(pageB, server, { hint: 'pf-pnl-b' });

    // Step 1 — block userB.
    const bRow = await waitForOnlineUser(pageA, userB.username);
    await bRow.click();
    await expect(pageA.locator(sel.userInfoCard)).toBeVisible({ timeout: 10_000 });
    await pageA.locator(sel.userInfoBlock).click();
    await expect(pageA.locator(sel.userInfoCard)).toBeHidden({ timeout: 10_000 });

    // Step 2 — open the settings drawer; assert the blacklist row.
    await pageA.locator(sel.sidebarSettingsBtn).click();
    await expect(pageA.locator(settingsPageSelector)).toBeVisible({ timeout: 5_000 });

    const panel = pageA.locator(blacklistPanel);
    await expect(panel).toBeVisible({ timeout: 5_000 });
    const row = panel.locator(blacklistRow);
    await expect(row).toHaveCount(1, { timeout: 5_000 });
    // The row carries userB's display name (nickname falls back to
    // username for users without a custom nickname).
    await expect(row).toContainText(userB.username);

    // Step 3 — click the Unblock button on the row. The list re-
    // renders with zero entries.
    await row.locator('button.blacklist-panel__unblock').click();
    await expect(panel.locator(blacklistRow)).toHaveCount(0, { timeout: 5_000 });
  });

  test('blacklist persists across page reload via localStorage', async ({
    pageA,
    pageB,
    server,
  }) => {
    await registerAndLogin(pageA, server, { hint: 'pf-rld-a' });
    const userB = await registerAndLogin(pageB, server, { hint: 'pf-rld-b' });

    const bRow = await waitForOnlineUser(pageA, userB.username);
    await bRow.click();
    await expect(pageA.locator(sel.userInfoCard)).toBeVisible({ timeout: 10_000 });
    await pageA.locator(sel.userInfoBlock).click();
    await expect(pageA.locator(sel.userInfoCard)).toBeHidden({ timeout: 10_000 });

    // The blacklist persists synchronously after `block()` finishes;
    // poll the localStorage key so we don't race the write.
    await expect
      .poll(
        async () =>
          pageA.evaluate(() => {
            const raw = localStorage.getItem('blacklist');
            if (!raw) return 0;
            try {
              const parsed = JSON.parse(raw) as Array<unknown>;
              return Array.isArray(parsed) ? parsed.length : 0;
            } catch {
              return 0;
            }
          }),
        { timeout: 5_000 },
      )
      .toBe(1);

    await pageA.reload();
    await waitForAppShell(pageA);

    // After reload, the blacklist panel is reachable via the
    // settings drawer; the previously-blocked entry survives.
    await pageA.locator(sel.sidebarSettingsBtn).click();
    await expect(pageA.locator(settingsPageSelector)).toBeVisible({ timeout: 5_000 });
    const panel = pageA.locator(blacklistPanel);
    await expect(panel).toBeVisible({ timeout: 5_000 });
    await expect(panel.locator(blacklistRow)).toHaveCount(1, { timeout: 5_000 });
    await expect(panel.locator(blacklistRow)).toContainText(userB.username);
  });

  test('nickname editor saves a new name and updates the auth signal (G25)', async ({
    pageA,
    server,
  }) => {
    const me = await registerAndLogin(pageA, server, { hint: 'pf-nick' });

    // Open the settings drawer — the editor is mounted under the
    // Account section.
    await pageA.locator(sel.sidebarSettingsBtn).click();
    await expect(pageA.locator(settingsPageSelector)).toBeVisible({ timeout: 5_000 });

    const editor = pageA.locator(sel.nicknameEditor);
    await expect(editor).toBeVisible({ timeout: 5_000 });

    // The default nickname mirrors the username (server-side
    // initialisation in `auth/mod.rs::User::new`).
    const input = pageA.locator(sel.nicknameEditorInput);
    await expect(input).toHaveValue(me.username);

    // Save button is disabled until the draft differs from the
    // persisted value.
    const save = pageA.locator(sel.nicknameEditorSave);
    await expect(save).toBeDisabled();

    // Pick a new nickname that satisfies `validate_nickname` (1-20
    // chars, no leading/trailing whitespace) and is independent of
    // the auto-generated username — concatenating "-edited" onto
    // the unique-username can overflow the textarea's `maxlength=20`
    // and accidentally produce a no-op write.
    const tag = Date.now().toString(36).slice(-6);
    const newNickname = `nk_${tag}`;
    expect(newNickname.length).toBeLessThanOrEqual(20);

    await input.fill(newNickname);
    await expect(input).toHaveValue(newNickname);
    await expect(save).toBeEnabled({ timeout: 3_000 });
    await save.click();

    // The auth signal mirror — `localStorage["auth_nickname"]` —
    // updates synchronously inside the click handler. Poll for it
    // so we don't race the serialisation step.
    await expect
      .poll(async () => pageA.evaluate(() => localStorage.getItem('auth_nickname')), {
        timeout: 5_000,
      })
      .toBe(newNickname);

    // Save button flips back to disabled because draft now matches
    // the persisted value.
    await expect(save).toBeDisabled({ timeout: 3_000 });

    // Close + re-open the settings drawer: the editor still shows
    // the new value (the in-memory auth signal carries it; we don't
    // need a reload to prove the in-session round-trip).
    await pageA.keyboard.press('Escape');
    await expect(pageA.locator(settingsPageSelector)).toBeHidden({ timeout: 3_000 });
    await pageA.locator(sel.sidebarSettingsBtn).click();
    await expect(pageA.locator(settingsPageSelector)).toBeVisible({ timeout: 3_000 });
    await expect(pageA.locator(sel.nicknameEditorInput)).toHaveValue(newNickname);
  });

  test('nickname survives a page reload (G28 server-side persistence)', async ({
    pageA,
    server,
  }) => {
    const me = await registerAndLogin(pageA, server, { hint: 'pf-nick-rld' });

    // Open settings + edit nickname (same shape as the in-session
    // test, abbreviated — we are exercising the cross-reload
    // contract here, not the editor UI itself).
    await pageA.locator(sel.sidebarSettingsBtn).click();
    await expect(pageA.locator(settingsPageSelector)).toBeVisible({ timeout: 5_000 });
    const input = pageA.locator(sel.nicknameEditorInput);
    await expect(input).toHaveValue(me.username);

    const tag = Date.now().toString(36).slice(-6);
    const persisted = `nk_${tag}`;
    expect(persisted.length).toBeLessThanOrEqual(20);

    await input.fill(persisted);
    await pageA.locator(sel.nicknameEditorSave).click();

    // Wait for the client-side write to localStorage so the value
    // has at least committed before we reload (avoids racing the
    // server roundtrip).
    await expect
      .poll(async () => pageA.evaluate(() => localStorage.getItem('auth_nickname')), {
        timeout: 5_000,
      })
      .toBe(persisted);

    // G28 — after reload the server's `AuthSuccess` carries the
    // new nickname (UserStore was persisted), so the localStorage
    // mirror retains the new value rather than being clobbered
    // back to `username` by `handle_auth_success`.
    await pageA.reload();
    await waitForAppShell(pageA);

    await expect
      .poll(async () => pageA.evaluate(() => localStorage.getItem('auth_nickname')), {
        timeout: 10_000,
      })
      .toBe(persisted);

    // Settings drawer surfaces the persisted value on re-open.
    await pageA.locator(sel.sidebarSettingsBtn).click();
    await expect(pageA.locator(settingsPageSelector)).toBeVisible({ timeout: 5_000 });
    await expect(pageA.locator(sel.nicknameEditorInput)).toHaveValue(persisted, {
      timeout: 5_000,
    });
  });
});
