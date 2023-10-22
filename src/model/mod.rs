pub mod chat_history;
pub mod message;
pub mod sdp;

pub use chat_history::*;
pub use message::*;
pub use sdp::*;

pub type Error = Box<dyn std::error::Error>;
pub type UResult<T> = std::result::Result<T, Error>;
#[derive(PartialEq, Clone)]
pub struct Option<T = String> {
  pub label: String,
  pub value: T,
}

#[derive(Clone)]
pub struct VisualizeColor {
  pub background: String,
  pub rect_color: String,
  pub opacity: f64,
}
