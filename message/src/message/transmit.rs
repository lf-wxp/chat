use serde::{Serialize, Deserialize};

use crate::{Room, Client, ResponseMessage};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ListMessage {
  pub room_list: Vec<Room>,
  pub client_list: Vec<Client>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct List {}

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

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub enum CallType {
  Video,
  Audio,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CallMessage {
  pub call_type: CallType,
  pub expired: String,
  pub confirm: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub enum CastMessage {
  Sdp(SdpMessage),
  Call(CallMessage),
  List(ListMessage),
  Ice(String),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Unicast {
  pub from: String,
  pub to: String,
  pub message: CastMessage,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Broadcast {
  pub from: String,
  pub message: CastMessage,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub enum Transmit {
  Broadcast(Broadcast),
  Unicast(Unicast),
  List(List),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TransmitMessage {
  pub from: String,
  pub message: CastMessage,
}

impl TransmitMessage {
  pub fn new(from: String, message: CastMessage) -> TransmitMessage {
    TransmitMessage { from, message }
  }

  pub fn to_resp_msg(from: String, message: CastMessage) -> ResponseMessage {
    ResponseMessage::Transmit(TransmitMessage { from, message })
  }
}
