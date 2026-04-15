//! Tests for CSS composes expansion logic.

use super::*;
use std::fs;

#[test]
fn test_expand_simple_composes() {
  let input = r#".btn-base {
  display: inline-flex;
  padding: 8px 16px;
  border-radius: 4px;
}

.btn-primary {
  composes: btn-base;
  background: blue;
  color: white;
}
"#;

  let output = expand_composes(input);

  // Should not contain `composes:`
  assert!(!output.contains("composes:"), "composes should be removed");

  // Should contain the inlined declarations from btn-base inside btn-primary
  assert!(output.contains("display: inline-flex;"), "Should inline display from btn-base");
  assert!(output.contains("padding: 8px 16px;"), "Should inline padding from btn-base");
  assert!(output.contains("border-radius: 4px;"), "Should inline border-radius from btn-base");

  // Should still contain btn-primary's own declarations
  assert!(output.contains("background: blue;"), "Should keep own background");
  assert!(output.contains("color: white;"), "Should keep own color");
}

#[test]
fn test_expand_multiple_composes() {
  let input = r#".base-layout {
  display: flex;
  align-items: center;
}

.base-spacing {
  padding: 8px;
  margin: 4px;
}

.card {
  composes: base-layout base-spacing;
  background: white;
}
"#;

  let output = expand_composes(input);

  assert!(!output.contains("composes:"));
  // Should have declarations from both composed classes
  assert!(output.contains("display: flex;"));
  assert!(output.contains("align-items: center;"));
  assert!(output.contains("padding: 8px;"));
  assert!(output.contains("margin: 4px;"));
  assert!(output.contains("background: white;"));
}

#[test]
fn test_no_composes_passthrough() {
  let input = r#".simple {
  color: red;
  font-size: 14px;
}
"#;

  let output = expand_composes(input);

  // Should be unchanged (minus any whitespace normalization)
  assert!(output.contains("color: red;"));
  assert!(output.contains("font-size: 14px;"));
  assert!(!output.contains("composes"));
}

#[test]
fn test_composes_does_not_inline_nested_rules() {
  let input = r#".btn-base {
  display: inline-flex;
  padding: 8px;

  &:hover {
    opacity: 0.8;
  }
}

.btn-primary {
  composes: btn-base;
  background: blue;
}
"#;

  let output = expand_composes(input);

  // Should inline only top-level declarations, not nested &:hover block
  assert!(output.contains("display: inline-flex;"));
  assert!(output.contains("padding: 8px;"));
  assert!(output.contains("background: blue;"));
  // The &:hover block should NOT be inlined into btn-primary
  // (it stays in btn-base only)
}

#[test]
fn test_composes_chain() {
  let input = r#".a {
  color: red;
}

.b {
  composes: a;
  font-size: 14px;
}

.c {
  composes: b;
  margin: 10px;
}
"#;

  let output = expand_composes(input);

  assert!(!output.contains("composes:"));
  // .c should get .b's declarations (font-size), but .b's composes
  // was already expanded, so .c gets font-size from .b's extracted declarations
  assert!(output.contains("margin: 10px;"));
}

#[test]
fn test_extract_class_declarations() {
  let css = r#".foo {
  color: red;
  font-size: 14px;

  &:hover {
    opacity: 0.5;
  }
}

.bar {
  background: blue;
}
"#;

  let map = extract_class_declarations(css);

  assert!(map.contains_key("foo"));
  let foo_decls = &map["foo"];
  assert!(foo_decls.iter().any(|d| d.contains("color: red")));
  assert!(foo_decls.iter().any(|d| d.contains("font-size: 14px")));
  // Should NOT contain the nested &:hover block
  assert!(!foo_decls.iter().any(|d| d.contains("opacity")));

  assert!(map.contains_key("bar"));
  let bar_decls = &map["bar"];
  assert!(bar_decls.iter().any(|d| d.contains("background: blue")));
}

#[test]
fn test_parse_top_level_declarations() {
  let block = r#"
  color: red;
  font-size: 14px;

  &:hover {
    opacity: 0.5;
  }

  padding: 8px;
"#;

  let decls = parse_top_level_declarations(block);

  assert_eq!(decls.len(), 3);
  assert!(decls[0].contains("color: red"));
  assert!(decls[1].contains("font-size: 14px"));
  assert!(decls[2].contains("padding: 8px"));
}

#[test]
fn test_collect_css_files() {
  let dir = tempfile::tempdir().unwrap();
  let css_path = dir.path().join("test.css");
  fs::write(&css_path, ".foo { color: red; }").unwrap();

  let sub_dir = dir.path().join("components");
  fs::create_dir(&sub_dir).unwrap();
  let sub_css = sub_dir.join("button.css");
  fs::write(&sub_css, ".btn { padding: 8px; }").unwrap();

  // Non-CSS file should be ignored
  let txt_path = dir.path().join("readme.txt");
  fs::write(&txt_path, "not css").unwrap();

  let files = collect_css_files(dir.path()).unwrap();
  assert_eq!(files.len(), 2);
  assert!(files.iter().any(|f| f.ends_with("test.css")));
  assert!(files.iter().any(|f| f.ends_with("button.css")));
}

#[test]
fn test_composes_with_comments() {
  let input = r#"/* Base button styles */
.btn-base {
  display: inline-flex;
  padding: 8px;
}

/* Primary variant */
.btn-primary {
  composes: btn-base;
  background: blue;
}
"#;

  let output = expand_composes(input);
  assert!(!output.contains("composes:"));
  assert!(output.contains("display: inline-flex;"));
  assert!(output.contains("background: blue;"));
}

#[test]
fn test_composes_unknown_class_ignored() {
  let input = r#".btn-primary {
  composes: nonexistent;
  background: blue;
}
"#;

  let output = expand_composes(input);
  // Should not crash, just skip the unknown class
  assert!(!output.contains("composes:"));
  assert!(output.contains("background: blue;"));
}
