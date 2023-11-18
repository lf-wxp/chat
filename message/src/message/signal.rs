use serde::{Deserialize, Serialize};

use crate::ResponseMessage;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub enum SdpType {
  Offer,
  Answer,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SdpMessage {
  pub sdp_type: SdpType,
  pub sdp: String,
}

impl SdpMessage {
  pub fn is_empty(&self) -> bool {
    self.sdp.is_empty()
  }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub enum CastMessage {
  Sdp(SdpMessage),
  Ice(String),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SignalMessage {
  pub from: String,
  pub to: String,
  pub message: CastMessage,
}

impl SignalMessage {
  pub fn new(from: String, to: String, message: CastMessage) -> SignalMessage {
    SignalMessage { from, to, message }
  }
}
