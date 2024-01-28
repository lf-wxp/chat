use std::rc::Rc;
use gloo_console::log;
use message::MediaType;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

use crate::utils::get_client;

#[hook]
pub fn use_client() -> Rc<dyn Fn(String)> {
  let call = Rc::new(move |callee: String| {
    spawn_local( async move {
      // if let Some(client) = get_client() {
        // let msg = client.request_media(callee, message::MediaType::Video).await;
        // log!("request_media response msg is", format!("{:?}", msg));
        // let _ = client.borrow_mut().request_connect(callee).await;
      // }

      let message = message::RequestMessageData::Media(message::MediaMessage {
        from: "1".to_string(),
        to: "2".to_string(),
        media_type: MediaType::Audio,
        expired: None,
        confirm: None,
      });
      let (sender, receiver) = postage::broadcast::channel::<String>(10);
      let mut request = crate::utils::Request::new(sender, receiver);
      let msg = request.request(message).await;
      log!("request_media response msg is", format!("{:?}", msg));
    })
  });
  call
}
