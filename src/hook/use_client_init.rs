use bounce::use_atom_setter;
use gloo_console::log;
use yew::prelude::*;
use yew::use_effect_with;

use crate::model::ConnectInfo;
use crate::{
  model::{Data, WsResponse},
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
      client
        .borrow_mut()
        .set_onmessage(Box::new(move |message: WsResponse| {
          if let Some(message) = message.data {
            match message {
              Data::ClientInfo(info) => {
                setter_clone(info);
              },
              Data::ClientList(list) => {
                users_setter(Users(list));
              },
              Data::RoomList(list) => todo!(),
              Data::Transmit(transmit) => todo!(),
              Data::ConnectInfo(info) => {
                let ConnectInfo { client_list, .. } = info;
                log!("user_list", format!("{:?}", client_list));
                users_setter(Users(client_list));
              },
            }
          }
        }));
    }
  })
}
