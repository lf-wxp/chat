/**
 * Unique user-name generation for parallel-safe E2E tests.
 *
 * Pattern: `t_{slug}_{timestampMs}_{random}`. The `t_` prefix keeps the name
 * inside the username regex (`^[a-zA-Z_][a-zA-Z0-9_]*$`) and well under the
 * 20-character cap enforced by `validate_username` on both client and server.
 */

import { randomBytes } from 'node:crypto';

const MAX_USERNAME_LEN = 20;

/** Slugify a hint into the username regex. */
function slugify(hint: string): string {
  return hint
    .replace(/[^a-zA-Z0-9_]/g, '_')
    .replace(/^[0-9_]+/, '')
    .toLowerCase()
    .slice(0, 4);
}

/** Generate a unique username keyed off a short hint and the current time. */
export function uniqueUsername(hint = 'u'): string {
  const slug = slugify(hint) || 'u';
  const ts = (Date.now() % 1_000_000).toString(36);
  const rnd = randomBytes(2).toString('hex');
  const candidate = `t_${slug}_${ts}_${rnd}`;
  return candidate.length > MAX_USERNAME_LEN
    ? candidate.slice(0, MAX_USERNAME_LEN).replace(/_$/, 'x')
    : candidate;
}

/** Default password for E2E tests. Meets the 8-char minimum. */
export const DEFAULT_PASSWORD = 'TestPass123!';
