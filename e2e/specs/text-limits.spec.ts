/**
 * Text-message edge cases: length cap, empty-message guard, HTML escape.
 *
 * Maps to: Requirement 16.4 gaps tracked by Wave P2-10.
 *
 * The composer (`components/chat_view/input_bar.rs`) enforces three
 * invariants that none of the existing specs cover:
 *
 *   1. **`MAX_TEXT_LENGTH = 10_000` characters** — when the draft
 *      exceeds the cap, the character counter flips to the
 *      `over-limit` modifier class, the textarea applies `maxlength`,
 *      and the send button's `can_send` memo collapses to `false` so
 *      the click no-ops. The button reports `disabled` to a11y tools
 *      via the rendered `disabled` attribute.
 *
 *   2. **Empty / whitespace-only message guard** — `can_send` requires
 *      `char_count > 0`. Pressing Enter with an empty draft (or
 *      typing-then-clearing) must not produce a wire frame; we assert
 *      that no `message-row` is rendered on the receiver during a
 *      short window. The Send button itself is also disabled.
 *
 *   3. **HTML entity rendering / XSS containment** — `&lt;script&gt;`-
 *      shaped raw input must NOT execute as a `<script>` element on
 *      the receiver: the rendered bubble carries the literal text and
 *      no `<script>` child node, and the page's `window.__xss_pwned`
 *      sentinel (which a successful injection would set) stays
 *      undefined. The renderer goes through pulldown-cmark's HTML
 *      escape path, so this is a guard against any future swap to a
 *      raw-HTML rendering strategy.
 */

import { sel } from '../utils/selectors.ts';
import {
  establishConnection,
  registerAndLogin,
  sendAndVerifyMessage,
} from '../fixtures/helpers.ts';
import { expect, test } from '../fixtures/test-base.ts';

test.describe('text limits', () => {
  test.beforeEach(async ({ pageA, pageB, server }) => {
    await registerAndLogin(pageA, server, { hint: 'tl-a' });
    const b = await registerAndLogin(pageB, server, { hint: 'tl-b' });
    await establishConnection(pageA, pageB, b.username);
  });

  test('over-limit draft disables the send button and flags the counter', async ({
    pageA,
    pageB,
  }) => {
    const textarea = pageA.locator(sel.chatInputTextarea);
    const sendBtn = pageA.locator(sel.chatInputSend);

    // First, send a normal message so we know the connection is
    // healthy and have a baseline count of receiver bubbles.
    const tag = Date.now().toString(36);
    const baseline = `baseline-${tag}`;
    await sendAndVerifyMessage(pageA, pageB, baseline);
    const beforeCount = await pageB.locator(sel.messageRow).count();

    // Push 10_001 characters into the draft. Playwright's `fill`
    // honours the textarea's `maxlength` attribute and would clamp
    // the value to 10_000, hiding the over-limit branch entirely.
    // We bypass the cap by setting `.value` directly via evaluate and
    // dispatching an `input` event so the Leptos handler picks it up.
    const tooLong = 'x'.repeat(10_001);
    await textarea.evaluate((el, value) => {
      const ta = el as HTMLTextAreaElement;
      // Lift the maxlength so the assignment is not silently truncated.
      ta.removeAttribute('maxlength');
      ta.value = value;
      ta.dispatchEvent(new Event('input', { bubbles: true }));
    }, tooLong);

    // The counter element flips to the `over-limit` modifier class.
    const counter = pageA.locator('.chat-input-counter');
    await expect(counter).toHaveClass(/over-limit/, { timeout: 5_000 });
    await expect(counter).toContainText('10001/10000');

    // The send button collapses `can_send` to false; rendered
    // `disabled` attribute must be present.
    await expect(sendBtn).toBeDisabled();

    // Pressing Enter on the textarea must NOT send: receiver row
    // count is unchanged after a short settle window.
    await textarea.press('Enter');
    await pageA.waitForTimeout(800);
    await expect(pageB.locator(sel.messageRow)).toHaveCount(beforeCount);
    await expect(
      pageB.locator(sel.messageRow, { hasText: tooLong.slice(0, 80) }),
    ).toHaveCount(0);
  });

  test('empty / whitespace-only draft cannot be sent', async ({ pageA, pageB }) => {
    const textarea = pageA.locator(sel.chatInputTextarea);
    const sendBtn = pageA.locator(sel.chatInputSend);

    // Snapshot the receiver bubble count before any attempted send.
    const beforeCount = await pageB.locator(sel.messageRow).count();

    // 1. Pristine empty draft → button disabled, Enter is a no-op.
    await expect(sendBtn).toBeDisabled();
    await textarea.click();
    await textarea.press('Enter');

    // 2. Typing then clearing reproduces the empty state.
    await textarea.fill('temporary');
    await expect(sendBtn).toBeEnabled();
    await textarea.fill('');
    await expect(sendBtn).toBeDisabled();
    await textarea.press('Enter');

    // 3. Whitespace-only string. The composer's `can_send` keys off
    //    `char_count > 0` (so `"   "` *is* enabled at the button
    //    level), but `do_send` then early-returns on
    //    `text.trim().is_empty()`. Either way no wire frame goes out.
    await textarea.fill('   ');
    await textarea.press('Enter');

    // Settle, then assert: no new message rendered on either side.
    await pageA.waitForTimeout(800);
    await expect(pageA.locator(sel.messageRow)).toHaveCount(0);
    await expect(pageB.locator(sel.messageRow)).toHaveCount(beforeCount);
  });

  test('HTML-shaped input is rendered as text, never as live markup', async ({
    pageA,
    pageB,
  }) => {
    const tag = Date.now().toString(36);
    // Plant an XSS sentinel on B; if the renderer ever evaluates a
    // <script> child this property will flip to truthy.
    await pageB.evaluate(() => {
      (window as unknown as { __xss_pwned?: boolean }).__xss_pwned = false;
    });

    const payload = `<script>window.__xss_pwned=true</script>xss-${tag}`;

    const textarea = pageA.locator(sel.chatInputTextarea);
    await textarea.fill(payload);
    await textarea.press('Enter');

    // Bubble appears on B keyed by the trailing literal — the text
    // content carries the original characters, not the executed
    // result. We use `xss-${tag}` (a portion that survives any
    // sanitisation) as the locator anchor.
    const row = pageB.locator(sel.messageRow, { hasText: `xss-${tag}` }).first();
    await expect(row).toBeVisible({ timeout: 20_000 });

    // No <script> element was injected into the bubble's subtree.
    await expect(row.locator('script')).toHaveCount(0);

    // The XSS sentinel was never set → the page never executed the
    // payload as live JS.
    const pwned = await pageB.evaluate(
      () => (window as unknown as { __xss_pwned?: boolean }).__xss_pwned === true,
    );
    expect(pwned).toBe(false);
  });
});
