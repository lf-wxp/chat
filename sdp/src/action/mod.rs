pub mod client;
pub mod connect;
pub mod list;
pub mod media;
pub mod room;
pub mod signal;

use message::{Action, ListAction, MessageType, RequestMessage, RequestMessageData, ResponseMessage};
use tokio_tungstenite::tungstenite::{self, Message};

use crate::data::get_client_map;

pub trait ParamResponseExecute {
  fn execute(&self, client_id: String, session_id: String) -> ResponseMessage;
}

pub trait UnicastMessage {
  fn get_to(&self) -> String;
  fn get_message(&self, session_id: String, message_type: MessageType) -> String;
}

pub trait BroadcastMessage {
  fn get_message(&self, session_id: String) -> String;
}

pub trait UnicastExecute {
  fn execute(&self, session_id: String, message_type: MessageType)
  where
    Self: UnicastMessage,
  {
    if let Some(peers) = get_client_map() {
      let target_peer = peers.get(&self.get_to()).unwrap();
      target_peer
        .tx
        .send(Message::Text(self.get_message(session_id, message_type)))
        .unwrap();
    }
  }
}

pub trait BroadcastExecute {
  fn execute(&self, session_id: String)
  where
    Self: BroadcastMessage,
  {
    if let Some(peers) = get_client_map() {
      let broadcast_recipients = peers.iter().map(|(_, ws_sink)| ws_sink);
      for rec in broadcast_recipients {
        let message = self.get_message(session_id.clone());
        rec.tx.send(Message::Text(message)).unwrap();
      }
    }
  }
}

pub trait ResponseExecute {
  fn execute(&self, session_id: String) -> ResponseMessage;
}

pub trait ParamResponseOptionExecute {
  fn execute(&self, client_id: String, session_id: Option<String>) -> Option<ResponseMessage>;
}

impl ParamResponseExecute for Action {
  fn execute(&self, client_id: String, session_id: String) -> ResponseMessage {
    match self {
      Action::Room(room_action) => room_action.execute(session_id),
      Action::Client(client_action) => client_action.execute(client_id, session_id),
      Action::List(list_action) => list_action.execute(session_id),
    }
  }
}

impl ParamResponseOptionExecute for RequestMessage {
  fn execute(&self, client_id: String, _session_id: Option<String>) -> Option<ResponseMessage> {
    let session_id = self.session_id.clone();
    let message_type = self.message_type.clone();
    match &self.message {
      RequestMessageData::Action(action) => Some(action.execute(client_id, session_id)),
      RequestMessageData::Media(media) => {
        media.execute(session_id, message_type);
        None
      }
      RequestMessageData::Connect(connect) => {
        connect.execute(session_id, message_type);
        None
      }
      RequestMessageData::Signal(signal) => {
        signal.execute(session_id, message_type);
        None
      }
    }
  }
}

pub fn msg_try_into(message: ResponseMessage) -> Result<tungstenite::Message, serde_json::Error> {
  Ok(tungstenite::Message::Text(serde_json::to_string(&message)?))
}
