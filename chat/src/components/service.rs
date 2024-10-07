use yew::{function_component, html, Html};

use crate::hook::{use_init, use_theme, use_ws_message_watch};

#[function_component]
pub fn Service() -> Html {
  use_init();
  use_theme();
  use_ws_message_watch();
  html! {}
}
