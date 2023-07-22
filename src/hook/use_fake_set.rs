use bounce::use_atom;
use std::collections::HashMap;
use yew::prelude::*;

use crate::{
  get_chat_history,
  store::{Conversation, User},
};

#[hook]
pub fn use_fake_set() -> () {
  let chat_history = get_chat_history();
  let binding = HashMap::new();
  let chat_history = match chat_history {
    Some(chat_history) => &chat_history.0,
    None => &binding,
  };
  let conversation_handle = use_atom::<Conversation>();
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
  conversation_handle.set(Conversation((*first_conversation).clone()));
  use_effect(move || {
    current_user_handle.set(User {
      uuid: last_message.uuid.clone(),
      name: last_message.name.clone(),
    });
  });
}
