mod action;
mod client;
mod room;
// mod transmit;
mod call;
mod signal;

pub use action::*;
pub use client::*;
pub use room::*;
// pub use transmit::*;
pub use call::*;
pub use signal::*;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub enum RequestMessage {
  Action(Action),
  Call(CallMessage),
  Signal(SignalMessage),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub enum ResponseMessage {
  Action(ActionMessage),
  Call(CallMessage),
  Signal(SignalMessage),
}


#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ListResponse;
