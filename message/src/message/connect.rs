use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ConnectState {
  New,
  Checking,
  Connected,
  Completed,
  Failed,
  Disconnected,
  Closed,
}
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ConnectMessage {
  pub from: String,
  pub to: String,
  pub state: ConnectState,
}
