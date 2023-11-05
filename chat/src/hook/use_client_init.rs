use bounce::use_atom_setter;
use gloo_console::log;
use message::ListMessage;
use message::{CastMessage, Data, ResponseMessage};
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
        .set_onmessage(Box::new(move |message: ResponseMessage| match message {
          ResponseMessage::Action(action) => {
            if let Some(message) = action.data {
              match message {
                Data::Client(info) => {
                  setter_clone(info.into());
                }
                Data::ClientList(list) => {
                  users_setter(Users(list.into_iter().map(|x| x.into()).collect()));
                }
                Data::RoomList(list) => todo!(),
              }
            }
          }
          ResponseMessage::Transmit(transmit) => {
            log!("user_list");
            match transmit.message {
              CastMessage::Call(call_message) => {}
              CastMessage::Sdp(call_message) => {}
              CastMessage::List(list_message) => {
                let ListMessage { client_list, .. } = list_message;
                log!("user_list", format!("{:?}", client_list));
                users_setter(Users(client_list.into_iter().map(|x| x.into()).collect()));
              }
              CastMessage::Ice(ice) => {}
            }
          }
        }));
    }
  })
}
