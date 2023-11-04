use message::{Broadcast, State, Transmit, TransmitMessage, Unicast, WsResponse};
use tokio_tungstenite::tungstenite::Message;

use crate::data::get_client_map;

impl TransmitExecute for Broadcast {
  fn execute(&self) -> WsResponse {
    match get_client_map() {
      Some(peers) => {
        let broadcast_recipients = peers
          .iter()
          .filter(|(uuid, _)| uuid != &&self.from)
          .map(|(_, ws_sink)| ws_sink);

        println!(
          "broadcast count, {:?}",
          broadcast_recipients.clone().count()
        );

        for recp in broadcast_recipients {
          if !self.message.is_empty() {
            let message = serde_json::to_string(&TransmitMessage {
              from: self.from.clone(),
              message: self.message.clone(),
            })
            .unwrap();
            recp.tx.send(Message::Text(message)).unwrap();
          };
        }
        WsResponse::new(State::success, "ok broadcast".to_owned(), None)
      }
      None => WsResponse::new(State::error, "get client map error".to_owned(), None),
    }
  }
}

impl TransmitExecute for Unicast {
  fn execute(&self) -> WsResponse {
    match get_client_map() {
      Some(peers) => {
        let target_peer = peers.get(&self.to).unwrap();
        let message = serde_json::to_string(&TransmitMessage {
          from: self.from.clone(),
          message: self.message.clone(),
        })
        .unwrap();
        target_peer.tx.send(Message::Text(message)).unwrap();

        WsResponse::new(State::success, "ok unicast".to_owned(), None)
      }
      None => WsResponse::new(State::error, "get client map error".to_owned(), None),
    }
  }
}

impl TransmitExecute for Transmit {
  fn execute(&self) -> WsResponse {
    match self {
      Transmit::Broadcast(broadcast) => broadcast.execute(),
      Transmit::Unicast(unicast) => unicast.execute(),
    }
  }
}

pub trait TransmitExecute {
  fn execute(&self) -> WsResponse;
}
