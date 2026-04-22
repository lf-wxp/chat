# Design Note — AuthPostRecovery Chain

> **Date**: 2026-04-20
> **Status**: Proposal (to be implemented incrementally across tasks 14–18)
> **Related Requirements**: Req 10.2 (Identity Recovery), Req 10.3 (WebRTC Recovery), Req 10.4 (Room Recovery), Req 10.5 (Call Recovery), Req 10.10 (Server Restart)
> **Related Code**: `frontend/src/signaling/connection/handlers.rs::handle_auth_success`

---

## 1. Problem Statement

After a successful `TokenAuth` (page refresh or reconnect), the client must restore multiple layers of state in the correct order. Currently, the recovery steps are spread across `handle_auth_success` as inline code:

1. Update `AuthState` signal
2. Persist `user_id` / `username` to localStorage
3. Start `UserStatusManager`
4. Rejoin active room (Req 10.4, added in Issue-1 fix)

As tasks 15–18 land, more recovery steps will be needed:

- **Task 15**: Rebuild WebRTC PeerConnections from `ActivePeersList` + ECDH re-negotiation (Req 10.3)
- **Task 15**: Recover active call state from localStorage (Req 10.5)
- **Task 18**: Resume media tracks if call recovery is confirmed by user

The `handle_auth_success` method will become increasingly complex. We need a structured approach to make recovery steps independently testable, orderable, and fault-tolerant.

---

## 2. Proposed Design: Recovery Step Chain

### 2.1 Core Abstraction

```rust
/// A single recovery step executed after TokenAuth succeeds.
///
/// Each step is independent: if one step fails, subsequent steps
/// still execute (fail-open), and the error is logged + optionally
/// surfaced as a toast.
pub(crate) trait RecoveryStep {
    /// Human-readable name for logging (e.g. "RoomRecovery").
    fn name(&self) -> &'static str;

    /// Execute the recovery step. Returns Ok(()) on success or
    /// Err(reason) on failure (logged, does not abort the chain).
    fn execute(&self, ctx: &RecoveryContext) -> Result<(), String>;
}

/// Shared context passed to every recovery step.
pub(crate) struct RecoveryContext {
    pub app_state: AppState,
    pub client: SignalingClient,
    pub auth_success: AuthSuccessInfo,
}

/// Lightweight copy of AuthSuccess fields needed by recovery steps.
pub(crate) struct AuthSuccessInfo {
    pub user_id: UserId,
    pub username: String,
}
```

### 2.2 Step Registry & Execution

```rust
pub(crate) struct AuthPostRecovery {
    steps: Vec<Box<dyn RecoveryStep>>,
}

impl AuthPostRecovery {
    pub fn new() -> Self {
        Self {
            steps: vec![
                Box::new(UpdateAuthStateStep),
                Box::new(PersistUserInfoStep),
                Box::new(StartStatusManagerStep),
                Box::new(RoomRecoveryStep),
                // Added by task 15:
                // Box::new(WebRtcRecoveryStep),
                // Box::new(CallRecoveryStep),
            ],
        }
    }

    pub fn run(&self, ctx: &RecoveryContext) {
        for step in &self.steps {
            match step.execute(ctx) {
                Ok(()) => {
                    console_log(&format!(
                        "[recovery] {} completed", step.name()
                    ));
                }
                Err(e) => {
                    console_warn(&format!(
                        "[recovery] {} failed: {}", step.name(), e
                    ));
                    // Continue to next step — fail-open policy
                }
            }
        }
    }
}
```

### 2.3 Step Implementations (Current — Task 14)

| Step | Struct | Req | Description |
|------|--------|-----|-------------|
| 1 | `UpdateAuthStateStep` | 10.2 | Update `app_state.auth` signal with server-confirmed `user_id` / `username`, preserving existing nickname |
| 2 | `PersistUserInfoStep` | 10.9.34 | Write `user_id` and `username` to localStorage |
| 3 | `StartStatusManagerStep` | 10.1.6a | Start `UserStatusManager` (activity tracking + 5-min auto-away) |
| 4 | `RoomRecoveryStep` | 10.4.18 | Read `active_room_id` from localStorage → send `JoinRoom` |

### 2.4 Future Steps (Tasks 15–18)

| Step | Struct | Task | Req | Description |
|------|--------|------|-----|-------------|
| 5 | `WebRtcRecoveryStep` | 15 | 10.3.10–16 | Wait for `ActivePeersList` from server → batch rebuild PeerConnections (3 concurrent, 15s timeout per batch) → ECDH re-negotiation per peer |
| 6 | `CallRecoveryStep` | 15/18 | 10.5.21–24 | Read `active_call` from localStorage → show "Resume call?" confirmation → re-request media permissions → add tracks to recovered PC |
| 7 | `ServerRestartDetectionStep` | — | 10.10.39 | Detect server restart (empty `ActivePeersList` + empty `RoomListUpdate`) → clear stale `active_room_id` / `active_call` → show "Server restarted" toast |

---

## 3. Execution Order & Dependencies

```
TokenAuth success (onmessage)
    │
    ▼
┌─────────────────────────────┐
│  1. UpdateAuthState         │  ← Must be first (other steps read auth)
│  2. PersistUserInfo         │  ← Independent, no dependency
│  3. StartStatusManager      │  ← Needs auth to be set
│  4. RoomRecovery            │  ← Needs WS connected (already true)
└─────────────────────────────┘
    │
    ▼  (server push: ActivePeersList arrives asynchronously)
    │
┌─────────────────────────────┐
│  5. WebRtcRecovery          │  ← Triggered by ActivePeersList message
│  6. CallRecovery            │  ← After WebRTC PCs are rebuilt
│  7. ServerRestartDetection  │  ← Triggered by empty lists
└─────────────────────────────┘
```

**Key insight**: Steps 1–4 are synchronous and execute immediately in `handle_auth_success`. Steps 5–7 are asynchronous and triggered by subsequent server messages (`ActivePeersList`, `RoomListUpdate`). The `RecoveryStep` trait handles both cases:

- Sync steps: `execute()` runs inline and returns immediately.
- Async steps: `execute()` spawns a `spawn_local` future and returns `Ok(())`. Completion is tracked via a `RwSignal<RecoveryStatus>` on `AppState`.

---

## 4. Error Handling Policy

| Scenario | Behavior |
|----------|----------|
| Step fails (e.g. `JoinRoom` send error) | Log warning, continue chain, show toast if user-facing |
| Step panics | Caught by WASM runtime, logged, chain aborted (acceptable — panics indicate bugs) |
| All steps complete | Hide "Restoring connections..." banner (Req 10.11.42) |
| Some steps failed | Show "Partial recovery — some connections could not be restored" toast |

---

## 5. Testing Strategy

Each `RecoveryStep` can be unit-tested independently by constructing a mock `RecoveryContext`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_room_recovery_step_with_no_active_room() {
        // RecoveryContext with no active_room_id in localStorage
        // → step should return Ok(()) and not send any message
    }

    #[test]
    fn test_room_recovery_step_with_invalid_uuid() {
        // localStorage has "not-a-uuid" → step should return Ok(())
        // and log a warning (fail-open)
    }
}
```

The chain itself is tested by verifying step ordering and fail-open behavior:

```rust
#[test]
fn test_chain_continues_after_step_failure() {
    // Create a chain with a failing step in the middle
    // → verify subsequent steps still execute
}
```

---

## 6. Migration Path

### Phase 1 (Current — Task 14)
The recovery logic remains inline in `handle_auth_success` as implemented. The `RecoveryStep` trait is **not** introduced yet — this design note serves as a blueprint.

### Phase 2 (Task 15)
When WebRTC recovery logic is added, refactor `handle_auth_success` to use the `AuthPostRecovery` chain. This is the natural inflection point because:
- Two more steps are added (WebRTC + Call recovery)
- The async nature of `ActivePeersList` processing demands a more structured approach
- The `RecoveryContext` pattern avoids passing 5+ parameters through nested closures

### Phase 3 (Task 18+)
Add media track recovery, server restart detection, and completion banner management as additional steps.

---

## 7. File Organization

```
frontend/src/signaling/
├── connection/
│   ├── mod.rs              # SignalingClient, connect/disconnect/send
│   ├── handlers.rs         # WS event handlers, delegates to recovery chain
│   ├── heartbeat.rs        # Ping/Pong + pong watchdog
│   └── recovery/           # NEW in task 15
│       ├── mod.rs          # AuthPostRecovery chain + RecoveryStep trait
│       ├── auth_state.rs   # UpdateAuthStateStep + PersistUserInfoStep
│       ├── status.rs       # StartStatusManagerStep
│       ├── room.rs         # RoomRecoveryStep
│       ├── webrtc.rs       # WebRtcRecoveryStep (task 15)
│       └── call.rs         # CallRecoveryStep (task 15/18)
```

This keeps each step in its own file (consistent with the project's "one concern per file" convention) and makes it trivial to add/remove/reorder steps.
