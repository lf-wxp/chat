use std::{cell::{OnceCell, RefCell}, rc::Rc};

use crate::{model::ChatHistory, utils::Client, store::{User, user}};

pub const IMAGE_FILE_SIZE: f64 = 10f64; // mb

static mut CHAT_HISTORY: OnceCell<ChatHistory> = OnceCell::new();

static mut CLIENT: OnceCell<Rc<RefCell<Client>>> = OnceCell::new();

pub fn get_chat_history() -> Option<&'static mut ChatHistory> {
  unsafe {
    CHAT_HISTORY.get_or_init(ChatHistory::default);
    CHAT_HISTORY.get_mut()
  }
}

pub fn set_client() {
  unsafe {
    let _ = CLIENT.set(Client::new(User::default()));
  }
}

pub fn get_client() -> Option<&'static mut Rc<RefCell<Client>>> {
  unsafe {
    CLIENT.get_or_init(|| Client::new(User::default()));
    CLIENT.get_mut()
  }
}
