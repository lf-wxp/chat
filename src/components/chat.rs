use bounce::use_atom_value;
use stylist::{self, style};
use yew::prelude::*;

use crate::{
  components::{ChatBox, ChatMessage},
  hook::use_chat_history,
  model::MessageAlignment,
  store::User,
  utils::style,
};

#[function_component]
pub fn Chat() -> Html {
  let class_name = format!("{} scroll-bar", get_class_name());
  let current_user = use_atom_value::<User>();
  let test_message = use_chat_history();

  let get_alignment = |uuid: String| {
    if uuid == current_user.uuid {
      MessageAlignment::Right
    } else {
      MessageAlignment::Left
    }
  };
  let get_name = |name: String| {
    if name == current_user.name {
      None
    } else {
      Some(name)
    }
  };

  html! {
    <div class={class_name}>
      { for test_message.iter().map(|msg| html! {
          <ChatMessage
            uuid={Some(msg.uuid.clone())}
            name={get_name(msg.name.clone())}
            alignment={get_alignment(msg.uuid.clone())}
            time={msg.time}
            message={msg.message.clone()}
          />
         })}
      <ChatBox />
    </div>
  }
}

fn get_class_name() -> String {
  style::get_class_name(style!(
    r#"
        inline-size: 300px;
        block-size: 100%;
        overflow: auto;

        &>div {
          margin-block-end: 30px;
        }
    "#
  ))
}
