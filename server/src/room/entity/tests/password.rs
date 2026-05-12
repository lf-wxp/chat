//! Password management tests.

use super::*;

#[test]
fn test_is_password_protected_false_by_default() {
  let room = create_test_room();
  assert!(!room.is_password_protected());
}

#[test]
fn test_set_password_success() {
  let mut room = create_test_room();
  room.set_password(Some("test1234")).unwrap();
  assert!(room.is_password_protected());
}

#[test]
fn test_set_password_too_short() {
  let mut room = create_test_room();
  let result = room.set_password(Some("abc"));
  assert!(result.is_err());
  match result.unwrap_err() {
    RoomError::InvalidPassword(msg) => assert!(msg.contains("at least 4 characters")),
    _ => panic!("Expected InvalidPassword error"),
  }
}

#[test]
fn test_set_password_empty_removes_password() {
  let mut room = create_test_room();
  room.set_password(Some("test1234")).unwrap();
  assert!(room.is_password_protected());

  room.set_password(None).unwrap();
  assert!(!room.is_password_protected());
}

#[test]
fn test_set_password_empty_string_removes_password() {
  let mut room = create_test_room();
  room.set_password(Some("test1234")).unwrap();

  room.set_password(Some("")).unwrap();
  assert!(!room.is_password_protected());
}

#[test]
fn test_verify_password_correct() {
  let mut room = create_test_room();
  room.set_password(Some("mypassword")).unwrap();

  assert!(room.verify_password("mypassword"));
}

#[test]
fn test_verify_password_incorrect() {
  let mut room = create_test_room();
  room.set_password(Some("mypassword")).unwrap();

  assert!(!room.verify_password("wrongpassword"));
}

#[test]
fn test_verify_password_no_password_set_empty_input() {
  let room = create_test_room();
  assert!(room.verify_password(""));
}

#[test]
fn test_verify_password_no_password_set_nonempty_input() {
  let room = create_test_room();
  assert!(!room.verify_password("something"));
}
