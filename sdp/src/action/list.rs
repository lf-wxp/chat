use message::{ActionMessage, ListAction, ListMessage, ResponseMessage };

use crate::data::{get_client_list, get_room_list};

use super::ResponseExecute;

impl ResponseExecute for ListAction {
  fn execute(&self, session_id: String) -> ResponseMessage {
    let room_list = get_room_list();
    let client_list = get_client_list();
    let list = ListMessage {
      room_list,
      client_list,
    };

    ActionMessage::to_resp_msg(session_id, ActionMessage::ListMessage(list))
  }
}
