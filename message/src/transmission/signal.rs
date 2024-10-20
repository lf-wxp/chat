use serde::{Deserialize, Serialize};

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
pub enum SignalMessage {
  Sdp(SdpMessage),
  Ice(String),
}
