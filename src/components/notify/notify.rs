use stylist::{self, style};
use yew::prelude::*;

use super::NoticeContext;
use crate::{
  components::{use_remove_notify, Notice, NoticeState},
  utils::style,
};

#[function_component]
pub fn Notify() -> Html {
  let class_name = get_class_name();
  let notice_list = use_context::<NoticeContext>().map_or(vec![], |x| x.0.clone());
  let notify_remove = use_remove_notify();

  let class_fn = |item: Notice| {
    let mut extra = "";
    if item.state == NoticeState::Perish {
      extra = "perish";
    }
    format!("notice {extra}")
  };

  let remove_fn = Callback::from(move |notice: Notice| {
    if notice.state == NoticeState::Perish {
      notify_remove(notice.id);
    }
  });

  html! {
    if notice_list.iter().len() > 0 {
      <div class={class_name} >
        { for notice_list.iter().map(|item|{
          let notice = item.clone();
          html!{
            <div
              key={item.id.clone()}
              class={class_fn(item.clone())}
              onanimationend={remove_fn.reform(move |_| notice.clone())}
            >
              {item.content.clone()}
            </div>
        }})}
      </div>
    }
  }
}

#[allow(non_upper_case_globals)]
fn get_class_name() -> String {
  style::get_class_name(style!(
    r#"
        position: fixed;
        inset-block-start: 20px;
        inset-inline: 0;
        
        .notice {
          padding: 5px 10px;
          margin: auto;
          color: var(--font-color);
          background: rgba(var(--theme-color-rgb), 0.8);
          border-radius: calc(var(--radius) / 3);
          backdrop-filter: blur(10px);
          inline-size: fit-content;
          min-inline-size: 100px;
          text-align: center;
          margin-block-end: 10px;
          transition: all 0.2s ease;
          animation: fadeIn 0.2s;
        }
        .perish {
          animation: fadeOut 0.2s;
        }

        @keyframes fadeIn {
          0% {
            opacity: 0;
            transform: translateY(-50%);
          } 
          100% {
            opacity: 1;
            transform: translateY(0);
          } 
        }
        @keyframes fadeOut {
          0% {
            opacity: 1;
            transform: translateY(0);
            margin-block-end: 10px;
          } 
          100% {
            opacity: 0;
            transform: translateY(-50%);
            margin-block-end: -28px;
          } 
        }
    "#
  ))
}
