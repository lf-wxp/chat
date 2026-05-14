/**
 * File transfer E2E tests (small text file via DataChannel).
 *
 * Maps to: Requirement 16.14 (File Transfer).
 *
 * Note: large-file (>100 MB) and dangerous-extension cases require asset
 * fixtures and are tagged `test.skip` until those fixtures are generated.
 */

import * as path from 'node:path';
import { fileURLToPath } from 'node:url';

import { sel } from '../utils/selectors.ts';
import { establishConnection, registerAndLogin } from '../fixtures/helpers.ts';
import { expect, test } from '../fixtures/test-base.ts';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const ASSETS_DIR = path.resolve(__dirname, '..', 'assets');

test.describe('file transfer', () => {
  test('A sends a small text file and B sees a file message card', async ({
    pageA,
    pageB,
    server,
  }) => {
    await registerAndLogin(pageA, server, { hint: 'a' });
    const b = await registerAndLogin(pageB, server, { hint: 'b' });
    await establishConnection(pageA, pageB, b.username);

    // Directly set files on the hidden file input. The chat-input-bar
    // renders three separate picker inputs (image, file, sticker); using
    // the dedicated `filePickerInput` selector avoids accidentally
    // matching the "Attach image" button whose aria-label also contains
    // "file|attach" via regex.
    const fileInput = pageA.locator(sel.filePickerInput);
    await fileInput.waitFor({ state: 'attached', timeout: 5_000 });
    await fileInput.setInputFiles(path.join(ASSETS_DIR, 'small.txt'));

    // The receiver should display a file message card.
    await expect(pageB.locator(sel.messageFile).first()).toBeVisible({ timeout: 30_000 });
  });
});
