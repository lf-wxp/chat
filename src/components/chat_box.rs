use bounce::use_atom_value;
use stylist::{self, style};
use yew::prelude::*;
use yew_icons::{Icon, IconId};

use crate::{
  components::{ChatText, EmojiBox},
  hook::use_chat,
  model::{ChatMessage, Message},
  store::User,
  utils::style,
};

#[function_component]
pub fn ChatBox() -> Html {
  let class_name = get_class_name();
  let user_name = use_atom_value::<User>();
  let text = use_state(|| "".to_string());
  let (add, update_state) = use_chat();

  let emoji_callback = {
    let text = text.clone();
    move |emoji: String| {
      text.set(format!("{}{}", *text, emoji));
    }
  };

  let send_callback = {
    let text = text.clone();
    move |_| {
      add(ChatMessage::new(
        user_name.name.clone(),
        Message::Text((*text).clone()),
      ));
      text.set("".to_string());
    }
  };

  let change_callback = {
    let text = text.clone();
    move |val: String| {
      text.set(val);
    }
  };

  html! {
    <div class={class_name}>
      <ChatText onsend={send_callback} text={(*text).clone()} onchange={change_callback} />
      <div class="chat-tool">
        <EmojiBox onclick={emoji_callback} />
        <Icon icon_id={IconId::FontAwesomeRegularImages} class="tool-icon" width="16px" height="16px" />
        <Icon icon_id={IconId::HeroiconsSolidMicrophone} class="tool-icon" width="16px" height="16px" />
      </div>
    </div>
  }
}

fn get_class_name() -> String {
  style::get_class_name(style!(
    r#"
      .chat-tool {
        margin-block-start: 5px;
        background: var(--theme-ancillary-color);
        border-radius: calc(var(--radius) / 2);
        inline-block: 100%;
        padding: 5px;
        display: flex;
      }
      .tool-icon {
        cursor: pointer;
        color: #8896a4;
        margin-inline-start: 10px;
        transition: all 0.2s ease;
      }
      .tool-icon:hover {
        color: #51b66d;
      }
    "#
  ))
}
