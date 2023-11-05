mod action;
mod client;
mod room;
mod transmit;

pub use action::*;
pub use client::*;
pub use room::*;
pub use transmit::*;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub enum RequestMessage {
  Action(Action),
  Transmit(Transmit),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub enum ResponseMessage {
  Action(ActionMessage),
  Transmit(TransmitMessage),
}

