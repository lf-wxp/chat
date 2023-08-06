use nanoid::nanoid;
use std::rc::Rc;
use yew::prelude::*;

use super::Notify;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum NoticeTag {
  Info,
  Success,
  Warning,
  Danger,
}

pub enum NoticeAction {
  Append(Notice),
  Remove(String),
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Notice {
  pub id: String,
  pub content: String,
  pub tag: NoticeTag,
  pub duration: Option<u32>,
}

impl Notice {
  pub fn new(content: String, tag: NoticeTag, duration: Option<u32>) -> Notice {
    let id = nanoid!();
    Notice {
      id,
      content,
      tag,
      duration,
    }
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoticeList(pub Vec<Notice>);

impl Reducible for NoticeList {
  type Action = NoticeAction;

  fn reduce(self: Rc<Self>, action: Self::Action) -> Rc<Self> {
    match action {
      NoticeAction::Append(notice) => {
        let mut notices = self.0.clone();
        notices.push(notice);
        Rc::new(NoticeList(notices))
      }
      NoticeAction::Remove(id) => {
        let idx = self
          .0
          .iter()
          .position(|x| x.id == id)
          .unwrap_or(usize::MAX);
        let mut notices = self.0.clone();
        notices.remove(idx);
        Rc::new(NoticeList(notices))
      }
    }
  }
}

impl Default for NoticeList {
  fn default() -> Self {
    NoticeList(vec![])
  }
}

pub type NoticeContext = UseReducerHandle<NoticeList>;

#[derive(Properties, Debug, PartialEq)]
pub struct NotifyProviderProps {
  #[prop_or_default]
  pub children: Children,
}

#[function_component]
pub fn NotifyProvider(props: &NotifyProviderProps) -> Html {
  let notice = use_reducer(|| NoticeList::default());

  html! {
    <ContextProvider<NoticeContext> context={notice}>
      {props.children.clone()}
      <Notify />
    </ContextProvider<NoticeContext>>
  }
}
