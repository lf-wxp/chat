use stylist::{self, style};
use web_sys::MouseEvent;
use yew::{prelude::*, Callback};

use crate::{model::Size, utils::style};

#[derive(Properties, PartialEq)]
pub struct Props {
  #[prop_or_default]
  pub onclick: Callback<MouseEvent>,
  #[prop_or_default]
  pub children: Html,
  #[prop_or_default]
  pub size: Size,
}

#[function_component]
pub fn Button(props: &Props) -> Html {
  let class_name = get_class_name();
  let onclick = props.onclick.clone();

  let onclick = Callback::from(move |e: MouseEvent| {
    onclick.emit(e);
  });

  let class = format!("{class_name} {}", props.size);

  html! {
    <button {onclick} {class}>{props.children.clone()}</button>
  }
}

#[allow(non_upper_case_globals)]
fn get_class_name() -> String {
  style::get_class_name(style!(
    r#"
      position: static;
      cursor: pointer;
      background: rgba(var(--theme-color-rgb), 0.6);
      border-radius: calc(var(--radius) / 2);
      display: inline-block;
      color: var(--font-color);
      padding: 5px 10px;
      border: 1px solid rgba(225, 225, 225, 0.1);
      transition: all 0.2s ease-in-out;
      margin: 5px;
      &:hover {
        background: rgba(var(--theme-color-rgb), 1);
      }
      &.media {
        block-size: 32px;
      }
      &.small {
        block-size: 24px;
      }
      &.large {
        block-size: 48px;
      }
    "#
  ))
}
