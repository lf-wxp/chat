use bounce::use_atom_value;
use gloo_console::log;
use stylist::{self, style};
use yew::prelude::*;
use yew_hooks::use_effect_once;

use crate::{
  components::{Button, DialogPortal, Input},
  hook::use_i18n,
  store::User,
  utils::style,
};

#[function_component]
pub fn Register() -> Html {
  let class_name = get_class_name();
  let current_user = use_atom_value::<User>();
  let visible = use_state(|| false);
  let i18n = use_i18n();
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
  let header = i18n.t("input your name");
  let confirm_text = i18n.t("confirm");
  let cancel_text = i18n.t("cancel");

  html! {
    <DialogPortal  visible={*visible} onclose={onclose} {header}>
      <div class={class_name}>
        <Input />
        <div class="button">
          <Button>
          { cancel_text}
          </Button>
          <Button>
          { confirm_text}
          </Button>
        </div>
      </div>
    </DialogPortal>
  }
}

#[allow(non_upper_case_globals)]
fn get_class_name() -> String {
  style::get_class_name(style!(
    r#"
      .button {
        display: flex;
        flex-flow: row nowrap;
        justify-content: flex-end;
        margin-block-start: 20px;
      }
    "#
  ))
}
