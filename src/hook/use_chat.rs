use std::rc::Rc;

use bounce::{use_atom, use_atom_value};
use yew::prelude::*;

use crate::{
  model::{ChatMessage, MessageState},
  store::{Chat, Refresh},
  utils::get_history,
};

type AddAction = Rc<dyn Fn(ChatMessage)>;
type UpdateAction = Rc<dyn Fn(String, MessageState)>;

#[hook]
pub fn use_chat() -> (AddAction, UpdateAction) {
  let chat = use_atom_value::<Chat>();
  let refresh = use_atom::<Refresh>();
  let add = {
    let chat = chat.clone();
    Rc::new(move |chat_message: ChatMessage| {
      if let Some(x) = get_history(&chat.0) {
        x.push(chat_message);
        refresh.set(refresh.refresh());
      }
    })
  };
  let update_state = Rc::new(move |uuid: String, state: MessageState| {
    if let Some(x) = get_history(&chat.0) {
      if let Some(x) = x.iter_mut().find(|x| x.uuid == uuid) {
        x.state = state;
      };
    }
  });
  (add, update_state)
}
