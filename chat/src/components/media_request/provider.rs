use message::MediaType;
use nanoid::nanoid;
use std::rc::Rc;
use yew::prelude::*;

use super::MediaRequest;

#[derive(Debug, Clone)]
pub enum CallbackType {
  Confirm,
  Reject,
}
pub enum MediaAction {
  Append(MediaMessage),
  Reject(String),
  Remove(String),
  Confirm(String),
  Callback(fn(MediaMessage, CallbackType)),
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
  pub confirm: Option<bool>,
  pub state: MediaState,
}

impl From<MediaMessage> for message::MediaMessage {
  fn from(value: MediaMessage) -> Self {
    let MediaMessage {
      from,
      from_name,
      to,
      media_type,
      confirm,
      ..
    } = value;
    Self {
      from,
      from_name,
      to,
      media_type,
      confirm,
    }
  }
}

impl From<message::MediaMessage> for MediaMessage {
  fn from(value: message::MediaMessage) -> Self {
    let message::MediaMessage {
      from,
      from_name,
      to,
      media_type,
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
      confirm,
    }
  }
}
#[derive(Default, Clone, PartialEq)]
pub struct MediaRequestProps {
  pub list: Vec<MediaMessage>,
  pub callback: Vec<fn(MediaMessage, CallbackType)>,
}

impl Reducible for MediaRequestProps {
  type Action = MediaAction;

  fn reduce(self: Rc<Self>, action: Self::Action) -> Rc<Self> {
    match action {
      MediaAction::Append(notice) => {
        let mut message = self.list.clone();
        message.push(notice);
        Rc::new(MediaRequestProps {
          list: message,
          ..(*self).clone()
        })
      }
      MediaAction::Remove(id) => {
        let idx = self
          .list
          .iter()
          .position(|x| x.id == id)
          .unwrap_or(usize::MAX);
        let mut message = self.list.clone();
        message.remove(idx);
        Rc::new(MediaRequestProps {
          list: message,
          ..(*self).clone()
        })
      }
      MediaAction::Confirm(id) | MediaAction::Reject(id) => {
        let idx = self
          .list
          .iter()
          .position(|x| x.id == id)
          .unwrap_or(usize::MAX);
        let mut message = self.list.clone();
        if let Some(notice) = message.get_mut(idx) {
          notice.state = MediaState::Perish;
        }
        Rc::new(MediaRequestProps {
          list: message,
          ..(*self).clone()
        })
      }
      MediaAction::Callback(function) => {
        let mut callback = self.callback.clone();
        callback.push(function);
        Rc::new(MediaRequestProps {
          callback,
          ..(*self).clone()
        })
      }
    }
  }
}

pub type MediaRequestContext = UseReducerHandle<MediaRequestProps>;

#[derive(Properties, Debug, PartialEq)]
pub struct MediaRequestProviderProps {
  #[prop_or_default]
  pub children: Children,
}

#[function_component]
pub fn MediaRequestProvider(props: &MediaRequestProviderProps) -> Html {
  let notice = use_reducer(MediaRequestProps::default);

  html! {
    <ContextProvider<MediaRequestContext> context={notice}>
      {props.children.clone()}
      <MediaRequest />
    </ContextProvider<MediaRequestContext>>
  }
}
