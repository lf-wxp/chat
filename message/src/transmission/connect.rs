use nanoid::nanoid;
use serde::{Deserialize, Serialize};

use super::media::MediaType;

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
  pub state: ConnectState,
  pub media_type: Option<MediaType>,
  pub session_id: String,
}

impl ConnectMessage {
  pub fn new(state: ConnectState, media_type: Option<MediaType>) -> Self {
    Self {
      state,
      media_type,
      session_id: nanoid!(),
    }
  }

  pub fn update_state(&mut self, state: ConnectState) {
    self.state = state;
  }
}
