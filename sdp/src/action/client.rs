use message::{
  ActionMessage, Client, ClientAction, Data, GetInfo, ListClient, ResponseMessage, State,
  UpdateName,
};

use crate::data::get_client_map;

impl ClientExecute for UpdateName {
  fn execute(&self, client_id: String) -> ResponseMessage {
    match get_client_map() {
      Some(map) => match map.get_mut(&client_id) {
        Some(client) => {
          client.update_name(self.name.clone());
          ActionMessage::to_resp_msg(State::Success, "success".to_owned(), None)
        }
        None => ActionMessage::to_resp_msg(State::Error, "update client name".to_owned(), None),
      },
      None => ActionMessage::to_resp_msg(State::Error, "update client name".to_owned(), None),
    }
  }
}

impl ClientExecute for ListClient {
  fn execute(&self, _client_id: String) -> ResponseMessage {
    match get_client_map() {
      Some(map) => {
        let list = map.values().map(Client::from).collect::<Vec<Client>>();
        ActionMessage::to_resp_msg(
          State::Success,
          "success".to_owned(),
          Some(Data::ClientList(list)),
        )
      }
      None => ActionMessage::to_resp_msg(State::Error, "error list room".to_owned(), None),
    }
  }
}

impl ClientExecute for GetInfo {
  fn execute(&self, client_id: String) -> ResponseMessage {
    match get_client_map() {
      Some(map) => {
        if let Some(client) = map.get(&client_id) {
          return ActionMessage::to_resp_msg(
            State::Success,
            "success".to_owned(),
            Some(Data::Client(Client::from(client))),
          );
        }
        ActionMessage::to_resp_msg(State::Error, "error get client info".to_owned(), None)
      }
      None => ActionMessage::to_resp_msg(State::Error, "error get client info".to_owned(), None),
    }
  }
}

impl ClientExecute for ClientAction {
  fn execute(&self, client_id: String) -> ResponseMessage {
    match self {
      ClientAction::UpdateName(update_name) => update_name.execute(client_id),
      ClientAction::ListClient(list_client) => list_client.execute(client_id),
      ClientAction::GetInfo(get_info) => get_info.execute(client_id),
    }
  }
}

pub trait ClientExecute {
  fn execute(&self, client_id: String) -> ResponseMessage;
}
