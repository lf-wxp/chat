use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum MediaType {
  #[default]
  Video,
  Audio,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MediaMessage {
  pub from: String,
  pub from_name: String,
  pub to: String,
  pub media_type: MediaType,
  pub confirm: Option<bool>,
}
