mod action;
mod client;
mod room;
mod media;
mod signal;
mod connect;

pub use action::*;
pub use client::*;
pub use room::*;
pub use media::*;
pub use signal::*;
pub use connect::*;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub enum RequestMessage {
  Action(Action),
  Connect(ConnectMessage),
  Media(MediaMessage),
  Signal(SignalMessage),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub enum ResponseMessage {
  Action(ActionMessage),
  Connect(ConnectMessage),
  Media(MediaMessage),
  Signal(SignalMessage),
}


#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ListResponse;
