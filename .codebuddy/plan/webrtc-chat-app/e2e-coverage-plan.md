# E2E Coverage Plan

> Baseline: commit `7e71047` — 25 / 25 Playwright tests pass (≈ 22 % of Req 16 AC
> coverage, ≈ 0 % coverage for Req 01–15). This document enumerates every
> remaining gap, proposes a concrete spec layout, and defines a rollout order
> optimised for "guards the worst regressions first".

---

## 1. Audit of current suite (baseline)

### 1.1 Specs already in repository (12 files / 25 tests)

| Spec file | Tests | Req 16 section |
|---|---|---|
| `smoke.spec.ts` | 2 | — |
| `auth.spec.ts` | 4 | 16.2 |
| `invitation.spec.ts` | 3 | 16.3 |
| `text-messaging.spec.ts` | 4 | 16.4 |
| `persistence.spec.ts` | 1 | 16.5 |
| `message-actions.spec.ts` | 3 | 16.9, 16.10, 16.15 |
| `reaction.spec.ts` | 1 | 16.11 |
| `conversation-list.spec.ts` | 1 | 16.12 |
| `file-transfer.spec.ts` | 1 | 16.14 |
| `disconnect.spec.ts` | 2 | 16.16 |
| `theme-a11y.spec.ts` | 2 | 16.18 |
| `e2ee.spec.ts` | 1 | 16.20 |

### 1.2 AC coverage matrix (Req 16)

| §    | Topic                       | AC-covered / AC-total | State |
| ---- | --------------------------- | --------------------- | ----- |
| 16.1 | Test infrastructure         | 7 / 7                 | ✅     |
| 16.2 | Register & login            | 4 / 6                 | 🟡     |
| 16.3 | Invitation & session        | 3 / 7                 | 🟡     |
| 16.4 | Text message                | 4 / 8                 | 🟡     |
| 16.5 | Persistence                 | 1 / 6                 | 🔴     |
| 16.6 | Sticker message             | 0 / 4                 | 🔴     |
| 16.7 | Voice message               | 0 / 5                 | 🔴     |
| 16.8 | Image message               | 0 / 5                 | 🔴     |
| 16.9 | Context-menu actions        | 3 / 10                | 🟡     |
| 16.10 | Forward                    | 1 / 4                 | 🟡     |
| 16.11 | Reaction                   | 1 / 5                 | 🔴     |
| 16.12 | Conversation list          | 1 / 7                 | 🔴     |
| 16.13 | @Mention                   | 0 / 4                 | 🔴     |
| 16.14 | File transfer              | 1 / 10                | 🔴     |
| 16.15 | Revoke w/ reply reference  | 1 / 3                 | 🟡     |
| 16.16 | Disconnect & reconnect     | 2 / 6                 | 🟡     |
| 16.17 | Multi-user (3 +)           | 0 / 5                 | 🔴     |
| 16.18 | Theme & a11y               | 2 / 6                 | 🟡     |
| 16.19 | Scroll behaviour           | 0 / 5                 | 🔴     |
| 16.20 | E2EE verification          | 1 / 5                 | 🔴     |

**Totals: 28 / ~115 AC ≈ 24 %.**

### 1.3 Requirements *outside* Req 16 — all untouched

| Req   | Topic                                   | E2E state           |
| ----- | --------------------------------------- | ------------------- |
| 01    | Signaling protocol                      | indirect only       |
| 02    | Chat detailed rules                     | partial (via 16.4)  |
| 03    | **A/V call**                            | ❌ no coverage       |
| 04    | **Room (multi-user chat)**              | ❌ no coverage       |
| 05    | E2EE rules                              | partial (via 16.20) |
| 06    | File-transfer rules                     | partial (via 16.14) |
| 07    | **A/V toggles (mic/cam/screen)**        | ❌ no coverage       |
| 09    | Discovery / online users                | partial (via 16.3)  |
| 10    | **Auth recovery (token refresh, ICE)**  | ❌ no coverage       |
| 11    | Persistence                             | partial (via 16.5)  |
| 12    | **Theater co-watch**                    | ❌ no coverage       |
| 13    | **Settings**                            | ❌ no coverage       |
| 14    | UI interaction                          | partial (via 16.18) |
| 15    | **Profile & permissions**               | ❌ no coverage       |

---

## 2. Non-functional gaps

| Area                   | Covered?     | Notes                                                   |
| ---------------------- | ------------ | ------------------------------------------------------- |
| Browser compat         | Chromium only | Per NFR this is acceptable — no plan to add FF/Safari  |
| Load / soak            | ❌            | 8-peer mesh, 100-msg burst, long session                |
| Performance budget     | ❌            | First paint < 2 s, send P95 < 200 ms                    |
| Mobile viewport        | ❌            | Touch events, responsive breakpoints                    |
| Network chaos          | ❌            | `context.setOffline(true)`, packet loss simulation      |
| Error injection        | ❌            | WebSocket disconnect mid-flow, server 5xx               |
| Security               | ❌            | CSP, XSS payload rejection                              |
| i18n                   | ❌            | zh/en switch, RTL layout                                |

---

## 3. Rollout plan

Three waves, each wave independently shippable (all tests green + pre-existing
tests still green + Rust gates still green).

### Wave P0 — "real regressions we've already had"

Goal: every bug we actually hit during Task 26 debugging now has a regression
guard, plus the biggest missing feature surfaces.

| # | New spec | Tests | Covers |
|---|----------|-------|--------|
| P0-1 | `multi-user.spec.ts` (uses `pageC`) | 4 | 16.17 mesh of 3 peers: invite → send → each side sees the message → cleanly drop C |
| P0-2 | `file-transfer-advanced.spec.ts` | 5 | 16.14: progress bar, cancel, dangerous-extension confirm dialog, hash mismatch card, 100 MB oversize reject |
| P0-3 | `reaction-sync.spec.ts` (extend existing file) | 4 | 16.11: add emoji on sender → visible on receiver, remove emoji, same-emoji aggregation count, different emojis stack |
| P0-4 | `invitation-edge.spec.ts` | 4 | 16.3: concurrent bidirectional invites → single merged peer, invitee from blacklist auto-declines, duplicate invite dedup, declined-then-re-invited path |
| P0-5 | `av-call-happy.spec.ts` | 3 | Req 03 quick path: A calls B, B accepts, both sides get a remote `MediaStream` track with ≥ 1 video frame; B hangs up → A ends |
| P0-6 | `room.spec.ts` | 4 | Req 04: create room, second user joins, send a room message, member list reflects add/remove |

**Wave P0 estimate:** +24 tests, 2 new shared helpers (`createRoom`,
`acceptCallInvite`), one new fixture-level helper (`mediaStats.ts` around
`page.evaluate(getStats)`). `pageC` already exists in `test-base.ts`.

### Wave P1 — "strong guards"

| # | New spec | Tests | Covers |
|---|----------|-------|--------|
| P1-1 | `mention.spec.ts` | 3 | 16.13: @ autocomplete, mention highlights on receiver, unread badge `+mention` marker |
| P1-2 | `persistence-extended.spec.ts` | 4 | 16.5 / Req 11: cross-tab state sync via BroadcastChannel, IDB hydration order, message-id dedup on double-deliver, large-scroll-back after refresh |
| P1-3 | `scroll.spec.ts` | 4 | 16.19: auto-stick-to-bottom, "new messages" pill when scrolled up, back-to-latest click, virtual-scroll 1 k messages without leak |
| P1-4 | `disconnect-advanced.spec.ts` | 3 | 16.16 / Req 10: ICE restart after transient drop (`context.setOffline` + re-enable), outbound queue flush after reconnect, token expiry → auto re-auth |
| P1-5 | `file-transfer-flow.spec.ts` | 3 | 16.14 part 2: pause / resume, network drop mid-transfer auto-resume, dangerous-extension save-anyway path |
| P1-6 | `e2ee-rotation.spec.ts` | 3 | 16.20 / Req 05: DataChannel frame payload is ciphertext (assert via `browser_run_code_unsafe` hooking DataChannel), tampered frame rejected, key rotation on re-invite |
| P1-7 | `context-menu-full.spec.ts` | 4 | 16.9 gaps: copy text, jump-to-quoted-message highlight flash, context menu on image/voice/file bubbles (differs by type) |

**Wave P1 estimate:** +24 tests, 1 small helper (`withOffline(page, fn)`).

### Wave P2 — "coverage completeness"

| # | New spec | Tests | Covers |
|---|----------|-------|--------|
| P2-1 | `sticker.spec.ts` | 3 | 16.6 happy path + empty sticker list fallback |
| P2-2 | `voice-message.spec.ts` | 3 | 16.7: record 2 s → send → receiver sees `message-voice` card with duration; cancel mid-record |
| P2-3 | `image-message.spec.ts` | 3 | 16.8: paste image, drag-and-drop image, receiver renders thumbnail |
| P2-4 | `conv-list-management.spec.ts` | 5 | 16.12: pin/unpin, mute, archive, search filter, delete conversation |
| P2-5 | `theater.spec.ts` | 3 | Req 12: owner selects local video, viewer joins, danmaku round-trip |
| P2-6 | `settings.spec.ts` | 3 | Req 13: locale switch refreshes UI strings, font-size scaling persists, data-export ZIP downloads |
| P2-7 | `profile.spec.ts` | 3 | Req 15: upload avatar, nickname edit round-trip, block/unblock user |
| P2-8 | `keyboard-a11y.spec.ts` | 4 | 16.18 gaps: Tab order through sidebar, ArrowUp/Down on conversation list, Enter to activate, focus ring visible |
| P2-9 | `forward-delivery.spec.ts` | 2 | 16.10 real delivery: forward from A→B to conversation with C, C actually receives the forwarded bubble |
| P2-10 | `text-limits.spec.ts` | 3 | 16.4 gaps: 10 000-char cap, empty-message guard, HTML entity escaping |

**Wave P2 estimate:** +32 tests.

### Totals after all waves

| Wave | New tests | Cumulative total | Req 16 AC coverage |
|------|-----------|-------------------|--------------------|
| Baseline | — | 25 | 24 % |
| + P0 | +24 | 49 | ~55 % |
| + P1 | +24 | 73 | ~78 % |
| + P2 | +32 | 105 | ~92 % |

The remaining ~8 % of AC is documentation-only or requires mobile/network-chaos
primitives explicitly marked out of scope in Req 16 NFRs.

---

## 4. Cross-cutting work to land alongside the waves

These are small investments that pay off across multiple specs; listed so they
can be scheduled independently of any single wave.

1. **Extend `sel` catalogue** (`e2e/utils/selectors.ts`) with testids we will need
   (`message-voice`, `call-accept-btn`, `room-members-list`, `sticker-grid`,
   etc.). Each wave should add its own block and the matching
   `data-testid="..."` frontend patches.
2. **New fixture: `withOffline(page, fn)`** — wraps `context.setOffline(true)` +
   runs fn + re-enables + waits for the sidebar connection badge to re-turn
   green. Enables P1-4 cleanly.
3. **New helper: `expectEncryptionReady(...)`** — already de-facto exists inside
   `establishConnection`, lift it to a named helper so multi-user specs can
   assert readiness per-peer.
4. **New fixture: `call-helpers.ts`** — `startCall`, `acceptCall`,
   `waitForRemoteFrame` (uses `page.evaluate` + `getStats()` `framesDecoded` ≥ 1
   as the readiness signal, analogous to the ECDH sentinel).
5. **Make `pageC` trivial to opt into** — it is already in `test-base.ts`; waves
   that need it just destructure `{ pageA, pageB, pageC }`.
6. **CI wiring** — add a dedicated `cargo make test-e2e` job with artifact
   upload on failure (traces, screenshots, server logs). Traces on failure are
   already enabled in `playwright.config.ts`.

---

## 5. Quality gates per wave

Before a wave is considered done:

1. **New tests pass three consecutive runs** under `--workers=1 --retries=0`
   (catches order-sensitive flake).
2. **Old tests still pass** under the default parallel config.
3. **Rust gates stay green** — `cargo fmt --all --check`,
   `cargo clippy --all-targets --all-features -- -D warnings`,
   `cargo test --lib` (1 943+ tests).
4. **No new `waitForTimeout` calls** — always route through a deterministic
   readiness signal (DOM attr, signal-driven element).
5. **One frontend `data-testid` per new assertion target** — no CSS-class or
   text-content-only selectors.

---

## 6. Execution order — concrete next steps

Pick any single item from **P0** to start. Suggested order, smallest-risk-first
so each lands on its own:

1. **P0-3** reaction-sync — extends an existing file, no new infrastructure.
2. **P0-4** invitation-edge — pure signaling, no media stack.
3. **P0-1** multi-user — introduces `pageC` usage pattern.
4. **P0-2** file-transfer-advanced — needs new testids on the file card
   (`file-progress`, `file-cancel-btn`, `file-hash-mismatch`,
   `file-danger-confirm`).
5. **P0-6** room — needs `room-create-btn`, `room-list`, `room-message`.
6. **P0-5** av-call-happy — last because it introduces the `mediaStats.ts`
   helper and the `getStats`-based readiness signal.

After P0 is green, re-measure AC coverage and repeat for P1, then P2.

---

## 7. Tracking

Progress is tracked inside this document only (no separate issue tracker
needed). When a wave item lands, replace its status emoji:

```
- [ ] P0-3 reaction-sync        → - [x] P0-3 reaction-sync (commit 7e71047)
```

### Status board

- [x] P0-1 multi-user.spec.ts — commit c98b91d
- [x] P0-2 file-transfer-advanced.spec.ts — commit 43433c8
- [x] P0-3 reaction-sync (extend reaction.spec.ts) — commit f35ecb8
- [x] P0-4 invitation-edge.spec.ts — commit 4a8b85b
- [ ] P0-5 av-call-happy.spec.ts
- [ ] P0-6 room.spec.ts
- [ ] P1-1 mention.spec.ts
- [ ] P1-2 persistence-extended.spec.ts
- [ ] P1-3 scroll.spec.ts
- [ ] P1-4 disconnect-advanced.spec.ts
- [ ] P1-5 file-transfer-flow.spec.ts
- [ ] P1-6 e2ee-rotation.spec.ts
- [ ] P1-7 context-menu-full.spec.ts
- [ ] P2-1 sticker.spec.ts
- [ ] P2-2 voice-message.spec.ts
- [ ] P2-3 image-message.spec.ts
- [ ] P2-4 conv-list-management.spec.ts
- [ ] P2-5 theater.spec.ts
- [ ] P2-6 settings.spec.ts
- [ ] P2-7 profile.spec.ts
- [ ] P2-8 keyboard-a11y.spec.ts
- [ ] P2-9 forward-delivery.spec.ts
- [ ] P2-10 text-limits.spec.ts
