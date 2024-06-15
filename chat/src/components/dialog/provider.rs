use nanoid::nanoid;
use std::rc::Rc;
use yew::prelude::*;

use super::DialogComponent;

pub enum DialogAction {
  Append(Dialog),
  Remove(String),
  PreRemove(String),
  Visible(bool),
  ClassVisible(bool),
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum DialogState {
  Exist,
  Perish,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Dialog {
  pub id: String,
  pub content: String,
  pub header: String,
  pub state: DialogState,
}

impl Dialog {
  pub fn new(content: String, header: String) -> Dialog {
    let id = nanoid!();
    Dialog {
      id,
      content,
      header,
      state: DialogState::Exist,
    }
  }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DialogContext {
  pub list: Vec<Dialog>,
  pub visible: bool,
  pub class_visible: bool,
}

impl Reducible for DialogContext {
  type Action = DialogAction;

  fn reduce(self: Rc<Self>, action: Self::Action) -> Rc<Self> {
    match action {
      DialogAction::Append(dialog) => {
        let mut list = self.list.clone();
        list.push(dialog);
        Rc::new(DialogContext { list, ..(*self) })
      }
      DialogAction::Remove(id) => {
        if let Some(idx) = self.list.iter().position(|x| x.id == id) {
          let mut list = self.list.clone();
          list.remove(idx);
          return Rc::new(DialogContext { list, ..(*self) });
        }
        self
      }
      DialogAction::PreRemove(id) => {
        if let Some(idx) = self.list.iter().position(|x| x.id == id) {
          let mut list = self.list.clone();
          if let Some(dialog) = list.get_mut(idx) {
            dialog.state = DialogState::Perish;
          }
          return Rc::new(DialogContext { list, ..(*self) });
        }
        self
      }
      DialogAction::Visible(visible) => Rc::new(DialogContext {
        visible,
        ..(*self).clone()
      }),
      DialogAction::ClassVisible(visible) => Rc::new(DialogContext {
        class_visible: visible,
        ..(*self).clone()
      }),
    }
  }
}

#[derive(Properties, Debug, PartialEq)]
pub struct DialogProviderProps {
  #[prop_or_default]
  pub children: Children,
}

#[function_component]
pub fn DialogProvider(props: &DialogProviderProps) -> Html {
  let dialog = use_reducer(DialogContext::default);

  html! {
    <ContextProvider<UseReducerHandle<DialogContext>> context={dialog}>
      {props.children.clone()}
      <DialogComponent />
    </ContextProvider<UseReducerHandle<DialogContext>>>
  }
}
