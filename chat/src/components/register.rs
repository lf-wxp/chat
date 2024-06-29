use bounce::{use_atom_value, use_selector_value};
use gloo_console::log;
use stylist::{self, style};
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;
use yew_hooks::use_effect_once;

use crate::{
  components::{use_notify, Button, DialogPortal, Input, NoticeTag},
  hook::use_i18n,
  store::{User, Users},
  utils::{get_client, style},
};

#[function_component]
pub fn Register() -> Html {
  let class_name = get_class_name();
  let users = use_selector_value::<Users>();
  let current_user = use_atom_value::<User>();
  let notify = use_notify();
  let value = use_state(|| "");
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

  let confirm = Callback::from(move |_| {
    let is_exist = users.is_exist(*value);
    if is_exist {
      notify(i18n.t("user name exist"), NoticeTag::Warning, None);
      return;
    }
    let name = (*value).to_string();
    if let Some(client) = get_client() {
      spawn_local(async move {
        log!("update name");
        client.update_name(name).await;
      })
    }
  });

  html! {
    <DialogPortal  visible={*visible} onclose={onclose} {header}>
      <div class={class_name}>
        <Input />
        <div class="button">
          <Button>
          { cancel_text}
          </Button>
          <Button onclick={confirm}>
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
