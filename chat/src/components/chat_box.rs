use bounce::use_atom_value;
use js_sys::ArrayBuffer;
use stylist::{self, style};
use web_sys::Blob;
use yew::prelude::*;
use yew_icons::{Icon, IconId};

use crate::{
  components::{ChatText, ChatValue, EmojiBox, ImageInput, Selection, VoiceInput},
  hook::{use_chat, use_click_exclusive},
  model::{ChatMessage, Message, MessageBinary},
  store::User,
  utils::{class_name_determine, get_string_len, style},
};

#[function_component]
pub fn ChatBox() -> Html {
  let class_name = get_class_name();
  let user_name = use_atom_value::<User>();
  let text = use_state(|| "".to_string());
  let visible = use_state(|| false);
  let selection = use_state(|| Selection {
    start: None,
    end: None,
  });
  let (add_message, _update_message_state) = use_chat();

  let emoji_class = class_name_determine(*visible, "tool-icon", "active");

  let update_selection = {
    let selection = selection.clone();
    move |idx: u32| {
      selection.set(Selection {
        start: Some(idx),
        end: Some(idx),
      })
    }
  };

  let modify_text =
    move |text: String, selection: UseStateHandle<Selection>, string: String| -> String {
      let Selection { start, end } = *selection;
      if start.is_none() || end.is_none() {
        return text;
      }
      let start = start.unwrap() as usize;
      let end = end.unwrap() as usize;
      let prefix: String = text.chars().take(start).collect();
      let suffix: String = text.chars().skip(end).collect();
      let len = get_string_len(&string);
      update_selection((len + start) as u32);
      format!("{}{}{}", prefix, string, suffix)
    };

  let emoji_callback = {
    let text = text.clone();
    let selection = selection.clone();
    move |emoji: String| {
      let content = modify_text((*text).clone(), selection.clone(), emoji);
      text.set(content);
    }
  };

  let send_callback = {
    let text = text.clone();
    let visible = visible.clone();
    let add = add_message.clone();
    let user_name = user_name.clone();
    move |_| {
      add(ChatMessage::new(
        user_name.name.clone(),
        Message::Text((*text).clone()),
      ));
      text.set("".to_string());
      visible.set(false);
    }
  };

  let change_callback = {
    let text = text.clone();
    let selection = selection.clone();
    move |val: ChatValue| {
      selection.set(val.selection);
      text.set(val.value);
    }
  };

  let emoji_visible_callback = {
    let visible = visible.clone();
    Callback::from(move |_| {
      visible.set(!*visible);
    })
  };

  let callback = {
    let visible = visible.clone();
    move || {
      visible.set(false);
    }
  };

  let image_input_callback = {
    let user_name = user_name.clone();
    let add = add_message.clone();
    Callback::from(move |buffer: ArrayBuffer| {
      add(ChatMessage::new(
        user_name.name.clone(),
        Message::Image(MessageBinary::Buffer(buffer)),
      ));
    })
  };

  let voice_input_callback = {
    let add = add_message.clone();
    let user_name = user_name.clone();
    Callback::from(move |blob: Blob| {
      add(ChatMessage::new(
        user_name.name.clone(),
        Message::Audio(MessageBinary::Blob(blob)),
      ));
    })
  };

  use_click_exclusive(vec![format!(".{}", class_name)], callback);

  html! {
    <div class={class_name}>
      <ChatText onsend={send_callback} text={(*text).clone()} onchange={change_callback} />
      <div class="emoji-box">
        if *visible {
          <EmojiBox onclick={emoji_callback} />
        }
      </div>
      <div class="chat-tool">
        <Icon onclick={emoji_visible_callback} icon_id={IconId::BootstrapEmojiSmile} class={Classes::from(emoji_class)} width="16px" height="16px" />
        <ImageInput onchange={image_input_callback} />
        <VoiceInput onchange={voice_input_callback} />
      </div>
    </div>
  }
}

#[allow(non_upper_case_globals)]
fn get_class_name() -> String {
  style::get_class_name(style!(
    r#"
      display:flex;
      flex-flow: column nowrap;
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
      .tool-icon:hover, .tool-icon.active {
        color: #51b66d;
      }
      .emoji-box:not(:empty) {
        margin-block-start: 5px;
      }
    "#
  ))
}
