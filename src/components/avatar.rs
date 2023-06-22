use stylist::{self, style};
use yew::prelude::*;

use crate::utils::{style, avatar};

#[derive(Properties, PartialEq)]
pub struct Props {
  pub name: AttrValue,
}

#[function_component]
pub fn Avatar(props: &Props) -> Html {
  let class_name = get_class_name();
  let avatar = avatar::Avatar::from(props.name.to_string());
  let avatar_html = Html::from_html_unchecked(avatar.image.into());
  html! {
    <avatar class={class_name} name={props.name.clone()}>
      {{ avatar_html }}
    </avatar>
  }
}

fn get_class_name() -> String {
  style::get_class_name(
    style!(
      r#"
        background: var(--theme-color);
        border-radius: var(--radius);
        display: inline-block;
        block-size: 40px;
        inline-size: 40px;
    "#
    )
  )
}
