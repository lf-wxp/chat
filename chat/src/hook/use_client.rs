use yew::prelude::*;

use crate::utils::get_client;

#[hook]
pub fn use_client() -> Box<dyn Fn(String)> {
  let call = Box::new(|callee: String| {
    if let Some(client) = get_client() {
      client.borrow_mut().call(callee);
    }
  });
  call
}
