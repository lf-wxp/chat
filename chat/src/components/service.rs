use yew::{function_component, Html, html};

use crate::hook::{use_theme, use_client_init};

#[function_component]
pub fn Service() -> Html {
  use_theme();
  use_client_init();
  html! { }
}
