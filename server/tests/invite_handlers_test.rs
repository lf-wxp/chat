//! Invite handlers integration tests

mod common;

use message::signal::{InviteType, SignalMessage};
use server::handler::invite_handlers;

// ========================================================================
// Connection Invite
// ========================================================================

#[tokio::test]
async fn test_connection_invite_to_online_user() {
  let state = common::new_state();
  let _rx_from = common::register_mock_user(&state, "user-1", "alice");
  let mut rx_to = common::register_mock_user(&state, "user-2", "bob");

  invite_handlers::handle_connection_invite(
    "user-1",
    "user-2".to_string(),
    Some("Want to chat?".to_string()),
    InviteType::Chat,
    &state,
  );

  let msgs = common::drain_messages(&mut rx_to);
  assert!(msgs.iter().any(|m| matches!(
    m,
    SignalMessage::ConnectionInvite { from, to, invite_type: InviteType::Chat, .. }
    if from == "user-1" && to == "user-2"
  )));
}

#[tokio::test]
async fn test_connection_invite_to_offline_user_stored() {
  let state = common::new_state();
  let _rx_from = common::register_mock_user(&state, "user-1", "alice");
  // user-2 is offline

  invite_handlers::handle_connection_invite(
    "user-1",
    "user-2".to_string(),
    None,
    InviteType::AudioCall,
    &state,
  );

  // Should be stored as offline invite
  assert!(state.inner().pending_invites.contains_key("user-2"));
  let invites = state.inner().pending_invites.get("user-2").unwrap();
  assert_eq!(invites.len(), 1);
}

#[tokio::test]
async fn test_connection_invite_duplicate_blocked() {
  let state = common::new_state();
  let mut rx_from = common::register_mock_user(&state, "user-1", "alice");
  let _rx_to = common::register_mock_user(&state, "user-2", "bob");

  // First invite
  invite_handlers::handle_connection_invite(
    "user-1",
    "user-2".to_string(),
    None,
    InviteType::Chat,
    &state,
  );

  // Clear messages
  common::drain_messages(&mut rx_from);

  // Second invite (should be blocked within 30 seconds)
  invite_handlers::handle_connection_invite(
    "user-1",
    "user-2".to_string(),
    None,
    InviteType::Chat,
    &state,
  );

  let msgs = common::drain_messages(&mut rx_from);
  assert!(
    msgs
      .iter()
      .any(|m| matches!(m, SignalMessage::Error { message, .. } if message.contains("Duplicate")))
  );
}

// ========================================================================
// Invite Response
// ========================================================================

#[test]
fn test_invite_response_accepted() {
  let state = common::new_state();
  let _rx_from = common::register_mock_user(&state, "user-1", "alice");
  let mut rx_to = common::register_mock_user(&state, "user-2", "bob");

  invite_handlers::handle_invite_response("user-1", "user-2".to_string(), true, &state);

  let msgs = common::drain_messages(&mut rx_to);
  assert!(msgs.iter().any(|m| matches!(
    m,
    SignalMessage::InviteResponse { from, to, accepted: true }
    if from == "user-1" && to == "user-2"
  )));
}

#[test]
fn test_invite_response_rejected() {
  let state = common::new_state();
  let _rx_from = common::register_mock_user(&state, "user-1", "alice");
  let mut rx_to = common::register_mock_user(&state, "user-2", "bob");

  invite_handlers::handle_invite_response("user-1", "user-2".to_string(), false, &state);

  let msgs = common::drain_messages(&mut rx_to);
  assert!(msgs.iter().any(|m| matches!(
    m,
    SignalMessage::InviteResponse {
      accepted: false,
      ..
    }
  )));
}

#[test]
fn test_invite_response_clears_active_invites() {
  let state = common::new_state();
  let _rx1 = common::register_mock_user(&state, "user-1", "alice");
  let _rx2 = common::register_mock_user(&state, "user-2", "bob");

  // Simulate active invite
  let key = format!("user-1->user-2:{:?}", InviteType::Chat);
  state.inner().active_invites.insert(key.clone(), i64::MAX);

  invite_handlers::handle_invite_response("user-2", "user-1".to_string(), true, &state);

  // Active invite should be cleared
  assert!(!state.inner().active_invites.contains_key(&key));
}

// ========================================================================
// Invite Link
// ========================================================================

#[tokio::test]
async fn test_create_invite_link() {
  let state = common::new_state();
  let mut rx = common::register_mock_user(&state, "user-1", "alice");

  invite_handlers::handle_create_invite_link("user-1", InviteType::Chat, None, &state);

  let msgs = common::drain_messages(&mut rx);
  assert!(msgs.iter().any(|m| matches!(
    m,
    SignalMessage::InviteLinkCreated {
      invite_type: InviteType::Chat,
      ..
    }
  )));

  // Invite link should be stored in state
  assert_eq!(state.inner().invite_links.len(), 1);
}

#[tokio::test]
async fn test_join_by_invite_link_chat() {
  let state = common::new_state();
  let mut rx_creator = common::register_mock_user(&state, "user-1", "alice");
  let mut rx_joiner = common::register_mock_user(&state, "user-2", "bob");

  // Create invite link
  invite_handlers::handle_create_invite_link("user-1", InviteType::Chat, None, &state);

  // Get invite code
  let msgs = common::drain_messages(&mut rx_creator);
  let code = msgs
    .iter()
    .find_map(|m| {
      if let SignalMessage::InviteLinkCreated { code, .. } = m {
        Some(code.clone())
      } else {
        None
      }
    })
    .expect("Should receive InviteLinkCreated");

  // user-2 joins via invite link
  invite_handlers::handle_join_by_invite_link("user-2", code.clone(), &state);

  // creator should receive ConnectionInvite
  let msgs = common::drain_messages(&mut rx_creator);
  assert!(
    msgs
      .iter()
      .any(|m| matches!(m, SignalMessage::ConnectionInvite { from, .. } if from == "user-2"))
  );

  // joiner should receive InviteResponse(accepted: true)
  let msgs = common::drain_messages(&mut rx_joiner);
  assert!(
    msgs
      .iter()
      .any(|m| matches!(m, SignalMessage::InviteResponse { accepted: true, .. }))
  );

  // Invite link should be marked as used
  let link = state.inner().invite_links.get(&code).unwrap();
  assert!(link.used);
}

#[test]
fn test_join_by_invite_link_expired() {
  let state = common::new_state();
  let _rx_creator = common::register_mock_user(&state, "user-1", "alice");
  let mut rx_joiner = common::register_mock_user(&state, "user-2", "bob");

  // Manually insert an expired invite link
  let entry = server::state::InviteLinkEntry {
    creator_id: "user-1".to_string(),
    creator_username: "alice".to_string(),
    invite_type: InviteType::Chat,
    room_id: None,
    expires_at: 0, // expired
    used: false,
  };
  state
    .inner()
    .invite_links
    .insert("expired-code".to_string(), entry);

  invite_handlers::handle_join_by_invite_link("user-2", "expired-code".to_string(), &state);

  let msgs = common::drain_messages(&mut rx_joiner);
  assert!(
    msgs.iter().any(
      |m| matches!(m, SignalMessage::InviteLinkError { reason } if reason.contains("expired"))
    )
  );
}

#[test]
fn test_join_by_invite_link_already_used() {
  let state = common::new_state();
  let _rx_creator = common::register_mock_user(&state, "user-1", "alice");
  let mut rx_joiner = common::register_mock_user(&state, "user-2", "bob");

  let entry = server::state::InviteLinkEntry {
    creator_id: "user-1".to_string(),
    creator_username: "alice".to_string(),
    invite_type: InviteType::Chat,
    room_id: None,
    expires_at: i64::MAX,
    used: true, // already used
  };
  state
    .inner()
    .invite_links
    .insert("used-code".to_string(), entry);

  invite_handlers::handle_join_by_invite_link("user-2", "used-code".to_string(), &state);

  let msgs = common::drain_messages(&mut rx_joiner);
  assert!(msgs.iter().any(
    |m| matches!(m, SignalMessage::InviteLinkError { reason } if reason.contains("already been used"))
  ));
}

#[test]
fn test_join_by_invite_link_self() {
  let state = common::new_state();
  let mut rx = common::register_mock_user(&state, "user-1", "alice");

  let entry = server::state::InviteLinkEntry {
    creator_id: "user-1".to_string(),
    creator_username: "alice".to_string(),
    invite_type: InviteType::Chat,
    room_id: None,
    expires_at: i64::MAX,
    used: false,
  };
  state
    .inner()
    .invite_links
    .insert("self-code".to_string(), entry);

  // User using their own invite link
  invite_handlers::handle_join_by_invite_link("user-1", "self-code".to_string(), &state);

  let msgs = common::drain_messages(&mut rx);
  assert!(msgs.iter().any(
    |m| matches!(m, SignalMessage::InviteLinkError { reason } if reason.contains("yourself"))
  ));
}

#[test]
fn test_join_by_invite_link_invalid_code() {
  let state = common::new_state();
  let mut rx = common::register_mock_user(&state, "user-1", "alice");

  invite_handlers::handle_join_by_invite_link("user-1", "invalid-code".to_string(), &state);

  let msgs = common::drain_messages(&mut rx);
  assert!(
    msgs.iter().any(
      |m| matches!(m, SignalMessage::InviteLinkError { reason } if reason.contains("invalid"))
    )
  );
}

// ========================================================================
// Rate Limiting
// ========================================================================

#[tokio::test]
async fn test_invite_rate_limiting() {
  let state = common::new_state();
  let mut rx_from = common::register_mock_user(&state, "user-1", "alice");

  // Register 10 different target users so duplicate-check doesn't block us
  for i in 0..10 {
    let uid = format!("target-{i}");
    let _rx = common::register_mock_user(&state, &uid, &format!("target{i}"));
  }

  // Send 10 invites (should all succeed)
  for i in 0..10 {
    invite_handlers::handle_connection_invite(
      "user-1",
      format!("target-{i}"),
      None,
      InviteType::Chat,
      &state,
    );
  }

  // Drain messages from the first 10 invites
  common::drain_messages(&mut rx_from);

  // 11th invite should be rate-limited
  let _rx_extra = common::register_mock_user(&state, "target-extra", "extra");
  invite_handlers::handle_connection_invite(
    "user-1",
    "target-extra".to_string(),
    None,
    InviteType::Chat,
    &state,
  );

  let msgs = common::drain_messages(&mut rx_from);
  assert!(
    msgs.iter().any(|m| matches!(
      m,
      SignalMessage::Error { code: 4002, message } if message.contains("rate")
    )),
    "11th invite should be rate-limited"
  );
}
