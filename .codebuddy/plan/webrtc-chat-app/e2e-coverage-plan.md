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

## 2.1 Discovered feature gaps (NOT E2E flakiness)

These are real product bugs uncovered while writing the P0 specs. The
specs are scoped around them; a follow-up engineering ticket should
land the fix and then a thin "round-trip" E2E test to lock it down.

| # | Surface | Gap | Discovered in |
|---|---------|-----|---------------|
| G1 | Room messaging | `DataChannelMessage::ChatText` (and siblings: `ChatSticker`, `ChatVoice`, `ChatImage`, `MessageReaction`, …) carry no `Option<RoomId>`. Inbound chat frames are unconditionally attributed to the direct conversation with the sender (`raw_frame.rs:219`). Result: messages typed inside a Chat-room conversation reach the receiver but appear in their direct-with-sender thread, not the room. The fix is to add `room_id: Option<RoomId>` to each Chat* variant, set it from `ConversationId::Room` on the send path (`chat::manager::wire::send_wire_out`), and honour it at the dispatch site. The existing `FileMetadata.room_id` is a working precedent. | P0-6 |
| G2 | Room creation membership | Server's `handle_create_room` only emits `RoomCreated` + `RoomListUpdate`, never a follow-up `RoomMemberUpdate` for the creator. Fixed locally for now by seeding `app_state.room_members` inside the frontend's `RoomCreated` handler so the creator's own join button reports `data-joined="true"` immediately. The proper fix is a server-side `RoomMemberUpdate` broadcast on create, identical to the join path. | P0-6 |
| G3 | A/V call has no entry point | The frontend shipped a complete call subsystem (`CallManager`, `IncomingCallModal`, `CallView`, `CallControls`, `VideoGrid`, `VideoTile`) but **no UI surface ever invoked `CallManager::initiate_call`**. Users could never start a call. Fixed in P0-5 by adding a `CallStartButton` rendered inside the chat view for `ConversationId::Room` conversations only, plus a `CALL_MANAGER_FALLBACK` thread-local mirroring the chat / file-transfer pattern (the WebSocket onmessage bridge runs detached from the Leptos owner so the previous `try_use_call_manager()` lookup silently dropped every incoming `CallInvite`). | P0-5 |
| G4 | Mid-call addTrack renegotiation drops the remote video | When a call is initiated on top of an existing data-channel-only `RTCPeerConnection`, `publish_to_peers` calls `addTrack(video)` which triggers `onnegotiationneeded`. The resulting offer fails on the caller's side with `InvalidAccessError: order of m-lines in subsequent offer doesn't match order from previous offer/answer`. The signaling layer eventually retries and SDP completes, but the remote `<video>` tile's `videoWidth` never crosses 0 in our 30 s window — `ontrack` either never fires or fires after the test gives up. P0-5's "Active" test asserts only the local preview path; a follow-up ticket should investigate the m-line ordering bug (likely missing transceiver pre-allocation when the peer connection was first created) and add the cross-peer remote-frame assertion. | P0-5 |
| G5 | Composer has no `@`-trigger autocomplete | Typing `@` in `chat-input-textarea` produces no suggestion list. The composer (`components/chat_view/input_bar.rs`) only consumes a one-shot `app_state.pending_mention` signal injected by the room member-list "Mention in chat" action. Plan §3 P1-1 originally listed an "@ autocomplete" assertion; that test was dropped because the surface does not exist. Once an autocomplete is implemented, a thin spec exercising `@a` → suggestion popup → click → token insertion should land. | P1-1 |
| G6 | No "+mention" unread badge variant | The sidebar conversation row renders a single `unread-count` badge (`components/sidebar/sidebar_conversation_item.rs`). There is no separate counter or visual marker for "unread messages that mentioned me", and no testid distinguishes the two cases. The `mentions_me` projection is computed and persisted, but never surfaced in the conversation list. Plan §3 P1-1 originally included a `+mention` badge assertion; it was dropped pending product work. | P1-1 |
| G7 | Chat-room ChatView does not mount the member list | The `MemberListPanel` (which renders `room-member-list` and the per-row context menu carrying the "Mention in chat" action) is mounted only by Theater rooms (`components/theater/*`), not by the regular Chat-room ChatView. Consequently, the only existing UI path to inject an `@mention` into the composer is unreachable from a Chat room. The mention-via-member-list flow is therefore deferred to the future Theater spec (Wave P2). | P1-1 |
| G8 | `ChatText` has no `mentions: Vec<UserId>` protocol field | Mentions are re-extracted on the receiver side from the plain-text content (`chat::routing::dispatch_incoming` → `mention::extract` + `mention::mentions(self_nick)`). This is implicitly case-insensitive nickname matching with no support for users whose nickname collides or who later rename. A future protocol revision should carry the resolved `mentions: Vec<UserId>` from sender to receiver, mirroring how reactions work; the corresponding spec will assert cross-peer parity between the sent list and the rendered highlight set. | P1-1 |
| G9 | No cross-tab sync for the same user | The frontend has zero usage of `BroadcastChannel`, the `'storage'` event, or any other cross-tab primitive. Two browser tabs of the same logged-in user open independent WebSocket sessions, write to separate in-memory `ChatManager`s, and only become consistent after a tab reload (which re-reads IDB). The plan §3 P1-2 "cross-tab sync" assertion was dropped pending a feature ticket that introduces a leader-election + BroadcastChannel mirror, mirroring how some web apps handle this. | P1-2 |
| G10 | `chat::manager::inbound::push_incoming` does not dedup by `message_id` | When the same `ChatText` arrives twice (e.g. peer's `ack_queue` replays after reconnect), the receiver appends a second `ChatMessage` to the conversation's `RwSignal<Vec<ChatMessage>>` without consulting the existing entries. The IDB write is a `put` keyed on `message_id` so the persisted store is dedup-safe, but the in-memory list (and hence the rendered bubble list) doubles. The fix is to bail out at the top of `push_incoming` when `state.messages` already contains an entry with the same id. The plan §3 P1-2 "dedup on double-deliver" assertion was dropped pending that fix. | P1-2 |
| G11 | `conversation_flags` were silently dropped between every page reload | The `persist_conversations` debounce timer arms a `set_timeout_once` callback to coalesce hot writes. That callback fires from the browser's timer queue with no surrounding Leptos reactive owner, so the inner `flush_conv_flags_to_idb` looked up `PersistenceManager` via `use_context::<PersistenceManager>()` and got `None` every single time — meaning pin / mute / archive flags were *never* written to IDB despite the in-memory `Conversation.pinned` flipping correctly. The companion `reconcile_conv_flags_from_idb` had the symmetric bug: it ran from `provide_app_state()` *before* `provide_persistence_manager()` had installed the context. Fixed in P1-2 by (a) introducing a `PERSISTENCE_MANAGER_FALLBACK` thread-local mirror (mirroring `CHAT_MANAGER_FALLBACK` etc.) and routing both call sites through `try_use_persistence_manager()`, and (b) moving the reconcile call from `provide_app_state` to `lib.rs::init` after the PM context exists. | P1-2 |
| G12 | No outbound message queue / offline buffering | `chat::manager::wire::send_wire_out` spawns each send into a `wasm_bindgen_futures::spawn_local` and `console.warn`s on failure with no retry. There is no client-side outbound buffer that would hold messages typed during a transient network drop and replay them after reconnect. The closest existing primitive — `chat::ack_queue` — only retries messages that were *successfully* sent on the wire but are awaiting a `MessageAck`. Plan §3 P1-4 originally called for an "outbound queue flush after reconnect" assertion; that path does not exist, and the P1-4 spec instead locks down the more common "user pauses → network blips → user resumes typing" flow via the WS-reconnect + online-list-rehydrate test. | P1-4 |
| G13 | No JWT refresh / silent re-auth | `auth::service::try_recover_auth` calls `is_jwt_expired` against a 7-day server-issued token and, on expiry, simply wipes the localStorage keys and routes the user back to the login page. There is no refresh-token round-trip, no proactive re-issuance before expiry, and no automatic re-auth attempt mid-session. Plan §3 P1-4 originally listed "token expiry → auto re-auth"; the P1-4 spec instead asserts the documented current behaviour ("expired token in storage routes to the auth page on reload") so a future refresh-token feature has a clear regression target to flip. | P1-4 |
| G14 | ~~No user-facing pause / resume controls for file transfers~~ **(resolved)** | `IncomingTransfer::user_paused: RwSignal<bool>` was added (defaulting to `false`); `FileTransferManager::on_file_chunk` now early-returns when the flag is set so the bitmap freezes at the pause point. Two new public APIs — `FileTransferManager::pause_inbound(&MessageId)` and `resume_inbound(&MessageId)` — flip the signal, swap `TransferStatus::InProgress` / `TransferStatus::Paused`, and (on resume) replay a `FileResumeRequest` for the still-missing chunks; the small-file fast-path goes straight to `finalise_inbound` when the bitmap turns out complete. `file_card.rs` exposes a `data-testid="file-pause"` button while the inbound transfer is in flight and a `data-testid="file-resume"` button while it is paused, with i18n entries `file.pause_transfer` / `file.resume_transfer`. `try_resume_inbound_from_peer` (the network-induced auto-resume) was tightened to skip transfers the user has explicitly paused, so a peer reconnect cannot clobber an intentional user pause. Covered by the `pause/resume` test in `file-transfer-flow.spec.ts` plus the native `user_paused_defaults_to_false_and_round_trips` unit test. | P1-5 |
| G15 | ~~No receiver-side dangerous-extension confirmation~~ **(resolved)** | The receiver's file card now hijacks the download click for `dangerous = true` inbound files. Instead of a single-click `<a download>` link, dangerous inbound transfers render a `data-testid="file-download-danger-btn"` button that calls `DialogState::confirm` with the new i18n key `file.save_anyway_detail`. Cancel is a no-op; OK synthesises a click on a transient hidden `<a download>` so the browser's download path runs. The danger badge and the button persist across the Cancel branch so the user can retry. Sender-side dangerous-extension confirm continues to flow through `file_picker.rs` unchanged. The two-branch contract (Cancel keeps card, OK triggers a real `download` event) is locked down by the `dangerous extension save-anyway dialog` test in `file-transfer-flow.spec.ts`. | P1-5 |
| G16 | No mid-session E2EE key rotation | `webrtc::encryption::PeerCrypto` derives the AES-256-GCM shared key from one ECDH round during connection bootstrap and keeps that key for the lifetime of the page. There is no rekeying timer, no rotation on re-invite, and no user-reachable "rotate keys" affordance. Plan §3 P1-6 "key rotation on re-invite" is therefore unrepresentable; the P1-6 spec covers ciphertext + tamper-rejection only. A future ticket should either ship periodic rotation (e.g. every 24 h or every N messages) or wire the existing one-shot ECDH into a re-invite flow that runs the handshake again. | P1-6 |
| G17 | `UserInfoCard` was clipped inside the sidebar and bypassed the shared `ModalWrapper` | The user-info card (the dialog that opens when you click an online user in the sidebar to send them a connection invite) had **two** independent bugs that combined to produce the "modal renders as a small rectangle inside the sidebar" symptom. **(a) Containing-block trap**: the card was rendered inside `<aside class="sidebar">`. The sidebar carries `backdrop-filter: blur(16px) saturate(1.8)` which, per the CSS Filter Effects / Backdrop Filter spec, **establishes a containing block for `position: fixed` descendants** — the same effect `transform`, `filter`, `perspective`, `will-change`, `contain: paint` and `view-transition-name` all have. The card's `.modal-backdrop` therefore resolved `inset: 0` against the sidebar's 16-rem column instead of the viewport. On top of that, glass.css's `.sidebar > * { position: relative; z-index: 1; }` (intended to hoist normal sidebar children above the noise grain layer) was also hitting the modal backdrop and downgrading its `position: fixed` to `position: relative` — verified live via `getComputedStyle` and a manual @layer-aware CSS rule walk. **(b) Component bypass**: `UserInfoCard` hand-rolled its own `<div class="modal-backdrop">…<div class="modal">…</div></div>` instead of using `ModalWrapper`, so it had no enter/exit transitions, no Escape-to-close, no outside-click dismissal animation and no consistent backdrop testid — the visual / interaction behaviour was already inconsistent with `CreateRoomModal` / `InviteMemberModal` / `PasswordPromptModal` even before the clipping became visible. Initially fixed by hoisting the card into `GlobalRoomModalState` + `ModalManager`; later fully resolved by G19 below, which makes every `ModalWrapper` portal-render to `#modal-root` so any consumer — local or global — gets the right containing block. | dialog-arch |
| G18 | Six in-app dialogs were re-implementing the modal shell from scratch | `IncomingInviteModal`, `ForwardModal`, `IncomingCallModal`, `CallRecoveryPrompt`, `chat_view::Dialog` (the alert/confirm primitive), and the original `UserInfoCard` each hand-rolled their own backdrop / dialog markup, ARIA attributes, focus handling, Escape listener and (where present) outside-click handler. Result: inconsistent enter/exit animations (some had none), inconsistent test ids (`invite-backdrop`, `dialog-overlay`, `forward-modal-backdrop`, `call-modal-overlay`, `user-info-backdrop` — five different names for the same concept), inconsistent dismiss semantics (some closed on backdrop click, some did not), inconsistent z-index, and three independent regression vectors for the G17 containing-block bug. Fixed by routing every one of them through `ModalWrapper`, deleting the bespoke CSS overlays (`.forward-modal-backdrop`, `.call-modal-overlay`, `.dialog-overlay`, `.modal-backdrop` hand-written in JSX) and shrinking each component to "header + body + footer inside `<ModalWrapper>`". `ModalWrapper` itself gained two new props — `dismiss_on_backdrop_click` and `dismiss_on_escape`, both default `true` — so consumers that *must* be answered (incoming-call, recovery-prompt) can opt out without re-implementing the wrapper. | dialog-arch |
| G19 | Portal-render every modal so no ancestor's containing block can capture them | The single most reliable fix for the entire family of "fixed modal trapped inside ancestor X" bugs is to stop rendering modals next to their triggers and instead teleport them out of the layout tree. `ModalWrapper` now wraps its output in `leptos::portal::Portal`, mounted at `#modal-root` (rendered by `ModalManager` directly under `<body>`) with a `<body>` fallback for the unauthenticated shell. Consequence: `position: fixed` on every dialog now resolves against the viewport regardless of where the consumer component lives in the tree — sidebar, chat-view, theater fullscreen container, drawers, anywhere. This subsumes G17's earlier ad-hoc fixes (hoisting overlays out of `.app`, and the `:not(.modal-backdrop)` exclusion in glass.css), which are kept as belt-and-braces. Implementation note: `ModalWrapper`'s `children` prop had to switch from `Children` (`FnOnce`) to `ChildrenFn` (`Fn + Send + Sync`) because `leptos::portal::Portal::children` requires a multiply-callable closure; the per-instance `class` / `testid` / `dialog_role` / `labelled_by` strings are now stored as `Arc<str>` so the children closure can clone them on every Portal rebuild. | dialog-arch |
| G20 | ~~Sidebar search input is rendered but has no behaviour~~ **(resolved)** | `components/sidebar/mod.rs` mounted a `<input type="search">` with no signal binding. **Resolved** by introducing a local `search_query: RwSignal<String>` and wrapping each of the three section memos (pinned / active / archived) in a `Signal::derive` that runs them through the new pure helper `filter_conversations_by_query` (case-insensitive substring match on `display_name`). The input now carries `data-testid="sidebar-search-input"` and the filter behaviour is covered by `conv-list-management.spec.ts` test 4 plus three Rust unit tests on the pure helper. | P2-4 |
| G21 | ~~No "Delete conversation" affordance on the conversation row~~ **(resolved)** | `SidebarConversationMenu` now exposes a fourth "Delete conversation" item routed through an inline `ModalWrapper` confirmation owned by `SidebarConversationItem` (so the modal is portal-rendered and reachable even with no active chat view). Clicking Confirm calls the existing `AppState::purge_conversation`, which now additionally fires `PersistenceManager::clear_conversation` to remove the message store + search-index rows in addition to the existing `conversation_flags` tombstone. The deletion contract (in-memory removal + IDB tombstone surviving a reload) is covered by `conv-list-management.spec.ts` tests 5 / 6. | P2-4 |
| G22 | `SidebarConversationItem` froze flag-derived rendering on first-render snapshot | The component is keyed by `conversation.id` in the parent `<For>`, so it does not re-mount when the underlying `Conversation` changes. The original implementation captured `pinned` / `muted` / `archived` / `unread_count` from the prop *by value* and threaded them into every `data-*` attribute, class branch and `Show` gate as plain bools — meaning toggles that did NOT cause a section reparent (most prominently mute/unmute, since muted rows stay in the Active section) became silent UI no-ops until reload. `last_message_preview` already had the symmetric bug fixed via `Signal::derive(... app_state.conversations ...)`; we extended the same treatment to every flag the row renders, derived as `Signal::derive` from the global `app_state.conversations` lookup. `SidebarConversationMenu`'s `pinned`/`muted`/`archived` props correspondingly switched from `bool` to `Signal<bool>` so the menu also reads fresh values when re-opened after a previous toggle. Discovered while writing the P2-4 mute test. | P2-4 |
| G23 | Sticker panel's full-screen backdrop intercepted pointer events on the glyph grid | `StickerPanel` rendered a `<div class="sticker-panel-backdrop">` sibling for outside-click dismiss with `position: fixed; inset: 0; z-index: calc(var(--z-popover) - 1);`. The panel itself used `position: absolute` anchored to `.chat-input-bar` — so the backdrop lived in the viewport stacking context while the panel was bound to whichever stacking context its `chat-input-bar` ancestor inherited. On the default layout the backdrop ended up *above* the panel in paint order (despite the lower z-index — z-index only compares within the same stacking context), and Chromium's hit-test for `firstItem.click()` resolved to the backdrop and timed out. A second portal-less fix attempt revealed the panel was also below the `top-bar` for similar reasons. The proper fix mirrors the modal-portal solution (G19): the panel now `leptos::portal::Portal`-mounts under `#modal-root` with `position: fixed; bottom: 5rem; left: 1rem;`, and outside-click is handled by a `pointerdown` listener pattern (the same one `SidebarConversationMenu` uses) so no full-screen backdrop element is needed at all. Discovered while writing the P2-1 sticker pick test. | P2-1 |
| G24 | `ImagePreviewOverlay` Escape handler was unreachable | The overlay attached `on:keydown` directly to its root `<div>` — but the div had no `tabindex`, no focusable child, and was never `focus()`ed when it mounted, so keyboard events never targeted it. The handler therefore never fired for keyboard-only users (Escape was supposed to dismiss the overlay) and `Locator.press("Escape")` in the E2E suite was a no-op. The shared pattern across every other dismissable surface (`ModalWrapper`, `SidebarConversationMenu`, `StickerPanel`) is to attach the Escape listener to `window` and gate on the visibility signal. `ImagePreviewOverlay` now follows the same pattern via `leptos_use::use_event_listener(use_window(), ev::keydown, ...)`. The root div also gained `tabindex="-1"` so the dialog is at least programmatically focusable for assistive tech. Discovered while writing the P2-3 image preview test. | P2-3 |
| G25 | ~~`NicknameEditor` component exists but is never mounted~~ **(resolved)** | `frontend/src/components/room/nickname_editor.rs` ships a complete `NicknameEditor` with validation, save flow, signaling broadcast and full testid coverage; the auth state's `auth_nickname` localStorage mirror is updated synchronously by the click handler. **Resolved** by mounting `<NicknameEditor />` inside the Settings drawer's Account section (above the Logout button). In-session round-trip (open → edit → Save → close + reopen drawer → input shows new value) is covered by `profile.spec.ts` test 4. **Page-reload persistence remains gated on G28** (the server-side handler does not write the new nickname to the User table, so `AuthSuccess` after reload re-emits `nickname = username` and overwrites the localStorage value). | P2-7 |
| G26 | ~~No avatar upload affordance anywhere~~ **(resolved)** | `AvatarChange` signaling message, `UserStore::set_avatar`, `handle_avatar_change` server handler, `AvatarEditor` component (Phase A: data URL, ≤32 KB client-side guard, remove-avatar → identicon fallback), and four e2e tests (avatar.spec.ts) are now live. `UserInfo.avatar_url: Option<String>` is wired from `AuthSuccess` through `handle_avatar_change` broadcast to all online peers' `UserListUpdate`. Phase B (CDN URL / server-side resize) is a future expansion. | P2-7 |
| G27 | ~~Theater video source round-trip needs a codec-compatible test fixture~~ **(resolved owner-side; cross-peer deferred)** | Two pieces shipped together: **(a)** a 536-byte `e2e/assets/tiny.webm` (VP8, 2×2, single frame, generated by ffmpeg) as the codec-compatible fixture, and **(b)** a product bug fix in `TheaterVideoPlayer` — the original single Effect did `let Some(el) = video_ref.get() else { return; }` *before* setting `state.has_video_source = true`, but the `<video>` element is gated behind that very flag in the parent `<Show>`. On the first source pick the ref was therefore always `None`, the early-return fired, and the flag never flipped, leaving the player stuck on the picker forever. Fixed by splitting the binding into two effects: the first flips `has_video_source` + `video_source_label` the instant `source` is set, and the second waits for the now-mounted `<video>` element's `node_ref` to populate before attaching the `src` (or `srcObject` for screen-share). The owner-side `theater.spec.ts` test now feeds `tiny.webm` via `setInputFiles`, polls `video.videoWidth === 2` + `readyState ≥ 2`, and asserts the `<video>.src` is a `blob:` object URL. The full cross-peer "viewer joins + remote frame + danmaku round-trip" assertion remains deferred: it requires a successful WebRTC `publish_to_peers` handshake on top of the new fixture, which adds room-join choreography that this gap deliberately scopes out. | P2-5 |
| G28 | ~~Nickname change is not persisted in the global User table~~ **(resolved)** | `server/ws/room/handle_nickname_change` previously only wrote the room-scoped `MemberInfo` and bailed out with `UserNotInRoom` when the user was editing their nickname outside any room (i.e. from the settings drawer). **Resolved** by introducing `UserStore::set_nickname` and calling it unconditionally from the WS handler before attempting the room-state mirror. The room broadcast now degrades gracefully to a debug log on `UserNotInRoom` instead of erroring. As a result, `AuthSuccess` after a page reload returns the canonical persisted nickname and the client's `localStorage["auth_nickname"]` mirror is no longer overwritten on rehydration. Cross-reload contract is locked down by `profile.spec.ts` test 5. | P2-7 |

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
- [x] P0-5 av-call-happy.spec.ts — commit f69b975
- [x] P0-6 room.spec.ts — commit 7bc1bc9
- [x] P1-1 mention.spec.ts — commit 0b0362c
- [x] P1-2 persistence-extended.spec.ts — commit 763189d
- [x] P1-3 scroll.spec.ts — commit c5ca49d
- [x] P1-4 disconnect-advanced.spec.ts — commit 2f90f4a
- [x] P1-5 file-transfer-flow.spec.ts — receiver-side
  pause/resume happy path with integrity check (G14 resolved),
  mid-transfer signaling-WS-drop integrity (file payload travels
  over DataChannel, so the transfer survives transient WS flaps),
  and dangerous-extension save-anyway confirm vs cancel (G15
  resolved). 1 MB / 4 MB in-memory `zeroBuffer` fixtures; no new
  on-disk asset.
- [x] P1-6 e2ee-rotation.spec.ts — commit 6d3ab54
- [x] P1-7 context-menu-full.spec.ts — commit feb87a5
- [x] P2-1 sticker.spec.ts
- [x] P2-2 voice-message.spec.ts
- [x] P2-3 image-message.spec.ts
- [x] P2-4 conv-list-management.spec.ts — pin / mute / archive +
  search filter (G20 resolved) + delete (G21 resolved with reload-
  survival assertion).
- [~] P2-5 theater.spec.ts — creator/source-picker + in-room chat
  round-trip + URL-picker validation + owner-side local-file
  playback with `<video>` mounted on a `blob:` object URL (G27
  resolved owner-side, including the `has_video_source`
  chicken-and-egg fix). Viewer-join + cross-peer danmaku still
  deferred — needs WebRTC publish_to_peers + room-join
  choreography on top of the fixture.
- [x] P2-6 settings.spec.ts
- [x] P2-7 profile.spec.ts (5 tests) + avatar.spec.ts (4 tests) —
  block / unblock / blacklist persistence + in-session nickname
  edit (G25 resolved) + cross-reload nickname persistence (G28
  resolved) + avatar upload / remove / cross-reload (G26 resolved,
  split into avatar.spec.ts).
- [x] P2-8 keyboard-a11y.spec.ts
- [x] P2-9 forward-delivery.spec.ts
- [x] P2-10 text-limits.spec.ts
