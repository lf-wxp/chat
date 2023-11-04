use message::{CreateRoom, Data, ListRoom, RemoveRoom, Room, RoomAction, State, WsResponse};

use crate::data::get_room_map;

impl RoomExecute for CreateRoom {
  fn execute(&self) -> WsResponse {
    let room = Room::new(
      self.name.to_owned(),
      self.desc.to_owned(),
      self.passwd.to_owned(),
    );
    match get_room_map() {
      Some(map) => {
        map.insert(room.uuid(), room);
        WsResponse::new(State::success, "success".to_owned(), None)
      }
      None => WsResponse::new(State::error, "create room error".to_owned(), None),
    }
  }
}

impl RoomExecute for RemoveRoom {
  fn execute(&self) -> WsResponse {
    let error = WsResponse::new(State::error, "remove room error".to_owned(), None);
    match get_room_map() {
      Some(map) => map.remove(&self.uuid).map_or(error.clone(), |_| {
        WsResponse::new(State::success, "success".to_owned(), None)
      }),
      None => error,
    }
  }
}

impl RoomExecute for ListRoom {
  fn execute(&self) -> WsResponse {
    match get_room_map() {
      Some(map) => {
        let list = map.values().cloned().collect::<Vec<Room>>();
        WsResponse::new(
          State::success,
          "success".to_owned(),
          Some(Data::RoomList(list)),
        )
      }
      None => WsResponse::new(State::error, "error list room".to_owned(), None),
    }
  }
}

impl RoomExecute for RoomAction {
  fn execute(&self) -> WsResponse {
    match self {
      RoomAction::Create(create_room) => create_room.execute(),
      RoomAction::Remove(remove_room) => remove_room.execute(),
      RoomAction::List(list_room) => list_room.execute(),
    }
  }
}

pub trait RoomExecute {
  fn execute(&self) -> WsResponse;
}
