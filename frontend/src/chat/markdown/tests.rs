use super::*;

#[test]
fn escape_html_escapes_angle_brackets() {
  assert_eq!(escape_html("<script>"), "&lt;script&gt;");
  assert_eq!(escape_html("a & b"), "a &amp; b");
}

#[test]
fn bold_and_italic() {
  assert_eq!(render("**bold**"), "<strong>bold</strong>");
  assert_eq!(render("*em*"), "<em>em</em>");
  assert_eq!(render("**a *b* c**"), "<strong>a <em>b</em> c</strong>");
}

#[test]
fn inline_code_is_escaped() {
  assert_eq!(render("`<x>`"), "<code>&lt;x&gt;</code>");
}

#[test]
fn fenced_code_block_escaped() {
  let html = render("```rust\nfn <T>() {}\n```");
  assert!(html.starts_with("<pre><code>"));
  assert!(html.contains("fn &lt;T&gt;()"));
  assert!(html.ends_with("</code></pre>"));
}

#[test]
fn autolink_wraps_url() {
  let html = render("see https://example.com/a?x=1 next");
  assert!(html.contains("<a href=\"https://example.com/a?x=1\""));
  assert!(html.contains("rel=\"noopener noreferrer\""));
}

#[test]
fn autolink_trims_trailing_punctuation() {
  let html = render("visit https://x.com.");
  assert!(html.contains("<a href=\"https://x.com\""));
  assert!(html.ends_with("."));
}

#[test]
fn xss_injection_blocked() {
  let html = render("<img src=x onerror=alert(1)>");
  assert!(!html.contains("<img"));
  assert!(html.contains("&lt;img"));
}

#[test]
fn plain_text_strips_markers() {
  assert_eq!(to_plain_text("**hello**"), "hello");
  assert_eq!(to_plain_text("a\nb"), "a b");
}
