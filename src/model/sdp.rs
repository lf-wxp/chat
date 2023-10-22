use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateRoom {
  name: String,
  desc: Option<String>,
  passwd: Option<String>,
}

#[derive(Serialize, Deserialize)]
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

#[derive(Serialize, Deserialize)]
pub struct ListRoom;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListRoomResponse {
  state: State,
  message: String,
  data: Vec<Room>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RoomAction {
  Create(CreateRoom),
  Remove(RemoveRoom),
  List(ListRoom),
}


#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UpdateName {
  name: String,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ListClient;

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GetInfo;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ClientAction {
  UpdateName(UpdateName),
  ListClient(ListClient),
  GetInfo(GetInfo),
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Action {
  Room(RoomAction),
  Client(ClientAction),
}


#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Broadcast {
  from: String,
  message: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Unicast {
  from: String,
  pub to: String,
  message: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Transmit {
  Broadcast(Broadcast),
  Unicast(Unicast),
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SdpMessage {
  Action(Action),
  Transmit(Transmit),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ClientInfo {
  pub name: String,
  pub uuid: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub enum Data {
  RoomList(Vec<Room>),
  ClientList(Vec<ClientInfo>),
  ClientInfo(ClientInfo),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SdpResponse {
  pub state: State,
  pub message: String,
  pub data: Option<Data>,
}
