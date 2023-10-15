use std::{cell::RefCell, rc::Rc};

use stylist::{self, style};
use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlMediaElement;
use yew::prelude::*;

use crate::utils::{style, WebRTC};

#[function_component]
pub fn VideoStream() -> Html {
  let class_name = get_class_name();
  let video_node_ref = use_node_ref();
  let webrtc: Rc<RefCell<Option<WebRTC>>> = use_mut_ref(Default::default);

  let video_node_clone = video_node_ref.clone();
  let webrtc_clone = webrtc.clone();
  let start_stream = {
    Callback::from(move |_| {
      let webrtc_clone = webrtc_clone.clone();
      if let Some(dom) = video_node_clone.cast::<HtmlMediaElement>() {
        let webrtc_clone = webrtc_clone.clone();
        spawn_local(async move {
          *webrtc_clone.borrow_mut() = WebRTC::new(dom).await.ok();
        })
      }
    })
  };

  html! {
    <>
      <div class={class_name}>
        <video ref={video_node_ref} autoplay={true} />
      </div>
      <button onclick={start_stream}>{{"stream"}}</button>
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
      video {
        border-radius: var(--radius);
        inline-size: 100%;
        block-size: 100%;
        aspect-ratio: 3 / 2;
        object-fit: cover;
      }
    "#
  ))
}
