use bounce::{use_atom_value, use_selector_value, use_slice_value};
use stylist::{self, style};
use yew::prelude::*;
use yew_icons::{Icon, IconId};

use crate::{
  components::{ChatBox, ChatMessage},
  model::MessageAlignment,
  store::{CurrentChat, HistoryMessage, User},
  utils::style,
};

#[function_component]
pub fn Chat() -> Html {
  let class_name = get_class_name();
  let current_user = use_atom_value::<User>();
  let current_chat = use_atom_value::<CurrentChat>();
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
    <section class={class_name}>
      <header>
        <span>
          { current_chat.name()}
        </span>
        <Icon
          class="more"
          icon_id={IconId::HeroiconsMiniSolidEllipsisHorizontal}
          width="16px"
          height="16px"
        />
      </header>
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
    </section>
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
      background: rgba(var(--theme-ancillary-color-rgb), 0.3);
      header {
        block-size: 50px;
        padding: 10px;
        margin-block-end: 10px;
        border-block-end: 1px solid var(--primary-color);
        color: var(--font-color);
        display: flex;
        justify-content: center;
        align-items: center;
      }
      header span {
        margin-inline-start: auto;
      }
      .more {
        margin-inline-start: auto;
        cursor: pointer;
        color: var(--primary-color);
      }
      .history-message {
        flex: 1;  
        padding: 10px;
        overflow: auto;
      }
      .history-message >div {
        margin-block-end: 30px;
      }
      .chat-box {
        padding: 10px;
        flex: 0; 
      }
    "#
  ))
}
