use std::sync::OnceLock;

use crate::{model::ChatHistory, utils::{Client, Link}, store::User};

pub const IMAGE_FILE_SIZE: f64 = 10f64; // mb

static mut CHAT_HISTORY: OnceLock<ChatHistory> = OnceLock::new();

static mut CLIENT: OnceLock<Client> = OnceLock::new();

static mut LINK: OnceLock<Link> = OnceLock::new();

pub fn get_chat_history() -> Option<&'static mut ChatHistory> {
  unsafe {
    CHAT_HISTORY.get_or_init(ChatHistory::default);
    CHAT_HISTORY.get_mut()
  }
}

pub fn get_client() -> Option<&'static mut Client> {
  unsafe {
    CLIENT.get_or_init(|| Client::new(User::default()));
    CLIENT.get_mut()
  }
}

pub fn get_link() -> Option<&'static mut Link> {
  unsafe {
    LINK.get_or_init(Link::new);
    LINK.get_mut()
  }
}
