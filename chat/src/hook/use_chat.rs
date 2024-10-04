use bounce::{use_atom_value, use_slice_dispatch};
use std::rc::Rc;
use yew::prelude::*;

use crate::{
  model::{ChatMessage, MessageState},
  store::{CurrentChat, Refresh, RefreshAction},
  utils::get_history,
};

type AddAction = Rc<dyn Fn(ChatMessage, Option<String>)>;
type UpdateAction = Rc<dyn Fn(String, MessageState)>;

#[hook]
pub fn use_chat() -> (AddAction, UpdateAction) {
  let current_chat = use_atom_value::<CurrentChat>();
  let refresh_dispatch = use_slice_dispatch::<Refresh>();
  let add = {
    let chat = current_chat.clone();
    Rc::new(move |chat_message: ChatMessage, chat_id: Option<String>| {
      let id = chat_id.unwrap_or(chat.id().to_string());
      if let Some(x) = get_history(&id) {
        x.push(chat_message);
        refresh_dispatch(RefreshAction::Toggle);
      }
    })
  };
  let update_state = Rc::new(move |uuid: String, state: MessageState| {
    if let Some(x) = get_history(current_chat.id()) {
      if let Some(x) = x.iter_mut().find(|x| x.uuid == uuid) {
        x.state = state;
      };
    }
  });
  (add, update_state)
}
