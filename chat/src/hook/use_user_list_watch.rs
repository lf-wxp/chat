use bounce::use_atom_setter;
use message::{ActionMessage, ResponseMessage, ResponseMessageData};
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;
use yew::use_effect_with;

use crate::{store::Users, utils::get_link};

#[hook]
pub fn use_user_list_watch() {
  let users_setter = use_atom_setter::<Users>();
  use_effect_with((), move |_| {
    if let Some(link) = get_link() {
      let mut receiver = link.receiver();
      spawn_local(async move {
        while let Ok(msg) = receiver.recv().await {
          if let Ok(ResponseMessage {
            message: ResponseMessageData::Action(ActionMessage::ClientList(message)),
            ..
          }) = serde_json::from_str::<ResponseMessage>(&msg)
          {
            users_setter(Users(message.into_iter().map(Into::into).collect()));
          }
        }
      })
    }
  })
}
