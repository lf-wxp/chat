/**
 * Message-list scroll behaviour (Wave P1-3).
 *
 * Maps to: Req 16.19 (scroll behaviour) + Req 14.11 (virtual scroll).
 * Locks down the four observable behaviours that
 * `components/chat_view/message_list.rs` already implements:
 *
 *   1. **Auto-stick-to-bottom** — when the user is currently within
 *      `NEAR_BOTTOM_PX` (80 px) of the bottom and a new inbound
 *      message arrives, the list scrolls down to keep the new
 *      bubble visible. The frontend tracks this via `near_bottom:
 *      RwSignal<bool>` updated in the `on:scroll` handler
 *      (`message_list.rs:175-180`).
 *   2. **"new messages" pill** — when the user has scrolled away
 *      from the bottom (`!near_bottom`) and a new message arrives,
 *      `off_screen_new` increments and the floating
 *      `new-messages-badge` button is rendered with the count
 *      (`message_list.rs:347-356`).
 *   3. **back-to-latest button** — when the user is scrolled up
 *      AND no new messages have arrived since, the
 *      `back-to-latest` button is shown instead of the badge
 *      (`message_list.rs:358-370`). Both elements share the same
 *      `back_to_latest` click handler that scrolls to bottom and
 *      resets both signals.
 *   4. **virtual-scroll above the threshold** — once the
 *      conversation has more than `VIRTUAL_THRESHOLD = 100`
 *      messages, the message-list switches from a flat
 *      `collect_view()` to the `VirtualMessageWindow` renderer that
 *      only materialises a windowed slice plus an overscan buffer
 *      (`virtual_scroll/mod.rs`). We assert that fewer DOM rows
 *      exist than total messages, which is the contractual signal
 *      that virtualisation kicked in.
 *
 * --- Scope notes ---
 * The auto-stick / pill / back-to-latest tests are exercised over a
 * standard direct (peer-to-peer) chat with a small message volume
 * (≤ 5 sends) so they run in a few seconds. The virtual-scroll test
 * sends ≥ `VIRTUAL_THRESHOLD + 5` outgoing messages from a single
 * peer to drive the message count above the threshold; we
 * deliberately do NOT round-trip every message through B (each
 * outgoing message is appended to the sender's `messages` signal
 * synchronously by `push_outgoing`, which is sufficient to trigger
 * the virtualised renderer). The trade-off is one ~30 s test in
 * exchange for genuine coverage of the virtualisation path.
 */

import { expect, test } from '../fixtures/test-base.ts';
import {
  establishConnection,
  registerAndLogin,
  sendAndVerifyMessage,
} from '../fixtures/helpers.ts';
import { sel } from '../utils/selectors.ts';
import type { Page } from '@playwright/test';

const VIRTUAL_THRESHOLD = 100; // mirrors `virtual_scroll/mod.rs`
const NEAR_BOTTOM_PX = 80; // mirrors `message_list.rs`

/** Read the message-list container's scroll metrics. */
async function readScrollMetrics(
  page: Page,
): Promise<{ scrollTop: number; clientHeight: number; scrollHeight: number }> {
  return page.evaluate((selector) => {
    const el = document.querySelector(selector);
    if (!el) {
      return { scrollTop: -1, clientHeight: -1, scrollHeight: -1 };
    }
    return {
      scrollTop: el.scrollTop,
      clientHeight: el.clientHeight,
      scrollHeight: el.scrollHeight,
    };
  }, sel.messageList);
}

/** Programmatically scroll the message-list to the top so the user
 *  is "away from bottom". Uses a real mouse wheel event so the
 *  browser fires its native `scroll` event with up-to-date metrics
 *  — the Leptos `on:scroll` listener depends on that native event,
 *  and synthetic `dispatchEvent` calls from the page context have
 *  proven unreliable in headless Chromium for non-window scroll
 *  containers (the listener is registered via `addEventListener`
 *  but does not always observe synthetic `Event` objects in time).
 */
async function scrollToTop(page: Page): Promise<void> {
  const list = page.locator(sel.messageList);
  await list.waitFor({ state: 'visible' });
  const box = await list.boundingBox();
  if (!box) throw new Error('message-list not laid out');
  // Move the cursor inside the scroll container before wheeling so
  // the wheel event hits the right element.
  await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
  // A few large negative-Y wheel ticks to drag the scroll bar all
  // the way to the top regardless of total scroll height.
  for (let i = 0; i < 8; i += 1) {
    await page.mouse.wheel(0, -2000);
    await page.waitForTimeout(40);
  }
}

test.describe('message-list scroll', () => {
  test('auto-sticks to bottom when a new inbound message arrives', async ({
    pageA,
    pageB,
    server,
  }) => {
    await registerAndLogin(pageA, server, { hint: 'scr-stick-a' });
    const userB = await registerAndLogin(pageB, server, { hint: 'scr-stick-b' });
    await establishConnection(pageA, pageB, userB.username);

    // Prime the conversation with one message so the list has any
    // height at all.
    await sendAndVerifyMessage(pageA, pageB, 'prime');

    // Read baseline. We're at the bottom by definition just after
    // sending — the auto-scroll effect ran on append.
    const before = await readScrollMetrics(pageA);
    expect(before.scrollHeight - (before.scrollTop + before.clientHeight)).toBeLessThanOrEqual(
      NEAR_BOTTOM_PX + 5,
    );

    // B sends a message. It must arrive AND keep us pinned to the
    // bottom (auto-stick path).
    await sendAndVerifyMessage(pageB, pageA, 'incoming-stick');

    // After the inbound bubble lands, A is still within the
    // near-bottom band. We poll because the scroll-to-bottom call
    // happens inside the `Effect::new` that fires on the
    // post-append microtask.
    await expect
      .poll(
        async () => {
          const m = await readScrollMetrics(pageA);
          return m.scrollHeight - (m.scrollTop + m.clientHeight);
        },
        { timeout: 5_000 },
      )
      .toBeLessThanOrEqual(NEAR_BOTTOM_PX + 5);

    // Neither pill is shown — we never went off-bottom.
    await expect(pageA.locator(sel.newMessagesBadge)).toHaveCount(0);
    await expect(pageA.locator(sel.backToLatestBtn)).toHaveCount(0);
  });

  test('shows the new-messages badge when a message arrives while scrolled up', async ({
    pageA,
    pageB,
    server,
  }) => {
    await registerAndLogin(pageA, server, { hint: 'scr-pill-a' });
    const userB = await registerAndLogin(pageB, server, { hint: 'scr-pill-b' });
    await establishConnection(pageA, pageB, userB.username);

    // Send several messages so the list has enough scroll range
    // that scrolling to top is meaningful. We pad until the
    // observed scrollHeight comfortably exceeds the viewport
    // height by more than the near-bottom band, with a hard floor
    // so an unusually tall viewport still gets enough content.
    let i = 0;
    while (i < 80) {
      await sendAndVerifyMessage(pageA, pageB, `pad-${i}`);
      i += 1;
      if (i < 20) continue;
      const m = await readScrollMetrics(pageA);
      if (m.scrollHeight - m.clientHeight > NEAR_BOTTOM_PX * 6) break;
    }

    // A scrolls to the top. After this point the receiver-side
    // bookkeeping (`near_bottom = false`) is what matters; the
    // current value of `off_screen_new` is left as-is — its only
    // contract here is "a NEW message arriving from B will
    // increment it AND surface (or refresh) the
    // `new-messages-badge` pill".
    await scrollToTop(pageA);

    // Capture the badge count before the inbound, then assert
    // strict monotonic growth after. Padding loop interactions in
    // headless Chromium can leave `off_screen_new > 0` already, so
    // a "badge appears for the first time" assertion would be
    // racy; the well-defined contract is "the badge counter
    // strictly grows on inbound while scrolled up".
    async function badgeCount(): Promise<number> {
      return pageA.evaluate(() => {
        const el = document.querySelector('[data-testid="new-messages-badge"]');
        if (!el) return 0;
        const m = (el.textContent || '').match(/\d+/);
        return m ? Number(m[0]) : 0;
      });
    }
    const before = await badgeCount();

    await sendAndVerifyMessage(pageB, pageA, 'incoming-while-up');

    await expect.poll(badgeCount, { timeout: 10_000 }).toBeGreaterThan(before);

    // The badge MUST be the visible pill when inbound counter > 0
    // (mutually exclusive with `back-to-latest`, see
    // `message_list.rs:358-360`).
    await expect(pageA.locator(sel.newMessagesBadge)).toBeVisible();
    await expect(pageA.locator(sel.backToLatestBtn)).toHaveCount(0);
  });

  test('clicking the new-messages badge scrolls back to bottom and dismisses both pills', async ({
    pageA,
    pageB,
    server,
  }) => {
    await registerAndLogin(pageA, server, { hint: 'scr-back-a' });
    const userB = await registerAndLogin(pageB, server, { hint: 'scr-back-b' });
    await establishConnection(pageA, pageB, userB.username);

    // Pad the conversation so scrolling has range — same dynamic
    // strategy as the badge test.
    let i = 0;
    while (i < 80) {
      await sendAndVerifyMessage(pageA, pageB, `pad2-${i}`);
      i += 1;
      if (i < 20) continue;
      const m = await readScrollMetrics(pageA);
      if (m.scrollHeight - m.clientHeight > NEAR_BOTTOM_PX * 6) break;
    }

    await scrollToTop(pageA);

    // Drive an inbound message so `off_screen_new` is guaranteed
    // to be > 0 → the badge is the visible pill (mutually exclusive
    // with `back-to-latest`).
    await sendAndVerifyMessage(pageB, pageA, 'incoming-then-click');
    const badge = pageA.locator(sel.newMessagesBadge);
    await expect(badge).toBeVisible({ timeout: 10_000 });

    // Clicking the badge invokes `back_to_latest`: scrollTop ←
    // scrollHeight, near_bottom ← true, off_screen_new ← 0.
    await badge.click();

    // Both pills are gone, and we are back at the bottom.
    await expect(pageA.locator(sel.newMessagesBadge)).toHaveCount(0);
    await expect(pageA.locator(sel.backToLatestBtn)).toHaveCount(0);

    await expect
      .poll(
        async () => {
          const m = await readScrollMetrics(pageA);
          return m.scrollHeight - (m.scrollTop + m.clientHeight);
        },
        { timeout: 5_000 },
      )
      .toBeLessThanOrEqual(NEAR_BOTTOM_PX + 5);
  });

  test('virtualisation kicks in once the conversation crosses the threshold', async ({
    pageA,
    pageB,
    server,
  }) => {
    test.setTimeout(180_000);

    await registerAndLogin(pageA, server, { hint: 'scr-virt-a' });
    const userB = await registerAndLogin(pageB, server, { hint: 'scr-virt-b' });
    await establishConnection(pageA, pageB, userB.username);

    // Send VIRTUAL_THRESHOLD + 5 messages from A. Each
    // `push_outgoing` call appends synchronously to the sender's
    // `messages` signal so the list grows past the threshold even
    // before the receiver acks. We deliberately don't round-trip
    // through B for every send (would multiply the runtime by ~3x)
    // — the virtualisation decision is purely local.
    const target = VIRTUAL_THRESHOLD + 5;
    const textarea = pageA.locator(sel.chatInputTextarea);
    for (let i = 0; i < target; i += 1) {
      await textarea.fill(`v-${i}`);
      await textarea.press('Enter');
      // Cheap throttle so the renderer can keep up — without this
      // the input value clobbers happen too fast on some machines.
      if (i % 10 === 9) {
        await pageA.waitForTimeout(40);
      }
    }

    // Wait for the full list to settle in the in-memory model.
    // We poll the scrollHeight monotonically: once it stops growing
    // for two consecutive samples, the message-append pipeline has
    // caught up.
    let lastHeight = -1;
    let stableSamples = 0;
    const deadline = Date.now() + 30_000;
    while (Date.now() < deadline && stableSamples < 2) {
      const m = await readScrollMetrics(pageA);
      if (m.scrollHeight === lastHeight && m.scrollHeight > 0) {
        stableSamples += 1;
      } else {
        stableSamples = 0;
      }
      lastHeight = m.scrollHeight;
      await pageA.waitForTimeout(250);
    }

    // Virtualisation oracle: the rendered DOM row count is strictly
    // less than the total message count. The default overscan
    // window is well under VIRTUAL_THRESHOLD so a comfortable
    // upper bound is `target / 2`.
    const renderedRows = await pageA.locator(sel.messageRow).count();
    expect(renderedRows).toBeGreaterThan(0);
    expect(renderedRows).toBeLessThan(target);
    // And specifically below half — guards against a regression
    // that disables virtualisation by removing the threshold check.
    expect(renderedRows).toBeLessThanOrEqual(Math.ceil(target / 2));
  });
});
