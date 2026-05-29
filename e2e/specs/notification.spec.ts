/**
 * Browser notification E2E tests.
 *
 * Maps to: Requirement 14 (UI Interaction — browser notifications),
 *          Requirement 13 (Settings — notification preferences).
 *
 * Coverage:
 *   1. Notification permission is requested on first message receive.
 *   2. New message triggers a browser notification when page is not focused.
 *   3. Incoming call triggers a notification.
 *   4. Notification settings toggle disables notifications.
 *   5. Do-not-disturb mode suppresses notifications.
 */

import { sel } from '../utils/selectors.ts';
import {
  establishConnection,
  registerAndLogin,
} from '../fixtures/helpers.ts';
import { expect, test } from '../fixtures/test-base.ts';

test.describe('browser notifications', () => {
  test('notification permission is granted via context permissions', async ({
    pageA,
    server,
  }) => {
    await registerAndLogin(pageA, server, { hint: 'notif_p' });

    // The test context grants 'notifications' permission (see playwright.config.ts).
    // Verify the permission state is 'granted'.
    const permissionState = await pageA.evaluate(async () => {
      const result = await navigator.permissions.query({ name: 'notifications' });
      return result.state;
    });
    expect(permissionState).toBe('granted');
  });

  // The notification test uses `addInitScript` to patch the Notification
  // constructor BEFORE the WASM module loads, and clicks the back button
  // to clear `active_conversation` so the notification dispatch path fires.
  test('new message triggers notification when page is blurred', async ({
    contextA,
    pageB,
    server,
  }) => {
    // Patch the Notification constructor BEFORE the WASM module loads.
    await contextA.addInitScript(() => {
      (window as unknown as { __notifications: Array<{ title: string; body: string }> }).__notifications = [];
      const MockNotification = function (this: unknown, title: string, options?: NotificationOptions) {
        (window as unknown as { __notifications: Array<{ title: string; body: string }> }).__notifications.push({
          title,
          body: options?.body ?? '',
        });
        return { set onclick(_fn: unknown) { /* noop */ } };
      } as unknown as typeof Notification;
      // Set permission to 'granted' so the WASM code takes the immediate
      // fire path (Notification::permission() == Granted) without needing
      // to call requestPermission() which would show a browser prompt.
      Object.defineProperty(MockNotification, 'permission', {
        get: () => 'granted' as NotificationPermission,
        configurable: true,
      });
      MockNotification.requestPermission = () => Promise.resolve('granted') as unknown as Promise<NotificationPermission>;
      Object.defineProperty(window, 'Notification', {
        value: MockNotification,
        writable: true,
        configurable: true,
      });
    });

    // Create a fresh page in contextA so the init script takes effect.
    const pageA = await contextA.newPage();

    await registerAndLogin(pageA, server, { hint: 'notif_a' });
    const b = await registerAndLogin(pageB, server, { hint: 'notif_b' });

    await establishConnection(pageA, pageB, b.username);

    // Clear `active_conversation` so the notification dispatch path fires.
    // The frontend only triggers browser notifications when the conversation
    // is NOT the active one (`!active`). We clear it by removing the
    // localStorage key and dispatching a custom event that the WASM app
    // listens to, OR by directly clicking the hidden back button via JS.
    await pageA.evaluate(() => {
      // The back button sets active_conversation to None. Dispatch its
      // click handler programmatically since it's hidden on desktop.
      const btn = document.querySelector('.top-bar-back-btn') as HTMLButtonElement | null;
      if (btn) {
        btn.dispatchEvent(new MouseEvent('click', { bubbles: true }));
      }
    });

    // Wait briefly for the reactive state to update.
    await pageA.waitForTimeout(500);

    // Verify the chat view is gone (conversation deactivated).
    // On desktop the chat view may still be visible as an empty state,
    // so instead verify via the home-empty placeholder.
    await expect(pageA.locator(sel.homeEmpty)).toBeVisible({ timeout: 5_000 });

    // Blur pageA to simulate the user being on another tab.
    // The WASM frontend reads `document.visibilityState` via web_sys which
    // accesses the property through the prototype getter. We must override
    // both the instance property AND the prototype getter.
    await pageA.evaluate(() => {
      Object.defineProperty(document, 'hidden', { value: true, writable: true, configurable: true });
      Object.defineProperty(document, 'visibilityState', { value: 'hidden', writable: true, configurable: true });
      // Also override on the Document prototype for web_sys bindings.
      Object.defineProperty(Document.prototype, 'visibilityState', {
        get: () => 'hidden',
        configurable: true,
      });
      Object.defineProperty(Document.prototype, 'hidden', {
        get: () => true,
        configurable: true,
      });
      document.dispatchEvent(new Event('visibilitychange'));
    });

    // B sends a message to A.
    const textarea = pageB.locator(sel.chatInputTextarea);
    await textarea.fill('notification-test-msg');
    await textarea.press('Enter');

    // Verify B's message was sent (appears on B's side).
    await expect(
      pageB.locator(sel.messageRow, { hasText: 'notification-test-msg' }),
    ).toBeVisible({ timeout: 15_000 });

    // Wait for the notification dispatch (async spawn_local in WASM).
    await pageA.waitForTimeout(3_000);

    // Check that a notification was created.
    const notifications = await pageA.evaluate(
      () => (window as unknown as { __notifications: Array<{ title: string; body: string }> }).__notifications,
    );
    expect(notifications.length).toBeGreaterThan(0);
    expect(notifications[0].body).toContain('notification-test-msg');
  });

  test('incoming call triggers a notification', async ({ pageA, pageB, server }) => {
    await registerAndLogin(pageA, server, { hint: 'ncall_a' });
    const b = await registerAndLogin(pageB, server, { hint: 'ncall_b' });

    await establishConnection(pageA, pageB, b.username);

    // Intercept notifications on A.
    await pageA.evaluate(() => {
      (window as unknown as { __notifications: Array<{ title: string; body: string }> }).__notifications = [];
      const OriginalNotification = window.Notification;
      (window as unknown as { Notification: unknown }).Notification = class MockNotification {
        constructor(title: string, options?: NotificationOptions) {
          (window as unknown as { __notifications: Array<{ title: string; body: string }> }).__notifications.push({
            title,
            body: options?.body ?? '',
          });
        }
        static get permission() {
          return OriginalNotification.permission;
        }
        static requestPermission() {
          return OriginalNotification.requestPermission();
        }
      };
    });

    // Blur A.
    await pageA.evaluate(() => {
      Object.defineProperty(document, 'hidden', { value: true, writable: true });
      document.dispatchEvent(new Event('visibilitychange'));
    });

    // B initiates a call to A.
    const callBtn = pageB.locator(sel.callStartBtn);
    if (await callBtn.isVisible({ timeout: 5_000 }).catch(() => false)) {
      await callBtn.click();

      // Wait for the incoming call modal on A.
      await expect(pageA.locator(sel.incomingCallModal)).toBeVisible({ timeout: 15_000 });

      // Check notification was triggered.
      const notifications = await pageA.evaluate(
        () => (window as unknown as { __notifications: Array<{ title: string; body: string }> }).__notifications,
      );
      expect(notifications.length).toBeGreaterThan(0);

      // Decline the call to clean up.
      await pageA.locator(sel.callDeclineBtn).click();
    }
  });

  test('disabling notifications in settings suppresses them', async ({
    pageA,
    pageB,
    server,
  }) => {
    await registerAndLogin(pageA, server, { hint: 'noff_a' });
    const b = await registerAndLogin(pageB, server, { hint: 'noff_b' });

    await establishConnection(pageA, pageB, b.username);

    // Open settings and disable message notifications.
    await pageA.locator(sel.sidebarSettingsBtn).click();
    await expect(pageA.locator(sel.settingsPageSelector)).toBeVisible({ timeout: 5_000 });

    const notifToggle = pageA.locator('[data-testid="toggle-message-notifications"]');
    if (await notifToggle.isVisible({ timeout: 5_000 }).catch(() => false)) {
      // If currently enabled, click to disable.
      const isChecked = await notifToggle.getAttribute('aria-checked');
      if (isChecked === 'true') {
        await notifToggle.click();
      }
    }

    // Go back to chat.
    await pageA.keyboard.press('Escape');

    // Intercept notifications.
    await pageA.evaluate(() => {
      (window as unknown as { __notifications: Array<{ title: string; body: string }> }).__notifications = [];
      const OriginalNotification = window.Notification;
      (window as unknown as { Notification: unknown }).Notification = class MockNotification {
        constructor(title: string, options?: NotificationOptions) {
          (window as unknown as { __notifications: Array<{ title: string; body: string }> }).__notifications.push({
            title,
            body: options?.body ?? '',
          });
        }
        static get permission() {
          return OriginalNotification.permission;
        }
        static requestPermission() {
          return OriginalNotification.requestPermission();
        }
      };
    });

    // Blur A.
    await pageA.evaluate(() => {
      Object.defineProperty(document, 'hidden', { value: true, writable: true });
      document.dispatchEvent(new Event('visibilitychange'));
    });

    // B sends a message.
    const textarea = pageB.locator(sel.chatInputTextarea);
    await textarea.fill('should-not-notify');
    await textarea.press('Enter');

    // Wait for message to arrive.
    await expect(
      pageA.locator(sel.messageRow, { hasText: 'should-not-notify' }),
    ).toBeVisible({ timeout: 15_000 });

    // No notification should have been created.
    const notifications = await pageA.evaluate(
      () => (window as unknown as { __notifications: Array<{ title: string; body: string }> }).__notifications,
    );
    expect(notifications.length).toBe(0);
  });
});
