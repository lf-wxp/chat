pub mod chat_history;
pub mod message;

pub use chat_history::*;
pub use message::*;

#[derive(PartialEq, Clone)]
pub struct Option<T = String> {
  pub label: String,
  pub value: T,
}
