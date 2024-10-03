use std::fmt::{self, Display, Formatter};
use serde::{Deserialize, Serialize};
use wasm_bindgen::JsValue;
use web_sys::{RtcIceCandidate, RtcIceCandidateInit};
use yew_icons::IconId;

pub mod chat_history;
pub mod channel_message;

pub use chat_history::*;
pub use channel_message::*;


pub type Error = Box<dyn std::error::Error>;
pub type UResult<T> = std::result::Result<T, Error>;
#[derive(PartialEq, Clone)]
pub struct Option<T = String> {
  pub label: String,
  pub value: T,
  pub icon: std::option::Option<IconId>
}

#[derive(Clone)]
pub struct VisualizeColor {
  pub background: String,
  pub rect_color: String,
  pub opacity: f64,
}

type CoreOption<T> = std::option::Option<T>;
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct IceCandidate {
  pub candidate: String,
  pub sdp_mid: CoreOption<String>,
  pub sdp_m_line_index: CoreOption<u16>,
  pub username_fragment: CoreOption<String>,
}

impl TryFrom<IceCandidate> for RtcIceCandidate {
  type Error = JsValue;
  fn try_from(value: IceCandidate) -> Result<Self, Self::Error> {
    let ice_candidate_init = RtcIceCandidateInit::new(&value.candidate);
    ice_candidate_init.set_candidate(&value.candidate);
    ice_candidate_init.set_sdp_mid(value.sdp_mid.as_deref());
    ice_candidate_init.set_sdp_m_line_index(value.sdp_m_line_index);
    RtcIceCandidate::new(&ice_candidate_init)
  }
}

#[derive(PartialEq, Default)]
pub enum Size {
  Small,
  #[default]
  Media,
  Large,
}

impl Display for Size {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    let t = match self {
      Size::Small => "small",
      Size::Media => "media",
      Size::Large => "large",
    };
    write!(f, "{}", t)
  }
}
