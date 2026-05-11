/**
 * @mention E2E tests.
 *
 * Wave P1-1 of the coverage plan. Locks down the three observable
 * behaviours of the current @mention implementation:
 *
 *   1. A mention of the recipient's nickname is detected on the
 *      receiver side: the `message-row` surfaces `data-mentions-me="true"`
 *      AND the rendered bubble content contains a
 *      `[data-testid="mention-highlight"]` span.
 *   2. The sender sees the SAME message, but with
 *      `data-mentions-me="false"` and no highlight span — a user cannot
 *      mention themself via an outgoing text they authored (the local
 *      projection always sets `mentions_me = false`).
 *   3. A mention whose target is NOT the recipient's nickname does
 *      not flag the message on the receiver side (negative path —
 *      anchors the boundary rule so a future broadening of the
 *      extractor shows up as a test change).
 *
 * --- Scope note / real product gaps ---
 * The tests deliberately avoid flows that would require features that
 * do not exist yet in the product:
 *
 *   (G5) No `@`-trigger autocomplete UI — typing `@` inside the
 *        composer does nothing. There is therefore no suggestion-list
 *        interaction to assert.
 *   (G6) The sidebar unread badge does not differentiate a
 *        mention-bearing unread from a plain unread — there is a
 *        single `unread-badge` testid. Any "+mention" badge
 *        assertion is deferred until the feature ships.
 *   (G7) The Chat-type room ChatView does NOT mount the
 *        `MemberListPanel` (only Theater rooms do). Consequently the
 *        "Mention in chat" context-menu entry is unreachable from a
 *        normal Chat room. Exercising the entry via the member list
 *        is deferred to the Theater spec (Wave P2).
 *   (G8) The wire protocol's `DataChannelMessage::ChatText` does not
 *        carry a `mentions: Vec<UserId>` field. Mentions are
 *        re-extracted on the receiver side from the plain-text
 *        content. The tests exercise that path directly — once a
 *        protocol-level mentions field lands, a new test will
 *        validate cross-peer parity between the sent list and the
 *        rendered list.
 *
 * All three tests run over a standard direct (peer-to-peer) chat, for
 * which the `ChatText` frame is correctly attributed to the direct
 * conversation (unlike room chats — see the room.spec.ts scope note).
 */

import { expect, test } from '../fixtures/test-base.ts';
import {
  establishConnection,
  registerAndLogin,
  sendAndVerifyMessage,
} from '../fixtures/helpers.ts';
import { sel } from '../utils/selectors.ts';

test.describe('@mention detection and rendering', () => {
  test('recipient sees mention-highlight span and data-mentions-me="true"', async ({
    pageA,
    pageB,
    server,
  }) => {
    const userA = await registerAndLogin(pageA, server, { hint: 'mention-hl-a' });
    const userB = await registerAndLogin(pageB, server, { hint: 'mention-hl-b' });
    await establishConnection(pageA, pageB, userB.username);

    const text = `@${userB.username} morning!`;
    const { receiverRow } = await sendAndVerifyMessage(pageA, pageB, text);

    // Receiver: the row announces itself as "this message mentions me".
    await expect(receiverRow).toHaveAttribute('data-mentions-me', 'true', {
      timeout: 10_000,
    });

    // Receiver: the rendered bubble content wraps the `@<selfNick>` token
    // in a testid-tagged span. The wrap only fires for the LOCAL user's
    // nickname (by design), so this is the canonical assertion site.
    const highlight = receiverRow.locator(sel.mentionHighlight);
    await expect(highlight).toBeVisible({ timeout: 5_000 });
    await expect(highlight).toHaveText(`@${userB.username}`);

    // Suppress the "userA is defined but not used" lint in TS strict mode.
    expect(userA.username).toBeTruthy();
  });

  test('sender sees the same message with mentions_me=false and no highlight', async ({
    pageA,
    pageB,
    server,
  }) => {
    const userA = await registerAndLogin(pageA, server, { hint: 'mention-self-a' });
    const userB = await registerAndLogin(pageB, server, { hint: 'mention-self-b' });
    await establishConnection(pageA, pageB, userB.username);

    const text = `@${userB.username} ping`;
    const { senderRow } = await sendAndVerifyMessage(pageA, pageB, text);

    // The sender's local projection of the outgoing message explicitly
    // sets `mentions_me = false` in `chat::manager::outbound` — a user
    // cannot mention themself by typing an outgoing text.
    await expect(senderRow).toHaveAttribute('data-mentions-me', 'false', {
      timeout: 10_000,
    });

    // The highlight span is only wrapped around `@<self_nickname>`.
    // Since A's `self_nickname = userA.username` (registration default),
    // and the outgoing text mentions userB, no highlight can be present
    // on the sender's bubble.
    await expect(senderRow.locator(sel.mentionHighlight)).toHaveCount(0);
    expect(userA.username).toBeTruthy();
  });

  test('mention of a different name does not flag the receiver', async ({
    pageA,
    pageB,
    server,
  }) => {
    const userA = await registerAndLogin(pageA, server, { hint: 'mention-neg-a' });
    const userB = await registerAndLogin(pageB, server, { hint: 'mention-neg-b' });
    await establishConnection(pageA, pageB, userB.username);

    // Intentionally mention a name that is guaranteed not to equal
    // userB's nickname (unique random prefix + suffix).
    const otherName = `nobody_${Math.random().toString(36).slice(2, 8)}`;
    const text = `heads up @${otherName} — ignore this`;
    const { receiverRow } = await sendAndVerifyMessage(pageA, pageB, text);

    await expect(receiverRow).toHaveAttribute('data-mentions-me', 'false', {
      timeout: 10_000,
    });
    await expect(receiverRow.locator(sel.mentionHighlight)).toHaveCount(0);
    expect(userA.username).toBeTruthy();
  });
});
