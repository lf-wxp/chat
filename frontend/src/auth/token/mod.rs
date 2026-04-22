//! JWT token persistence and recovery.
//!
//! Manages storing and loading authentication data from localStorage
//! for the page-refresh recovery flow (Req 10.2).

use crate::state::AuthState;
use crate::utils;
use message::UserId;

/// localStorage keys for auth persistence.
const KEY_TOKEN: &str = "auth_token";
pub(crate) const KEY_USER_ID: &str = "auth_user_id";
pub(crate) const KEY_USERNAME: &str = "auth_username";
const KEY_NICKNAME: &str = "auth_nickname";
/// localStorage key for user avatar data URI (Req 10.9.34, P0-2 fix).
///
/// Stores the Identicon-generated SVG data URI by default. Will store
/// custom avatar data URI when user uploads a custom avatar in future.
const KEY_AVATAR: &str = "auth_avatar";
/// localStorage key for the currently active room (Req 10.4, 10.9.34).
///
/// Stores the room ID of the room the user is currently in. Cleared when
/// the user leaves the room. Used for room-state recovery on page refresh.
const KEY_ACTIVE_ROOM_ID: &str = "active_room_id";
/// localStorage key for the current call state (Req 10.5, 10.9.34).
///
/// Stores a JSON representation of the active call (room_id + call type).
/// Cleared when the call ends. Used for call-state recovery on page refresh.
const KEY_ACTIVE_CALL: &str = "active_call";
/// localStorage key for the currently active conversation (Req 10.9.34).
///
/// Kept here (rather than in `state.rs`) so `clear_auth_storage` can wipe
/// it as part of the logout flow without importing state internals.
pub(crate) const KEY_ACTIVE_CONVERSATION: &str = "active_conversation_id";
/// localStorage key for the user's custom signature (Req 10.1.6, Issue-5 fix).
const KEY_SIGNATURE: &str = "auth_signature";

/// Save auth state to localStorage.
///
/// Persists all auth fields including the avatar data URI so that they are
/// available immediately on page refresh without re-computing (Req 10.9.34).
///
/// The avatar is always written from `auth.avatar`, so any in-memory update
/// (e.g. a custom-uploaded avatar, or a preserved avatar during auth
/// recovery) is persisted and survives page refresh (P0-2 fix).
pub fn save_auth_to_storage(auth: &AuthState) {
  utils::save_to_local_storage(KEY_TOKEN, &auth.token);
  utils::save_to_local_storage(KEY_USER_ID, &auth.user_id.to_string());
  utils::save_to_local_storage(KEY_USERNAME, &auth.username);
  utils::save_to_local_storage(KEY_NICKNAME, &auth.nickname);

  // Always persist the current avatar so that changes (e.g. a different
  // Identicon after username change, or a future custom upload) survive
  // page refresh. Previously this only wrote the Identicon when
  // localStorage was empty, which meant any in-memory avatar update
  // was lost on refresh (P0-2 fix).
  utils::save_to_local_storage(KEY_AVATAR, &auth.avatar);
  // Persist signature so it survives page refresh (Issue-5 fix).
  utils::save_to_local_storage(KEY_SIGNATURE, &auth.signature);
}

/// Load auth state from localStorage.
///
/// Returns `None` if any required field is missing or invalid.
pub fn load_auth_from_storage() -> Option<AuthState> {
  let token = utils::load_from_local_storage(KEY_TOKEN)?;
  let user_id_str = utils::load_from_local_storage(KEY_USER_ID)?;
  let username = utils::load_from_local_storage(KEY_USERNAME)?;

  // Skip if token is empty (cleared on logout)
  if token.is_empty() {
    return None;
  }

  let user_id = match uuid::Uuid::parse_str(&user_id_str) {
    Ok(uuid) => UserId::from_uuid(uuid),
    Err(_) => {
      // The stored user_id is corrupted (not a valid UUID). Clear all
      // auth data so the next refresh does not keep trying to parse the
      // same invalid value, which would leave the client in an
      // inconsistent state (valid token but no usable user_id).
      // (P1-4 fix)
      clear_auth_storage();
      return None;
    }
  };
  let nickname = utils::load_from_local_storage(KEY_NICKNAME).unwrap_or_else(|| username.clone());
  let avatar = load_avatar_from_storage()
    .unwrap_or_else(|| crate::identicon::generate_identicon_data_uri(&username));
  let signature = utils::load_from_local_storage(KEY_SIGNATURE).unwrap_or_default();

  Some(AuthState {
    user_id,
    token,
    username,
    nickname,
    avatar,
    signature,
  })
}

/// Load the avatar data URI from localStorage.
///
/// Returns the stored data URI, or generates a fresh Identicon from the
/// stored username as a fallback (Req 10.9.34, P0-2 fix).
#[must_use]
pub fn load_avatar_from_storage() -> Option<String> {
  // Prefer the persisted avatar (may be a custom upload in future).
  if let Some(avatar) = utils::load_from_local_storage(KEY_AVATAR)
    && !avatar.is_empty()
  {
    return Some(avatar);
  }
  // Fallback: re-derive from the stored username.
  let username = utils::load_from_local_storage(KEY_USERNAME)?;
  Some(crate::identicon::generate_identicon_data_uri(&username))
}

/// Save the active room ID to localStorage (Req 10.4, P0-3 fix).
///
/// Called when the user joins a room. Pass `None` to clear.
pub fn save_active_room_id(room_id: Option<&str>) {
  match room_id {
    Some(id) => utils::save_to_local_storage(KEY_ACTIVE_ROOM_ID, id),
    None => utils::remove_from_local_storage(KEY_ACTIVE_ROOM_ID),
  }
}

/// Load the active room ID from localStorage (Req 10.4, P0-3 fix).
#[must_use]
pub fn load_active_room_id() -> Option<String> {
  let val = utils::load_from_local_storage(KEY_ACTIVE_ROOM_ID)?;
  if val.is_empty() { None } else { Some(val) }
}

/// Save the active call state to localStorage (Req 10.5, P0-3 fix).
///
/// `call_json` should be a JSON string representing the call state
/// (e.g. `{"room_id":"...","call_type":"audio"}`). Pass `None` to clear.
pub fn save_active_call(call_json: Option<&str>) {
  match call_json {
    Some(json) => utils::save_to_local_storage(KEY_ACTIVE_CALL, json),
    None => utils::remove_from_local_storage(KEY_ACTIVE_CALL),
  }
}

/// Load the active call state from localStorage (Req 10.5, P0-3 fix).
#[must_use]
pub fn load_active_call() -> Option<String> {
  let val = utils::load_from_local_storage(KEY_ACTIVE_CALL)?;
  if val.is_empty() { None } else { Some(val) }
}

/// Clear all auth data from localStorage.
///
/// Uses `removeItem` so that subsequent reads return `None` cleanly
/// rather than an empty string (Req 10.9.35f). Also clears the
/// active conversation, room, and call pointers so a fresh login
/// starts clean (Req 10.9.34, P0-3 fix).
pub fn clear_auth_storage() {
  utils::remove_from_local_storage(KEY_TOKEN);
  utils::remove_from_local_storage(KEY_USER_ID);
  utils::remove_from_local_storage(KEY_USERNAME);
  utils::remove_from_local_storage(KEY_NICKNAME);
  utils::remove_from_local_storage(KEY_AVATAR);
  utils::remove_from_local_storage(KEY_SIGNATURE);
  utils::remove_from_local_storage(KEY_ACTIVE_CONVERSATION);
  utils::remove_from_local_storage(KEY_ACTIVE_ROOM_ID);
  utils::remove_from_local_storage(KEY_ACTIVE_CALL);
}

#[cfg(test)]
mod tests;
