/**
 * Real cross-peer forward delivery (Wave P2-9).
 *
 * `message-actions.spec.ts` already asserts that the forward modal
 * opens when the action button is clicked, but it never actually
 * picks a target conversation and never asserts that the receiver of
 * the forwarded copy renders the bubble. This spec closes that gap
 * with a 3-peer mesh:
 *
 *   * `pageA` connects to `pageB`     (so A has conv A↔B).
 *   * `pageA` connects to `pageC`     (so A has conv A↔C — the
 *                                       forward target).
 *   * `pageB` sends one message to A; A then forwards that bubble to
 *     the A↔C conversation. The expected outcome is:
 *       1. The forward modal closes.
 *       2. A's A↔C conversation gains a Forwarded bubble carrying
 *          the original text.
 *       3. C's A↔C conversation gains a corresponding Forwarded
 *          bubble — i.e. the wire frame really crossed the data
 *          channel, not just a local UI illusion.
 *
 * Then: chain-forwarding the just-forwarded bubble must be blocked.
 * The composer / modal renders the localized error string and never
 * dispatches a second forward (Req 4.6.x — error code `cht104`).
 */

import { sel } from '../utils/selectors.ts';
import {
  establishConnection,
  registerAndLogin,
  sendAndVerifyMessage,
} from '../fixtures/helpers.ts';
import { expect, test } from '../fixtures/test-base.ts';

test.describe('forward delivery', () => {
  test('forwarded message is delivered to the target conversation peer', async ({
    pageA,
    pageB,
    pageC,
    server,
  }) => {
    await registerAndLogin(pageA, server, { hint: 'fd-a' });
    const userB = await registerAndLogin(pageB, server, { hint: 'fd-b' });
    const userC = await registerAndLogin(pageC, server, { hint: 'fd-c' });

    // A forms two independent direct conversations.
    await establishConnection(pageA, pageB, userB.username);
    await establishConnection(pageA, pageC, userC.username);

    // Make sure A is focused on the conversation with B before B
    // sends — otherwise the inbound bubble would surface in a
    // background conversation and our forward action couldn't find
    // it through the visible message list. `establishConnection`
    // ends with both pages parked in their freshly-opened chat view;
    // A's last-active conversation is A↔C, so we click back to B.
    await pageA
      .locator(`${sel.sidebarConversationItem}:has-text("${userB.username}")`)
      .first()
      .click();
    await expect(pageA.locator(sel.chatView)).toBeVisible();

    // B sends a message to A which we will forward.
    const tag = Date.now().toString(36);
    const original = `forward-src-${tag}`;
    await sendAndVerifyMessage(pageB, pageA, original);

    // A hovers the bubble and clicks the forward action.
    const sourceRow = pageA.locator(sel.messageRow, { hasText: original }).first();
    await sourceRow.hover();
    await sourceRow.locator(sel.messageActionForward).click();

    const modal = pageA.locator(sel.forwardModal);
    await expect(modal).toBeVisible();

    // The candidate list is keyed by display name. We pick the row
    // matching userC.username — `establishConnection` already created
    // the A↔C conversation so it must appear in the list. Use a
    // text-contained <li> selector inside the modal scope.
    const targetOption = modal
      .locator('li[role="option"]', { hasText: userC.username })
      .first();
    await expect(targetOption).toBeVisible({ timeout: 10_000 });
    await targetOption.click();

    // Modal closes once `forward_message` returns Some.
    await expect(modal).toBeHidden({ timeout: 10_000 });

    // A is still focused on conv A↔B (forward sends to a different
    // conversation without switching the focus). Switch to A↔C to
    // confirm A's local bubble was appended there.
    await pageA
      .locator(`${sel.sidebarConversationItem}:has-text("${userC.username}")`)
      .first()
      .click();

    // The forwarded bubble on A's A↔C conversation carries the
    // original text. The Forwarded variant wraps the original
    // content; we don't depend on the wrapper styling — just on the
    // original text being rendered as a child somewhere in the
    // bubble. Limit the search to the chat view so we don't
    // accidentally match A's own bubble in the A↔B history (the
    // sidebar still shows A↔B with `original` as its preview
    // snippet).
    const aForwardedRow = pageA
      .locator(sel.chatView)
      .locator(sel.messageRow, { hasText: original })
      .first();
    await expect(aForwardedRow).toBeVisible({ timeout: 15_000 });

    // C receives the forwarded bubble across the wire on A↔C.
    const cForwardedRow = pageC.locator(sel.messageRow, { hasText: original }).first();
    await expect(cForwardedRow).toBeVisible({ timeout: 30_000 });
  });

  test('chain-forwarding an already-forwarded bubble is blocked', async ({
    pageA,
    pageB,
    pageC,
    server,
  }) => {
    await registerAndLogin(pageA, server, { hint: 'fd-chain-a' });
    const userB = await registerAndLogin(pageB, server, { hint: 'fd-chain-b' });
    const userC = await registerAndLogin(pageC, server, { hint: 'fd-chain-c' });

    await establishConnection(pageA, pageB, userB.username);
    await establishConnection(pageA, pageC, userC.username);

    // Step 1 — get a Forwarded bubble onto A's A↔C conversation by
    // forwarding a message from B.
    await pageA
      .locator(`${sel.sidebarConversationItem}:has-text("${userB.username}")`)
      .first()
      .click();
    const tag = Date.now().toString(36);
    const original = `chain-src-${tag}`;
    await sendAndVerifyMessage(pageB, pageA, original);

    const sourceRow = pageA.locator(sel.messageRow, { hasText: original }).first();
    await sourceRow.hover();
    await sourceRow.locator(sel.messageActionForward).click();

    const modal1 = pageA.locator(sel.forwardModal);
    await expect(modal1).toBeVisible();
    await modal1
      .locator('li[role="option"]', { hasText: userC.username })
      .first()
      .click();
    await expect(modal1).toBeHidden({ timeout: 10_000 });

    // Switch to the A↔C conversation where the Forwarded bubble now
    // lives.
    await pageA
      .locator(`${sel.sidebarConversationItem}:has-text("${userC.username}")`)
      .first()
      .click();
    const forwardedRow = pageA
      .locator(sel.chatView)
      .locator(sel.messageRow, { hasText: original })
      .first();
    await expect(forwardedRow).toBeVisible({ timeout: 15_000 });

    // Step 2 — try to forward the Forwarded bubble. The modal must
    // surface the chain-forbidden alert and refuse to dispatch.
    await forwardedRow.hover();
    await forwardedRow.locator(sel.messageActionForward).click();

    const modal2 = pageA.locator(sel.forwardModal);
    await expect(modal2).toBeVisible();

    // The alert div is rendered as soon as the Modal is opened with
    // a chain-forwarded source (the Memo `is_chain_forward` short-
    // circuits the candidate listbox).
    const alert = modal2.locator('div[role="alert"]');
    await expect(alert).toBeVisible({ timeout: 5_000 });
    await expect(alert).not.toBeEmpty();

    // The candidate listbox must not be rendered when chain
    // forwarding is detected.
    await expect(modal2.locator('ul[role="listbox"]')).toHaveCount(0);
  });
});
