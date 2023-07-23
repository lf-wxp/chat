use bounce::{BounceStates, Selector};
use std::rc::Rc;

use crate::{model::ChatMessage, utils::get_history};

use super::{Chat, Refresh};

#[derive(PartialEq)]
pub struct HistoryMessage(pub Vec<ChatMessage>);

impl Selector for HistoryMessage {
  fn select(states: &BounceStates) -> Rc<Self> {
    let chat = states.get_atom_value::<Chat>();
    states.get_atom_value::<Refresh>();
    let message = get_history(&chat.0).map_or(vec![], |x| x.to_vec());
    Rc::from(HistoryMessage(message))
  }
}
