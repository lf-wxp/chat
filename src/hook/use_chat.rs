use std::rc::Rc;

use bounce::{use_atom, use_atom_value};
use yew::prelude::*;

use crate::{
  model::{ChatMessage, MessageState},
  store::{Chat, Refresh},
  utils::get_history,
};

#[hook]
pub fn use_chat() -> (Rc<dyn Fn(ChatMessage)>, Rc<dyn Fn(String, MessageState)>) {
  let chat = use_atom_value::<Chat>();
  let refresh = use_atom::<Refresh>();
  let add = {
    let chat = chat.clone();
    Rc::new(move |chat_message: ChatMessage| {
      get_history(&chat.0).and_then(|x| {
        x.push(chat_message);
        refresh.set(refresh.refresh());
        Some(())
      });
    })
  };
  let update_state = Rc::new(move |uuid: String, state: MessageState| {
    get_history(&chat.0).and_then(|x| {
      x.iter_mut().find(|x| x.uuid == uuid).and_then(|x| {
        x.state = state;
        Some(())
      });
      Some(())
    });
  });
  (add, update_state)
}
