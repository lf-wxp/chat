use message::{
  ActionMessage, CreateRoom, Data, ListRoom, RemoveRoom, ResponseMessage, Room, RoomAction, State,
};

use crate::{action::ResponseExecute, data::get_room_map};

impl ResponseExecute for CreateRoom {
  fn execute(&self) -> ResponseMessage {
    let room = Room::new(
      self.name.to_owned(),
      self.desc.to_owned(),
      self.passwd.to_owned(),
    );
    match get_room_map() {
      Some(map) => {
        map.insert(room.uuid(), room);
        ActionMessage::to_resp_msg(State::Success, "success".to_owned(), None)
      }
      None => ActionMessage::to_resp_msg(State::Error, "create room error".to_owned(), None),
    }
  }
}

impl ResponseExecute for RemoveRoom {
  fn execute(&self) -> ResponseMessage {
    let error = ActionMessage::to_resp_msg(State::Error, "remove room error".to_owned(), None);
    match get_room_map() {
      Some(map) => map.remove(&self.uuid).map_or(error.clone(), |_| {
        ActionMessage::to_resp_msg(State::Success, "success".to_owned(), None)
      }),
      None => error,
    }
  }
}

impl ResponseExecute for ListRoom {
  fn execute(&self) -> ResponseMessage {
    match get_room_map() {
      Some(map) => {
        let list = map.values().cloned().collect::<Vec<Room>>();
        ActionMessage::to_resp_msg(
          State::Success,
          "success".to_owned(),
          Some(Data::RoomList(list)),
        )
      }
      None => ActionMessage::to_resp_msg(State::Error, "error list room".to_owned(), None),
    }
  }
}

impl ResponseExecute for RoomAction {
  fn execute(&self) -> ResponseMessage {
    match self {
      RoomAction::Create(create_room) => create_room.execute(),
      RoomAction::Remove(remove_room) => remove_room.execute(),
      RoomAction::List(list_room) => list_room.execute(),
    }
  }
}
