
use chrono::prelude::*;
use stylist::{self, style};
use yew::prelude::*;

use crate::{
  model::{Message, MessageAlignment},
  utils::style,
};

#[derive(Properties, PartialEq)]
pub struct Props {
  pub name: Option<AttrValue>,
  pub uuid: Option<AttrValue>,
  pub alignment: MessageAlignment,
  pub time: DateTime<Utc>,
  pub message: Message,
}

#[function_component]
pub fn ChatBox(props: &Props) -> Html {
  let class_name = get_class_name();

  html! {
    <div class={class_name}>
      {"chat box"}
    </div>
  }
}

fn get_class_name() -> String {
  style::get_class_name(style!(
    r#"
    "#
  ))
}
