pub mod message;
#[macro_use]
pub mod channel;

// pub use action_channel::*;
// pub use connect_channel::*;
pub use message::*;
// pub use signal_channel::*;
// pub use media_channel::*;

channel!(
  ActionChannel,
  Action,
  RequestMessage::Action,
  ActionMessage,
  ResponseMessage::Action
);

channel!(
  ConnectChannel,
  ConnectMessage,
  RequestMessage::Connect,
  ConnectMessage,
  ResponseMessage::Connect
);
channel!(
  MediaChannel,
  MediaMessage,
  RequestMessage::Media,
  MediaMessage,
  ResponseMessage::Media
);

pub trait Signal {
  fn send_offer(&mut self, _sdp: String);
  fn send_answer(&mut self, _sdp: String);
  fn send_ice(&mut self, _ice: String);
  fn set_receive_offer(&mut self, callback: Box<dyn Fn(String)>);
  fn set_receive_answer(&mut self, callback: Box<dyn Fn(String)>);
  fn set_receive_ice(&mut self, callback: Box<dyn Fn(String)>);
}
