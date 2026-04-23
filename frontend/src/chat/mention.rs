//! Parse `@mention` tokens from chat text.
//!
//! A mention has the shape `@<name>` where `<name>` matches the
//! `[A-Za-z0-9_\-\.\p{L}]+` class (so Unicode nicknames like `@Taro`
//! work). Parsing is purely lexical — matching a mention to an actual
//! `UserId` is the caller's responsibility because the available member
//! set is contextual (direct chat vs. room).

/// Find every `@name` token in `source` and return the list of
/// lexemes (without the leading `@`).
#[must_use]
pub fn extract(source: &str) -> Vec<String> {
  let bytes = source.as_bytes();
  let mut out = Vec::new();
  let mut i = 0;
  while i < bytes.len() {
    if bytes[i] == b'@' {
      // Require the preceding character to be whitespace, CJK
      // punctuation, or the start of input so `email@host` is not
      // treated as a mention.
      if i > 0 {
        // Walk backwards to decode the previous UTF-8 character.
        let prev_str = &bytes[..i];
        if let Ok(s) = std::str::from_utf8(prev_str)
          && let Some(prev_ch) = s.chars().next_back()
          && !is_pre_at_boundary(prev_ch)
        {
          i += 1;
          continue;
        }
      }
      let start = i + 1;
      let mut j = start;
      while j < bytes.len() {
        if is_mention_boundary(bytes, j) {
          break;
        }
        // Advance past the full UTF-8 character so we don't stop
        // mid-character.
        let remaining = &bytes[j..];
        if let Ok(s) = std::str::from_utf8(remaining) {
          let ch_len = s.chars().next().map_or(1, char::len_utf8);
          j += ch_len;
        } else {
          j += 1;
        }
      }
      if j > start {
        // Extract as a proper &str — must cope with multi-byte UTF-8.
        if let Ok(name) = std::str::from_utf8(&bytes[start..j]) {
          out.push(name.to_string());
        }
      }
      i = j;
    } else {
      i += 1;
    }
  }
  out
}

/// Check whether a character is a valid boundary before an `@`
/// mention. This includes ASCII whitespace, CJK punctuation, and
/// common delimiters so that `（@小明）` correctly extracts `小明`.
fn is_pre_at_boundary(ch: char) -> bool {
  ch.is_whitespace() || is_cjk_punctuation(ch)
}

/// Check whether a byte terminates a mention name.
///
/// Includes ASCII whitespace/punctuation and the leading bytes of
/// common CJK punctuation characters (。、，！？；：""''（）【】《》)
/// so that `@小明。` stops at the full-width period and `@小明，你好`
/// stops before the comma.
///
/// For multi-byte UTF-8 sequences (CJK characters), we peek at the
/// full codepoint to distinguish CJK punctuation (which terminates)
/// from CJK ideographs / letters (which are valid in names).
fn is_mention_boundary(bytes: &[u8], pos: usize) -> bool {
  let byte = bytes[pos];
  if byte <= 0x7F {
    // ASCII: whitespace and common punctuation terminate the mention.
    return byte == b' '
      || byte == b'\n'
      || byte == b'\t'
      || byte == b','
      || byte == b'.'
      || byte == b'!'
      || byte == b'?'
      || byte == b';'
      || byte == b':'
      || byte == b')'
      || byte == b'('
      || byte == b']'
      || byte == b'[';
  }
  // Multi-byte UTF-8: decode the codepoint and check whether it is
  // CJK punctuation (which terminates the mention) or a CJK
  // ideograph / letter (which is valid in a name).
  let remaining = &bytes[pos..];
  if let Ok(s) = std::str::from_utf8(remaining)
    && let Some(ch) = s.chars().next()
  {
    return is_cjk_punctuation(ch);
  }
  // Invalid UTF-8 — treat as boundary to be safe.
  true
}

/// Whether a Unicode codepoint is a CJK punctuation character that
/// should terminate a mention name.
fn is_cjk_punctuation(ch: char) -> bool {
  matches!(
    ch,
    // CJK-specific punctuation
    '\u{3001}' // 、
      | '\u{3002}' // 。
      | '\u{FF0C}' // ，(fullwidth)
      | '\u{FF01}' // ！(fullwidth)
      | '\u{FF1F}' // ？(fullwidth)
      | '\u{FF1B}' // ；(fullwidth)
      | '\u{FF1A}' // ：(fullwidth)
      | '\u{300C}' // 「
      | '\u{300D}' // 」
      | '\u{300E}' // 『
      | '\u{300F}' // 』
      | '\u{201C}' // "
      | '\u{201D}' // "
      | '\u{2018}' // '
      | '\u{2019}' // '
      | '\u{FF08}' // （(fullwidth)
      | '\u{FF09}' // ）(fullwidth)
      | '\u{3010}' // 【
      | '\u{3011}' // 】
      | '\u{300A}' // 《
      | '\u{300B}' // 》
      | '\u{2026}' // …
      | '\u{2014}' // —
      | '\u{FF0E}' // ．(fullwidth)
  )
}

/// Whether `names` contains a case-insensitive match for `target`.
#[must_use]
pub fn mentions(names: &[String], target: &str) -> bool {
  let target_lc = target.to_lowercase();
  names.iter().any(|n| n.to_lowercase() == target_lc)
}

// ── Tests ──

#[cfg(test)]
mod tests {
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
}
