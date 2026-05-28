/**
 * Lifecycle management for a signaling-server child process used by E2E tests.
 *
 * The server is a release-mode binary at `target/release/chat-server`. It is
 * spawned with a random available port, fed environment variables that point
 * to the freshly built `frontend/dist`, and given a deterministic JWT secret.
 *
 * Each spec file should request a fresh `ServerInstance` (typically via the
 * test-base fixture) so that in-memory state (users, rooms, sessions) does not
 * bleed across spec files.
 */

import { type ChildProcess, spawn } from 'node:child_process';
import { createServer } from 'node:net';
import { existsSync, mkdtempSync } from 'node:fs';
import { tmpdir } from 'node:os';
import * as path from 'node:path';
import { fileURLToPath } from 'node:url';
import { setTimeout as sleep } from 'node:timers/promises';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = path.resolve(__dirname, '..', '..');
// The server crate is named `server` in its Cargo.toml `[[bin]]` section,
// so the release binary lives at `target/release/server`.
const SERVER_BINARY = path.join(REPO_ROOT, 'target', 'release', 'server');
const FRONTEND_DIST = path.join(REPO_ROOT, 'frontend', 'dist');
// Stickers are not yet bundled into `frontend/public/`; if the directory does
// not exist we fall back to a temporary empty one so the server's `ServeDir`
// can still mount.
const DEFAULT_STICKERS_DIR = path.join(REPO_ROOT, 'frontend', 'public', 'stickers');

/** Per-test-file server handle. */
export interface ServerInstance {
  readonly port: number;
  readonly baseUrl: string;
  readonly wsUrl: string;
  readonly logs: string[];
  /** Stop the server gracefully. Idempotent. */
  stop(): Promise<void>;
}

/** Find a random free TCP port by letting the OS bind one and reading it back. */
async function findFreePort(): Promise<number> {
  return new Promise<number>((resolve, reject) => {
    const srv = createServer();
    srv.unref();
    srv.on('error', reject);
    srv.listen(0, '127.0.0.1', () => {
      const addr = srv.address();
      if (addr === null || typeof addr === 'string') {
        srv.close();
        reject(new Error('Failed to obtain a free port'));
        return;
      }
      const { port } = addr;
      srv.close(() => resolve(port));
    });
  });
}

/** Poll the health endpoint until it returns 200 or we run out of attempts. */
async function waitForHealth(
  baseUrl: string,
  attempts = 30,
  intervalMs = 500,
): Promise<void> {
  for (let i = 0; i < attempts; i += 1) {
    try {
      const response = await fetch(`${baseUrl}/api/health`, {
        signal: AbortSignal.timeout(2_000),
      });
      if (response.ok) {
        return;
      }
    } catch {
      // not ready yet
    }
    await sleep(intervalMs);
  }
  throw new Error(`Signaling server did not become healthy at ${baseUrl}`);
}

/** Start the signaling server on a random free port. */
export async function startServer(): Promise<ServerInstance> {
  // Pre-flight checks with friendly errors so failures point to the missing
  // build step rather than to a generic ENOENT from spawn.
  if (!existsSync(SERVER_BINARY)) {
    throw new Error(
      `Server binary not found at ${SERVER_BINARY}. ` +
        `Run \`cargo make e2e-build\` (or \`cargo build --release -p server\`) first.`,
    );
  }
  if (!existsSync(path.join(FRONTEND_DIST, 'index.html'))) {
    throw new Error(
      `Frontend dist not found at ${FRONTEND_DIST}. ` +
        `Run \`cargo make e2e-build\` (or \`cd frontend && trunk build --release\`) first.`,
    );
  }

  const stickersDir = existsSync(DEFAULT_STICKERS_DIR)
    ? DEFAULT_STICKERS_DIR
    : mkdtempSync(path.join(tmpdir(), 'e2e-stickers-'));

  // Retry up to 3 times to handle TOCTOU port races: findFreePort()
  // releases the port before the server binds, so another process
  // may grab it in between (os error 48: Address already in use).
  const MAX_PORT_RETRIES = 3;
  let lastError: Error | null = null;

  for (let attempt = 0; attempt < MAX_PORT_RETRIES; attempt += 1) {
    const port = await findFreePort();
    const baseUrl = `http://127.0.0.1:${port}`;
    const wsUrl = `ws://127.0.0.1:${port}/ws`;

    const env: NodeJS.ProcessEnv = {
      ...process.env,
      PORT: String(port),
      JWT_SECRET: 'e2e-test-jwt-secret-do-not-use-in-production',
      RUST_LOG: process.env.RUST_LOG ?? 'warn,server=info',
      RUST_LOG_FORMAT: 'pretty',
      LOG_OUTPUT: 'stdout',
      STATIC_DIR: FRONTEND_DIST,
      STICKERS_DIR: stickersDir,
      // Disable public STUN/TURN servers for E2E runs. Both peers are on
      // the same host (127.0.0.1); host ICE candidates are sufficient and
      // do not require an external STUN lookup. Leaving the default
      // `stun:stun.l.google.com:19302` in place would make Chromium
      // block ICE gathering on an unreachable DNS resolve in sandboxed
      // CI environments, causing the peer connection to stay in
      // `Connecting` until it times out as `Failed` ~15 s later.
      STUN_TURN_SERVERS: '',
      // Disable the embedded STUN service. Without this, the parallel
      // E2E workers would race each other for UDP port 3478 — the
      // first wins, the rest log a startup warning and clients fall
      // back to host candidates anyway, but we'd be relying on flaky
      // ordering. Setting `STUN_PORT=0` skips the bind altogether.
      STUN_PORT: '0',
    };

    const child: ChildProcess = spawn(SERVER_BINARY, [], {
      cwd: REPO_ROOT,
      env,
      stdio: ['ignore', 'pipe', 'pipe'],
    });

    const logs: string[] = [];
    const captureLog = (chunk: Buffer | string): void => {
      const text = typeof chunk === 'string' ? chunk : chunk.toString('utf8');
      if (process.env.E2E_SERVER_LOG === '1') {
        // Forward to test runner stdout for live debugging.
        process.stdout.write(`[server:${port}] ${text}`);
      }
      for (const line of text.split('\n')) {
        if (line.trim().length > 0) {
          logs.push(line);
          // Cap memory: keep at most 5_000 most-recent lines per server.
          if (logs.length > 5_000) {
            logs.splice(0, logs.length - 5_000);
          }
        }
      }
    };
    child.stdout?.on('data', captureLog);
    child.stderr?.on('data', captureLog);

    let exited = false;
    child.on('exit', (code, signal) => {
      exited = true;
      logs.push(`[server] exited code=${code ?? 'null'} signal=${signal ?? 'null'}`);
    });

    // If the binary fails to start (e.g. missing build), `waitForHealth` will
    // reject after the configured retry budget. Surface logs in that case.
    try {
      await waitForHealth(baseUrl);
    } catch (err) {
      if (!exited) {
        child.kill('SIGKILL');
      }
      // Check if the failure was due to port conflict (EADDRINUSE).
      const logsText = logs.join('\n');
      if (logsText.includes('Address already in use') && attempt < MAX_PORT_RETRIES - 1) {
        // Port was stolen between findFreePort() and server bind; retry.
        lastError = err as Error;
        await sleep(100);
        continue;
      }
      throw new Error(
        `${(err as Error).message}\n--- server logs (last 50 lines) ---\n${logs.slice(-50).join('\n')}`,
      );
    }

    let stopped = false;
    const stop = async (): Promise<void> => {
      if (stopped || exited) {
        stopped = true;
        return;
      }
      stopped = true;
      await new Promise<void>((resolve) => {
        const timer = setTimeout(() => {
          if (!exited) {
            child.kill('SIGKILL');
          }
          resolve();
        }, 5_000);
        child.once('exit', () => {
          clearTimeout(timer);
          resolve();
        });
        child.kill('SIGTERM');
      });
    };

    return { port, baseUrl, wsUrl, logs, stop };
  }

  // All retries exhausted — should not reach here, but satisfy TypeScript.
  throw lastError ?? new Error('Failed to start server after port retries');
}
