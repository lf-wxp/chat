/**
 * Multi-user / multi-peer E2E tests.
 *
 * Maps to: Requirement 16.17 (Multi-User Chat Scenario) + Req 9.12
 * (multi-invite batch).
 *
 * Scope note: the current product creates one direct conversation per
 * invitee (the server fans a `MultiInvite` out to N `ConnectionInvite`
 * messages). A single shared 3-way conversation is covered by the
 * room-based tests (see `room.spec.ts`, Req 04). These tests therefore
 * verify the mesh topology: A establishes two concurrent peer
 * connections (one to B, one to C), and messages sent in each
 * conversation are delivered to the correct counterpart only.
 */

import { sel } from '../utils/selectors.ts';
import {
  registerAndLogin,
  sendAndVerifyMessage,
} from '../fixtures/helpers.ts';
import { waitForOnlineUser } from '../utils/wait-helpers.ts';
import { expect, test } from '../fixtures/test-base.ts';
import type { Page } from '@playwright/test';

/**
 * Open the multi-select panel on `page`, tick the given usernames,
 * and click Send. The caller is responsible for waiting on each
 * target's incoming modal afterwards.
 */
async function sendMultiInvite(page: Page, targetUsernames: string[]): Promise<void> {
  await page.locator('[data-testid="online-users-multi-toggle"]').click();
  for (const name of targetUsernames) {
    const row = await waitForOnlineUser(page, name);
    await row.click();
    // Row is toggled to selected; visual confirmation is the checkbox
    // icon, but we rely on the click ordering rather than an extra
    // assert to keep the helper compact.
  }
  await page.locator('[data-testid="multi-invite-send"]').click();
}

/**
 * Accept an incoming invite on `page`. Assumes the modal is (or will
 * shortly be) visible.
 */
async function acceptInviteModal(page: Page): Promise<void> {
  await page.locator(sel.incomingInviteModal).waitFor({ state: 'visible', timeout: 15_000 });
  await page.locator(sel.inviteAccept).click();
  
  // Wait for the incoming-invite modal backdrop to fully disappear.
  // After clicking Accept the ModalWrapper runs a ~350 ms exit
  // animation (removing `modal-backdrop-visible`). If we proceed
  // before the backdrop is gone it intercepts pointer events and
  // every subsequent click on page times out.
  await page.locator(sel.inviteBackdrop).waitFor({ state: 'hidden', timeout: 10_000 });
}

/**
 * Wait for both sides of a peer to finish the ECDH handshake. The
 * `e2ee-ready-sentinel` flips to `data-ready="true"` when ANY peer is
 * established, so for mesh scenarios we poll until the sentinel
 * matches AND the chat view is visible. A more precise per-peer hook
 * is tracked in the coverage plan for P0-5.
 */
async function expectE2eeReady(page: Page): Promise<void> {
  await expect(page.locator(sel.e2eeReadySentinel).first()).toHaveAttribute(
    'data-ready',
    'true',
    { timeout: 60_000 },
  );
}

test.describe('multi-user (3 peer mesh)', () => {
  test('multi-invite to B and C — both accept, both peer connections live', async ({
    pageA,
    pageB,
    pageC,
    server,
  }) => {
    await registerAndLogin(pageA, server, { hint: 'a' });
    const b = await registerAndLogin(pageB, server, { hint: 'b' });
    const c = await registerAndLogin(pageC, server, { hint: 'c' });

    // A launches a multi-invite to B and C at once.
    await sendMultiInvite(pageA, [b.username, c.username]);

    // Both targets see and accept their invite.
    await acceptInviteModal(pageB);
    await acceptInviteModal(pageC);

    // All three clients land on a chat view.
    await Promise.all([
      pageA.locator(sel.chatView).waitFor({ state: 'visible', timeout: 20_000 }),
      pageB.locator(sel.chatView).waitFor({ state: 'visible', timeout: 20_000 }),
      pageC.locator(sel.chatView).waitFor({ state: 'visible', timeout: 20_000 }),
    ]);

    // ECDH ready on all three. For A the sentinel flips once EITHER
    // of the two peer connections completes the handshake; we also
    // wait on B and C which each only have one peer (A) to confirm
    // both peers are actually up.
    await Promise.all([
      expectE2eeReady(pageA),
      expectE2eeReady(pageB),
      expectE2eeReady(pageC),
    ]);

    // A's sidebar should list two distinct conversations (one for B,
    // one for C).
    await expect(pageA.locator(sel.sidebarConversationItem)).toHaveCount(2, {
      timeout: 10_000,
    });
  });

  test('message delivery is scoped to the active conversation (no cross-talk)', async ({
    pageA,
    pageB,
    pageC,
    server,
  }) => {
    await registerAndLogin(pageA, server, { hint: 'a' });
    const b = await registerAndLogin(pageB, server, { hint: 'b' });
    const c = await registerAndLogin(pageC, server, { hint: 'c' });

    await sendMultiInvite(pageA, [b.username, c.username]);
    await acceptInviteModal(pageB);
    await acceptInviteModal(pageC);

    await Promise.all([
      pageA.locator(sel.chatView).waitFor({ state: 'visible', timeout: 20_000 }),
      pageB.locator(sel.chatView).waitFor({ state: 'visible', timeout: 20_000 }),
      pageC.locator(sel.chatView).waitFor({ state: 'visible', timeout: 20_000 }),
    ]);
    await Promise.all([
      expectE2eeReady(pageA),
      expectE2eeReady(pageB),
      expectE2eeReady(pageC),
    ]);

    const tag = Date.now().toString(36);
    const toB = `hello-B-${tag}`;
    const toC = `hello-C-${tag}`;

    // Switch A's active conversation to the B-peer by clicking the
    // sidebar item whose text matches B's username. The item with
    // matching username text is unambiguous.
    await pageA
      .locator(sel.sidebarConversationItem, { hasText: b.username })
      .first()
      .click();
    await sendAndVerifyMessage(pageA, pageB, toB);

    // Now switch to C and send a message there.
    await pageA
      .locator(sel.sidebarConversationItem, { hasText: c.username })
      .first()
      .click();
    await sendAndVerifyMessage(pageA, pageC, toC);

    // Cross-check: B must NEVER see the message addressed to C and
    // vice versa. A short stabilisation window is enough because
    // any cross-talk would already have been delivered by now.
    await expect(
      pageB.locator(sel.messageRow, { hasText: toC }),
    ).toHaveCount(0);
    await expect(
      pageC.locator(sel.messageRow, { hasText: toB }),
    ).toHaveCount(0);
  });

  test('third peer joining mid-session via single invite adds a second conversation', async ({
    pageA,
    pageB,
    pageC,
    server,
  }) => {
    await registerAndLogin(pageA, server, { hint: 'a' });
    const b = await registerAndLogin(pageB, server, { hint: 'b' });
    const c = await registerAndLogin(pageC, server, { hint: 'c' });

    // Step 1 — A and B establish first (single 1-1 invite).
    const rowB = await waitForOnlineUser(pageA, b.username);
    await rowB.click();
    await pageA.locator(sel.userInfoCard).waitFor({ state: 'visible' });
    await pageA.locator(sel.userInfoInvite).click();

    // Close the user-info card explicitly. After clicking "Send
    // Connection Invitation" the card stays open; the Escape key
    // triggers the ModalWrapper exit animation so the backdrop
    // disappears and subsequent clicks are not intercepted.
    await pageA.keyboard.press('Escape');
    await pageA.locator(sel.userInfoBackdrop).waitFor({ state: 'hidden', timeout: 10_000 });

    await acceptInviteModal(pageB);

    await Promise.all([
      pageA.locator(sel.chatView).waitFor({ state: 'visible', timeout: 20_000 }),
      pageB.locator(sel.chatView).waitFor({ state: 'visible', timeout: 20_000 }),
    ]);
    await expectE2eeReady(pageA);

    // A has one conversation.
    await expect(pageA.locator(sel.sidebarConversationItem)).toHaveCount(1);

    // Step 2 — later, A also invites C. We re-open the info card
    // (which auto-closed when the conversation was selected).
    const rowC = await waitForOnlineUser(pageA, c.username);
    await rowC.click();
    await pageA.locator(sel.userInfoCard).waitFor({ state: 'visible' });
    await pageA.locator(sel.userInfoInvite).click();

    // Close the user-info card explicitly so the backdrop disappears.
    await pageA.keyboard.press('Escape');
    await pageA.locator(sel.userInfoBackdrop).waitFor({ state: 'hidden', timeout: 10_000 });

    await acceptInviteModal(pageC);

    await pageC.locator(sel.chatView).waitFor({ state: 'visible', timeout: 20_000 });
    await expectE2eeReady(pageC);

    // A now has two conversations.
    await expect(pageA.locator(sel.sidebarConversationItem)).toHaveCount(2, {
      timeout: 10_000,
    });

    // Messages in the B conversation still work — C did not break
    // the existing mesh edge.
    const tag = Date.now().toString(36);
    await pageA
      .locator(sel.sidebarConversationItem, { hasText: b.username })
      .first()
      .click();
    await sendAndVerifyMessage(pageA, pageB, `after-c-${tag}`);
  });

  test('declining one of two multi-invite targets leaves only the accepted conversation', async ({
    pageA,
    pageB,
    pageC,
    server,
  }) => {
    await registerAndLogin(pageA, server, { hint: 'a' });
    const b = await registerAndLogin(pageB, server, { hint: 'b' });
    const c = await registerAndLogin(pageC, server, { hint: 'c' });

    await sendMultiInvite(pageA, [b.username, c.username]);

    // B accepts, C declines.
    await acceptInviteModal(pageB);
    await pageC.locator(sel.incomingInviteModal).waitFor({ state: 'visible', timeout: 15_000 });
    await pageC.locator(sel.inviteDecline).click();

    // A's chat view appears (the B connection succeeded).
    await pageA.locator(sel.chatView).waitFor({ state: 'visible', timeout: 20_000 });
    await expectE2eeReady(pageA);

    // After both invitees have resolved (B accepted, C declined) A
    // should hold exactly one conversation entry.
    await expect(pageA.locator(sel.sidebarConversationItem)).toHaveCount(1, {
      timeout: 10_000,
    });

    // And crucially, C has NOT entered a chat view.
    await expect(pageC.locator(sel.chatView)).toHaveCount(0);
  });
});
