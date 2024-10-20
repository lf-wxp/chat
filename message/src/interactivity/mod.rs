use client::{Client, ClientAction};
use room::{CreateRoom, Room, RoomAction};
use serde::{Deserialize, Serialize};

mod client;
mod room;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ListMessage {
  pub room_list: Vec<Room>,
  pub client_list: Vec<Client>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub enum InteractivityInput {
  Client(ClientAction),
  Room(RoomAction),
}

impl InteractivityInput {
  pub fn update_client_name(name: String) -> Self {
    Self::Client(ClientAction::update_name(name))
  }
  pub fn list_client() -> Self {
    Self::Client(ClientAction::list_client())
  }
  pub fn get_client_info() -> Self {
    Self::Client(ClientAction::get_info())
  }
  pub fn list_room() -> Self {
    Self::Room(RoomAction::list())
  }
  pub fn remove_room(uuid: String) -> Self {
    Self::Room(RoomAction::remove(uuid))
  }
  pub fn create_room(info: CreateRoom) -> Self {
    Self::Room(RoomAction::Create(info))
  }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub enum InteractivityOutput {
  RoomList(Vec<Room>),
  ClientList(Vec<Client>),
  Client(Client),
  ListMessage(ListMessage),
  Error(Option<String>),
  Success,
}
