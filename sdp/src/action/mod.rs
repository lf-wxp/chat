pub mod client;
pub mod connect;
pub mod response;
pub mod room;

use message::{Action, WsMessage, WsResponse};
use tokio_tungstenite::tungstenite;

use crate::action::{
  client::ClientExecute, connect::ConnectExecute, response::TransmitExecute, room::RoomExecute,
};

impl ActionExecute for Action {
  fn execute(&self, client_id: String) -> WsResponse {
    match self {
      Action::Room(room_action) => room_action.execute(),
      Action::Client(client_action) => client_action.execute(client_id),
    }
  }
}
pub trait ActionExecute {
  fn execute(&self, client_id: String) -> WsResponse;
}

impl Execute for WsMessage {
  fn execute(&self, client_id: String) -> WsResponse {
    match self {
      WsMessage::Action(action) => action.execute(client_id),
      WsMessage::Transmit(transmit) => transmit.execute(),
      WsMessage::Connect(connect) => connect.execute(),
    }
  }
}

pub trait Execute {
  fn execute(&self, client_id: String) -> WsResponse;
}

pub fn msg_try_into(message: WsResponse) -> Result<tungstenite::Message, serde_json::Error> {
  Ok(tungstenite::Message::Text(serde_json::to_string(&message)?))
}
