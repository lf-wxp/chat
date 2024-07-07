mod item;

pub use item::*;

use super::get_client;
use futures::Future;
use std::pin::Pin;
use wasm_bindgen_futures::spawn_local;

type Callback = Box<dyn FnOnce(&mut Client) -> Pin<Box<dyn Future<Output = ()> + '_>>>;

pub fn get_client_execute(callback: Callback) {
  spawn_local(async move {
    if let Some(client) = get_client() {
      let fut = callback(client);
      fut.await;
    }
  });
}
