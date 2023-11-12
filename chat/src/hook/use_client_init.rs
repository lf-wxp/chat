use bounce::use_atom_setter;
use gloo_console::log;
use message::{ActionMessage, Data, ListMessage};
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
      client
        .borrow_mut()
        .set_onmessage(Box::new(move |message: ActionMessage| {
          if let Some(message) = message.data {
            match message {
              Data::Client(info) => {
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
        }));
    }
  })
}
