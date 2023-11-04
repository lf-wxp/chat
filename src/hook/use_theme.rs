use bounce::use_atom_value;
use web_sys::HtmlElement;
use yew::prelude::*;

use crate::{
  store::Theme,
  utils::query_selector,
};

#[hook]
pub fn use_theme() {
  let theme = use_atom_value::<Theme>();
  use_effect_with(theme,|theme| {
    if let Some(element) = query_selector::<HtmlElement>("html") {
      let css_text = theme.get_css_text();
      element.style().set_css_text(&css_text);
    }
  });
}
