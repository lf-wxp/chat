pub mod client;
pub mod transmit;
pub mod room;

use message::{Action, RequestMessage, ResponseMessage};
use tokio_tungstenite::tungstenite;

use crate::action::{
  client::ClientExecute, transmit::TransmitExecute, room::RoomExecute,
};

impl ActionExecute for Action {
  fn execute(&self, client_id: String) -> ResponseMessage {
    match self {
      Action::Room(room_action) => room_action.execute(),
      Action::Client(client_action) => client_action.execute(client_id),
    }
  }
}
pub trait ActionExecute {
  fn execute(&self, client_id: String) -> ResponseMessage;
}

impl Execute for RequestMessage {
  fn execute(&self, client_id: String) -> Option<ResponseMessage> {
    match self {
      RequestMessage::Action(action) => Some(action.execute(client_id)),
      RequestMessage::Transmit(transmit) => {
        transmit.execute();
        None
      }
    }
  }
}

pub trait Execute {
  fn execute(&self, client_id: String) -> Option<ResponseMessage>;
}

pub fn msg_try_into(message: ResponseMessage) -> Result<tungstenite::Message, serde_json::Error> {
  Ok(tungstenite::Message::Text(serde_json::to_string(&message)?))
}
