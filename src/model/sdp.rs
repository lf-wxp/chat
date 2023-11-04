use serde::{Deserialize, Serialize};

use crate::store::User;

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CreateRoom {
  name: String,
  desc: Option<String>,
  passwd: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RemoveRoom {
  uuid: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub enum State {
  success,
  error,
}
#[derive(Serialize, Deserialize, Clone, Debug)]  
#[serde(rename_all = "camelCase")]
pub struct Room {
  desc: Option<String>,
	users: Vec<String>,
	uuid: String,
	name: String,
	passwd: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ListRoom;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListRoomResponse {
  state: State,
  message: String,
  data: Vec<Room>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub enum RoomAction {
  Create(CreateRoom),
  Remove(RemoveRoom),
  List(ListRoom),
}


#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UpdateName {
  name: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ListClient;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GetInfo;

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub enum ClientAction {
  UpdateName(UpdateName),
  ListClient(ListClient),
  GetInfo(GetInfo),
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub enum Action {
  Room(RoomAction),
  Client(ClientAction),
}


#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Broadcast {
  from: String,
  message: SdpMessage,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub enum CallType {
  Offer,
  Answer,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SdpMessage {
  pub call_type: CallType,
  pub sdp: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Unicast {
  pub from: String,
  pub to: String,
  pub message: SdpMessage,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub enum Transmit {
  Broadcast(Broadcast),
  Unicast(Unicast),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct  TransmitMessage {
  pub from: String,
  pub message: SdpMessage,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub enum WsMessage {
  Action(Action),
  Transmit(Transmit),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ConnectInfo {
  pub room_list: Vec<Room>,
  pub client_list: Vec<User>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub enum Data {
  RoomList(Vec<Room>),
  ClientList(Vec<User>),
  ClientInfo(User),
  ConnectInfo(ConnectInfo),
  Transmit(TransmitMessage),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct WsResponse {
  pub state: State,
  pub message: String,
  pub data: Option<Data>,
}
