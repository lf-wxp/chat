use serde::{Deserialize, Serialize};

use crate::{Client, ClientAction, MessageType, ResponseMessage, ResponseMessageData, Room, RoomAction};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub enum State {
  Success,
  Error,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ListAction;

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub enum Action {
  Room(RoomAction),
  Client(ClientAction),
  List(ListAction),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ListMessage {
  pub room_list: Vec<Room>,
  pub client_list: Vec<Client>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub enum ActionMessage {
  RoomList(Vec<Room>),
  ClientList(Vec<Client>),
  Client(Client),
  ListMessage(ListMessage),
  Error,
  Success,
}
impl ActionMessage {
  pub fn to_resp_msg(session_id: String, message: ActionMessage) -> ResponseMessage {
    ResponseMessage {
      session_id,
      message: ResponseMessageData::Action(message),
      message_type: MessageType::Response,
    }
  }
}
