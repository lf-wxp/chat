use bounce::use_atom_setter;
use gloo_console::log;
use message::{
  Action, ActionMessage, ClientAction, GetInfo, RequestMessageData, ResponseMessageData,
};
use message::{ListAction, ListMessage};
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;
use yew::use_effect_with;

use crate::store::{User, Users};
use crate::utils::{get_link, Request};

#[hook]
pub fn use_client_init() {
  let user_setter = use_atom_setter::<User>();
  let users_setter = use_atom_setter::<Users>();

  use_effect_with((), move |_| {
    if let Some(link) = get_link() {
      let setter_clone = user_setter.clone();
      let receiver = link.receiver();
      let sender = link.sender();
      spawn_local(async move {
        let message = RequestMessageData::Action(Action::List(ListAction));
        let mut request = Request::new(sender, receiver);
        log!("list await start");
        let futures = request.feature();
        request.request(message);
        let msg =  futures.await;
        log!("list await end", format!("{:?}", &msg));
        if let ResponseMessageData::Action(ActionMessage::ListMessage(list_message)) = msg {
          let ListMessage { client_list, .. } = list_message;
          log!("user_list", format!("{:?}", client_list));
          users_setter(Users(client_list.into_iter().map(|x| x.into()).collect()));
        }
      });

      let receiver = link.receiver();
      let sender = link.sender();
      spawn_local(async move {
        let message = RequestMessageData::Action(Action::Client(ClientAction::GetInfo(GetInfo)));
        let mut request = Request::new(sender, receiver);
        log!("info await start");
        let futures = request.feature();
        request.request(message);
        let msg =  futures.await;
        log!("info await ", format!("{:?}", &msg));
        if let ResponseMessageData::Action(ActionMessage::Client(info)) = msg {
          setter_clone(info.into());
        }
      });
    }
  })
}
