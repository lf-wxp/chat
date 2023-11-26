use std::rc::Rc;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

use crate::utils::get_client;

#[hook]
pub fn use_client() -> Rc<dyn Fn(String)> {
  let call = Rc::new(move |callee: String| {
    spawn_local( async move {
      if let Some(client) = get_client() {
        let _ = client.borrow_mut().connect(callee).await;
      }
    })
  });
  call
}
