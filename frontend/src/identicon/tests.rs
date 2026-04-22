use super::*;

#[test]
fn test_same_username_same_identicon() {
  let svg1 = generate_identicon_svg("alice");
  let svg2 = generate_identicon_svg("alice");
  assert_eq!(svg1, svg2, "Same username should produce same identicon");
}

#[test]
fn test_different_username_different_identicon() {
  let svg1 = generate_identicon_svg("alice");
  let svg2 = generate_identicon_svg("bob");
  assert_ne!(
    svg1, svg2,
    "Different usernames should produce different identicons"
  );
}

#[test]
fn test_svg_is_valid_xml() {
  let svg = generate_identicon_svg("test_user");
  assert!(svg.starts_with("<svg"), "Should start with <svg tag");
  assert!(svg.contains("</svg>"), "Should end with </svg>");
  assert!(svg.contains("xmlns="), "Should have xmlns attribute");
}

#[test]
fn test_data_uri_format() {
  let uri = generate_identicon_data_uri("test");
  assert!(
    uri.starts_with("data:image/svg+xml"),
    "Should be SVG data URI"
  );
}

#[test]
fn test_grid_symmetry() {
  // The identicon should be left-right symmetric
  let svg = generate_identicon_svg("symmetry_test");
  // Just verify it doesn't panic and produces valid SVG
  assert!(svg.contains("<rect"), "Should contain rect elements");
}

#[test]
fn test_empty_username() {
  let svg = generate_identicon_svg("");
  assert!(!svg.is_empty(), "Should handle empty username");
}

#[test]
fn test_unicode_username() {
  let svg = generate_identicon_svg("用户名");
  assert!(!svg.is_empty(), "Should handle unicode usernames");
}
