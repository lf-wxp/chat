//! Connection invite message tests.

use super::*;

#[test]
fn test_connection_invite_roundtrip() {
  let msg = ConnectionInvite {
    from: UserId::new(),
    to: UserId::new(),
    note: Some("Let's chat!".to_string()),
  };
  let encoded = bitcode::encode(&msg);
  let decoded: ConnectionInvite = bitcode::decode(&encoded).expect("Failed to decode");
  assert_eq!(msg, decoded);
}

#[test]
fn test_invite_accepted_roundtrip() {
  let msg = InviteAccepted {
    from: UserId::new(),
    to: UserId::new(),
  };
  let encoded = bitcode::encode(&msg);
  let decoded: InviteAccepted = bitcode::decode(&encoded).expect("Failed to decode");
  assert_eq!(msg, decoded);
}

#[test]
fn test_invite_declined_roundtrip() {
  let msg = InviteDeclined {
    from: UserId::new(),
    to: UserId::new(),
  };
  let encoded = bitcode::encode(&msg);
  let decoded: InviteDeclined = bitcode::decode(&encoded).expect("Failed to decode");
  assert_eq!(msg, decoded);
}

#[test]
fn test_invite_timeout_roundtrip() {
  let msg = InviteTimeout {
    from: UserId::new(),
    to: UserId::new(),
  };
  let encoded = bitcode::encode(&msg);
  let decoded: InviteTimeout = bitcode::decode(&encoded).expect("Failed to decode");
  assert_eq!(msg, decoded);
}

#[test]
fn test_multi_invite_roundtrip() {
  let msg = MultiInvite {
    from: UserId::new(),
    targets: vec![UserId::new(), UserId::new()],
  };
  let encoded = bitcode::encode(&msg);
  let decoded: MultiInvite = bitcode::decode(&encoded).expect("Failed to decode");
  assert_eq!(msg, decoded);
}

#[test]
fn test_signaling_message_invite_accepted_roundtrip() {
  let msg = SignalingMessage::InviteAccepted(InviteAccepted {
    from: UserId::new(),
    to: UserId::new(),
  });
  let encoded = bitcode::encode(&msg);
  let decoded: SignalingMessage = bitcode::decode(&encoded).expect("Failed to decode");
  assert_eq!(msg, decoded);
}

#[test]
fn test_signaling_message_invite_declined_roundtrip() {
  let msg = SignalingMessage::InviteDeclined(InviteDeclined {
    from: UserId::new(),
    to: UserId::new(),
  });
  let encoded = bitcode::encode(&msg);
  let decoded: SignalingMessage = bitcode::decode(&encoded).expect("Failed to decode");
  assert_eq!(msg, decoded);
}

#[test]
fn test_discriminator_invite_messages() {
  let uid1 = UserId::new();
  let uid2 = UserId::new();
  assert_eq!(
    SignalingMessage::ConnectionInvite(ConnectionInvite {
      from: uid1.clone(),
      to: uid2.clone(),
      note: None
    })
    .discriminator(),
    CONNECTION_INVITE
  );
  assert_eq!(
    SignalingMessage::InviteAccepted(InviteAccepted {
      from: uid1.clone(),
      to: uid2.clone()
    })
    .discriminator(),
    INVITE_ACCEPTED
  );
  assert_eq!(
    SignalingMessage::InviteDeclined(InviteDeclined {
      from: uid1.clone(),
      to: uid2.clone()
    })
    .discriminator(),
    INVITE_DECLINED
  );
  assert_eq!(
    SignalingMessage::InviteTimeout(InviteTimeout {
      from: uid1.clone(),
      to: uid2.clone()
    })
    .discriminator(),
    INVITE_TIMEOUT
  );
  assert_eq!(
    SignalingMessage::MultiInvite(MultiInvite {
      from: uid1,
      targets: vec![]
    })
    .discriminator(),
    MULTI_INVITE
  );
}
