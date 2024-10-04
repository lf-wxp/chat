use bounce::use_slice_dispatch;
use gloo_console::log;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;
use yew::use_effect_with;

use crate::{
  hook::use_chat,
  model::{ChannelMessage, Information},
  store::{Chats, ChatsAction},
  utils::get_client,
};
#[hook]
pub fn use_client_message_watch() {
  let chats_dispatch = use_slice_dispatch::<Chats>();
  let (add, _update) = use_chat();
  use_effect_with((), move |_| {
    if let Some(client) = get_client() {
      spawn_local(async move {
        while let Ok(msg) = client.receiver.recv().await {
          match ChannelMessage::from(msg) {
            ChannelMessage::Information(msg) => {
              let Information { chat, message } = msg;
              let mut chat = chat;
              let chat_id = chat.id.clone();
              chat.update_name(&client.user());
              chats_dispatch(ChatsAction::Append(chat));
              add(message, Some(chat_id));
            }
            ChannelMessage::Command => {}
          }
        }
      });
    }
  })
}
