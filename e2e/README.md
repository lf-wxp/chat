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
│   ├── smoke.spec.ts                  # Infrastructure sanity check
│   ├── auth.spec.ts                   # Req 16.2 — Registration & login
│   ├── auth-logout.spec.ts            # Req 10.4/10.7 — Logout & single-device policy
│   ├── invitation.spec.ts             # Req 16.3 — Connection invitation
│   ├── invitation-bidirectional.spec.ts # Req 9.13 — Bidirectional invite conflict
│   ├── invitation-edge.spec.ts        # Req 9 — Edge cases & rate limiting
│   ├── invitation-timeout.spec.ts     # Req 9 — Invite timeout handling
│   ├── multi-invite.spec.ts           # Req 9.10-9.12 — Multi-user invitation
│   ├── blacklist.spec.ts              # Req 9.2/9.17 — Block/unblock users
│   ├── text-messaging.spec.ts         # Req 16.4 — Text message send/receive
│   ├── text-limits.spec.ts            # Req 2 — Message length validation
│   ├── url-auto-detection.spec.ts     # Req 2 — URL auto-detection in messages
│   ├── message-actions.spec.ts        # Req 16.9/16.10/16.15 — Reply/revoke/copy
│   ├── message-status-indicator.spec.ts # Req 2 — Message delivery status
│   ├── mention.spec.ts                # Req 2 — @mention functionality
│   ├── reaction.spec.ts               # Req 16.11 — Message reactions
│   ├── forward-delivery.spec.ts       # Req 2.13 — Message forwarding
│   ├── revoked-reply-update.spec.ts   # Req 2 — Revoked message in reply
│   ├── sticker.spec.ts                # Req 2 — Sticker messages
│   ├── voice-message.spec.ts          # Req 2 — Voice messages
│   ├── image-message.spec.ts          # Req 2 — Image messages
│   ├── file-transfer.spec.ts          # Req 16.14 — Basic file transfer
│   ├── file-transfer-flow.spec.ts     # Req 6 — Full file transfer flow
│   ├── file-transfer-advanced.spec.ts # Req 6 — Advanced transfer scenarios
│   ├── file-dangerous-ext.spec.ts     # Req 6 — Dangerous file extension warning
│   ├── persistence.spec.ts            # Req 16.5 — Message persistence
│   ├── persistence-extended.spec.ts   # Req 11 — Extended persistence scenarios
│   ├── conversation-list.spec.ts      # Req 16.12 — Conversation list
│   ├── conv-list-management.spec.ts   # Req 7.7 — Pin/mute/archive
│   ├── context-menu-full.spec.ts      # Req 14 — Context menu interactions
│   ├── scroll.spec.ts                 # Req 14.11 — Message list scrolling
│   ├── room.spec.ts                   # Req 4 — Room creation & joining
│   ├── room-management.spec.ts        # Req 4/15 — Kick/mute/ban/promote/demote/transfer
│   ├── room-password.spec.ts          # Req 4 — Room password protection
│   ├── multi-user.spec.ts             # Req 4 — Multi-user room scenarios
│   ├── profile.spec.ts                # Req 15 — User profile & nickname
│   ├── av-call-happy.spec.ts          # Req 3 — Audio/video call happy path
│   ├── theater.spec.ts                # Req 12 — Theater mode basics
│   ├── theater-full.spec.ts           # Req 12 — Full theater scenarios
│   ├── connection-recovery.spec.ts    # Req 10.3/11.3 — Refresh recovery & resend
│   ├── disconnect.spec.ts             # Req 16.16 — Disconnect detection
│   ├── disconnect-advanced.spec.ts    # Req 16.16 — Advanced disconnect scenarios
│   ├── e2ee.spec.ts                   # Req 16.20 — E2EE verification
│   ├── e2ee-rotation.spec.ts          # Req 5 — E2EE key rotation
│   ├── settings.spec.ts              # Req 13 — Settings drawer
│   ├── theme-a11y.spec.ts            # Req 16.18 — Theme & accessibility
│   ├── keyboard-a11y.spec.ts         # Req 14 — Keyboard navigation
│   ├── avatar.spec.ts                # Req 10.6 — Identicon avatar
│   ├── pwa-offline.spec.ts           # Req 25 — PWA offline & service worker
│   └── notification.spec.ts          # Req 14 — Browser notifications
├── utils/
│   ├── selectors.ts     # Centralised data-testid catalogue
│   ├── users.ts         # Unique-username generator
│   ├── mediaStats.ts    # WebRTC media stats helpers
│   └── wait-helpers.ts  # Domain-aware waiters
└── assets/              # Test fixtures (small.txt, demo.mp4, etc.)
```

## Coverage Matrix

| Requirement | Spec File(s) | Scenarios |
|-------------|-------------|-----------|
| Req 1 (Signaling) | smoke, disconnect, connection-recovery | Health check, WebSocket lifecycle, reconnection |
| Req 2 (Chat) | text-messaging, message-actions, mention, sticker, voice-message, image-message, forward-delivery, reaction, revoked-reply-update, text-limits, url-auto-detection | All message types, actions, formatting |
| Req 3 (AV Call) | av-call-happy | Call initiation, mode switch, hang up |
| Req 4 (Room) | room, room-management, room-password, multi-user | CRUD, password, permissions, multi-user |
| Req 5 (E2EE) | e2ee, e2ee-rotation | Key exchange, encryption verification, rotation |
| Req 6 (File Transfer) | file-transfer, file-transfer-flow, file-transfer-advanced, file-dangerous-ext | Transfer flow, resume, dangerous extensions |
| Req 7 (AV Features) | conv-list-management | Pin/mute/archive conversations |
| Req 9 (Discovery) | invitation, invitation-bidirectional, invitation-edge, invitation-timeout, multi-invite, blacklist | All invitation flows, edge cases, blacklist |
| Req 10 (Auth) | auth, auth-logout, connection-recovery | Register, login, logout, session recovery, single-device |
| Req 11 (Persistence) | persistence, persistence-extended | IndexedDB storage, history, search |
| Req 12 (Theater) | theater, theater-full | Theater creation, playback, danmaku, subtitles |
| Req 13 (Settings) | settings | Theme, font, locale preferences |
| Req 14 (UI) | scroll, context-menu-full, keyboard-a11y, theme-a11y, notification | Scrolling, context menus, a11y, notifications |
| Req 15 (Permissions) | room-management, profile | Kick/mute/ban/promote/demote, nickname |
| Req 25 (PWA) | pwa-offline | Offline banner, service worker, reconnection |

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
