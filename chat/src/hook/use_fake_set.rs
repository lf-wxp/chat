use bounce::use_atom;
use bounce::use_atom_setter;
use nanoid::nanoid;
use std::collections::HashMap;
use yew::prelude::*;
use yew::use_effect_with;

use crate::{
  store::{Chat, CurrentChat, User},
  utils::get_chat_history,
};

#[hook]
pub fn use_fake_set() -> () {
  let chat_history = get_chat_history();
  let conversation_handle = use_atom_setter::<CurrentChat>();
  let current_user_handle = use_atom::<User>();
  let chat_history = chat_history.map_or(HashMap::new(), |x| x.0.clone());
  let last_message = chat_history
    .clone()
    .values()
    .next()
    .unwrap_or(&Vec::new())
    .last()
    .cloned();
  let first_key = chat_history
    .keys()
    .next()
    .map_or("".to_string(), |v| v.to_string())
    .clone();
  use_effect_with((), move |_| {
    conversation_handle(CurrentChat(
      Some(Chat {
        id: first_key.to_string(),
        name: "".to_string(),
        users: vec![User {
          uuid: nanoid!(),
          name: "".to_string(),
        }],
      })
      .clone(),
    ));
  });
  use_effect_with((), move |_| {
    if let Some(message) = last_message {
      current_user_handle.set(User {
        uuid: message.uuid.clone(),
        name: message.name.clone(),
      });
    }
  });
}
