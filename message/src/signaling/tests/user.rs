//! User discovery message tests.

use super::*;
use crate::types::UserInfo;

#[test]
fn test_user_list_update_roundtrip() {
  let msg = UserListUpdate {
    users: vec![
      UserInfo {
        user_id: UserId::new(),
        username: "alice".to_string(),
        nickname: "Alice".to_string(),
        status: UserStatus::Online,
        avatar_url: None,
        bio: "Hello".to_string(),
        created_at_nanos: 1_000_000_000,
        last_seen_nanos: 2_000_000_000,
      },
      UserInfo {
        user_id: UserId::new(),
        username: "bob".to_string(),
        nickname: "Bob".to_string(),
        status: UserStatus::Away,
        avatar_url: None,
        bio: String::new(),
        created_at_nanos: 1_500_000_000,
        last_seen_nanos: 2_500_000_000,
      },
    ],
  };
  let encoded = bitcode::encode(&msg);
  let decoded: UserListUpdate = bitcode::decode(&encoded).expect("Failed to decode");
  assert_eq!(msg, decoded);
}

#[test]
fn test_user_status_change_roundtrip() {
  let msg = UserStatusChange {
    user_id: UserId::new(),
    status: UserStatus::Busy,
    signature: Some("In a meeting".to_string()),
  };
  let encoded = bitcode::encode(&msg);
  let decoded: UserStatusChange = bitcode::decode(&encoded).expect("Failed to decode");
  assert_eq!(msg, decoded);
}

#[test]
fn test_nickname_change_roundtrip() {
  let msg = NicknameChange {
    user_id: UserId::new(),
    new_nickname: "New Nick".to_string(),
  };
  let encoded = bitcode::encode(&msg);
  let decoded: NicknameChange = bitcode::decode(&encoded).expect("Failed to decode");
  assert_eq!(msg, decoded);
}

#[test]
fn test_discriminator_user_discovery() {
  assert_eq!(
    SignalingMessage::UserListUpdate(UserListUpdate { users: vec![] }).discriminator(),
    USER_LIST_UPDATE
  );
  assert_eq!(
    SignalingMessage::UserStatusChange(UserStatusChange {
      user_id: UserId::new(),
      status: UserStatus::Online,
      signature: None
    })
    .discriminator(),
    USER_STATUS_CHANGE
  );
}
