use bounce::{use_atom, use_atom_value};
use stylist::{self, style};
use yew::prelude::*;

use crate::{
  components::ChatMessage,
  model::MessageAlignment,
  store::{MessageBunch, User},
  utils::style,
};

#[function_component]
pub fn Chat() -> Html {
  let class_name = format!("{} scroll-bar", get_class_name());
  let current_user_handle = use_atom::<User>();
  let current_user = use_atom_value::<User>();
  let message = &use_atom_value::<MessageBunch>().0;
  let test_message = message.values().collect::<Vec<_>>()[0];

  // for test
  let len = test_message.len();
  let first = test_message.get(len - 1).unwrap().clone();
  use_effect(move || {
    current_user_handle.set(User {
      uuid: first.uuid.clone(),
      name: first.name.clone(),
    });
  });

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
            time={msg.time.clone()}
            message={msg.message.clone()}
          />
         })}
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
