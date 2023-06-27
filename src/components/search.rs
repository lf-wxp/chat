use bounce::{use_atom, use_atom_value};
use stylist::{self, style};
use wasm_bindgen::JsCast;
use web_sys::HtmlInputElement;
use yew::{prelude::*, Callback};
use yew_icons::{Icon, IconId};

use crate::{store::FilterWord, utils::style};

#[function_component]
pub fn Search() -> Html {
  let class_name = get_class_name();

  let filter_word_handle = use_atom::<FilterWord>();
  let filter_word = use_atom_value::<FilterWord>();

  let onclick = {
    let filter_word_clone = filter_word_handle.clone();
    Callback::from(move |_: MouseEvent| {
      filter_word_clone.set(FilterWord("".to_string()));
    })
  };

  let oninput = Callback::from(move |e: InputEvent| {
    let input = e
      .target()
      .and_then(|t| t.dyn_into::<HtmlInputElement>().ok());
    if let Some(input) = input {
      filter_word_handle.set(FilterWord(input.value()));
    }
  });

  let icon_id = if filter_word.0.is_empty() {
    IconId::BootstrapSearch
  } else {
    IconId::FontAwesomeSolidXmark
  };

  html! {
    <section class={class_name}>
      <input type="text" {oninput} value={filter_word.0.clone()} />
      <Icon  {icon_id} width="16px" height="16px" {onclick} />
    </section>
  }
}

fn get_class_name() -> String {
  style::get_class_name(style!(
    r#"
        border: 1px solid rgba(225, 225, 225, 0.1);
        background: var(--theme-color);
        padding-inline: 8px;
        border-radius: calc(var(--radius) / 2);
        color: white;
        block-size: 32px;
        transition: all .2s ease;
        display: flex;
        justify-content: space-between;
        align-items: center;
        :focus-within {
          border-color: rgba(225, 225, 225, 0.2);
        }
        input {
          border: none;
          background: none;
          outline: none;
          color: inherit;
          block-size: 100%;
          flex: 1 1 auto;
          margin-inline-end: 8px;
        }
        svg {
          flex: 0 0 auto;
        }
    "#
  ))
}
