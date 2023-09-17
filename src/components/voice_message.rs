use gloo_console::log;
use js_sys::ArrayBuffer;
use stylist::{self, style};
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;
use yew_icons::{Icon, IconId};

use crate::{
  hook::use_wave_surfer,
  utils::style,
};

#[derive(Properties, PartialEq)]
pub struct Props {
  pub message: ArrayBuffer,
}

#[function_component]
pub fn VoiceMessage(props: &Props) -> Html {
  let class_name = get_class_name();
  let playing = use_state(|| false);
  let duration = use_state(|| 0f64);
  let (wrap_node, start, stop, load) = use_wave_surfer();

  let icon = if *playing {
    IconId::BootstrapPauseFill
  } else {
    IconId::BootstrapPlayFill
  };

  let onclick = {
    Callback::from(move |_| {
      let val = *playing;
      if val {
        stop();
      } else {
        start();
      }
      playing.set(!val);
    })
  };

  let array_buffer = props.message.clone();
  let duration = duration.clone();
  use_effect(move || {
    spawn_local(async move {
      let _ = load(array_buffer).await;
    });
  });

  html! {
    <div class={class_name}>
      <Icon {onclick} icon_id={icon} class="icon" width="16px" height="16px" />
      <div class="wrap" ref={wrap_node}></div>
      <span class="duration">{*duration}</span>
    </div>
  }
}

#[allow(non_upper_case_globals)]
fn get_class_name() -> String {
  style::get_class_name(style!(
    r#"
        display: flex;
        align-items: center;
        position: relative;
        inline-size: 100%;
        block-size: 100%;
        .duration {
          font-size: 12px;
          color: var(--font-color);
        }
        .icon {
          cursor: pointer;
        }
        .wrap {
          flex: 1 1 auto;
          inline-size: 0;
          block-size: 100%;
        }
    "#
  ))
}
