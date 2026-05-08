//! Unit tests for the i18n helpers module.

use super::*;

#[test]
fn test_parse_locale_en() {
  assert_eq!(parse_locale("en"), Some(Locale::en));
  assert_eq!(parse_locale("en-US"), Some(Locale::en));
}

#[test]
fn test_parse_locale_zh() {
  assert_eq!(parse_locale("zh"), Some(Locale::zh_CN));
  assert_eq!(parse_locale("zh-CN"), Some(Locale::zh_CN));
  assert_eq!(parse_locale("zh-TW"), Some(Locale::zh_CN));
}

#[test]
fn test_parse_locale_es() {
  assert_eq!(parse_locale("es"), Some(Locale::es));
  assert_eq!(parse_locale("es-ES"), Some(Locale::es));
  assert_eq!(parse_locale("es-MX"), Some(Locale::es));
  assert_eq!(parse_locale("es-AR"), Some(Locale::es));
}

#[test]
fn test_parse_locale_unknown() {
  assert_eq!(parse_locale("fr"), None);
  assert_eq!(parse_locale(""), None);
  assert_eq!(parse_locale("de"), None);
}

#[test]
fn test_locale_equality() {
  assert_eq!(Locale::en, Locale::en);
  assert_ne!(Locale::en, Locale::zh_CN);
  assert_ne!(Locale::en, Locale::es);
  assert_ne!(Locale::zh_CN, Locale::es);
}

#[test]
fn test_default_locale() {
  assert_eq!(Locale::default(), Locale::en);
}
