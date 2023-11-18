use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub enum CallType {
  Video,
  Audio,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CallMessage {
  pub from: String,
  pub to: String,
  pub call_type: CallType,
  pub expired: Option<String>,
  pub confirm: Option<bool>,
}
