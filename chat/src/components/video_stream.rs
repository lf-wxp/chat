use gloo_console::log;
use stylist::{self, style};
use yew::prelude::*;

use crate::{
  components::{use_register_callback, CallbackType},
  utils::style,
};

#[function_component]
pub fn VideoStream() -> Html {
  let class_name = get_class_name();
  use_register_callback(|message, callback_type| {
    log!("media replay message", format!("{:?}", message));
    match callback_type {
      CallbackType::Confirm => todo!(),
      CallbackType::Reject => todo!(),
    };
  });

  html! {
    <>
      <div class={class_name}>
        <video class="local-stream" autoplay={true} />
        <video class="remote-stream" autoplay={true} />
      </div>
    </>
  }
}

#[allow(non_upper_case_globals)]
fn get_class_name() -> String {
  style::get_class_name(style!(
    r#"
      background: var(--theme-ancillary-color);
      border-radius: var(--radius);
      overflow: hidden;
      position: relative;
      video {
        border-radius: var(--radius);
        aspect-ratio: 3 / 2;
        object-fit: cover;
      }
      .local-stream {
        inline-size: 100%;
        block-size: 100%;
      }
      .remote-stream {
        position: absolute; 
        inset-inline-end: 0;
        inset-block-start: 0;
        inline-size: 40%;
      }
    "#
  ))
}
