use bounce::{use_atom_value, use_selector_value};
use stylist::{self, style};
use yew::prelude::*;

use crate::{
  components::{ChatBox, ChatMessage},
  model::MessageAlignment,
  store::{HistoryMessage, User},
  utils::style,
};

#[function_component]
pub fn Chat() -> Html {
  let class_name = get_class_name();
  let current_user = use_atom_value::<User>();
  let history_message = use_selector_value::<HistoryMessage>();

  let get_alignment = |name: String| {
    if name == current_user.name {
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
      <div class="history-message scroll-bar">
      { for history_message.0.iter().map(|msg| html! {
          <ChatMessage
            key={msg.uuid.clone()}
            uuid={Some(msg.uuid.clone())}
            name={get_name(msg.name.clone())}
            alignment={get_alignment(msg.name.clone())}
            time={msg.time}
            message={msg.message.clone()}
          />
         })}
      </div>
      <div class="chat-box">
        <ChatBox />
      </div>
    </div>
  }
}

#[allow(non_upper_case_globals)]
fn get_class_name() -> String {
  style::get_class_name(style!(
    r#"
        inline-size: 300px;
        block-size: 100%;
        display: flex;
        flex-flow: column nowrap;

        .history-message {
          flex: 1;  
          overflow: auto;
        }
        .history-message >div {
          margin-block-end: 30px;
        }
        .chat-box {
          flex: 0; 
        }
    "#
  ))
}
