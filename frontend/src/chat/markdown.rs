//! Minimal Markdown renderer with built-in XSS filtering.
//!
//! Implemented by hand (rather than pulling a heavyweight crate) so the
//! WASM bundle size stays small and the allow-list is explicit. Supports:
//!
//! * Bold: `**text**`
//! * Italic: `*text*`
//! * Inline code: `` `text` ``
//! * Fenced code blocks: ```` ```lang\ncode\n``` ````
//! * Autolinks: bare `http(s)://...` URLs are wrapped in `<a>` tags.
//!
//! Everything else is HTML-escaped. The output is always safe to inject
//! via `inner_html`.

/// Render Markdown source to sanitised HTML.
#[must_use]
pub fn render(source: &str) -> String {
  // Fast path: empty input.
  if source.is_empty() {
    return String::new();
  }

  // Split fenced code blocks first so their contents are rendered as
  // plain text (no further markdown is applied inside them).
  let mut out = String::with_capacity(source.len() + 16);
  let mut remainder = source;
  while let Some(idx) = remainder.find("```") {
    let (head, tail) = remainder.split_at(idx);
    out.push_str(&render_inline(head));

    // Skip opening ```
    let after_open = &tail[3..];
    if let Some(close) = after_open.find("```") {
      // Strip optional language tag on the opening fence.
      let mut fenced = &after_open[..close];
      if let Some(nl) = fenced.find('\n') {
        fenced = &fenced[nl + 1..];
      }
      out.push_str("<pre><code>");
      out.push_str(&escape_html(fenced));
      out.push_str("</code></pre>");
      remainder = &after_open[close + 3..];
    } else {
      // Unterminated fence -> treat the remaining text as inline.
      out.push_str(&render_inline(&remainder[idx..]));
      return out;
    }
  }
  out.push_str(&render_inline(remainder));
  out
}

fn render_inline(source: &str) -> String {
  if source.is_empty() {
    return String::new();
  }

  // Strategy: walk the input character-by-character and emit an output
  // string. We handle bold/italic/inline-code/autolinks via a small
  // pushdown automaton; anything unmatched is HTML-escaped.

  let bytes = source.as_bytes();
  let mut out = String::with_capacity(source.len() + 8);
  let mut i = 0;

  while i < bytes.len() {
    let c = bytes[i];
    match c {
      b'`' => {
        // Inline code spans.
        if let Some(end) = find_unescaped(bytes, i + 1, b'`') {
          let inner = &source[i + 1..end];
          out.push_str("<code>");
          out.push_str(&escape_html(inner));
          out.push_str("</code>");
          i = end + 1;
          continue;
        }
      }
      b'*' => {
        // `**` -> bold; `*` -> italic.
        let is_double = bytes.get(i + 1) == Some(&b'*');
        if is_double {
          if let Some(end) = find_pattern(bytes, i + 2, b"**") {
            let inner = &source[i + 2..end];
            out.push_str("<strong>");
            out.push_str(&render_inline(inner));
            out.push_str("</strong>");
            i = end + 2;
            continue;
          }
        } else if let Some(end) = find_unescaped(bytes, i + 1, b'*') {
          let inner = &source[i + 1..end];
          out.push_str("<em>");
          out.push_str(&render_inline(inner));
          out.push_str("</em>");
          i = end + 1;
          continue;
        }
      }
      b'h' | b'H' if bytes[i..].starts_with(b"http://") || bytes[i..].starts_with(b"https://") => {
        let end = scan_url(bytes, i);
        let url = &source[i..end];
        out.push_str("<a href=\"");
        out.push_str(&escape_attr(url));
        out.push_str("\" target=\"_blank\" rel=\"noopener noreferrer\">");
        out.push_str(&escape_html(url));
        out.push_str("</a>");
        i = end;
        continue;
      }
      b'\n' => {
        out.push_str("<br />");
        i += 1;
        continue;
      }
      _ => {}
    }
    // Default: escape the next UTF-8 character.
    let ch_len = utf8_char_len(bytes, i);
    let slice = &source[i..i + ch_len];
    out.push_str(&escape_html(slice));
    i += ch_len;
  }
  out
}

fn utf8_char_len(bytes: &[u8], idx: usize) -> usize {
  let b = bytes[idx];
  // ASCII (b < 0x80) or UTF-8 continuation byte (0x80..=0xBF) both
  // occupy a single byte from the indexer's point of view.
  if b < 0xC0 {
    1
  } else if b < 0xE0 {
    2
  } else if b < 0xF0 {
    3
  } else {
    4
  }
  .min(bytes.len() - idx)
}

fn find_unescaped(bytes: &[u8], from: usize, needle: u8) -> Option<usize> {
  let mut i = from;
  while i < bytes.len() {
    if bytes[i] == b'\\' && i + 1 < bytes.len() {
      i += 2;
      continue;
    }
    if bytes[i] == needle {
      return Some(i);
    }
    i += 1;
  }
  None
}

fn find_pattern(bytes: &[u8], from: usize, needle: &[u8]) -> Option<usize> {
  if needle.is_empty() || from > bytes.len() {
    return None;
  }
  let mut i = from;
  while i + needle.len() <= bytes.len() {
    if &bytes[i..i + needle.len()] == needle {
      return Some(i);
    }
    i += 1;
  }
  None
}

fn scan_url(bytes: &[u8], from: usize) -> usize {
  let mut j = from;
  while j < bytes.len() {
    let b = bytes[j];
    // Stop on whitespace or common trailing punctuation.
    if b == b' ' || b == b'\n' || b == b'\t' || b == b')' || b == b'"' || b == b'<' || b == b'>' {
      break;
    }
    j += 1;
  }
  // Trim trailing punctuation that is usually not part of the URL.
  while j > from {
    let last = bytes[j - 1];
    if last == b'.' || last == b',' || last == b';' || last == b':' || last == b'!' || last == b'?'
    {
      j -= 1;
    } else {
      break;
    }
  }
  j
}

/// Escape HTML special characters so user input cannot inject tags or
/// attributes.
#[must_use]
pub fn escape_html(input: &str) -> String {
  let mut out = String::with_capacity(input.len());
  for c in input.chars() {
    match c {
      '&' => out.push_str("&amp;"),
      '<' => out.push_str("&lt;"),
      '>' => out.push_str("&gt;"),
      '"' => out.push_str("&quot;"),
      '\'' => out.push_str("&#39;"),
      _ => out.push(c),
    }
  }
  out
}

/// Escape a string for inclusion inside an HTML attribute value.
fn escape_attr(input: &str) -> String {
  // Attributes share the same special characters; we just reuse the HTML
  // escaper so quotes become `&quot;` and break out of the enclosing
  // attribute impossible.
  escape_html(input)
}

/// Remove all Markdown formatting and return a plain-text preview.
#[must_use]
pub fn to_plain_text(source: &str) -> String {
  let mut out = String::with_capacity(source.len());
  let bytes = source.as_bytes();
  let mut i = 0;
  while i < bytes.len() {
    let c = bytes[i];
    match c {
      b'*' | b'`' | b'\\' | b'#' | b'>' => {
        i += 1;
      }
      b'\n' => {
        out.push(' ');
        i += 1;
      }
      _ => {
        let len = utf8_char_len(bytes, i);
        out.push_str(&source[i..i + len]);
        i += len;
      }
    }
  }
  out.trim().to_string()
}

#[cfg(test)]
mod tests;
