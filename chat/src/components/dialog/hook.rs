use std::rc::Rc;
use yew::prelude::*;

use super::{Dialog, DialogAction, DialogContext };

#[hook]
pub fn use_dialog() -> Rc<dyn Fn(String, String)> {
  let dialog_list = use_context::<DialogContext>();
  Rc::new(
    move |header: String, content: String| {
      if let Some(notice) = &dialog_list {
        let append_notice = Dialog::new(content, header);
        notice.dispatch(DialogAction::Append(append_notice))
      }
    },
  )
}

#[hook]
pub fn use_remove_dialog() -> Rc<dyn Fn(String)> {
  let dialog_list = use_context::<DialogContext>();
  Rc::new(
    move |id: String| {
      if let Some(notice) = &dialog_list {
        notice.dispatch(DialogAction::Remove(id))
      }
    },
  )
}

#[hook]
pub fn use_pre_remove_dialog() -> Rc<dyn Fn(String)> {
  let dialog_list = use_context::<DialogContext>();
  Rc::new(
    move |id: String| {
      if let Some(notice) = &dialog_list {
        notice.dispatch(DialogAction::PreRemove(id))
      }
    },
  )
}
