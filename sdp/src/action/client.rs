use message::{
  ActionMessage, Client, ClientAction, GetInfo, ListClient, ResponseMessage, UpdateName,
};

use crate::{action::ParamResponseExecute, data::get_client_map};

impl ParamResponseExecute for UpdateName {
  fn execute(&self, client_id: String, session_id: String) -> ResponseMessage {
    match get_client_map() {
      Some(map) => match map.get_mut(&client_id) {
        Some(client) => {
          client.update_name(self.name.clone());
          ActionMessage::to_resp_msg(session_id, ActionMessage::Success)
        }
        None => ActionMessage::to_resp_msg(session_id, ActionMessage::Error),
      },
      None => ActionMessage::to_resp_msg(session_id, ActionMessage::Error),
    }
  }
}

impl ParamResponseExecute for ListClient {
  fn execute(&self, _client_id: String, session_id: String) -> ResponseMessage {
    match get_client_map() {
      Some(map) => {
        let list = map.values().map(Client::from).collect::<Vec<Client>>();
        ActionMessage::to_resp_msg(session_id, ActionMessage::ClientList(list))
      }
      None => ActionMessage::to_resp_msg(session_id, ActionMessage::Error),
    }
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
        ActionMessage::to_resp_msg(session_id, ActionMessage::Error)
      }
      None => ActionMessage::to_resp_msg(session_id, ActionMessage::Error),
    }
  }
}

impl ParamResponseExecute for ClientAction {
  fn execute(&self, client_id: String, session_id: String) -> ResponseMessage {
    match self {
      ClientAction::UpdateName(update_name) => update_name.execute(client_id, session_id),
      ClientAction::ListClient(list_client) => list_client.execute(client_id, session_id),
      ClientAction::GetInfo(get_info) => get_info.execute(client_id, session_id),
    }
  }
}
