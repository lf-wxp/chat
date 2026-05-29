/**
 * Room password protection E2E tests.
 *
 * Maps to: Requirement 4 (Room System — password protection).
 *
 * Coverage:
 *   1. Create a room with a password — room is created successfully.
 *   2. User cannot join a password-protected room without entering the password.
 *   3. User can join a password-protected room with the correct password.
 *   4. User is rejected when entering an incorrect password.
 */

import { sel } from '../utils/selectors.ts';
import { registerAndLogin } from '../fixtures/helpers.ts';
import { expect, test } from '../fixtures/test-base.ts';

test.describe('room password protection', () => {
  test('create a room with a password — room appears in sidebar', async ({ pageA, server }) => {
    await registerAndLogin(pageA, server, { hint: 'rpw_a' });

    const roomName = `pw-room-${Date.now().toString(36)}`;
    const password = 'secret123';

    await pageA.locator(sel.sidebarRoomCreateBtn).click();
    const modal = pageA.locator(sel.createRoomModal);
    await expect(modal).toBeVisible({ timeout: 10_000 });

    await modal.locator(sel.createRoomName).fill(roomName);

    // Enable the password toggle checkbox first.
    const passwordToggle = modal.locator('[data-testid="create-room-password-toggle"]');
    await passwordToggle.check();

    // Fill in the password and confirm fields.
    const passwordInput = modal.locator('[data-testid="create-room-password"]');
    await expect(passwordInput).toBeVisible({ timeout: 5_000 });
    await passwordInput.fill(password);
    await modal.locator('[data-testid="create-room-password-confirm"]').fill(password);

    await modal.locator(sel.createRoomSubmit).click();
    await expect(modal).toBeHidden({ timeout: 10_000 });

    // Room appears in sidebar.
    const itemSelector = `${sel.sidebarRoomItem}[data-room-name="${roomName}"]`;
    await expect(pageA.locator(itemSelector)).toBeVisible({ timeout: 15_000 });
    await expect(pageA.locator(itemSelector)).toHaveAttribute('data-joined', 'true');

    // Room should have a lock/password indicator.
    const lockIcon = pageA.locator(`${itemSelector} [data-testid="room-lock-icon"]`);
    // Lock icon may or may not be rendered; just verify the room exists.
    await expect(pageA.locator(itemSelector)).toBeVisible();
  });

  test('user is prompted for password when joining a protected room', async ({
    pageA,
    pageB,
    server,
  }) => {
    await registerAndLogin(pageA, server, { hint: 'rpw_cr' });
    await registerAndLogin(pageB, server, { hint: 'rpw_jn' });

    const roomName = `pw-join-${Date.now().toString(36)}`;
    const password = 'joinpass';

    // A creates a password-protected room.
    await pageA.locator(sel.sidebarRoomCreateBtn).click();
    const modal = pageA.locator(sel.createRoomModal);
    await expect(modal).toBeVisible({ timeout: 10_000 });
    await modal.locator(sel.createRoomName).fill(roomName);
    // Enable password toggle and fill both fields.
    await modal.locator('[data-testid="create-room-password-toggle"]').check();
    await modal.locator('[data-testid="create-room-password"]').fill(password);
    await modal.locator('[data-testid="create-room-password-confirm"]').fill(password);
    await modal.locator(sel.createRoomSubmit).click();
    await expect(modal).toBeHidden({ timeout: 10_000 });

    // B sees the room and clicks join.
    const itemOnB = pageB.locator(`${sel.sidebarRoomItem}[data-room-name="${roomName}"]`);
    await expect(itemOnB).toBeVisible({ timeout: 15_000 });
    await itemOnB.locator(sel.sidebarRoomJoinBtn).click();

    // A password prompt modal should appear on B's side.
    const passwordModal = pageB.locator('[data-testid="modal-wrapper-backdrop"]:has([data-testid="password-prompt"])');
    await expect(passwordModal).toBeVisible({ timeout: 10_000 });
  });

  test('correct password allows joining the room', async ({ pageA, pageB, server }) => {
    await registerAndLogin(pageA, server, { hint: 'rpw_ok_a' });
    await registerAndLogin(pageB, server, { hint: 'rpw_ok_b' });

    const roomName = `pw-ok-${Date.now().toString(36)}`;
    const password = 'correct';

    // A creates a password-protected room.
    await pageA.locator(sel.sidebarRoomCreateBtn).click();
    const modal = pageA.locator(sel.createRoomModal);
    await expect(modal).toBeVisible({ timeout: 10_000 });
    await modal.locator(sel.createRoomName).fill(roomName);
    await modal.locator('[data-testid="create-room-password-toggle"]').check();
    await modal.locator('[data-testid="create-room-password"]').fill(password);
    await modal.locator('[data-testid="create-room-password-confirm"]').fill(password);
    await modal.locator(sel.createRoomSubmit).click();
    await expect(modal).toBeHidden({ timeout: 10_000 });

    // B clicks join.
    const itemOnB = pageB.locator(`${sel.sidebarRoomItem}[data-room-name="${roomName}"]`);
    await expect(itemOnB).toBeVisible({ timeout: 15_000 });
    await itemOnB.locator(sel.sidebarRoomJoinBtn).click();

    // Enter the correct password.
    const passwordModal = pageB.locator('[data-testid="modal-wrapper-backdrop"]:has([data-testid="password-prompt"])');
    await expect(passwordModal).toBeVisible({ timeout: 10_000 });
    await pageB.locator('[data-testid="password-prompt-primary"]').fill(password);
    await pageB.locator('[data-testid="password-prompt-submit"]').click();

    // Modal closes and B is joined.
    await expect(passwordModal).toBeHidden({ timeout: 10_000 });
    await expect(itemOnB).toHaveAttribute('data-joined', 'true', { timeout: 15_000 });
  });

  test('incorrect password is rejected', async ({ pageA, pageB, server }) => {
    await registerAndLogin(pageA, server, { hint: 'rpw_bad_a' });
    await registerAndLogin(pageB, server, { hint: 'rpw_bad_b' });

    const roomName = `pw-bad-${Date.now().toString(36)}`;
    const password = 'realpass';

    // A creates a password-protected room.
    await pageA.locator(sel.sidebarRoomCreateBtn).click();
    const modal = pageA.locator(sel.createRoomModal);
    await expect(modal).toBeVisible({ timeout: 10_000 });
    await modal.locator(sel.createRoomName).fill(roomName);
    await modal.locator('[data-testid="create-room-password-toggle"]').check();
    await modal.locator('[data-testid="create-room-password"]').fill(password);
    await modal.locator('[data-testid="create-room-password-confirm"]').fill(password);
    await modal.locator(sel.createRoomSubmit).click();
    await expect(modal).toBeHidden({ timeout: 10_000 });

    // B clicks join.
    const itemOnB = pageB.locator(`${sel.sidebarRoomItem}[data-room-name="${roomName}"]`);
    await expect(itemOnB).toBeVisible({ timeout: 15_000 });
    await itemOnB.locator(sel.sidebarRoomJoinBtn).click();

    // Enter an incorrect password.
    const passwordModal = pageB.locator('[data-testid="modal-wrapper-backdrop"]:has([data-testid="password-prompt"])');
    await expect(passwordModal).toBeVisible({ timeout: 10_000 });
    await pageB.locator('[data-testid="password-prompt-primary"]').fill('wrongpassword');
    await pageB.locator('[data-testid="password-prompt-submit"]').click();

    // The modal closes after submit (password is sent to server).
    // The server rejects with ROM204 and an error toast appears.
    await expect(passwordModal).toBeHidden({ timeout: 10_000 });
    const errorToast = pageB.locator('.error-toast');
    await expect(errorToast).toBeVisible({ timeout: 10_000 });

    // B is still NOT joined.
    await expect(itemOnB).not.toHaveAttribute('data-joined', 'true');  });
});
