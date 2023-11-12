use message::{
  ActionMessage, Broadcast, Client, Data, ListMessage, ListResponse, Room, State, Transmit,
  TransmitMessage, Unicast,
};
use tokio_tungstenite::tungstenite::Message;

use crate::{
  action::VoidExecute,
  data::{get_client_map, get_room_map},
};

impl VoidExecute for Broadcast {
  fn execute(&self) {
    if let Some(peers) = get_client_map() {
      let broadcast_recipients = peers
        .iter()
        .filter(|(uuid, _)| uuid != &&self.from)
        .map(|(_, ws_sink)| ws_sink);

      println!(
        "broadcast count, {:?}",
        broadcast_recipients.clone().count()
      );

      for rec in broadcast_recipients {
        let message = serde_json::to_string(&TransmitMessage::to_resp_msg(
          self.from.clone(),
          self.message.clone(),
        ))
        .unwrap();
        rec.tx.send(Message::Text(message)).unwrap();
      }
    }
  }
}

impl VoidExecute for Unicast {
  fn execute(&self) {
    if let Some(peers) = get_client_map() {
      let target_peer = peers.get(&self.to).unwrap();
      let message = serde_json::to_string(&TransmitMessage::to_resp_msg(
        self.from.clone(),
        self.message.clone(),
      ))
      .unwrap();
      target_peer.tx.send(Message::Text(message)).unwrap();
    }
  }
}

impl VoidExecute for Transmit {
  fn execute(&self) {
    match self {
      Transmit::Broadcast(broadcast) => broadcast.execute(),
      Transmit::Unicast(unicast) => unicast.execute(),
    }
  }
}

impl VoidExecute for ListResponse {
  fn execute(&self) {
    if let Some(peers) = get_client_map() {
      let broadcast_recipients = peers.iter().map(|(_, ws_sink)| ws_sink);

      println!(
        "broadcast count, {:?}",
        broadcast_recipients.clone().count()
      );

      let room_list = get_room_map().map_or(vec![], |x| x.values().cloned().collect::<Vec<Room>>());
      let client_list = get_client_map().map_or(vec![], |x| {
        x.values().map(Client::from).collect::<Vec<Client>>()
      });
      let list = ListMessage {
        room_list,
        client_list,
      };

      for rec in broadcast_recipients {
        let message = serde_json::to_string(&ActionMessage::to_resp_msg(
          State::Success,
          "success".to_owned(),
          Some(Data::ListMessage(list.clone())),
        ))
        .unwrap();
        rec.tx.send(Message::Text(message)).unwrap();
      }
    }
  }
}
