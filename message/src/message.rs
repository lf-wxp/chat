use serde::{Deserialize, Serialize};
use nanoid::nanoid;

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CreateRoom {
  pub name: String,
  pub desc: Option<String>,
  pub passwd: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RemoveRoom {
  pub uuid: String,
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

impl Room {
  pub fn new(name: String, desc: Option<String>, passwd: Option<String>) -> Room {
    Room { desc, users: vec![], uuid: nanoid!(), name, passwd }
  }
  pub fn uuid(&self) -> String {
    self.uuid.clone()
  }
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
  pub name: String,
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
  pub from: String,
  pub message: SdpMessage,
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

impl SdpMessage {
  pub fn is_empty(&self) -> bool {
    self.sdp.is_empty()
  }
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
pub struct TransmitMessage {
  pub from: String,
  pub message: SdpMessage,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub enum WsMessage {
  Action(Action),
  Transmit(Transmit),
  Connect(Connect),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ConnectInfo {
  pub room_list: Vec<Room>,
  pub client_list: Vec<ClientInfo>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub enum Data {
  RoomList(Vec<Room>),
  ClientList(Vec<ClientInfo>),
  ClientInfo(ClientInfo),
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

impl WsResponse {
  pub fn new(state: State, message: String, data: Option<Data>) -> WsResponse {
    WsResponse {
      state,
      message,
      data,
    }
  }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ClientInfo {
  pub name: String,
  pub uuid: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Connect;
