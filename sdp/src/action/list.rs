use message::{ActionMessage, Client, ListAction, ListMessage, ResponseMessage, Room};

use crate::data::{get_client_map, get_room_map};

use super::ResponseExecute;

impl ResponseExecute for ListAction {
  fn execute(&self, session_id: String) -> ResponseMessage {
    let room_list = get_room_map().map_or(vec![], |x| x.values().cloned().collect::<Vec<Room>>());
    let client_list = get_client_map().map_or(vec![], |x| {
      x.values().map(Client::from).collect::<Vec<Client>>()
    });
    let list = ListMessage {
      room_list,
      client_list,
    };

    ActionMessage::to_resp_msg(session_id, ActionMessage::ListMessage(list))
  }
}
