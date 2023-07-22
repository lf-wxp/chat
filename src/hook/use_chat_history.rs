use bounce::use_atom_value;
use yew::prelude::*;

use crate::{get_chat_history, model::ChatMessage, store::Conversation};

#[hook]
pub fn use_chat_history() -> Vec<ChatMessage> {
  let chat_history = get_chat_history();
  let conversation = &use_atom_value::<Conversation>().0;
  match chat_history {
    Some(chat_history) => chat_history
      .0
      .get(conversation)
      .unwrap_or(&Vec::new())
      .to_vec(),
    None => vec![],
  }
}
