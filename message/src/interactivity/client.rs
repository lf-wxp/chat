use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ListClient;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GetInfo;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UpdateName {
  pub name: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub enum ClientAction {
  UpdateName(UpdateName),
  ListClient(ListClient),
  GetInfo(GetInfo),
}

impl ClientAction {
  pub fn update_name(name: String) -> Self {
    Self::UpdateName(UpdateName { name })
  }  
  pub fn list_client() -> Self {
    Self::ListClient(ListClient)
  }  
  pub fn get_info() -> Self {
    Self::GetInfo(GetInfo)
  }  
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Client {
  pub name: String,
  pub uuid: String,
}
