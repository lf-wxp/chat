use bounce::use_atom_setter;
use gloo_console::log;
use message::ListMessage;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;
use yew::use_effect_with;

use crate::store::{User, Users};
use crate::utils::get_client;

#[hook]
pub fn use_init() {
  let user_setter = use_atom_setter::<User>();
  let users_setter = use_atom_setter::<Users>();
  use_effect_with((), move |_| {
    if let Some(client) = get_client() {
      spawn_local(async move {
        if let Some(info) = client.get_init_info().await {
          user_setter(info.into());
        }
        if let Some(list_message) = client.get_user_list().await {
          let ListMessage { client_list, .. } = list_message;
          log!("user_list", format!("{:?}", client_list));
          users_setter(Users(client_list.into_iter().map(|x| x.into()).collect()));
        }
      });
    }
  })
}
