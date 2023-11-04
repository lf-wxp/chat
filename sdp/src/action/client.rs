use message::{ClientAction, ClientInfo, Data, GetInfo, ListClient, State, UpdateName, WsResponse};

use crate::data::get_client_map;

impl ClientExecute for UpdateName {
  fn execute(&self, client_id: String) -> WsResponse {
    match get_client_map() {
      Some(map) => match map.get_mut(&client_id) {
        Some(client) => {
          client.update_name(self.name.clone());
          WsResponse::new(State::success, "success".to_owned(), None)
        }
        None => WsResponse::new(State::error, "update client name".to_owned(), None),
      },
      None => WsResponse::new(State::error, "update client name".to_owned(), None),
    }
  }
}

impl ClientExecute for ListClient {
  fn execute(&self, _client_id: String) -> WsResponse {
    match get_client_map() {
      Some(map) => {
        let list = map
          .values()
          .map(ClientInfo::from)
          .collect::<Vec<ClientInfo>>();
        WsResponse::new(
          State::success,
          "success".to_owned(),
          Some(Data::ClientList(list)),
        )
      }
      None => WsResponse::new(State::error, "error list room".to_owned(), None),
    }
  }
}

impl ClientExecute for GetInfo {
  fn execute(&self, client_id: String) -> WsResponse {
    match get_client_map() {
      Some(map) => {
        if let Some(client) = map.get(&client_id) {
          return WsResponse::new(
            State::success,
            "success".to_owned(),
            Some(Data::ClientInfo(ClientInfo::from(client))),
          );
        }
        WsResponse::new(State::error, "error get client info".to_owned(), None)
      }
      None => WsResponse::new(State::error, "error get client info".to_owned(), None),
    }
  }
}

impl ClientExecute for ClientAction {
  fn execute(&self, client_id: String) -> WsResponse {
    match self {
      ClientAction::UpdateName(update_name) => update_name.execute(client_id),
      ClientAction::ListClient(list_client) => list_client.execute(client_id),
      ClientAction::GetInfo(get_info) => get_info.execute(client_id),
    }
  }
}

pub trait ClientExecute {
  fn execute(&self, client_id: String) -> WsResponse;
}
