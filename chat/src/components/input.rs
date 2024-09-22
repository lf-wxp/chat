use stylist::{self, style};
use web_sys::HtmlInputElement;
use yew::{prelude::*, Callback};
use yew_icons::{Icon, IconId};

use crate::{
  model::Size,
  utils::{get_target, style},
};

#[derive(Properties, PartialEq)]
pub struct Props {
  #[prop_or_default]
  pub onchange: Callback<String>,
  #[prop_or_default]
  pub onclick: Callback<()>,
  #[prop_or_default]
  pub onenter: Callback<()>,
  #[prop_or_default]
  pub size: Size,
  #[prop_or_default]
  pub value: String,
  #[prop_or_default]
  pub icon: Option<IconId>,
}

#[function_component]
pub fn Input(props: &Props) -> Html {
  let class_name = get_class_name();
  let onchange = props.onchange.clone();
  let onclick_clone = props.onclick.clone();
  let onenter = props.onenter.clone();

  let oninput = Callback::from(move |e: InputEvent| {
    if let Some(target) = get_target::<InputEvent, HtmlInputElement>(e) {
      onchange.emit(target.value());
    }
  });
  let onclick = Callback::from(move |_e: MouseEvent| {
    onclick_clone.emit(());
  });
  let onkeydown = Callback::from(move |e: KeyboardEvent| {
    if e.key() == "Enter" {
      onenter.emit(());
    }
  });
  let class = format!("{class_name} {}", props.size);

  html! {
    <section {class}>
      <input type="text" {oninput} value={props.value.clone()} {onkeydown} />
      if props.icon.is_some() {
        <Icon  icon_id={props.icon.unwrap()} width="16px" height="16px" {onclick} />
      }
    </section>
  }
}

#[allow(non_upper_case_globals)]
fn get_class_name() -> String {
  style::get_class_name(style!(
    r#"
      border: 1px solid rgba(225, 225, 225, 0.1);
      background: var(--theme-color);
      padding-inline: 8px;
      border-radius: calc(var(--radius) / 2);
      color: white;
      block-size: 32px;
      transition: all .2s ease-in-out;
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
