//! Connection management tests

mod common;

use message::signal::{SignalMessage, UserStatus};

#[test]
fn test_send_to_user_online() {
  let state = common::new_state();
  let mut rx = common::register_mock_user(&state, "user-1", "alice");

  let msg = SignalMessage::Pong;
  state.send_to_user("user-1", &msg);

  let messages = common::drain_messages(&mut rx);
  assert_eq!(messages.len(), 1);
  assert_eq!(messages[0], SignalMessage::Pong);
}

#[test]
fn test_send_to_user_offline_no_panic() {
  let state = common::new_state();
  // Sending a message to a nonexistent user should not panic
  let msg = SignalMessage::Pong;
  state.send_to_user("nonexistent", &msg);
}

#[test]
fn test_broadcast_to_all() {
  let state = common::new_state();
  let mut rx1 = common::register_mock_user(&state, "user-1", "alice");
  let mut rx2 = common::register_mock_user(&state, "user-2", "bob");

  let msg = SignalMessage::Pong;
  state.broadcast(&msg, None);

  assert!(common::has_message(&mut rx1, |m| *m == SignalMessage::Pong));
  assert!(common::has_message(&mut rx2, |m| *m == SignalMessage::Pong));
}

#[test]
fn test_broadcast_with_exclude() {
  let state = common::new_state();
  let mut rx1 = common::register_mock_user(&state, "user-1", "alice");
  let mut rx2 = common::register_mock_user(&state, "user-2", "bob");

  let msg = SignalMessage::Pong;
  state.broadcast(&msg, Some("user-1"));

  // user-1 is excluded and should not receive the message
  let msgs1 = common::drain_messages(&mut rx1);
  assert!(msgs1.is_empty());

  // user-2 should receive the message
  assert!(common::has_message(&mut rx2, |m| *m == SignalMessage::Pong));
}

#[test]
fn test_broadcast_user_list() {
  let state = common::new_state();
  let mut rx1 = common::register_mock_user(&state, "user-1", "alice");
  let mut rx2 = common::register_mock_user(&state, "user-2", "bob");

  state.broadcast_user_list();

  // Both users should receive UserListUpdate
  let msgs1 = common::drain_messages(&mut rx1);
  assert!(
    msgs1
      .iter()
      .any(|m| matches!(m, SignalMessage::UserListUpdate { users } if users.len() == 2))
  );

  let msgs2 = common::drain_messages(&mut rx2);
  assert!(
    msgs2
      .iter()
      .any(|m| matches!(m, SignalMessage::UserListUpdate { users } if users.len() == 2))
  );
}

#[test]
fn test_register_connection() {
  let state = common::new_state();

  // Create a session first
  let session = server::auth::UserSession {
    user_id: "user-1".to_string(),
    username: "alice".to_string(),
    password_hash: "hash".to_string(),
    status: UserStatus::Online,
    avatar: None,
    signature: None,
  };
  state.inner().sessions.insert("user-1".to_string(), session);

  let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
  state.register_connection("user-1".to_string(), "alice".to_string(), tx);

  // Should be registered
  assert!(state.inner().connections.contains_key("user-1"));

  // register_connection triggers broadcast_user_list
  let messages = common::drain_messages(&mut rx);
  assert!(
    messages
      .iter()
      .any(|m| matches!(m, SignalMessage::UserListUpdate { .. }))
  );
}

#[test]
fn test_unregister_connection() {
  let state = common::new_state();
  let mut rx1 = common::register_mock_user(&state, "user-1", "alice");
  let mut rx2 = common::register_mock_user(&state, "user-2", "bob");

  state.unregister_connection("user-1");

  // user-1 should have been removed
  assert!(!state.inner().connections.contains_key("user-1"));

  // user-2 should receive the updated user list (only 1 user)
  let msgs2 = common::drain_messages(&mut rx2);
  assert!(
    msgs2
      .iter()
      .any(|m| matches!(m, SignalMessage::UserListUpdate { users } if users.len() == 1))
  );

  // user-1's rx should no longer receive messages (connection removed)
  let msgs1 = common::drain_messages(&mut rx1);
  assert!(msgs1.is_empty());
}

#[test]
fn test_register_delivers_pending_invites() {
  let state = common::new_state();

  // Store a pending offline invite first
  let invite = SignalMessage::ConnectionInvite {
    from: "user-2".to_string(),
    to: "user-1".to_string(),
    message: Some("hello".to_string()),
    invite_type: message::signal::InviteType::Chat,
  };
  state
    .inner()
    .pending_invites
    .entry("user-1".to_string())
    .or_default()
    .push(invite);

  // Create a session
  let session = server::auth::UserSession {
    user_id: "user-1".to_string(),
    username: "alice".to_string(),
    password_hash: "hash".to_string(),
    status: UserStatus::Online,
    avatar: None,
    signature: None,
  };
  state.inner().sessions.insert("user-1".to_string(), session);

  // Register the connection
  let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
  state.register_connection("user-1".to_string(), "alice".to_string(), tx);

  // Should receive the pending offline invite
  let messages = common::drain_messages(&mut rx);
  assert!(
    messages
      .iter()
      .any(|m| matches!(m, SignalMessage::ConnectionInvite { from, .. } if from == "user-2"))
  );

  // Pending invites should have been cleared
  assert!(!state.inner().pending_invites.contains_key("user-1"));
}
