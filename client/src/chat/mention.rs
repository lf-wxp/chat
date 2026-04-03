//! @Mention functionality for chat input
//!
//! Provides helper functions for detecting and inserting @mentions in chat messages.

/// Insert selected username into the input text, replacing the current @query
///
/// Finds the last occurrence of `@query` and replaces it with `@username `.
pub fn insert_mention(username: &str, input_text: &str, mention_query: &str) -> String {
  use std::fmt::Write;
  let current = input_text;
  let query = mention_query;

  // Find the last @query and replace with @username + space
  if let Some(at_pos) = current.rfind(&format!("@{query}")) {
    let mut new_text = current[..at_pos].to_string();
    let _ = write!(new_text, "@{username} ");
    new_text.push_str(&current[at_pos + 1 + query.len()..]);
    new_text
  } else {
    current.to_string()
  }
}

/// Extract @usernames from text
///
/// Scans text for all `@username` patterns and returns deduplicated usernames.
/// `known_usernames` is used for exact matching - only returns known usernames.
pub fn extract_mentions(text: &str, known_usernames: &[String]) -> Vec<String> {
  let mut mentions = Vec::new();
  let mut remaining = text;

  while let Some(at_pos) = remaining.find('@') {
    let after_at = &remaining[at_pos + 1..];
    let name_end = after_at
      .find(|c: char| !c.is_alphanumeric() && c != '_' && c != '-')
      .unwrap_or(after_at.len());

    if name_end > 0 {
      let name = &after_at[..name_end];
      if known_usernames.iter().any(|n| n == name) && !mentions.contains(&name.to_string()) {
        mentions.push(name.to_string());
      }
    }

    remaining = &after_at[name_end.max(1)..];
  }

  mentions
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_extract_mentions_basic() {
    let text = "Hello @alice and @bob!";
    let known = vec![
      "alice".to_string(),
      "bob".to_string(),
      "charlie".to_string(),
    ];
    let mentions = extract_mentions(text, &known);
    assert_eq!(mentions, vec!["alice", "bob"]);
  }

  #[test]
  fn test_extract_mentions_unknown() {
    let text = "Hello @alice and @unknown!";
    let known = vec!["alice".to_string()];
    let mentions = extract_mentions(text, &known);
    assert_eq!(mentions, vec!["alice"]);
  }

  #[test]
  fn test_extract_mentions_duplicates() {
    let text = "@alice @alice @bob @alice";
    let known = vec!["alice".to_string(), "bob".to_string()];
    let mentions = extract_mentions(text, &known);
    assert_eq!(mentions, vec!["alice", "bob"]);
  }

  #[test]
  fn test_insert_mention_basic() {
    let input = "Hello @ali";
    let result = insert_mention("alice", input, "ali");
    assert_eq!(result, "Hello @alice ");
  }

  #[test]
  fn test_insert_mention_middle() {
    let input = "Hello @ali how are you?";
    let result = insert_mention("alice", input, "ali");
    assert_eq!(result, "Hello @alice  how are you?");
  }
}
