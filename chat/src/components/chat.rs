use bounce::{use_atom_value, use_selector_value};
use stylist::{self, style};
use wasm_bindgen::JsCast;
use web_sys::{HtmlDivElement, ScrollBehavior, ScrollToOptions};
use yew::prelude::*;
use yew_hooks::use_size;
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
  let content_box_ref = use_node_ref();
  let message_list_ref = use_node_ref();
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

  let container = content_box_ref.clone();
  let size = use_size(message_list_ref.clone());
  use_effect_with(size, move |(_width, height): &(u32, u32)| {
    if let Some(container) = container
      .get()
      .and_then(|div| div.dyn_into::<HtmlDivElement>().ok())
    {
      let options = ScrollToOptions::new();
      options.set_behavior(ScrollBehavior::Smooth);
      options.set_top((*height).into());
      container.scroll_to_with_scroll_to_options(&options);
    }
  });

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
      <div class="history-message scroll-bar" ref={content_box_ref}>
          <div ref={message_list_ref} >
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
