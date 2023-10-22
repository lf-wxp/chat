use bounce::use_atom;
use std::collections::HashMap;
use yew::prelude::*;
use yew::use_effect_with;

use crate::{
  store::{Chat, User},
  utils::get_chat_history,
};

#[hook]
pub fn use_fake_set() -> () {
  let chat_history = get_chat_history();
  let binding = HashMap::new();
  let chat_history = match chat_history {
    Some(chat_history) => &chat_history.0,
    None => &binding,
  };
  let conversation_handle = use_atom::<Chat>();
  let current_user_handle = use_atom::<User>();
  let binding = "".to_owned();
  let first_conversation = chat_history.keys().next().unwrap_or(&binding);
  let last_message = chat_history
    .values()
    .next()
    .unwrap()
    .last()
    .unwrap()
    .clone();
  conversation_handle.set(Chat((*first_conversation).clone()));
  use_effect_with((), move |_| {
    current_user_handle.set(User {
      uuid: last_message.uuid.clone(),
      name: last_message.name.clone(),
    });
  });
}
