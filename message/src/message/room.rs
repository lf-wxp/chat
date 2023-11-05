use nanoid::nanoid;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Room {
  desc: Option<String>,
  users: Vec<String>,
  uuid: String,
  name: String,
  passwd: Option<String>,
}

impl Room {
  pub fn new(name: String, desc: Option<String>, passwd: Option<String>) -> Room {
    Room {
      desc,
      users: vec![],
      uuid: nanoid!(),
      name,
      passwd,
    }
  }
  pub fn uuid(&self) -> String {
    self.uuid.clone()
  }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CreateRoom {
  pub name: String,
  pub desc: Option<String>,
  pub passwd: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RemoveRoom {
  pub uuid: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ListRoom;

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub enum RoomAction {
  Create(CreateRoom),
  Remove(RemoveRoom),
  List(ListRoom),
}
