pub mod action_channel;
pub mod connect_channel;
pub mod message;
pub mod signal_channel;
pub mod media_channel;
pub mod channel;

pub use action_channel::*;
pub use connect_channel::*;
pub use message::*;
pub use signal_channel::*;
pub use media_channel::*;
pub use channel::*;

pub trait Signal {
  fn send_offer(&mut self, _sdp: String);
  fn send_answer(&mut self, _sdp: String);
  fn send_ice(&mut self, _ice: String);
  fn set_receive_offer(&mut self, callback: Box<dyn Fn(String)>);
  fn set_receive_answer(&mut self, callback: Box<dyn Fn(String)>);
  fn set_receive_ice(&mut self, callback: Box<dyn Fn(String)>);
}
