use yew::{function_component, Html, html};

use crate::hook::{use_theme, use_init, use_user_list_watch};

#[function_component]
pub fn Service() -> Html {
  use_init();
  use_theme();
  use_user_list_watch();
  html! { }
}
