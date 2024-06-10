use nanoid::nanoid;
use std::rc::Rc;
use yew::prelude::*;

use super::DialogComponent;

pub enum DialogAction {
  Append(Dialog),
  Remove(String),
  PreRemove(String),
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
pub struct DialogList(pub Vec<Dialog>);

impl Reducible for DialogList {
  type Action = DialogAction;

  fn reduce(self: Rc<Self>, action: Self::Action) -> Rc<Self> {
    match action {
      DialogAction::Append(dialog) => {
        let mut dialogs = self.0.clone();
        dialogs.push(dialog);
        Rc::new(DialogList(dialogs))
      }
      DialogAction::Remove(id) => {
        let idx = self
          .0
          .iter()
          .position(|x| x.id == id)
          .unwrap_or(usize::MAX);
        let mut dialogs = self.0.clone();
        dialogs.remove(idx);
        Rc::new(DialogList(dialogs))
      }
      DialogAction::PreRemove(id) => {
        let idx = self
          .0
          .iter()
          .position(|x| x.id == id)
          .unwrap_or(usize::MAX);
        let mut dialogs = self.0.clone();
        if let Some(dialog) = dialogs.get_mut(idx) {
          dialog.state = DialogState::Perish;
        }
        Rc::new(DialogList(dialogs))
      }
    }
  }
}

pub type DialogContext = UseReducerHandle<DialogList>;

#[derive(Properties, Debug, PartialEq)]
pub struct DialogProviderProps {
  #[prop_or_default]
  pub children: Children,
}

#[function_component]
pub fn DialogProvider(props: &DialogProviderProps) -> Html {
  let dialog = use_reducer(DialogList::default);

  html! {
    <ContextProvider<DialogContext> context={dialog}>
      {props.children.clone()}
      <DialogComponent />
    </ContextProvider<DialogContext>>
  }
}
