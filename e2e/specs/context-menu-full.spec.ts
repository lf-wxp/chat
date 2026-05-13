/**
 * Context-menu / message-actions extended coverage (Wave P1-7).
 *
 * Maps to: Req 16.9 message context-menu actions. The existing
 * `message-actions.spec.ts` already covers reply, forward, and
 * revoke. This spec fills in the remaining gaps from plan §3:
 *
 *   1. **Copy text** — clicking `message-action-copy` writes the
 *      message's plain-text projection to the system clipboard.
 *      Verified by reading back from `navigator.clipboard.readText()`
 *      via `page.evaluate`.
 *   2. **Jump-to-quoted-message highlight flash** — replying to a
 *      message renders a clickable reply-block on the new bubble;
 *      clicking it scrolls the original into view AND attaches the
 *      `message-highlight` CSS class for 1.5 s
 *      (`message_list.rs:434-441`).
 *   3. **Action toolbar present on file message bubbles** — the
 *      hover-action toolbar (reply / react / forward / copy) must
 *      render on file-card bubbles too, not just plain-text ones.
 *      Locks down the assertion that the toolbar is content-type-
 *      agnostic. (Image / voice bubbles are deferred to the
 *      P2-2 / P2-3 specs.)
 *
 * --- Scope notes ---
 * Plan §3 P1-7 originally listed four sub-features. The fourth —
 * "context menu on image / voice / file bubbles (differs by type)"
 * — is partially covered here (file-only) because:
 *   * The image / voice content paths are blocked on the missing
 *     P2-2 (`voice-message`) and P2-3 (`image-message`) specs and
 *     their associated fixtures (microphone capture / clipboard
 *     image paste). They will land naturally alongside those
 *     specs in Wave P2.
 *   * The toolbar itself does NOT differentiate by content type —
 *     it always renders the same reply / react / forward / copy
 *     buttons. The "differs by type" wording in the plan was
 *     aspirational; today the only content-typed differentiation
 *     is the bubble body, not the actions row.
 */

import { expect, test, type Page } from '../fixtures/test-base.ts';
import {
  establishConnection,
  registerAndLogin,
  sendAndVerifyMessage,
} from '../fixtures/helpers.ts';
import { sel } from '../utils/selectors.ts';

/** Read the system clipboard from inside `page`. Requires the
 *  `clipboard-read` permission (granted by every context fixture
 *  in `test-base.ts`). */
async function readClipboard(page: Page): Promise<string> {
  return page.evaluate(async () => {
    try {
      return await navigator.clipboard.readText();
    } catch {
      return '';
    }
  });
}

test.describe('message-actions extended', () => {
  test('copy action writes the message body to the clipboard', async ({
    pageA,
    pageB,
    server,
  }) => {
    await registerAndLogin(pageA, server, { hint: 'cmf-cp-a' });
    const userB = await registerAndLogin(pageB, server, { hint: 'cmf-cp-b' });
    await establishConnection(pageA, pageB, userB.username);

    const tag = `clip-${Math.random().toString(36).slice(2, 10)}`;
    const { senderRow } = await sendAndVerifyMessage(pageA, pageB, tag);

    // Hover to make sure the actions toolbar is mounted (some
    // CSS strategies hide it until hover; the testid query
    // succeeds regardless because the buttons live in the DOM,
    // but the click is more reliable with the row focused).
    await senderRow.hover();
    await senderRow.locator(sel.messageActionCopy).click();

    // Clipboard now contains the plain-text projection of the
    // message. The projection is the markdown source for plain
    // text messages, so the literal `tag` is sufficient.
    await expect.poll(async () => readClipboard(pageA), { timeout: 5_000 }).toContain(tag);
  });

  test('clicking a reply-block on a quoted message jumps + flashes the original', async ({
    pageA,
    pageB,
    server,
  }) => {
    await registerAndLogin(pageA, server, { hint: 'cmf-jp-a' });
    const userB = await registerAndLogin(pageB, server, { hint: 'cmf-jp-b' });
    await establishConnection(pageA, pageB, userB.username);

    const original = `orig-${Math.random().toString(36).slice(2, 10)}`;
    await sendAndVerifyMessage(pageA, pageB, original);

    // Locate the original on the receiver side, then issue a reply
    // FROM B so the reply-block lives in B's chat view.
    const origOnB = pageB.locator(sel.messageRow, { hasText: original }).first();
    await origOnB.hover();
    await origOnB.locator(sel.messageActionReply).click();
    await expect(pageB.locator(sel.replyPreviewBar)).toBeVisible({ timeout: 5_000 });

    const replyText = `reply-${Math.random().toString(36).slice(2, 10)}`;
    await sendAndVerifyMessage(pageB, pageA, replyText);

    // The reply on B carries the reply-block; its `data-target-id`
    // points at the original `MessageId`. We don't assert the id
    // value (it's a UUID generated client-side); we just click it
    // and verify the original gets the highlight class.
    const replyBubble = pageB.locator(sel.messageRow, { hasText: replyText }).first();
    const replyBlock = replyBubble.locator(sel.replyBlock);
    await expect(replyBlock).toBeVisible();
    await replyBlock.click();

    // The original gets `message-highlight` for 1.5 s. We assert
    // the class is present within a generous window, then that
    // it eventually clears.
    await expect.poll(
      async () =>
        (await origOnB.getAttribute('class'))?.includes('message-highlight') ?? false,
      { timeout: 3_000 },
    ).toBe(true);

    await expect.poll(
      async () =>
        (await origOnB.getAttribute('class'))?.includes('message-highlight') ?? false,
      { timeout: 4_000 },
    ).toBe(false);
  });

  test('action toolbar (reply / react / forward / copy) is rendered on file-card bubbles', async ({
    pageA,
    pageB,
    server,
  }) => {
    await registerAndLogin(pageA, server, { hint: 'cmf-fl-a' });
    const userB = await registerAndLogin(pageB, server, { hint: 'cmf-fl-b' });
    await establishConnection(pageA, pageB, userB.username);

    // Drive the file picker with a tiny in-memory file. We reuse
    // the technique from `file-transfer.spec.ts`: set the input
    // via `setInputFiles` with an explicit Buffer payload so we
    // don't depend on local filesystem state.
    const fileInput = pageA.locator(sel.filePickerInput);
    await fileInput.setInputFiles({
      name: 'note.txt',
      mimeType: 'text/plain',
      buffer: Buffer.from('hello-from-context-menu-test', 'utf8'),
    });

    // Both sides see the file card.
    const senderCard = pageA.locator(sel.messageFile).first();
    const receiverCard = pageB.locator(sel.messageFile).first();
    await expect(senderCard).toBeVisible({ timeout: 30_000 });
    await expect(receiverCard).toBeVisible({ timeout: 30_000 });

    // The bubble that wraps the file card on the sender side must
    // expose the same action toolbar as a text bubble. We resolve
    // the enclosing message-row, then assert each action testid
    // is present on it.
    const senderRow = pageA.locator(sel.messageRow).filter({ has: senderCard });
    await senderRow.hover();
    await expect(senderRow.locator(sel.messageActionReply)).toBeVisible();
    await expect(senderRow.locator(sel.messageActionReact)).toBeVisible();
    await expect(senderRow.locator(sel.messageActionForward)).toBeVisible();
    await expect(senderRow.locator(sel.messageActionCopy)).toBeVisible();
  });
});
