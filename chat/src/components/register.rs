use bounce::use_atom_value;
use gloo_console::log;
use yew::prelude::*;
use yew_hooks::use_effect_once;

use crate::{components::DialogPortal, store::User};

#[function_component]
pub fn Register() -> Html {
  let current_user = use_atom_value::<User>();
  let visible = use_state(|| false);
  let get_name = |name: String| {
    if name == current_user.name {
      None
    } else {
      Some(name)
    }
  };
  let visible_clone = visible.clone();
  use_effect_once(move || {
    visible_clone.set(true);
    log!("set visible true");
    || {}
  });
  
  let visible_clone = visible.clone();
  let onclose = move |_| {
    visible_clone.set(false);
  };

  html! {
    <DialogPortal visible={*visible} onclose={onclose}>
      { "click hello"}
    </DialogPortal>
  }
}
