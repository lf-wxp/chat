use crate::{model::CallType, utils::get_client};

pub fn call(to: String, call_type: CallType) {
  if let Some(client) = get_client() {
    client.borrow_mut().call(to, call_type);
  }
}
