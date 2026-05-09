/**
 * Reusable helpers shared across spec files.
 *
 * Each helper is intentionally narrow:
 * - `registerAndLogin` — bring a fresh page from a blank state to a usable shell.
 * - `establishConnection` — issue + accept a connection invitation between
 *   two pages so subsequent assertions can target the chat view.
 * - `sendAndVerifyMessage` — send a text message and wait for it to appear on
 *   the receiver, returning the locators on both sides for further assertions.
 */

import { type Locator, type Page, expect } from '@playwright/test';

import type { ServerInstance } from './server.ts';
import { sel } from '../utils/selectors.ts';
import { DEFAULT_PASSWORD, uniqueUsername } from '../utils/users.ts';
import { waitForAppShell, waitForOnlineUser } from '../utils/wait-helpers.ts';

/** Result of `registerAndLogin`. */
export interface LoggedInUser {
  username: string;
  password: string;
}

/**
 * Wait until the user's WebSocket signaling connection is established and the
 * post-auth `UserListUpdate` has been applied to the local state.
 *
 * The check polls the application's `online_users` signal length (read
 * directly from `localStorage` is not possible — `online_users` is in-memory
 * only — so we rely on the rendered DOM list as the readiness oracle). Either
 * the list is non-empty (other users are online too) or the connection badge
 * has been "connected" for at least one render frame.
 */
export async function waitForSignalingReady(
  page: Page,
  _selfUsername: string,
  timeoutMs = 20_000,
): Promise<void> {
  // 1. Sidebar connection status flips to `--connected`.
  await expect(
    page.locator(`${sel.sidebarConnectionStatus}.sidebar-connection-status--connected`),
  ).toBeVisible({ timeout: timeoutMs });

  // 2. Give the post-auth UserListUpdate a moment to propagate. We give the
  //    handler one round-trip; a more deterministic signal (e.g. a
  //    `data-ready` attribute on the panel) would require frontend changes.
  await page.waitForTimeout(300);
}

/**
 * Navigate `page` to the app, register a brand-new user, and wait for the
 * main application shell AND a fully-established signaling session. Returns
 * the credentials so further actions (re-login, multi-tab) can reuse them.
 */
export async function registerAndLogin(
  page: Page,
  server: ServerInstance,
  options: { username?: string; password?: string; hint?: string } = {},
): Promise<LoggedInUser> {
  const username = options.username ?? uniqueUsername(options.hint);
  const password = options.password ?? DEFAULT_PASSWORD;

  await page.goto(`${server.baseUrl}/`);
  // The auth page renders LoginForm by default — switch to register.
  await page.locator(sel.authPage).waitFor({ state: 'visible' });
  // The "Create account" link is rendered only when LoginForm is shown.
  const switchBtn = page.locator(sel.authSwitchToRegister);
  if (await switchBtn.isVisible()) {
    await switchBtn.click();
  }

  await page.locator(sel.registerForm).waitFor({ state: 'visible' });
  await page.locator(sel.registerUsername).fill(username);
  await page.locator(sel.registerPassword).fill(password);
  await page.locator(sel.registerConfirmPassword).fill(password);
  await page.locator(sel.registerSubmit).click();

  await waitForAppShell(page);
  await waitForSignalingReady(page, username);
  return { username, password };
}

/**
 * From `pageA`, find user `usernameB` in the online list, open the info card,
 * send a connection invitation. From `pageB`, accept the incoming invitation.
 *
 * Resolves once both pages display the chat view (DataChannel established).
 */
export async function establishConnection(
  pageA: Page,
  pageB: Page,
  usernameB: string,
): Promise<void> {
  // Wait for B to surface in A's online list.
  const userBRow = await waitForOnlineUser(pageA, usernameB);
  await userBRow.click();

  // Info card opens; click "Send Connection Invitation".
  const userInfoCard = pageA.locator(sel.userInfoCard);
  await userInfoCard.waitFor({ state: 'visible' });
  await pageA.locator(sel.userInfoInvite).click();

  // Incoming invite modal on B; click Accept.
  const invite = pageB.locator(sel.incomingInviteModal);
  await invite.waitFor({ state: 'visible', timeout: 15_000 });
  await pageB.locator(sel.inviteAccept).click();

  // Both sides land on the chat view once the data channel is open.
  await Promise.all([
    pageA.locator(sel.chatView).waitFor({ state: 'visible', timeout: 20_000 }),
    pageB.locator(sel.chatView).waitFor({ state: 'visible', timeout: 20_000 }),
  ]);

  // Wait for the E2EE bootstrap (DataChannel open -> ECDH key exchange
  // -> HKDF -> AES-256-GCM key import) to complete on BOTH sides
  // before any application-level message is sent. Frames sent before
  // `PeerEncryptionStatus::established = true` are dropped at the
  // encryption layer. The chat view always renders an
  // `e2ee-ready-sentinel` whose `data-ready` attribute flips to
  // `"true"` once the handshake completes, so we can poll that
  // attribute deterministically. If the handshake genuinely stalls
  // the timeout below fires with a clear message — preferable to a
  // flaky fixed `waitForTimeout`.
  await Promise.all([
    expect(pageA.locator(sel.e2eeReadySentinel).first()).toHaveAttribute(
      'data-ready',
      'true',
      { timeout: 30_000 },
    ),
    expect(pageB.locator(sel.e2eeReadySentinel).first()).toHaveAttribute(
      'data-ready',
      'true',
      { timeout: 30_000 },
    ),
  ]);
}

/** Locators returned by `sendAndVerifyMessage` for further assertions. */
export interface SendResult {
  senderRow: Locator;
  receiverRow: Locator;
}

/**
 * Type `messageContent` into the sender's input bar, press Enter, then wait
 * for the same content to appear on the receiver. Returns matching locators
 * on both sides.
 *
 * The send is retried up to `maxAttempts` times with a short backoff because
 * the very first send right after the chat view appears can race the ECDH
 * handshake — the encryption layer drops frames sent before
 * `EncryptionState::Established`. Each retry redrafts the same message text
 * (idempotent because the application deduplicates by `message_id`).
 */
export async function sendAndVerifyMessage(
  senderPage: Page,
  receiverPage: Page,
  messageContent: string,
  options: { maxAttempts?: number; perAttemptTimeoutMs?: number } = {},
): Promise<SendResult> {
  const maxAttempts = options.maxAttempts ?? 3;
  const perAttemptTimeoutMs = options.perAttemptTimeoutMs ?? 8_000;

  const senderRow = senderPage.locator(sel.messageRow, { hasText: messageContent }).first();
  const receiverRow = receiverPage.locator(sel.messageRow, { hasText: messageContent }).first();
  const textarea = senderPage.locator(sel.chatInputTextarea);

  // First send.
  await textarea.fill(messageContent);
  await textarea.press('Enter');

  for (let attempt = 1; attempt <= maxAttempts; attempt += 1) {
    try {
      await expect(senderRow).toBeVisible({ timeout: perAttemptTimeoutMs });
      await expect(receiverRow).toBeVisible({ timeout: perAttemptTimeoutMs });
      return { senderRow, receiverRow };
    } catch (err) {
      if (attempt === maxAttempts) {
        throw err;
      }
      // Retry: re-fill and resend. The application's ACK/dedup layer
      // makes this safe — duplicate sends are coalesced by `message_id`
      // because we use the same content but a fresh send entry.
      await senderPage.waitForTimeout(1_500);
      await textarea.fill(messageContent);
      await textarea.press('Enter');
    }
  }

  // Unreachable: the catch above either returns or throws.
  return { senderRow, receiverRow };
}
