use message::{
  ActionMessage, Client, ClientAction, GetInfo, ListClient, ResponseMessage, UpdateName,
};

use crate::{
  action::{BroadcastExecute, BroadcastMessage, ParamResponseExecute},
  data::{get_client_list, get_client_map},
};

impl ParamResponseExecute for UpdateName {
  fn execute(&self, client_id: String, session_id: String) -> ResponseMessage {
    let id = session_id.clone();
    let client_list = get_client_list();
    let is_name_exist = client_list.iter().any(|item| item.name == self.name);
    if is_name_exist {
      return ActionMessage::to_resp_msg(
        session_id,
        ActionMessage::Error(Some("name exists".to_string())),
      );
    }
    let message = match get_client_map() {
      Some(map) => match map.get_mut(&client_id) {
        Some(client) => {
          client.update_name(self.name.clone());
          ActionMessage::to_resp_msg(session_id, ActionMessage::Success)
        }
        None => ActionMessage::to_resp_msg(session_id, ActionMessage::Error(None)),
      },
      None => ActionMessage::to_resp_msg(session_id, ActionMessage::Error(None)),
    };
    BroadcastExecute::execute(&ListClient {}, id);
    message
  }
}

impl ParamResponseExecute for ListClient {
  fn execute(&self, _client_id: String, session_id: String) -> ResponseMessage {
    match get_client_map() {
      Some(map) => {
        let list = map.values().map(Client::from).collect::<Vec<Client>>();
        ActionMessage::to_resp_msg(session_id, ActionMessage::ClientList(list))
      }
      None => ActionMessage::to_resp_msg(session_id, ActionMessage::Error(None)),
    }
  }
}
impl BroadcastExecute for ListClient {}
impl BroadcastMessage for ListClient {
  fn get_message(&self, session_id: String) -> Vec<u8> {
    let message = match get_client_map() {
      Some(map) => {
        let list = map.values().map(Client::from).collect::<Vec<Client>>();
        ActionMessage::to_resp_msg(session_id, ActionMessage::ClientList(list))
      }
      None => ActionMessage::to_resp_msg(session_id, ActionMessage::Error(None)),
    };
    bincode::serialize(&message).unwrap()
  }
}

impl ParamResponseExecute for GetInfo {
  fn execute(&self, client_id: String, session_id: String) -> ResponseMessage {
    match get_client_map() {
      Some(map) => {
        if let Some(client) = map.get(&client_id) {
          return ActionMessage::to_resp_msg(
            session_id,
            ActionMessage::Client(Client::from(client)),
          );
        }
        ActionMessage::to_resp_msg(session_id, ActionMessage::Error(None))
      }
      None => ActionMessage::to_resp_msg(session_id, ActionMessage::Error(None)),
    }
  }
}

impl ParamResponseExecute for ClientAction {
  fn execute(&self, client_id: String, session_id: String) -> ResponseMessage {
    match self {
      ClientAction::UpdateName(update_name) => update_name.execute(client_id, session_id),
      ClientAction::ListClient(list_client) => {
        ParamResponseExecute::execute(list_client, client_id, session_id)
      }
      ClientAction::GetInfo(get_info) => get_info.execute(client_id, session_id),
    }
  }
}
