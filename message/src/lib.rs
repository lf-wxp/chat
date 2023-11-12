pub mod action_channel;
pub mod message;
pub mod signal_channel;

pub use action_channel::*;
pub use message::*;
pub use signal_channel::*;

pub trait SignalSend {
  fn send_offer(&mut self, _sdp: String) {}
  fn send_answer(&mut self, _sdp: String) {}
  fn send_ice(&mut self, _ice: String) {}
}

pub trait Channel {
  fn send(&mut self, _message: &str) {}
  fn onmessage(&mut self, _callback: Box<dyn Fn(&str)>) {}
}
