use yew::{function_component, html, Html};

use crate::hook::{
  use_client_message_watch, use_init, use_media_request_watch, use_theme, use_user_list_watch,
};

#[function_component]
pub fn Service() -> Html {
  use_init();
  use_theme();
  use_user_list_watch();
  use_media_request_watch();
  use_client_message_watch();
  html! {}
}
