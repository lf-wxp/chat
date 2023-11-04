use yew::prelude::*;

use crate::hook::use_fake_set;

#[function_component]
pub fn FakeSet() -> Html {
  use_fake_set();
  html! {}
}
