use bounce::{use_atom_setter, use_slice_dispatch};
use message::ChatMessage;
use message::{ActionMessage, MessageType, ResponseMessage, ResponseMessageData};
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;
use yew::use_effect_with;

use crate::{
  components::use_media_request,
  hook::use_chat,
  store::{Chats, ChatsAction, Users, Chat},
  utils::get_client,
};

#[hook]
pub fn use_ws_message_watch() {
  let use_media = use_media_request();
  let users_setter = use_atom_setter::<Users>();
  let chats_dispatch = use_slice_dispatch::<Chats>();
  let (add, _update) = use_chat();
  use_effect_with((), move |_| {
    if let Some(client) = get_client() {
      let mut receiver = client.link.receiver();
      spawn_local(async move {
        while let Ok(msg) = receiver.recv().await {
          let msg = bincode::deserialize::<ResponseMessage>(&msg);
          if let Ok(ResponseMessage {
            message: ResponseMessageData::Action(ActionMessage::ClientList(message)),
            ..
          }) = &msg
          {
            users_setter(Users(message.clone().into_iter().map(Into::into).collect()));
          }

          if let Ok(ResponseMessage {
            message: ResponseMessageData::Media(message),
            message_type,
            session_id,
          }) = &msg
          {
            if MessageType::Request == *message_type {
              use_media(message.clone(), session_id.clone());
            }
          }

          if let Ok(ResponseMessage {
            message: ResponseMessageData::Chat(ChatMessage { message, chat, .. }),
            ..
          }) = &msg
          {
            let mut chat: Chat = chat.clone().into();
            let chat_id = chat.id.clone();
            chat.update_name(&client.user());
            chats_dispatch(ChatsAction::Append(chat));
            add(message.clone(), Some(chat_id));
          }
        }
      })
    }
  })
}
