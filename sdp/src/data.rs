use std::{
  collections::HashMap,
  sync::OnceLock,
};

use message::Room;

use crate::client::Client;

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
