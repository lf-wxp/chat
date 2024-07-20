use std::rc::Rc;
use yew::prelude::*;
use yew_hooks::use_effect_once;

use super::{CallbackType, MediaAction, MediaMessage, MediaRequestContext};

#[hook]
pub fn use_media_request() -> Rc<dyn Fn(message::MediaMessage)> {
  let message_ctx = use_context::<MediaRequestContext>();

  Rc::new(move |message: message::MediaMessage| {
    if let Some(item) = &message_ctx {
      let append_item = MediaMessage::from(message);
      let item_clone = item.clone();
      let id = append_item.id.clone();
      let on_timeout = move || {
        item_clone.clone().dispatch(MediaAction::Reject(id));
      };
      let time = gloo_timers::callback::Timeout::new(30 * 1000, on_timeout);
      time.forget();
      item.dispatch(MediaAction::Append(append_item))
    }
  })
}

#[hook]
pub fn use_media_remove() -> Rc<dyn Fn(String)> {
  let message_ctx = use_context::<MediaRequestContext>();
  Rc::new(move |id: String| {
    if let Some(item) = &message_ctx {
      item.dispatch(MediaAction::Remove(id))
    }
  })
}

#[hook]
pub fn use_media_reject() -> Rc<dyn Fn(String)> {
  let message_ctx = use_context::<MediaRequestContext>();
  Rc::new(move |id: String| {
    if let Some(item) = &message_ctx {
      item.dispatch(MediaAction::Reject(id))
    }
  })
}

#[hook]
pub fn use_media_confirm() -> Rc<dyn Fn(String)> {
  let message_ctx = use_context::<MediaRequestContext>();
  Rc::new(move |id: String| {
    if let Some(item) = &message_ctx {
      item.dispatch(MediaAction::Confirm(id))
    }
  })
}

#[hook]
pub fn use_register_callback(callback: fn(MediaMessage, CallbackType)) {
  let message_ctx = use_context::<MediaRequestContext>();
  use_effect_once(move || {
    if let Some(item) = &message_ctx {
      item.dispatch(MediaAction::Callback(callback));
    }
    || {}
  });
}
