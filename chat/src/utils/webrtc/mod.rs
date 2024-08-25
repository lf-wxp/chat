mod future;
mod rtc;
mod rtc_link;

#[macro_use]
mod event;

pub use future::*;
pub use rtc_link::*;

use message::ConnectState;
use web_sys::RtcIceConnectionState;

pub fn to_connect_state(state: RtcIceConnectionState) -> ConnectState {
  match state {
    RtcIceConnectionState::New => ConnectState::New,
    RtcIceConnectionState::Checking => ConnectState::Checking,
    RtcIceConnectionState::Connected => ConnectState::Connected,
    RtcIceConnectionState::Completed => ConnectState::Completed,
    RtcIceConnectionState::Failed => ConnectState::Failed,
    RtcIceConnectionState::Disconnected => ConnectState::Disconnected,
    RtcIceConnectionState::Closed => ConnectState::Closed,
    _ => ConnectState::New,
  }
}



