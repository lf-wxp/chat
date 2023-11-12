pub mod client;
pub mod room;
pub mod transmit;

use message::{Action, RequestMessage, ResponseMessage};
use tokio_tungstenite::tungstenite;

pub trait ParamResponseExecute {
  fn execute(&self, client_id: String) -> ResponseMessage;
}

pub trait VoidExecute {
  fn execute(&self);
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
      RequestMessage::Transmit(transmit) => {
        transmit.execute();
        None
      }
    }
  }
}

pub fn msg_try_into(message: ResponseMessage) -> Result<tungstenite::Message, serde_json::Error> {
  Ok(tungstenite::Message::Text(serde_json::to_string(&message)?))
}
