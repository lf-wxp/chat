use bounce::use_atom_value;
use gloo_console::log;
use js_sys::JsString;
use stylist::{self, style};
use wasm_bindgen::JsCast;
use web_sys::HtmlInputElement;
use yew::prelude::*;
use yew_icons::{Icon, IconId};

use crate::{
  hook::use_chat,
  model::{ChatMessage, Message, MessageBinary},
  store::User,
  utils::{get_target, style, Websocket, SocketMessage},
};

#[function_component]
pub fn WaveTest() -> Html {
  let class_name = get_class_name();
  let input_node_ref = use_node_ref();
  let user_name = use_atom_value::<User>();
  let (add_message, _update_message_state) = use_chat();
  let onclick = {
    let input_node_ref = input_node_ref.clone();
    Callback::from(move |_| {
      if let Some(input) = input_node_ref
        .get()
        .and_then(|input| input.dyn_into::<HtmlInputElement>().ok())
      {
        wasm_bindgen_futures::spawn_local(async move { input.click() });
      }
    })
  };
  let onchange = Callback::from(move |e: Event| {
    let target = get_target::<Event, HtmlInputElement>(e);
    let add = add_message.clone();
    let user_name = user_name.clone();
    if let Some(target) = target {
      if let Some(file) = target.files().and_then(|x| x.get(0)) {
        wasm_bindgen_futures::spawn_local(async move {
          add(ChatMessage::new(
            user_name.name.clone(),
            Message::Audio(MessageBinary::File(file)),
          ));
        });
      }
    }
  });
  use_effect_with((), move |_| {
    let ws_client = Websocket::new("ws://127.0.0.1:8888").unwrap();
    let mut client = ws_client.borrow_mut();
    client.set_onopen(Box::new(move || {
      log!("websocket start");
    }));
    client.set_onmessage(Box::new(|msg: SocketMessage| {
      log!("receive message", format!("{:?}", msg));
    }));
    client.send(SocketMessage::Str(JsString::from("hello"))).unwrap();
  });

  html! {
    <div class={class_name}>
      <input type="file" accept="*/*" class="fake-input" ref={input_node_ref} {onchange} />
      <Icon {onclick} icon_id={IconId::FontAwesomeRegularImages} class="icon" width="16px" height="16px" />
    </div>
  }
}

#[allow(non_upper_case_globals)]
fn get_class_name() -> String {
  style::get_class_name(style!(
    r#"
      inline-size: 300px;
      block-size: 50px;
      position: relative;
      margin-inline-start: 10px;
      .canvas_container {
        block-size: 100%;
      }
      .fake-input {
        display: none;
      }
      .icon {
        cursor: pointer;
        color: #8896a4;
        transition: all 0.2s ease;
      }
      .icon:hover {
        color: #51b66d;
      }
    "#
  ))
}
