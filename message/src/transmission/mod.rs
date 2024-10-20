use connect::ConnectMessage;
use media::MediaMessage;
use serde::{Deserialize, Serialize};
use signal::SignalMessage;

mod connect;
mod media;
mod signal;

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub enum TransmissionData {
  Connect(ConnectMessage),
  Media(MediaMessage),
  Signal(SignalMessage),
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Transmission {
  pub from: String,
  pub to: String,
  pub data: TransmissionData,
}

impl Transmission {
  pub fn new(
    from: String,
    to: String,
    data: TransmissionData,
  ) -> Self {
    Self {
      from,
      to,
      data,
    }
  }
}
