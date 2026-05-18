/**
 * Settings drawer E2E coverage (Wave P2-6).
 *
 * The settings drawer (`data-testid="settings-page"`) is opened from
 * the sidebar footer (`sidebar-settings-btn`). Inside it, the
 * Appearance section exposes a tight set of preference toggles whose
 * effect propagates to `<html>` attributes via the effects in
 * `app.rs`:
 *
 *   * `theme` → `<html data-theme="light|dark|system">`
 *     (segmented control, `data-testid="theme-group"`).
 *   * `locale` → persisted under `localStorage["settings_locale"]`
 *     (segmented control, `data-testid="language-group"`).
 *   * `font_scale` → `<html data-font-scale="small|medium|large">`
 *     (segmented control, `data-testid="font-size-small|...|-large"`).
 *
 * Coverage:
 *   1. Theme: clicking the dark / light buttons flips `<html
 *      data-theme>` AND persists under `settings_theme`.
 *   2. Font scale: clicking `font-size-large` flips
 *      `<html data-font-scale="large">`; survives reload.
 *   3. Locale: clicking the zh-CN tile writes
 *      `localStorage["settings_locale"] = "zh-CN"`.
 */

import { sel } from '../utils/selectors.ts';
import { registerAndLogin } from '../fixtures/helpers.ts';
import { expect, test } from '../fixtures/test-base.ts';
import { waitForAppShell } from '../utils/wait-helpers.ts';

const settingsPageSelector = '[data-testid="settings-page"]';

async function openSettings(page: import('@playwright/test').Page): Promise<void> {
  await page.locator(sel.sidebarSettingsBtn).click();
  await expect(page.locator(settingsPageSelector)).toBeVisible({ timeout: 5_000 });
}

test.describe('settings drawer', () => {
  test('theme segmented control flips <html data-theme> and persists to settings_theme', async ({
    pageA,
    server,
  }) => {
    await registerAndLogin(pageA, server, { hint: 'st-thm' });
    await openSettings(pageA);

    const themeGroup = pageA.locator('[data-testid="theme-group"]');
    await expect(themeGroup).toBeVisible();

    // The three buttons inside the segmented control are rendered in
    // light → dark → system order; we pick by index and use
    // aria-pressed as the readiness oracle so we don't depend on
    // i18n labels.
    const themeButtons = themeGroup.locator('button');
    await expect(themeButtons).toHaveCount(3);

    // Activate "dark" (index 1).
    await themeButtons.nth(1).click();
    await expect(themeButtons.nth(1)).toHaveAttribute('aria-pressed', 'true', {
      timeout: 3_000,
    });
    await expect(pageA.locator('html')).toHaveAttribute('data-theme', 'dark', {
      timeout: 3_000,
    });

    // The Effect in app.rs mirrors the signal to
    // localStorage["settings_theme"]. Poll until the trailing-edge
    // write completes — there is no explicit signal so a short poll
    // is the deterministic option.
    await expect
      .poll(async () => pageA.evaluate(() => localStorage.getItem('settings_theme')), {
        timeout: 5_000,
      })
      .toBe('dark');

    // Activate "light" (index 0) and re-confirm both sides flip.
    await themeButtons.nth(0).click();
    await expect(themeButtons.nth(0)).toHaveAttribute('aria-pressed', 'true');
    await expect(pageA.locator('html')).toHaveAttribute('data-theme', 'light');
    await expect
      .poll(async () => pageA.evaluate(() => localStorage.getItem('settings_theme')), {
        timeout: 5_000,
      })
      .toBe('light');
  });

  test('font scale selection flips <html data-font-scale> and survives reload', async ({
    pageA,
    server,
  }) => {
    await registerAndLogin(pageA, server, { hint: 'st-fnt' });
    await openSettings(pageA);

    // Default is medium — assert the baseline so the test is
    // self-contained.
    await expect(pageA.locator('html')).toHaveAttribute('data-font-scale', 'medium');

    await pageA.locator('[data-testid="font-size-large"]').click();
    await expect(pageA.locator('html')).toHaveAttribute('data-font-scale', 'large', {
      timeout: 3_000,
    });

    // Reload — the settings store hydrates from localStorage during
    // bootstrap, so the `<html>` attribute should be re-applied.
    await pageA.reload();
    await waitForAppShell(pageA);
    await expect(pageA.locator('html')).toHaveAttribute('data-font-scale', 'large', {
      timeout: 5_000,
    });
  });

  test('language selector persists settings_locale to localStorage', async ({
    pageA,
    server,
  }) => {
    await registerAndLogin(pageA, server, { hint: 'st-lng' });
    await openSettings(pageA);

    const langGroup = pageA.locator('[data-testid="language-group"]');
    await expect(langGroup).toBeVisible();
    const langButtons = langGroup.locator('button');
    await expect(langButtons).toHaveCount(3);

    // Click the second button (zh-CN). Order in
    // `appearance_section.rs` is en → zh-CN → es.
    await langButtons.nth(1).click();
    await expect(langButtons.nth(1)).toHaveAttribute('aria-pressed', 'true', {
      timeout: 3_000,
    });

    await expect
      .poll(async () => pageA.evaluate(() => localStorage.getItem('settings_locale')), {
        timeout: 5_000,
      })
      .toBe('zh-CN');

    // Revert to English and confirm round-trip.
    await langButtons.nth(0).click();
    await expect(langButtons.nth(0)).toHaveAttribute('aria-pressed', 'true');
    await expect
      .poll(async () => pageA.evaluate(() => localStorage.getItem('settings_locale')), {
        timeout: 5_000,
      })
      .toBe('en');
  });
});
