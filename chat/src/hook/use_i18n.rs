use yew::prelude::*;
use yew_i18n::{use_translation, YewI18n};

use crate::utils::get_window;

#[hook]
pub fn use_i18n() -> YewI18n {
  let binding = get_window()
    .navigator()
    .language()
    .unwrap_or("en".to_string());
  let language = binding.split_at(2).0;
  let mut i18n = use_translation();
  let _ = i18n.set_translation_language(language);
  i18n
}
