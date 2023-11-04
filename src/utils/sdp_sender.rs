use gloo_console::log;
use wasm_bindgen::JsCast;
use web_sys::{HtmlMediaElement, MediaStream, MediaStreamTrack};

use crate::utils::{get_client, query_selector};

pub async fn call(to: String) {
  if let Some(client) = get_client() {
    client.borrow_mut().call(to).await;
  }
}

pub fn set_dom_stream(selector: &str, stream: Option<&MediaStream>) {
  if let Some(dom) = query_selector::<HtmlMediaElement>(selector) {
    if let Some(stream) = dom.src_object() {
      log!("local dom", &stream);
      stream
        .get_tracks()
        .iter()
        .filter_map(|track| track.dyn_into::<MediaStreamTrack>().ok())
        .for_each(|media_stream_track| {
          stream.remove_track(&media_stream_track);
        });
    }
    dom.set_src_object(stream);
  }
}
