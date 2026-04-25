use super::*;

#[test]
fn extracts_basic_mention() {
  assert_eq!(extract("hi @bob"), vec!["bob"]);
}

#[test]
fn extracts_multiple_mentions() {
  assert_eq!(extract("@a and @b"), vec!["a", "b"]);
}

#[test]
fn ignores_email_like_tokens() {
  assert!(extract("foo@bar.com").is_empty());
}

#[test]
fn supports_unicode_nicknames() {
  assert_eq!(extract("hello @小明"), vec!["小明"]);
}

#[test]
fn stops_at_cjk_punctuation() {
  assert_eq!(extract("@小明，你好"), vec!["小明"]);
  assert_eq!(extract("@小明。"), vec!["小明"]);
  assert_eq!(extract("@小明！"), vec!["小明"]);
  assert_eq!(extract("@小明、test"), vec!["小明"]);
}

#[test]
fn allows_cjk_name_with_fullwidth_paren() {
  assert_eq!(extract("（@小明）"), vec!["小明"]);
}

#[test]
fn mentions_is_case_insensitive() {
  let names = vec!["Bob".to_string()];
  assert!(mentions(&names, "bob"));
  assert!(!mentions(&names, "alice"));
}
