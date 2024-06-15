use std::rc::Rc;
use yew::prelude::*;

use super::{Dialog, DialogAction, DialogContext};

#[hook]
pub fn use_dialog() -> Rc<dyn Fn(String, String)> {
  let dialog_context = use_context::<UseReducerHandle<DialogContext>>();
  Rc::new(move |header: String, content: String| {
    if let Some(dialog) = &dialog_context {
      let append_dialog = Dialog::new(content, header);
      dialog.dispatch(DialogAction::Append(append_dialog))
    }
  })
}

#[hook]
pub fn use_remove_dialog() -> Rc<dyn Fn(String)> {
  let dialog_context = use_context::<UseReducerHandle<DialogContext>>();
  Rc::new(move |id: String| {
    if let Some(dialog) = &dialog_context {
      dialog.dispatch(DialogAction::Remove(id))
    }
  })
}

#[hook]
pub fn use_pre_remove_dialog() -> Rc<dyn Fn(String)> {
  let dialog_context = use_context::<UseReducerHandle<DialogContext>>();
  Rc::new(move |id: String| {
    if let Some(dialog) = &dialog_context {
      dialog.dispatch(DialogAction::PreRemove(id))
    }
  })
}
