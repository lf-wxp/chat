use serde::{Deserialize, Serialize};

use crate::{Client, ClientAction, Room, RoomAction, ResponseMessage};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub enum State {
  Success,
  Error,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub enum Action {
  Room(RoomAction),
  Client(ClientAction),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub enum Data {
  RoomList(Vec<Room>),
  ClientList(Vec<Client>),
  Client(Client),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ActionMessage {
  pub state: State,
  pub message: String,
  pub data: Option<Data>,
}

impl ActionMessage {
  pub fn new(state: State, message: String, data: Option<Data>) -> ActionMessage {
    ActionMessage {
      state,
      message,
      data,
    }
  }
  pub fn to_resp_msg(state: State, message: String, data: Option<Data>) -> ResponseMessage {
    ResponseMessage::Action(ActionMessage::new(state, message, data))
  }
}
