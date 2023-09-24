use bounce::use_atom_value;
use yew::prelude::*;

use crate::{
  store::Theme,
  utils::query_selector,
};

#[hook]
pub fn use_theme() {
  let theme = use_atom_value::<Theme>();
  use_effect_with_deps(
    |theme| {
      if let Some(element) = query_selector("html") {
        let css_text = theme.get_css_text();
        element.style().set_css_text(&css_text);
      }
    },
    theme,
  );
}
