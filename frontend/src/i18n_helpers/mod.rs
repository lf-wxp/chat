//! Internationalization helper functions.
//!
//! Provides browser locale detection and persistence utilities
//! that complement the auto-generated `crate::i18n` module.

use crate::i18n::Locale;

/// Detect preferred locale from browser environment.
///
/// Detection order:
/// 1. localStorage `settings_locale` key (legacy `locale` key fallback)
/// 2. Browser `navigator.language`
/// 3. Default: English
pub fn detect_browser_locale() -> Locale {
  if let Some(window) = web_sys::window() {
    // 1. Check localStorage (new key first, legacy fallback).
    if let Ok(Some(storage)) = window.local_storage() {
      let stored = storage
        .get_item("settings_locale")
        .ok()
        .flatten()
        .or_else(|| storage.get_item("locale").ok().flatten());
      if let Some(locale_str) = stored
        && let Some(locale) = parse_locale(&locale_str)
      {
        return locale;
      }
    }

    // 2. Check browser language
    if let Some(lang) = window.navigator().language()
      && let Some(locale) = parse_locale(&lang)
    {
      return locale;
    }
  }

  // 3. Default
  Locale::default()
}

/// Parse a locale string into a `Locale` enum variant.
pub fn parse_locale(s: &str) -> Option<Locale> {
  if s.starts_with("zh") {
    Some(Locale::zh_CN)
  } else if s.starts_with("es") {
    Some(Locale::es)
  } else if s.starts_with("en") {
    Some(Locale::en)
  } else {
    None
  }
}

/// Persist the selected locale to localStorage.
pub fn persist_locale(locale: Locale) {
  if let Some(window) = web_sys::window()
    && let Ok(Some(storage)) = window.local_storage()
  {
    let locale_str = match locale {
      Locale::en => "en",
      Locale::zh_CN => "zh-CN",
      Locale::es => "es",
    };
    let _ = storage.set_item("settings_locale", locale_str);
  }
}

#[cfg(test)]
mod tests;
