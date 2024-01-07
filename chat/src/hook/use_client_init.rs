use bounce::use_atom_setter;
use futures::StreamExt;
use gloo_console::log;
use message::{ActionMessage, ResponseMessage};
use message::{Data, ListMessage};
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
      user_setter(client.borrow_mut().user());
      let setter_clone = user_setter.clone();
      let client_clone = client.clone();
      spawn_local(async move {
        while let Some(msg) = client_clone.borrow_mut().receiver.next().await {
          if let Ok(ResponseMessage::Action(ActionMessage {
            data: Some(message),
            ..
          })) = serde_json::from_str::<ResponseMessage>(&msg)
          {
            match message {
              Data::Client(info) => {
                client_clone.borrow_mut().user.uuid = info.uuid.clone();
                setter_clone(info.into());
              }
              Data::ClientList(list) => {
                users_setter(Users(list.into_iter().map(|x| x.into()).collect()));
              }
              Data::RoomList(list) => todo!(),
              Data::ListMessage(list_message) => {
                let ListMessage { client_list, .. } = list_message;
                log!("user_list", format!("{:?}", client_list));
                users_setter(Users(client_list.into_iter().map(|x| x.into()).collect()));
              }
            }
          }
        }
      });
      client.borrow_mut().get_init_info();
    }
  })
}
