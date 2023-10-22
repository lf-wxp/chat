use bounce::{use_atom_setter, use_atom_value};
use yew::prelude::*;
use yew::use_effect_with;

use crate::{
  model::{Data, SdpResponse},
  store::User,
  utils::get_client,
};

#[hook]
pub fn use_client() {
  let user_setter = use_atom_setter::<User>();
  let user = use_atom_value::<User>();
  use_effect_with((), move |_| {
    if let Some(client) = get_client() {
      user_setter(client.borrow_mut().user());
      let setter_clone = user_setter.clone();
      client
        .borrow_mut()
        .set_onmessage(Box::new(move |message: SdpResponse| {
          if let Some(Data::ClientInfo(info)) = message.data {
            setter_clone(User {
              uuid: info.uuid,
              name: user.name.clone(),
            });
          }
        }));
    }
  })
}
