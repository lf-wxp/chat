use message::{
  ActionMessage, CreateRoom, ListRoom, RemoveRoom, ResponseMessage, Room, RoomAction, State,
};

use crate::{action::ResponseExecute, data::get_room_map};

impl ResponseExecute for CreateRoom {
  fn execute(&self, session_id: String) -> ResponseMessage {
    let room = Room::new(
      self.name.to_owned(),
      self.desc.to_owned(),
      self.passwd.to_owned(),
    );
    match get_room_map() {
      Some(map) => {
        map.insert(room.uuid(), room);
        ActionMessage::to_resp_msg(session_id, ActionMessage::Success)
      }
      None => ActionMessage::to_resp_msg(session_id, ActionMessage::Error),
    }
  }
}

impl ResponseExecute for RemoveRoom {
  fn execute(&self, session_id: String) -> ResponseMessage {
    let error = ActionMessage::to_resp_msg(session_id.clone(), ActionMessage::Error);
    match get_room_map() {
      Some(map) => map.remove(&self.uuid).map_or(error.clone(), |_| {
        ActionMessage::to_resp_msg(session_id, ActionMessage::Success)
      }),
      None => error,
    }
  }
}

impl ResponseExecute for ListRoom {
  fn execute(&self, session_id: String) -> ResponseMessage {
    match get_room_map() {
      Some(map) => {
        let list = map.values().cloned().collect::<Vec<Room>>();
        ActionMessage::to_resp_msg(session_id, ActionMessage::RoomList(list))
      }
      None => ActionMessage::to_resp_msg(session_id, ActionMessage::Error),
    }
  }
}

impl ResponseExecute for RoomAction {
  fn execute(&self, session_id: String) -> ResponseMessage {
    match self {
      RoomAction::Create(create_room) => create_room.execute(session_id),
      RoomAction::Remove(remove_room) => remove_room.execute(session_id),
      RoomAction::List(list_room) => list_room.execute(session_id),
    }
  }
}
