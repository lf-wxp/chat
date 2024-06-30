use std::{collections::HashMap, sync::OnceLock};

use message::{self, Room};

use crate::Client;

type ClientMap = HashMap<String, Client>;
type RoomMap = HashMap<String, Room>;

static mut CLIENT_MAP: OnceLock<ClientMap> = OnceLock::new();
static mut ROOM_MAP: OnceLock<RoomMap> = OnceLock::new();

pub fn get_client_map() -> Option<&'static mut ClientMap> {
  unsafe {
    CLIENT_MAP.get_or_init(ClientMap::default);
    CLIENT_MAP.get_mut()
  }
}

pub fn get_room_map() -> Option<&'static mut RoomMap> {
  unsafe {
    ROOM_MAP.get_or_init(RoomMap::default);
    ROOM_MAP.get_mut()
  }
}

pub fn get_client_list() -> Vec<message::Client> {
  get_client_map().map_or(vec![], |x| {
    x.values()
      .map(message::Client::from)
      .collect::<Vec<message::Client>>()
  })
}

pub fn get_room_list() -> Vec<Room> {
  get_room_map().map_or(vec![], |x| x.values().cloned().collect::<Vec<Room>>())
}
