use bounce::{BounceStates, Selector};
use message::Information;
use std::rc::Rc;

use crate::{
  store::{CurrentChat, Refresh},
  utils::get_history,
};

#[derive(PartialEq)]
pub struct HistoryMessage(pub Vec<Information>);

impl Selector for HistoryMessage {
  fn select(states: &BounceStates) -> Rc<Self> {
    let current_chat = states.get_atom_value::<CurrentChat>();
    states.get_slice_value::<Refresh>();
    let id = current_chat.id();
    let message = get_history(id).map_or(vec![], |x| x.to_vec());
    Rc::from(HistoryMessage(message))
  }
}
