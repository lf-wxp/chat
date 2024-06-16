use bounce::{use_atom, use_atom_value};
use yew::{prelude::*, Callback};
use yew_icons::IconId;

use crate::{components::Input, store::FilterWord};

#[function_component]
pub fn Search() -> Html {
  let filter_word_handle = use_atom::<FilterWord>();
  let filter_word = use_atom_value::<FilterWord>();

  let onclick = {
    let filter_word_clone = filter_word_handle.clone();
    Callback::from(move |_: ()| {
      filter_word_clone.set(FilterWord("".to_string()));
    })
  };

  let onchange = Callback::from(move |val: String| {
    filter_word_handle.set(FilterWord(val));
  });

  let icon_id = if filter_word.0.is_empty() {
    IconId::BootstrapSearch
  } else {
    IconId::FontAwesomeSolidXmark
  };

  html! {
    <Input {onchange} value={filter_word.0.clone()} {onclick} icon={Some(icon_id)} />
  }
}
