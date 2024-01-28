use message::{ActionMessage, Client, ListMessage, ListResponse, Room};

use crate::data::{get_client_map, get_room_map};

use super::{BroadcastExecute, BroadcastMessage};

impl BroadcastExecute for ListResponse {}
impl BroadcastMessage for ListResponse {
  fn get_message(&self, session_id: String) -> String {
    let room_list = get_room_map().map_or(vec![], |x| x.values().cloned().collect::<Vec<Room>>());
    let client_list = get_client_map().map_or(vec![], |x| {
      x.values().map(Client::from).collect::<Vec<Client>>()
    });
    let list = ListMessage {
      room_list,
      client_list,
    };
    serde_json::to_string(&ActionMessage::to_resp_msg(
      session_id,
      ActionMessage::ListMessage(list.clone()),
    ))
    .unwrap_or("".to_string())
  }
}
