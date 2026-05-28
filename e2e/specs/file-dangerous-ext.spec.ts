/**
 * Dangerous file-extension E2E tests (Req 16.14 AC-4).
 *
 * When a received file has a dangerous extension (.exe, .bat, .cmd, .sh,
 * .js, .vbs, .wsf) the file card must display a security warning badge
 * and the download button must trigger a confirmation dialog.
 */

import { sel } from '../utils/selectors.ts';
import { establishConnection, registerAndLogin } from '../fixtures/helpers.ts';
import { expect, test } from '../fixtures/test-base.ts';

const DANGEROUS_EXTS = ['exe', 'bat', 'cmd', 'sh', 'js', 'vbs', 'wsf'];

async function sendDangerousFile(
  pageA: any,
  pageB: any,
  ext: string,
) {
  const fileName = `malicious.${ext}`;

  // A uploads the dangerous file using the hidden file input.
  const fileInput = pageA.locator(sel.filePickerInput);
  await fileInput.waitFor({ state: 'attached', timeout: 5_000 });
  // 1 KiB binary blob with the extension in the name.
  const buffer = Buffer.alloc(1024, 0xAB);
  await fileInput.setInputFiles({
    name: fileName,
    mimeType: 'application/octet-stream',
    buffer,
  });

  // The sender sees a confirmation dialog because the extension is
  // flagged as dangerous (Req 6.8b). Confirm to proceed with the send.
  await expect(pageA.locator(sel.dialog)).toBeVisible({ timeout: 5_000 });
  await pageA.locator(sel.dialogOk).click();
  await expect(pageA.locator(sel.dialog)).not.toBeVisible({ timeout: 5_000 });

  // B receives the file card.
  const cardB = pageB
    .locator(sel.messageFile)
    .filter({ hasText: fileName })
    .first();
  await expect(cardB).toBeVisible({ timeout: 30_000 });
  return { cardB, fileName };
}

test.describe('dangerous file extension', () => {
  for (const ext of DANGEROUS_EXTS) {
    test(`.${ext} file shows danger badge on receiver`, async ({ pageA, pageB, server }) => {
      await registerAndLogin(pageA, server, { hint: `dang-${ext}-a` });
      const b = await registerAndLogin(pageB, server, { hint: `dang-${ext}-b` });
      await establishConnection(pageA, pageB, b.username);

      const { cardB } = await sendDangerousFile(pageA, pageB, ext);

      // Danger badge must be visible.
      await expect(cardB.locator(sel.fileDangerBadge)).toBeVisible({ timeout: 5_000 });
      await expect(cardB.locator(sel.fileExtDanger)).toBeVisible({ timeout: 5_000 });

      // The download button should have the danger variant test-id.
      await expect(cardB.locator(sel.fileDownloadDangerBtn)).toBeVisible({ timeout: 5_000 });
    });
  }

  test('download of dangerous file requires confirmation dialog', async ({
    pageA,
    pageB,
    server,
  }) => {
    await registerAndLogin(pageA, server, { hint: 'dang-dl-a' });
    const b = await registerAndLogin(pageB, server, { hint: 'dang-dl-b' });
    await establishConnection(pageA, pageB, b.username);

    const { cardB } = await sendDangerousFile(pageA, pageB, 'exe');

    // Click the danger download button.
    await cardB.locator(sel.fileDownloadDangerBtn).click();

    // A confirmation dialog must appear.
    await expect(pageB.locator(sel.dialog)).toBeVisible({ timeout: 5_000 });
    await expect(pageB.locator(sel.dialogMessage)).toContainText(/dangerous|security|risk/i);

    // Cancelling the dialog leaves the file card unchanged.
    await pageB.locator(sel.dialogCancel).click();
    await expect(pageB.locator(sel.dialog)).not.toBeVisible();
    await expect(cardB).toBeVisible();

    // Re-clicking and confirming proceeds (we don't actually download).
    await cardB.locator(sel.fileDownloadDangerBtn).click();
    await expect(pageB.locator(sel.dialog)).toBeVisible({ timeout: 5_000 });
    await pageB.locator(sel.dialogOk).click();
    await expect(pageB.locator(sel.dialog)).not.toBeVisible();
  });
});
