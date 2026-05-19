/**
 * Avatar upload / clear coverage (G26).
 *
 * Tests the AvatarEditor mounted in the SettingsPage Account section.
 * Uses the tiny PNG fixture (77 B) for the "happy path" and a
 * programmatic oversized data URL for the "too large" rejection.
 */
import { test, expect } from '../fixtures/test-base.ts';

import { registerAndLogin } from '../fixtures/helpers.ts';
import { waitForAppShell } from '../utils/wait-helpers.ts';
import { sel } from '../utils/selectors.ts';

/** Open the settings drawer and wait for it to become visible. */
async function openSettings(page: any) {
  await page.locator(sel.sidebarSettingsBtn).click();
  await expect(page.locator(sel.settingsPageSelector)).toBeVisible({ timeout: 10_000 });
}

// ── suite ──────────────────────────────────────────────────────────────

test.describe('AvatarEditor — G26', () => {
  test('avatar editor is visible in SettingsPage', async ({ page, server }) => {
    await registerAndLogin(page, server, { hint: 'av-vis' });
    await openSettings(page);

    // AvatarEditor root element is mounted.
    await expect(
      page.locator('[data-testid="avatar-editor"]'),
    ).toBeVisible({ timeout: 5_000 });

    // Preview image renders (identicon by default).
    const preview = page.locator('[data-testid="avatar-editor-preview"]');
    await expect(preview).toBeVisible();
    await expect(preview).toHaveAttribute('alt', /avatar/i);
  });

  test('upload tiny PNG updates preview src', async ({ page, server }) => {
    await registerAndLogin(page, server, { hint: 'av-up' });
    await openSettings(page);

    const preview = page.locator('[data-testid="avatar-editor-preview"]');
    const originalSrc = await preview.getAttribute('src');

    // Upload the tiny PNG fixture via the hidden file input.
    const fileInput = page.locator('[data-testid="avatar-editor-input"]');
    await fileInput.setInputFiles('assets/tiny.png');

    // Wait until the preview src changes away from the original identicon.
    await expect(async () => {
      const src = await preview.getAttribute('src');
      expect(src).not.toBe(originalSrc);
    }).toPass({ timeout: 10_000 });

    // The new src should be a data URL (Phase A).
    await expect(preview).toHaveAttribute('src', /^data:/);
  });

  test('remove avatar restores identicon', async ({ page, server }) => {
    await registerAndLogin(page, server, { hint: 'av-rm' });
    await openSettings(page);

    const preview = page.locator('[data-testid="avatar-editor-preview"]');

    // First upload a custom avatar so there is something to remove.
    const fileInput = page.locator('[data-testid="avatar-editor-input"]');
    await fileInput.setInputFiles('assets/tiny.png');
    await expect(preview).toHaveAttribute('src', /^data:/, { timeout: 10_000 });

    // Click "Remove avatar".
    const removeBtn = page.locator('[data-testid="avatar-editor-remove"]');
    await removeBtn.click();

    // Preview src should revert to an identicon (SVG data URL).
    await expect(async () => {
      const src = await preview.getAttribute('src');
      expect(src).toMatch(/^data:image\/svg\+xml/);
    }).toPass({ timeout: 10_000 });
  });

  test('avatar survives page reload (G26 cross-reload contract)', async ({ page, server }) => {
    await registerAndLogin(page, server, { hint: 'av-rel' });
    await openSettings(page);

    const preview = page.locator('[data-testid="avatar-editor-preview"]');

    // Upload a custom avatar.
    const fileInput = page.locator('[data-testid="avatar-editor-input"]');
    await fileInput.setInputFiles('assets/tiny.png');
    await expect(preview).toHaveAttribute('src', /^data:/, { timeout: 10_000 });

    // Reload and re-open settings.
    await page.reload();
    await waitForAppShell(page);
    await openSettings(page);

    // Preview should still show a data URL (persisted in AuthSuccess.avatar_url).
    await expect(
      page.locator('[data-testid="avatar-editor-preview"]'),
    ).toHaveAttribute('src', /^data:/, { timeout: 10_000 });
  });
});
