use message::{ClientInfo, Connect, ConnectInfo, Data, Room, State, WsResponse};
use tokio_tungstenite::tungstenite::Message;

use crate::data::{get_client_map, get_room_map};

impl ConnectExecute for Connect {
  fn execute(&self) -> WsResponse {
    match get_client_map() {
      Some(peers) => {
        let broadcast_recipients = peers.iter().map(|(_, ws_sink)| ws_sink);

        println!(
          "broadcast count, {:?}",
          broadcast_recipients.clone().count()
        );

        let room_list =
          get_room_map().map_or(vec![], |x| x.values().cloned().collect::<Vec<Room>>());
        let client_list = get_client_map().map_or(vec![], |x| {
          x.values()
            .map(ClientInfo::from)
            .collect::<Vec<ClientInfo>>()
        });
        let connect_info = ConnectInfo {
          room_list,
          client_list,
        };

        let message = WsResponse::new(
          State::success,
          "ok".to_owned(),
          Some(Data::ConnectInfo(connect_info)),
        );

        for recp in broadcast_recipients {
          recp
            .tx
            .send(Message::Text(serde_json::to_string(&message).unwrap()))
            .unwrap();
        }

        message
      }
      None => WsResponse::new(
        State::error,
        "get room and client info error".to_owned(),
        None,
      ),
    }
  }
}
pub trait ConnectExecute {
  fn execute(&self) -> WsResponse;
}
