//! Input sanitization module — XSS protection and basic content filtering.
//!
//! All user-supplied text (room names, descriptions, danmaku, etc.) MUST pass
//! through [`sanitize_user_input`] before being stored or broadcast.

// =============================================================================
// XSS Protection
// =============================================================================

/// Escape HTML special characters to prevent XSS injection.
///
/// Replaces `& < > " '` with their HTML entity equivalents.
pub fn escape_html(text: &str) -> String {
  let mut result = String::with_capacity(text.len());
  for ch in text.chars() {
    match ch {
      '&' => result.push_str("&amp;"),
      '<' => result.push_str("&lt;"),
      '>' => result.push_str("&gt;"),
      '"' => result.push_str("&quot;"),
      '\'' => result.push_str("&#x27;"),
      _ => result.push(ch),
    }
  }
  result
}

/// Strip all HTML tags from the input string.
///
/// This is a lightweight tag stripper — it removes anything between `<` and `>`.
pub fn strip_html_tags(text: &str) -> String {
  let mut result = String::with_capacity(text.len());
  let mut inside_tag = false;
  for ch in text.chars() {
    match ch {
      '<' => inside_tag = true,
      '>' if inside_tag => inside_tag = false,
      _ if !inside_tag => result.push(ch),
      _ => {}
    }
  }
  result
}

// =============================================================================
// Sensitive Word Filter
// =============================================================================

/// Built-in sensitive word list for basic content moderation.
///
/// This is intentionally minimal — production deployments should load a
/// comprehensive word list from configuration or an external file.
const SENSITIVE_WORDS: &[&str] = &[
  // Common profanity / abuse patterns (English)
  "fuck",
  "shit",
  "asshole",
  "bitch",
  "bastard",
  "dick",
  "nigger",
  "faggot",
  // Common profanity / abuse patterns (Chinese)
  "操你妈",
  "草泥马",
  "傻逼",
  "他妈的",
  "妈的",
  "狗日的",
  "混蛋",
  "王八蛋",
  "去死",
  "废物",
  "垃圾",
  "白痴",
  "脑残",
];

/// Check whether the input text contains any sensitive words.
///
/// Returns `true` if at least one sensitive word is found (case-insensitive).
pub fn contains_sensitive_words(text: &str) -> bool {
  let lower = text.to_lowercase();
  SENSITIVE_WORDS.iter().any(|w| lower.contains(w))
}

/// Replace sensitive words with asterisks of the same character length.
///
/// Matching is case-insensitive. Works on character level to correctly handle
/// multi-byte characters (e.g. Chinese).
pub fn filter_sensitive_words(text: &str) -> String {
  // Work entirely in character space to avoid byte-boundary issues with CJK text.
  let chars: Vec<char> = text.chars().collect();
  let lower_chars: Vec<char> = text.to_lowercase().chars().collect();
  let mut masked = chars.clone();

  for &word in SENSITIVE_WORDS {
    let word_chars: Vec<char> = word.to_lowercase().chars().collect();
    if word_chars.is_empty() {
      continue;
    }
    let wlen = word_chars.len();
    let mut i = 0;
    while i + wlen <= lower_chars.len() {
      if lower_chars[i..i + wlen] == word_chars[..] {
        for j in i..i + wlen {
          masked[j] = '*';
        }
        i += wlen;
      } else {
        i += 1;
      }
    }
  }

  masked.into_iter().collect()
}

// =============================================================================
// Length Validation
// =============================================================================

/// Maximum allowed length for room names.
pub const MAX_ROOM_NAME_LEN: usize = 64;

/// Maximum allowed length for room descriptions.
pub const MAX_DESCRIPTION_LEN: usize = 256;

/// Maximum allowed length for danmaku text.
pub const MAX_DANMAKU_LEN: usize = 100;

/// Maximum allowed length for user status signatures.
pub const MAX_SIGNATURE_LEN: usize = 128;

/// Maximum allowed length for invite messages.
pub const MAX_INVITE_MESSAGE_LEN: usize = 200;

/// Truncate a string to at most `max_chars` characters.
///
/// If truncation occurs, an ellipsis (`…`) is appended.
pub fn truncate(text: &str, max_chars: usize) -> String {
  let chars: Vec<char> = text.chars().collect();
  if chars.len() <= max_chars {
    return text.to_string();
  }
  let mut truncated: String = chars[..max_chars.saturating_sub(1)].iter().collect();
  truncated.push('…');
  truncated
}

// =============================================================================
// Unified Sanitization API
// =============================================================================

/// Sanitization result with optional warning.
#[derive(Debug, Clone)]
pub struct SanitizeResult {
  /// The sanitized text, safe for storage and broadcast.
  pub text: String,
  /// If `true`, the original text contained sensitive words that were filtered.
  pub had_sensitive_content: bool,
}

/// Sanitize user input: strip HTML tags, filter sensitive words, and truncate.
///
/// This is the primary entry point for all user-supplied text on the server.
///
/// # Arguments
/// * `text` — raw user input
/// * `max_len` — maximum character length (0 = no limit)
pub fn sanitize_user_input(text: &str, max_len: usize) -> SanitizeResult {
  // Step 1: Strip HTML tags (prevents stored XSS)
  let stripped = strip_html_tags(text);

  // Step 2: Trim whitespace
  let trimmed = stripped.trim();

  // Step 3: Truncate if needed
  let truncated = if max_len > 0 {
    truncate(trimmed, max_len)
  } else {
    trimmed.to_string()
  };

  // Step 4: Filter sensitive words
  let had_sensitive_content = contains_sensitive_words(&truncated);
  let filtered = if had_sensitive_content {
    filter_sensitive_words(&truncated)
  } else {
    truncated
  };

  SanitizeResult {
    text: filtered,
    had_sensitive_content,
  }
}

/// Convenience wrapper: sanitize a room name.
pub fn sanitize_room_name(name: &str) -> SanitizeResult {
  sanitize_user_input(name, MAX_ROOM_NAME_LEN)
}

/// Convenience wrapper: sanitize a room description.
pub fn sanitize_description(desc: &str) -> SanitizeResult {
  sanitize_user_input(desc, MAX_DESCRIPTION_LEN)
}

/// Convenience wrapper: sanitize danmaku text.
pub fn sanitize_danmaku(text: &str) -> SanitizeResult {
  sanitize_user_input(text, MAX_DANMAKU_LEN)
}

/// Convenience wrapper: sanitize an invite message.
pub fn sanitize_invite_message(msg: &str) -> SanitizeResult {
  sanitize_user_input(msg, MAX_INVITE_MESSAGE_LEN)
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
  use super::*;

  // ---- XSS ----

  #[test]
  fn test_escape_html_special_chars() {
    assert_eq!(
      escape_html(r#"<script>alert("xss")&'</script>"#),
      "&lt;script&gt;alert(&quot;xss&quot;)&amp;&#x27;&lt;/script&gt;"
    );
  }

  #[test]
  fn test_escape_html_plain_text() {
    assert_eq!(escape_html("hello world"), "hello world");
  }

  #[test]
  fn test_strip_html_tags() {
    assert_eq!(
      strip_html_tags("<b>bold</b> and <script>evil()</script>"),
      "bold and evil()"
    );
  }

  #[test]
  fn test_strip_html_tags_no_tags() {
    assert_eq!(strip_html_tags("no tags here"), "no tags here");
  }

  // ---- Sensitive words ----

  #[test]
  fn test_contains_sensitive_words_positive() {
    assert!(contains_sensitive_words("you are a Bastard"));
  }

  #[test]
  fn test_contains_sensitive_words_negative() {
    assert!(!contains_sensitive_words("hello world"));
  }

  #[test]
  fn test_contains_sensitive_words_chinese() {
    assert!(contains_sensitive_words("你真是个傻逼"));
  }

  #[test]
  fn test_filter_sensitive_words_replaces() {
    let result = filter_sensitive_words("you are a bastard");
    assert!(!result.contains("bastard"));
    assert!(result.contains("*******"));
  }

  #[test]
  fn test_filter_sensitive_words_clean_text() {
    let input = "hello world";
    assert_eq!(filter_sensitive_words(input), input);
  }

  // ---- Truncation ----

  #[test]
  fn test_truncate_short_text() {
    assert_eq!(truncate("hello", 10), "hello");
  }

  #[test]
  fn test_truncate_long_text() {
    let result = truncate("this is a very long text", 10);
    assert!(result.len() <= 30); // byte length may vary with ellipsis
    assert!(result.ends_with('…'));
  }

  #[test]
  fn test_truncate_chinese() {
    let result = truncate("这是一段很长的中文文本需要截断", 5);
    assert!(result.ends_with('…'));
    assert!(result.chars().count() <= 5);
  }

  // ---- Unified API ----

  #[test]
  fn test_sanitize_user_input_strips_html() {
    let result = sanitize_user_input("<b>room</b>", 64);
    assert_eq!(result.text, "room");
    assert!(!result.had_sensitive_content);
  }

  #[test]
  fn test_sanitize_user_input_filters_profanity() {
    let result = sanitize_user_input("you bastard", 64);
    assert!(result.had_sensitive_content);
    assert!(!result.text.contains("bastard"));
  }

  #[test]
  fn test_sanitize_user_input_truncates() {
    let result = sanitize_user_input("a]".repeat(100).as_str(), 10);
    assert!(result.text.chars().count() <= 10);
  }

  #[test]
  fn test_sanitize_room_name() {
    let result = sanitize_room_name("<script>alert('xss')</script>My Room");
    assert_eq!(result.text, "alert('xss')My Room");
    assert!(!result.had_sensitive_content);
  }

  #[test]
  fn test_sanitize_danmaku_with_profanity() {
    let result = sanitize_danmaku("傻逼弹幕");
    assert!(result.had_sensitive_content);
    assert!(!result.text.contains("傻逼"));
  }
}
