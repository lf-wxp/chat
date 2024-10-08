use bounce::use_atom_value;
use js_sys::ArrayBuffer;
use message::{Information, Message};
use stylist::{self, style};
use yew::prelude::*;
use yew_icons::{Icon, IconId};

use crate::{
  components::{ChatText, ChatValue, EmojiBox, ImageInput, Selection, VoiceInput},
  hook::{use_chat, use_click_exclusive},
  store::{CurrentChat, User},
  utils::{array_buffer_to_vec, class_name_determine, get_client_execute, get_string_len, style},
};

#[function_component]
pub fn ChatBox() -> Html {
  let class_name = get_class_name();
  let user = use_atom_value::<User>();
  let current_chat = use_atom_value::<CurrentChat>();
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
    let user = user.clone();
    let current_chat = current_chat.clone();
    move |_| {
      if text.is_empty() { return };
      let current_chat = current_chat.clone();
      let message = Information::new(user.name.clone(), Message::Text((*text).clone()));
      let user_uuid = user.uuid.clone();
      add(message.clone(), None);
      get_client_execute(Box::new(|client| {
        Box::pin(async move {
          let remote_ids = current_chat.remote_client_ids(&user_uuid);
          let chat = current_chat.0.clone().unwrap();
          client.send_message(remote_ids, message, chat.into()).await;
        })
      }));
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
    let user = user.clone();
    let add = add_message.clone();
    let user = user.clone();
    let current_chat = current_chat.clone();
    Callback::from(move |buffer: ArrayBuffer| {
      let user_uuid = user.uuid.clone();
      let buffer = array_buffer_to_vec(&buffer);
      let message = Information::new(user.name.clone(), Message::Image(buffer));
      let current_chat = current_chat.clone();
      add(message.clone(), None);
      get_client_execute(Box::new(|client| {
        Box::pin(async move {
          let remote_ids = current_chat.remote_client_ids(&user_uuid);
          let chat = current_chat.0.clone().unwrap();
          client.send_message(remote_ids, message, chat.into()).await;
        })
      }));
    })
  };

  let voice_input_callback = {
    let add = add_message.clone();
    let user = user.clone();
    Callback::from(move |buffer: ArrayBuffer| {
      let buffer = array_buffer_to_vec(&buffer);
      add(
        Information::new(user.name.clone(), Message::Audio(buffer)),
        None,
      );
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
        <Icon onclick={emoji_visible_callback}
          icon_id={IconId::BootstrapEmojiSmile}
          class={Classes::from(emoji_class)}
          width="16px"
          height="16px"
        />
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
