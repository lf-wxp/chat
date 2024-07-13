use message::MediaType;
use nanoid::nanoid;
use std::rc::Rc;
use yew::prelude::*;

use super::MediaRequest;

pub enum MediaAction {
  Append(MediaMessage),
  Reject(String),
  Remove(String),
  Confirm(String),
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum MediaState {
  Exist,
  Perish,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct MediaMessage {
  pub id: String,
  pub from: String,
  pub from_name: String,
  pub to: String,
  pub media_type: MediaType,
  pub expired: Option<String>,
  pub confirm: Option<bool>,
  pub state: MediaState,
}

impl From<message::MediaMessage> for MediaMessage {
  fn from(value: message::MediaMessage) -> Self {
    let message::MediaMessage {
      from,
      from_name,
      to,
      media_type,
      expired,
      confirm,
    } = value;
    let id = nanoid!();
    Self {
      id,
      state: MediaState::Exist,
      from,
      to,
      from_name,
      media_type,
      expired,
      confirm,
    }
  }
}

#[derive(PartialEq, Default, Clone, Eq)]
pub struct MediaRequestList(pub Vec<MediaMessage>);

impl Reducible for MediaRequestList {
  type Action = MediaAction;

  fn reduce(self: Rc<Self>, action: Self::Action) -> Rc<Self> {
    match action {
      MediaAction::Append(notice) => {
        let mut message = self.0.clone();
        message.push(notice);
        Rc::new(MediaRequestList(message))
      }
      MediaAction::Remove(id) => {
        let idx = self.0.iter().position(|x| x.id == id).unwrap_or(usize::MAX);
        let mut message = self.0.clone();
        message.remove(idx);
        Rc::new(MediaRequestList(message))
      }
      MediaAction::Confirm(id) | MediaAction::Reject(id) => {
        let idx = self.0.iter().position(|x| x.id == id).unwrap_or(usize::MAX);
        let mut message = self.0.clone();
        if let Some(notice) = message.get_mut(idx) {
          notice.state = MediaState::Perish;
        }
        Rc::new(MediaRequestList(message))
      }
    }
  }
}

pub type MediaRequestContext = UseReducerHandle<MediaRequestList>;

#[derive(Properties, Debug, PartialEq)]
pub struct MediaRequestProviderProps {
  #[prop_or_default]
  pub children: Children,
}

#[function_component]
pub fn MediaRequestProvider(props: &MediaRequestProviderProps) -> Html {
  let notice = use_reducer(MediaRequestList::default);

  html! {
    <ContextProvider<MediaRequestContext> context={notice}>
      {props.children.clone()}
      <MediaRequest />
    </ContextProvider<MediaRequestContext>>
  }
}
