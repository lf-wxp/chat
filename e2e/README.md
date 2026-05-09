# WebRTC Chat E2E Tests

Playwright-based end-to-end tests covering Requirement 16 (E2E Messaging Test).

## Prerequisites

- Node.js ≥ 20
- Rust toolchain (see top-level README)
- Built artefacts:
  - `target/release/chat-server` (`cargo build --release -p chat-server`)
  - `frontend/dist/` (`cd frontend && trunk build --release`)

## Setup

```bash
cd e2e
npm install
npx playwright install --with-deps chromium
```

## Run

```bash
# From the repository root:
cargo make test-e2e             # Build everything + run tests headless

# From e2e/:
npm test                        # Run headless
npm run test:headed             # Run with a visible browser window
npm run test:ui                 # Run in Playwright's interactive UI
npm run report                  # Open the HTML report
```

## Layout

```
e2e/
├── fixtures/
│   ├── server.ts        # Spawn signaling server on random port
│   ├── test-base.ts     # Extended test fixture with two browser contexts
│   └── helpers.ts       # registerAndLogin / establishConnection / send...
├── specs/               # One file per requirement section
│   ├── smoke.spec.ts            # Infrastructure sanity check
│   ├── auth.spec.ts             # Req 16.2
│   ├── invitation.spec.ts       # Req 16.3
│   ├── text-messaging.spec.ts   # Req 16.4
│   ├── persistence.spec.ts      # Req 16.5
│   ├── conversation-list.spec.ts # Req 16.12
│   ├── message-actions.spec.ts  # Req 16.9 / 16.10 / 16.15
│   ├── reaction.spec.ts         # Req 16.11
│   ├── file-transfer.spec.ts    # Req 16.14
│   ├── disconnect.spec.ts       # Req 16.16
│   ├── theme-a11y.spec.ts       # Req 16.18
│   └── e2ee.spec.ts             # Req 16.20
├── utils/
│   ├── selectors.ts     # Centralised data-testid catalogue
│   ├── users.ts         # Unique-username generator
│   └── wait-helpers.ts  # Domain-aware waiters
└── assets/              # Test fixtures (small.txt, etc.)
```

## Design notes

- **Server lifecycle**: Each spec file gets its own `chat-server` child process
  on a random free port. State is in-memory — running specs in parallel is
  safe because they cannot interfere with each other.
- **WebRTC**: Chromium runs with `--use-fake-device-for-media-stream` and
  `--use-fake-ui-for-media-stream` so calls and media flows do not require
  hardware.
- **Stability**: Tests rely exclusively on `data-testid` selectors and
  domain-meaningful waiters (`waitForOnlineUser`, `waitForChatView`) — no
  raw `setTimeout` polling.
- **Independence**: Every test generates fresh usernames via
  `uniqueUsername()` to avoid colliding with siblings or replays.
