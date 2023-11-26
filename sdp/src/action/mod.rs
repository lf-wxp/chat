pub mod connect;
pub mod client;
pub mod list;
pub mod room;
pub mod signal;
pub mod media;

use message::{Action, RequestMessage, ResponseMessage};
use tokio_tungstenite::tungstenite::{self, Message};

use crate::data::get_client_map;

pub trait ParamResponseExecute {
  fn execute(&self, client_id: String) -> ResponseMessage;
}

pub trait UnicastMessage {
  fn get_to(&self) -> String;
  fn get_message(&self) -> String;
}

pub trait BroadcastMessage {
  fn get_message(&self) -> String;
}

pub trait UnicastExecute {
  fn execute(&self)
  where
    Self: UnicastMessage,
  {
    if let Some(peers) = get_client_map() {
      let target_peer = peers.get(&self.get_to()).unwrap();
      target_peer
        .tx
        .send(Message::Text(self.get_message()))
        .unwrap();
    }
  }
}

pub trait BroadcastExecute {
  fn execute(&self)
  where
    Self: BroadcastMessage,
  {
    if let Some(peers) = get_client_map() {
      let broadcast_recipients = peers.iter().map(|(_, ws_sink)| ws_sink);
      for rec in broadcast_recipients {
        let message = self.get_message();
        rec.tx.send(Message::Text(message)).unwrap();
      }
    }
  }
}

pub trait ResponseExecute {
  fn execute(&self) -> ResponseMessage;
}

pub trait ParamResponseOptionExecute {
  fn execute(&self, client_id: String) -> Option<ResponseMessage>;
}

impl ParamResponseExecute for Action {
  fn execute(&self, client_id: String) -> ResponseMessage {
    match self {
      Action::Room(room_action) => room_action.execute(),
      Action::Client(client_action) => client_action.execute(client_id),
    }
  }
}

impl ParamResponseOptionExecute for RequestMessage {
  fn execute(&self, client_id: String) -> Option<ResponseMessage> {
    match self {
      RequestMessage::Action(action) => Some(action.execute(client_id)),
      RequestMessage::Media(media) => {
        media.execute();
        None
      },
      RequestMessage::Connect(connect) => {
        connect.execute();
        None
      },
      RequestMessage::Signal(signal) => {
        signal.execute();
        None
      }
    }
  }
}

pub fn msg_try_into(message: ResponseMessage) -> Result<tungstenite::Message, serde_json::Error> {
  Ok(tungstenite::Message::Text(serde_json::to_string(&message)?))
}
