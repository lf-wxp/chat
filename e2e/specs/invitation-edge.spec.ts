/**
 * Invitation edge-case E2E tests.
 *
 * Maps to: Requirement 16.3 (Connection Invitation & Chat Session
 * Establishment), Req 9.9 (duplicate invitation guard), Req 9.2
 * (blacklist auto-decline).
 *
 * Covers scenarios the baseline `invitation.spec.ts` does not:
 *   1. Blocked inviter → auto-decline, no modal shown on the blocked
 *      side (Req 9.2 blacklist).
 *   2. Duplicate outbound invite is suppressed: while the first invite
 *      is still pending, clicking the button again does not spawn a
 *      second invite modal on the receiver.
 *   3. After a decline, the inviter can issue a fresh invite and this
 *      time the invitee accepts → both sides reach the chat view.
 *   4. A decline clears the Connecting indicator and re-enables the
 *      invite button so the next attempt starts from a clean state.
 *
 * Note on bidirectional-merge coverage: the server has logic to merge
 * concurrent invites from both directions (see
 * `server::ws::invite::handle_invite`), but exercising it end-to-end
 * requires B to send an invite while still having A's incoming-invite
 * modal on screen. The modal captures keyboard + focus and declines
 * the pending invite on any dismiss (Escape / backdrop), which loses
 * the bidirectional state. Covering that branch reliably needs a
 * JS-side signalling hook — deferred to a later wave; unit tests in
 * `server/src/ws/invite/tests.rs` already cover the merge algorithm.
 */

import { sel } from '../utils/selectors.ts';
import { registerAndLogin } from '../fixtures/helpers.ts';
import { waitForOnlineUser } from '../utils/wait-helpers.ts';
import { expect, test } from '../fixtures/test-base.ts';

test.describe('invitation edge cases', () => {
  test('invites from a blocked user are auto-declined without a modal', async ({
    pageA,
    pageB,
    server,
  }) => {
    const a = await registerAndLogin(pageA, server, { hint: 'a' });
    const b = await registerAndLogin(pageB, server, { hint: 'b' });

    // B blocks A via the user info card.
    const rowOnB = await waitForOnlineUser(pageB, a.username);
    await rowOnB.click();
    await pageB.locator(sel.userInfoCard).waitFor({ state: 'visible' });
    await pageB.locator(sel.userInfoBlock).click();
    // Press Escape to close the card. The user-info card's Escape
    // handler is a plain modal-close — unlike the incoming-invite
    // modal which declines the invite.
    await pageB.keyboard.press('Escape');
    // Wait for the backdrop to fully disappear before proceeding
    await pageB.locator(sel.userInfoBackdrop).waitFor({ state: 'hidden', timeout: 10_000 });

    // Capture B's console logs to debug the blacklist check.
    const bLogs: string[] = [];
    pageB.on('console', (msg) => {
      bLogs.push(msg.text());
    });

    // A sends an invite to B.
    const rowOnA = await waitForOnlineUser(pageA, b.username);
    await rowOnA.click();
    await pageA.locator(sel.userInfoCard).waitFor({ state: 'visible' });
    await pageA.locator(sel.userInfoInvite).click();

    // Wait a bit for the invite to be processed on B's side.
    await pageB.waitForTimeout(3000);

    // Dump captured logs for debugging.
    // eslint-disable-next-line no-console
    for (const log of bLogs) {
      // eslint-disable-next-line no-console
      console.log(log);
    }

    // B's incoming-invite modal must NOT appear.    // decline timer fires within AUTO_DECLINE_MIN_MS..AUTO_DECLINE_MAX_MS
    // (~0.5–2 s) so a 5 s polling window comfortably contains both
    // extremes plus a safety margin.
    await expect(pageB.locator(sel.incomingInviteModal)).toHaveCount(0, { timeout: 5_000 });

    // A eventually receives the decline (Connecting indicator clears).
    await expect(pageA.locator(sel.userInfoConnecting)).toBeHidden({ timeout: 15_000 });

    // Neither side has a chat view.
    await expect(pageA.locator(sel.chatView)).toHaveCount(0);
    await expect(pageB.locator(sel.chatView)).toHaveCount(0);
  });

  test('duplicate outbound invite is suppressed while pending', async ({
    pageA,
    pageB,
    server,
  }) => {
    await registerAndLogin(pageA, server, { hint: 'a' });
    const b = await registerAndLogin(pageB, server, { hint: 'b' });

    const rowOnA = await waitForOnlineUser(pageA, b.username);
    await rowOnA.click();
    await pageA.locator(sel.userInfoCard).waitFor({ state: 'visible' });

    // First click: sends the invite.
    await pageA.locator(sel.userInfoInvite).click();

    // Wait for B to receive the modal — once B has the incoming
    // invite the pending state on A is guaranteed.
    await expect(pageB.locator(sel.incomingInviteModal)).toBeVisible({ timeout: 10_000 });

    // Second click while pending — no new ConnectionInvite should be
    // emitted. `force: true` bypasses Playwright's disabled-button
    // heuristics; the app relies on an internal pending guard, not
    // on the HTML disabled attribute.
    await pageA.locator(sel.userInfoInvite).click({ force: true });

    // A duplicate invite, if it leaked through, would cause B's
    // modal to re-render / flicker a second invite entry. We assert
    // the observable effect on B: the modal count stays at exactly
    // one across a 2 s stabilisation window.
    for (let i = 0; i < 4; i += 1) {
      await expect(pageB.locator(sel.incomingInviteModal)).toHaveCount(1);
      await pageA.waitForTimeout(500);
    }

    // The invite button on A is still rendered (in its "pending"
    // variant) — the duplicate click did not reset the card.
    await expect(pageA.locator(sel.userInfoInvite)).toBeVisible();
  });

  test('decline then re-invite succeeds', async ({ pageA, pageB, server }) => {
    await registerAndLogin(pageA, server, { hint: 'a' });
    const b = await registerAndLogin(pageB, server, { hint: 'b' });

    // First round: A invites, B declines.
    const rowOnA = await waitForOnlineUser(pageA, b.username);
    await rowOnA.click();
    await pageA.locator(sel.userInfoCard).waitFor({ state: 'visible' });
    await pageA.locator(sel.userInfoInvite).click();

    await pageB.locator(sel.incomingInviteModal).waitFor({ state: 'visible', timeout: 15_000 });
    await pageB.locator(sel.inviteDecline).click();

    // The Connecting indicator on A goes away — freeing the invite
    // button for a second attempt.
    await expect(pageA.locator(sel.userInfoConnecting)).toBeHidden({ timeout: 15_000 });

    // Second round: A invites again, B accepts this time. The info
    // card might have auto-closed on decline, so re-open it if
    // necessary before clicking invite.
    const cardA = pageA.locator(sel.userInfoCard);
    if (!(await cardA.isVisible())) {
      await (await waitForOnlineUser(pageA, b.username)).click();
      await cardA.waitFor({ state: 'visible' });
    }
    await pageA.locator(sel.userInfoInvite).click();

    await pageB.locator(sel.incomingInviteModal).waitFor({ state: 'visible', timeout: 15_000 });
    // ModalWrapper schedules `modal-backdrop-visible` on the next
    // animation frame (≈20 ms after mount) so the entry transition
    // can play. Clicking the Accept button while that transition
    // is still in-flight makes Playwright report the element as
    // "not stable" and either retry or detach. Wait for the
    // visible-class to land before interacting.
    await expect(pageB.locator(sel.inviteBackdrop)).toHaveClass(/modal-backdrop-visible/, {
      timeout: 5_000,
    });
    await pageB.locator(sel.inviteAccept).click();

    // Wait for the incoming-invite modal backdrop to fully disappear.
    // After clicking Accept the ModalWrapper runs a ~350 ms exit
    // animation (removing `modal-backdrop-visible`). If we proceed
    // before the backdrop is gone it intercepts pointer events and
    // every subsequent click on pageB times out.
    await pageB.locator(sel.inviteBackdrop).waitFor({ state: 'hidden', timeout: 10_000 });

    // Both sides land on chat view AND complete ECDH.
    await Promise.all([
      pageA.locator(sel.chatView).waitFor({ state: 'visible', timeout: 20_000 }),
      pageB.locator(sel.chatView).waitFor({ state: 'visible', timeout: 20_000 }),
    ]);
    await expect(pageA.locator(sel.e2eeReadySentinel).first()).toHaveAttribute(
      'data-ready',
      'true',
      { timeout: 60_000 },
    );
    await expect(pageB.locator(sel.e2eeReadySentinel).first()).toHaveAttribute(
      'data-ready',
      'true',
      { timeout: 60_000 },
    );
  });

  test('decline clears the connecting indicator and re-enables invite', async ({
    pageA,
    pageB,
    server,
  }) => {
    await registerAndLogin(pageA, server, { hint: 'a' });
    const b = await registerAndLogin(pageB, server, { hint: 'b' });

    const rowOnA = await waitForOnlineUser(pageA, b.username);
    await rowOnA.click();
    await pageA.locator(sel.userInfoCard).waitFor({ state: 'visible' });

    // A is initially able to click the invite button.
    const inviteBtn = pageA.locator(sel.userInfoInvite);
    await expect(inviteBtn).toBeEnabled();

    // Send the invite; B declines it immediately.
    await inviteBtn.click();
    await pageB.locator(sel.incomingInviteModal).waitFor({ state: 'visible', timeout: 15_000 });
    await pageB.locator(sel.inviteDecline).click();

    // After the decline propagates back, A's Connecting indicator
    // must disappear AND the invite button must be clickable again
    // (re-opening the card if it auto-closed).
    await expect(pageA.locator(sel.userInfoConnecting)).toBeHidden({ timeout: 15_000 });
    if (!(await pageA.locator(sel.userInfoCard).isVisible())) {
      await (await waitForOnlineUser(pageA, b.username)).click();
      await pageA.locator(sel.userInfoCard).waitFor({ state: 'visible' });
    }
    await expect(pageA.locator(sel.userInfoInvite)).toBeEnabled();
  });
});
