use std::rc::Rc;
use yew::prelude::*;

use super::{NoticeAction, NoticeContext, NoticeTag};

#[hook]
pub fn use_notify() -> Rc<dyn Fn(String, NoticeTag, Option<u32>)> {
  let notice_list = use_context::<NoticeContext>();

  Rc::new(
    move |content: String, tag: NoticeTag, duration: Option<u32>| {
      if let Some(notice) = &notice_list {
        let append_notice = crate::components::Notice::new(content, tag, duration);
        if let Some(duration) = duration {
          let notice = notice.clone();
          let append_notice = append_notice.clone();
          let on_timeout = move || {
            notice.clone().dispatch(NoticeAction::Remove(append_notice.id));
          };
          let time = gloo_timers::callback::Timeout::new(duration * 1000, on_timeout);
          time.forget();
        }
        notice.dispatch(NoticeAction::Append(append_notice))
      }
    },
  )
}
