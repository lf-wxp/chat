use bounce::{use_atom, use_selector_value};
use gloo_console::log;
use message::{ActionMessage, ResponseMessageData::Action};
use stylist::{self, style};
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;
use yew_hooks::use_effect_once;

use crate::{
  components::{use_notify, Button, DialogPortal, Input, NoticeTag},
  hook::use_i18n,
  store::{User, Users},
  utils::{get_client, get_client_execute, style},
};

#[function_component]
pub fn Register() -> Html {
  let class_name = get_class_name();
  let users = use_selector_value::<Users>();
  let user = use_atom::<User>();
  let notify = use_notify();
  let value = use_state(|| "".to_string());
  let visible = use_state(|| false);
  let i18n = use_i18n();
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

  let visible_clone = visible.clone();
  let value_clone = value.clone();
  let confirm = use_callback(value.clone(), move |_, value| {
    let is_exist = users.is_exist(value);
    if is_exist {
      notify(i18n.t("user name exist"), NoticeTag::Warning, None);
      return;
    }
    let name = (*value).to_string();
    let user = user.clone();
    let visible_clone = visible_clone.clone();
    let value_clone = value_clone.clone();
    get_client_execute(Box::new(|client| {
      Box::pin(async move {
        if let Action(ActionMessage::Success) = client.update_name(name.clone()).await {
          client.set_name(name.clone());
          user.set(User {
            name,
            uuid: user.uuid.clone(),
          });
          value_clone.set("".to_string());
          visible_clone.set(false);
          log!("update user name");
        }
      })
    }));
  });

  let value_clone = value.clone();
  let onchange = Callback::from(move |val: String| {
    value_clone.set(val);
  });

  html! {
    <DialogPortal  visible={*visible} onclose={onclose} {header}>
      <div class={class_name}>
        <Input value={(*value).clone()} {onchange} />
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
