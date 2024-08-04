use message::MessageType;
use message::{ResponseMessage, ResponseMessageData};
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;
use yew::use_effect_with;

use crate::{components::use_media_request, utils::get_link};

#[hook]
pub fn use_media_request_watch() {
  let use_media = use_media_request();
  use_effect_with((), move |_| {
    if let Some(link) = get_link() {
      let mut receiver = link.receiver();
      spawn_local(async move {
        while let Ok(msg) = receiver.recv().await {
          if let Ok(ResponseMessage {
            message: ResponseMessageData::Media(message),
            message_type,
            session_id,
          }) = serde_json::from_str::<ResponseMessage>(&msg)
          {
            if MessageType::Request == message_type {
              use_media(message, session_id);
            }
          }
        }
      })
    }
  })
}
