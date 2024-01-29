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

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub enum MessageType {
  Request,
  Response,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RequestMessage {
  pub session_id: String,
  pub message: RequestMessageData,
  pub message_type: MessageType,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub enum RequestMessageData {
  Action(Action),
  Connect(ConnectMessage),
  Media(MediaMessage),
  Signal(SignalMessage),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ResponseMessage {
  pub session_id: String,
  pub message: ResponseMessageData,
  pub message_type: MessageType,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub enum ResponseMessageData {
  Action(ActionMessage),
  Connect(ConnectMessage),
  Media(MediaMessage),
  Signal(SignalMessage),
}
