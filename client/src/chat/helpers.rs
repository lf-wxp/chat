//! Chat helper functions
//!
//! Utility functions for base64 encoding, file icon mapping, etc.

use std::fmt::Write;

/// Simple base64 encoding (used for thumbnail display)
pub(super) fn base64_encode(data: &[u8]) -> String {
  const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
  let mut result = String::new();
  let mut i = 0;
  while i < data.len() {
    let b0 = data[i] as u32;
    let b1 = if i + 1 < data.len() {
      data[i + 1] as u32
    } else {
      0
    };
    let b2 = if i + 2 < data.len() {
      data[i + 2] as u32
    } else {
      0
    };

    result.push(CHARS[((b0 >> 2) & 0x3F) as usize] as char);
    result.push(CHARS[(((b0 & 0x3) << 4) | ((b1 >> 4) & 0xF)) as usize] as char);

    if i + 1 < data.len() {
      result.push(CHARS[(((b1 & 0xF) << 2) | ((b2 >> 6) & 0x3)) as usize] as char);
    } else {
      result.push('=');
    }

    if i + 2 < data.len() {
      result.push(CHARS[(b2 & 0x3F) as usize] as char);
    } else {
      result.push('=');
    }

    i += 3;
  }
  result
}

/// Get file icon emoji by MIME type
pub(super) fn get_file_icon(mime_type: &str) -> &'static str {
  if mime_type.starts_with("image/") {
    "🖼️"
  } else if mime_type.starts_with("video/") {
    "🎬"
  } else if mime_type.starts_with("audio/") {
    "🎵"
  } else if mime_type.contains("pdf") {
    "📄"
  } else if mime_type.contains("zip") || mime_type.contains("rar") || mime_type.contains("tar") {
    "📦"
  } else if mime_type.contains("text") {
    "📝"
  } else {
    "📎"
  }
}

// =============================================================================
// Markdown Rendering
// =============================================================================

/// Escape HTML special characters (XSS prevention)
fn escape_html(text: &str) -> String {
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

/// Convert URLs in text to clickable links
fn linkify(text: &str) -> String {
  let mut result = String::new();
  let mut remaining = text;

  while !remaining.is_empty() {
    // Find URLs starting with http:// or https://
    if let Some(start) = remaining
      .find("http://")
      .or_else(|| remaining.find("https://"))
    {
      // Add text before the URL
      result.push_str(&remaining[..start]);

      let url_part = &remaining[start..];
      // Find URL end position (whitespace or end of line)
      let end = url_part
        .find(|c: char| c.is_whitespace() || c == '"' || c == '\'' || c == ')' || c == ']')
        .unwrap_or(url_part.len());

      let url = &url_part[..end];
      // Strip trailing punctuation
      let url = url.trim_end_matches(['.', ',', ';', '!', '?']);

      let _ = write!(
        result,
        r#"<a href="{url}" target="_blank" rel="noopener noreferrer" class="chat-link">{url}</a>"#
      );

      let consumed = start + url.len();
      remaining = &remaining[consumed..];
    } else {
      result.push_str(remaining);
      break;
    }
  }

  result
}

/// Parse inline Markdown syntax (bold, italic, inline code, strikethrough)
fn parse_inline_markdown(text: &str) -> String {
  let mut result = String::new();
  let chars: Vec<char> = text.chars().collect();
  let len = chars.len();
  let mut i = 0;

  while i < len {
    // Inline code `code`
    if chars[i] == '`'
      && let Some(end) = find_closing(&chars, i + 1, '`')
    {
      let code_text: String = chars[i + 1..end].iter().collect();
      let _ = write!(result, r#"<code class="inline-code">{code_text}</code>"#);
      i = end + 1;
      continue;
    }

    // Bold **text**
    if i + 1 < len
      && chars[i] == '*'
      && chars[i + 1] == '*'
      && let Some(end) = find_closing_pair(&chars, i + 2, '*', '*')
    {
      let inner: String = chars[i + 2..end].iter().collect();
      let _ = write!(result, "<strong>{}</strong>", parse_inline_markdown(&inner));
      i = end + 2;
      continue;
    }

    // Italic *text*
    if chars[i] == '*'
      && i + 1 < len
      && chars[i + 1] != '*'
      && let Some(end) = find_closing(&chars, i + 1, '*')
    {
      let inner: String = chars[i + 1..end].iter().collect();
      let _ = write!(result, "<em>{}</em>", parse_inline_markdown(&inner));
      i = end + 1;
      continue;
    }

    // Strikethrough ~~text~~
    if i + 1 < len
      && chars[i] == '~'
      && chars[i + 1] == '~'
      && let Some(end) = find_closing_pair(&chars, i + 2, '~', '~')
    {
      let inner: String = chars[i + 2..end].iter().collect();
      let _ = write!(result, "<del>{}</del>", parse_inline_markdown(&inner));
      i = end + 2;
      continue;
    }

    result.push(chars[i]);
    i += 1;
  }

  result
}

/// Find position of single-character closing marker
fn find_closing(chars: &[char], start: usize, marker: char) -> Option<usize> {
  chars
    .iter()
    .position(|&c| c == marker)
    .and_then(|pos| {
      let idx = pos;
      if idx >= start { Some(idx) } else { None }
    })
    .or_else(|| {
      chars[start..]
        .iter()
        .position(|&c| c == marker)
        .map(|pos| pos + start)
    })
}

/// Find position of double-character closing marker (e.g. ** or ~~)
fn find_closing_pair(chars: &[char], start: usize, c1: char, c2: char) -> Option<usize> {
  let mut i = start;
  while i + 1 < chars.len() {
    if chars[i] == c1 && chars[i + 1] == c2 {
      return Some(i);
    }
    i += 1;
  }
  None
}

/// Render `@username` as highlighted mention tags
///
/// `known_names` is the list of known usernames; only matching @username will be highlighted.
/// If `known_names` is empty, all `@word` patterns will be highlighted.
fn render_mentions(text: &str, known_names: &[String]) -> String {
  let mut result = String::new();
  let mut remaining = text;

  while let Some(at_pos) = remaining.find('@') {
    // Add text before @
    result.push_str(&remaining[..at_pos]);

    let after_at = &remaining[at_pos + 1..];
    // Extract username after @ (alphanumeric, underscore, hyphen)
    let name_end = after_at
      .find(|c: char| !c.is_alphanumeric() && c != '_' && c != '-')
      .unwrap_or(after_at.len());

    if name_end == 0 {
      // No valid characters after @, output as-is
      result.push('@');
      remaining = after_at;
      continue;
    }

    let name = &after_at[..name_end];

    // Check if it's a known username (highlight all if list is empty)
    let should_highlight = known_names.is_empty() || known_names.iter().any(|n| n == name);

    if should_highlight {
      let _ = write!(
        result,
        r#"<span class="mention" data-username="{name}">@{name}</span>"#
      );
    } else {
      result.push('@');
      result.push_str(name);
    }

    remaining = &after_at[name_end..];
  }

  result.push_str(remaining);
  result
}

/// Lightweight Markdown renderer
///
/// Supported syntax:
/// - `**bold**`
/// - `*italic*`
/// - `` `inline code` ``
/// - ` ```code block``` `
/// - `~~strikethrough~~`
/// - `@username` mention highlighting
/// - Auto-detect URLs as clickable links
/// - Newlines converted to `<br>`
///
/// All input is HTML-escaped first to prevent XSS.
/// The `mentions` parameter is the list of @-mentioned usernames for precise highlighting.
pub(super) fn render_markdown(text: &str) -> String {
  render_markdown_with_mentions(text, &[])
}

/// Markdown rendering with mention list
pub(super) fn render_markdown_with_mentions(text: &str, mention_names: &[String]) -> String {
  let escaped = escape_html(text);
  let mut html = String::new();
  let mut in_code_block = false;
  let mut code_block_content = String::new();

  for line in escaped.split('\n') {
    let trimmed = line.trim();

    // Code block start/end ```
    if trimmed.starts_with("```") {
      if in_code_block {
        // End code block
        let _ = write!(
          html,
          r#"<pre class="code-block"><code>{}</code></pre>"#,
          code_block_content.trim_end()
        );
        code_block_content.clear();
        in_code_block = false;
      } else {
        // Start code block
        in_code_block = true;
      }
      continue;
    }

    if in_code_block {
      if !code_block_content.is_empty() {
        code_block_content.push('\n');
      }
      code_block_content.push_str(line);
      continue;
    }

    // Normal line: parse inline Markdown, then auto-link URLs, then render @mentions
    let parsed = parse_inline_markdown(line);
    let linked = linkify(&parsed);
    let mentioned = render_mentions(&linked, mention_names);

    if !html.is_empty() {
      html.push_str("<br>");
    }
    html.push_str(&mentioned);
  }

  // Output unclosed code block content anyway
  if in_code_block && !code_block_content.is_empty() {
    let _ = write!(
      html,
      r#"<pre class="code-block"><code>{}</code></pre>"#,
      code_block_content.trim_end()
    );
  }

  html
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
  use super::*;
  use wasm_bindgen_test::wasm_bindgen_test;

  wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

  // =========================================================================
  // base64 encoding tests
  // =========================================================================

  #[wasm_bindgen_test]
  fn test_base64_encode_empty() {
    assert_eq!(base64_encode(b""), "");
  }

  #[wasm_bindgen_test]
  fn test_base64_encode_single_byte() {
    // 'A' = 0x41 -> base64 "QQ=="
    assert_eq!(base64_encode(b"A"), "QQ==");
  }

  #[wasm_bindgen_test]
  fn test_base64_encode_two_bytes() {
    // "AB" -> base64 "QUI="
    assert_eq!(base64_encode(b"AB"), "QUI=");
  }

  #[wasm_bindgen_test]
  fn test_base64_encode_three_bytes() {
    // "ABC" -> base64 "QUJD" (no padding)
    assert_eq!(base64_encode(b"ABC"), "QUJD");
  }

  #[wasm_bindgen_test]
  fn test_base64_encode_hello_world() {
    assert_eq!(base64_encode(b"Hello, World!"), "SGVsbG8sIFdvcmxkIQ==");
  }

  #[wasm_bindgen_test]
  fn test_base64_encode_binary_data() {
    let data: Vec<u8> = (0..=255).collect();
    let encoded = base64_encode(&data);
    // 256 bytes -> ceil(256/3)*4 = 344 characters
    assert_eq!(encoded.len(), 344);
    // Should not contain non-base64 characters
    assert!(
      encoded
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '=')
    );
  }

  // =========================================================================
  // File icon tests
  // =========================================================================

  #[wasm_bindgen_test]
  fn test_get_file_icon_image() {
    assert_eq!(get_file_icon("image/png"), "🖼️");
    assert_eq!(get_file_icon("image/jpeg"), "🖼️");
    assert_eq!(get_file_icon("image/webp"), "🖼️");
  }

  #[wasm_bindgen_test]
  fn test_get_file_icon_video() {
    assert_eq!(get_file_icon("video/mp4"), "🎬");
  }

  #[wasm_bindgen_test]
  fn test_get_file_icon_audio() {
    assert_eq!(get_file_icon("audio/mpeg"), "🎵");
  }

  #[wasm_bindgen_test]
  fn test_get_file_icon_pdf() {
    assert_eq!(get_file_icon("application/pdf"), "📄");
  }

  #[wasm_bindgen_test]
  fn test_get_file_icon_archive() {
    assert_eq!(get_file_icon("application/zip"), "📦");
    assert_eq!(get_file_icon("application/x-rar"), "📦");
    assert_eq!(get_file_icon("application/x-tar"), "📦");
  }

  #[wasm_bindgen_test]
  fn test_get_file_icon_text() {
    assert_eq!(get_file_icon("text/plain"), "📝");
  }

  #[wasm_bindgen_test]
  fn test_get_file_icon_unknown() {
    assert_eq!(get_file_icon("application/octet-stream"), "📎");
  }

  // =========================================================================
  // HTML escaping tests
  // =========================================================================

  #[wasm_bindgen_test]
  fn test_escape_html_no_special_chars() {
    assert_eq!(escape_html("hello world"), "hello world");
  }

  #[wasm_bindgen_test]
  fn test_escape_html_all_special_chars() {
    assert_eq!(
      escape_html(r#"<script>alert("xss")&'</script>"#),
      "&lt;script&gt;alert(&quot;xss&quot;)&amp;&#x27;&lt;/script&gt;"
    );
  }

  #[wasm_bindgen_test]
  fn test_escape_html_empty() {
    assert_eq!(escape_html(""), "");
  }

  #[wasm_bindgen_test]
  fn test_escape_html_chinese() {
    assert_eq!(escape_html("你好<世界>"), "你好&lt;世界&gt;");
  }

  // =========================================================================
  // URL linkification tests
  // =========================================================================

  #[wasm_bindgen_test]
  fn test_linkify_no_url() {
    assert_eq!(linkify("hello world"), "hello world");
  }

  #[wasm_bindgen_test]
  fn test_linkify_http_url() {
    let result = linkify("visit http://example.com today");
    assert!(result.contains(r#"<a href="http://example.com""#));
    assert!(result.contains("target=\"_blank\""));
  }

  #[wasm_bindgen_test]
  fn test_linkify_https_url() {
    let result = linkify("check https://example.com/path?q=1");
    assert!(result.contains(r#"<a href="https://example.com/path?q=1""#));
  }

  #[wasm_bindgen_test]
  fn test_linkify_url_with_trailing_punctuation() {
    let result = linkify("see https://example.com.");
    // Trailing period should not be included in URL
    assert!(result.contains(r#"<a href="https://example.com""#));
  }

  #[wasm_bindgen_test]
  fn test_linkify_multiple_urls() {
    let result = linkify("http://a.com and https://b.com");
    assert!(result.contains("http://a.com"));
    assert!(result.contains("https://b.com"));
  }

  // =========================================================================
  // Inline Markdown tests
  // =========================================================================

  #[wasm_bindgen_test]
  fn test_inline_code() {
    let result = parse_inline_markdown("use `println!` macro");
    assert!(result.contains("<code class=\"inline-code\">println!</code>"));
  }

  #[wasm_bindgen_test]
  fn test_bold() {
    let result = parse_inline_markdown("this is **bold** text");
    assert!(result.contains("<strong>bold</strong>"));
  }

  #[wasm_bindgen_test]
  fn test_italic() {
    let result = parse_inline_markdown("this is *italic* text");
    assert!(result.contains("<em>italic</em>"));
  }

  #[wasm_bindgen_test]
  fn test_strikethrough() {
    let result = parse_inline_markdown("this is ~~deleted~~ text");
    assert!(result.contains("<del>deleted</del>"));
  }

  #[wasm_bindgen_test]
  fn test_nested_bold_italic() {
    // Nested bold and italic
    let result = parse_inline_markdown("**bold** and *italic*");
    assert!(result.contains("<strong>bold</strong>"));
    assert!(result.contains("<em>italic</em>"));
  }

  #[wasm_bindgen_test]
  fn test_plain_text_unchanged() {
    assert_eq!(parse_inline_markdown("hello world"), "hello world");
  }

  // =========================================================================
  // @mention rendering tests
  // =========================================================================

  #[wasm_bindgen_test]
  fn test_render_mentions_known_user() {
    let result = render_mentions("hello @alice", &["alice".to_string()]);
    assert!(result.contains(r#"<span class="mention" data-username="alice">@alice</span>"#));
  }

  #[wasm_bindgen_test]
  fn test_render_mentions_unknown_user() {
    let result = render_mentions("hello @bob", &["alice".to_string()]);
    // bob is not in the known list, should not be highlighted
    assert!(!result.contains("class=\"mention\""));
    assert!(result.contains("@bob"));
  }

  #[wasm_bindgen_test]
  fn test_render_mentions_empty_known_list() {
    // When list is empty, all @word patterns are highlighted
    let result = render_mentions("hello @anyone", &[]);
    assert!(result.contains(r#"<span class="mention""#));
  }

  #[wasm_bindgen_test]
  fn test_render_mentions_at_only() {
    // Standalone @ should not trigger highlighting
    let result = render_mentions("email: user@ domain", &[]);
    assert!(!result.contains("class=\"mention\""));
  }

  #[wasm_bindgen_test]
  fn test_render_mentions_multiple() {
    let result = render_mentions("@alice and @bob", &["alice".to_string(), "bob".to_string()]);
    assert!(result.contains("@alice</span>"));
    assert!(result.contains("@bob</span>"));
  }

  // =========================================================================
  // Full Markdown rendering tests
  // =========================================================================

  #[wasm_bindgen_test]
  fn test_render_markdown_plain_text() {
    let result = render_markdown("hello world");
    assert_eq!(result, "hello world");
  }

  #[wasm_bindgen_test]
  fn test_render_markdown_xss_prevention() {
    let result = render_markdown("<script>alert('xss')</script>");
    assert!(!result.contains("<script>"));
    assert!(result.contains("&lt;script&gt;"));
  }

  #[wasm_bindgen_test]
  fn test_render_markdown_code_block() {
    let input = "```\nlet x = 1;\nlet y = 2;\n```";
    let result = render_markdown(input);
    assert!(result.contains("<pre class=\"code-block\">"));
    assert!(result.contains("<code>"));
    assert!(result.contains("let x = 1;"));
  }

  #[wasm_bindgen_test]
  fn test_render_markdown_multiline() {
    let result = render_markdown("line1\nline2\nline3");
    assert!(result.contains("<br>"));
  }

  #[wasm_bindgen_test]
  fn test_render_markdown_with_mentions() {
    let result = render_markdown_with_mentions("hello @alice", &["alice".to_string()]);
    assert!(result.contains("class=\"mention\""));
  }

  #[wasm_bindgen_test]
  fn test_render_markdown_unclosed_code_block() {
    let input = "```\nunclosed code";
    let result = render_markdown(input);
    // Unclosed code blocks should still be rendered
    assert!(result.contains("<pre class=\"code-block\">"));
    assert!(result.contains("unclosed code"));
  }

  #[wasm_bindgen_test]
  fn test_render_markdown_url_auto_link() {
    let result = render_markdown("visit https://example.com");
    assert!(result.contains("<a href="));
  }

  // =========================================================================
  // find_closing / find_closing_pair helper function tests
  // =========================================================================

  #[wasm_bindgen_test]
  fn test_find_closing_found() {
    let chars: Vec<char> = "hello`world".chars().collect();
    assert_eq!(find_closing(&chars, 0, '`'), Some(5));
  }

  #[wasm_bindgen_test]
  fn test_find_closing_not_found() {
    let chars: Vec<char> = "hello world".chars().collect();
    assert_eq!(find_closing(&chars, 0, '`'), None);
  }

  #[wasm_bindgen_test]
  fn test_find_closing_pair_found() {
    let chars: Vec<char> = "bold** rest".chars().collect();
    assert_eq!(find_closing_pair(&chars, 0, '*', '*'), Some(4));
  }

  #[wasm_bindgen_test]
  fn test_find_closing_pair_not_found() {
    let chars: Vec<char> = "no closing".chars().collect();
    assert_eq!(find_closing_pair(&chars, 0, '*', '*'), None);
  }
}
