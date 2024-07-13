use std::rc::Rc;
use yew::prelude::*;

use super::{MediaAction, MediaMessage, MediaRequestContext};

#[hook]
pub fn use_media_request() -> Rc<dyn Fn(message::MediaMessage)> {
  let message_list = use_context::<MediaRequestContext>();

  Rc::new(move |message: message::MediaMessage| {
    if let Some(item) = &message_list {
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
  let message_list = use_context::<MediaRequestContext>();
  Rc::new(move |id: String| {
    if let Some(item) = &message_list {
      item.dispatch(MediaAction::Remove(id))
    }
  })
}

#[hook]
pub fn use_media_reject() -> Rc<dyn Fn(String)> {
  let message_list = use_context::<MediaRequestContext>();
  Rc::new(move |id: String| {
    if let Some(item) = &message_list {
      item.dispatch(MediaAction::Reject(id))
    }
  })
}

#[hook]
pub fn use_media_confirm() -> Rc<dyn Fn(String)> {
  let message_list = use_context::<MediaRequestContext>();
  Rc::new(move |id: String| {
    if let Some(item) = &message_list {
      item.dispatch(MediaAction::Confirm(id))
    }
  })
}
