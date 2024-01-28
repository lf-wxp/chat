use bounce::use_atom_setter;
use futures::StreamExt;
use gloo_console::log;
use message::ListMessage;
use message::{ActionMessage, ResponseMessage, ResponseMessageData};
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;
use yew::use_effect_with;

use crate::{
  store::{User, Users},
  utils::get_client,
};

#[hook]
pub fn use_client_init() {
  let user_setter = use_atom_setter::<User>();
  let users_setter = use_atom_setter::<Users>();

  use_effect_with((), move |_| {
    if let Some(client) = get_client() {
      user_setter(client.user());
      let setter_clone = user_setter.clone();
      let mut receiver = client.receiver();
      let client_clone = client;
      spawn_local(async move {
        while let Some(msg) = receiver.next().await {
          log!("read_receiver msg", format!("{:}", &msg));
          if let Ok(ResponseMessage {
            message: ResponseMessageData::Action(message),
            ..
          }) = serde_json::from_str::<ResponseMessage>(&msg)
          {
            match message {
              ActionMessage::Client(info) => {
                client_clone.user.uuid = info.uuid.clone();
                setter_clone(info.into());
              }
              ActionMessage::ClientList(list) => {
                users_setter(Users(list.into_iter().map(|x| x.into()).collect()));
              }
              ActionMessage::RoomList(_list) => todo!(),
              ActionMessage::ListMessage(list_message) => {
                let ListMessage { client_list, .. } = list_message;
                log!("user_list", format!("{:?}", client_list));
                users_setter(Users(client_list.into_iter().map(|x| x.into()).collect()));
              }
              _ => todo!(),
            }
          }
        }
      });
    }
    if let Some(client) = get_client() {
      client.get_init_info();
    }
  })
}
