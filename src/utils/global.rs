use std::cell::OnceCell;

use crate::model::ChatHistory;

static mut CHAT_HISTORY: OnceCell<ChatHistory> = OnceCell::new();

pub fn get_chat_history() -> Option<&'static mut ChatHistory> {
  unsafe {
    CHAT_HISTORY.get_or_init(|| ChatHistory::default());
    CHAT_HISTORY.get_mut()
  }
}
